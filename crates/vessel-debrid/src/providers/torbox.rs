//! TorBox REST API v1 provider.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use vessel_domain::{
    DebridAccountStatus, DebridCloudItem, DebridCloudStatus, DebridProvider, DebridProviderKind,
    DebridResolveOutcome, ResolvedHttpLink, SecretStore, TorrentTarget, VesselError, VesselResult,
};

use crate::episode_matcher::{TorrentFileCandidate, select_best_file};
use crate::provider::{encode_form, map_http_err, map_status_err, parse_json_response, require_secret};

pub const TORBOX_SECRET_KEY: &str = "debrid.torbox.api_key";

#[derive(Clone)]
pub struct TorBoxProvider {
    client: Client,
    secrets: Arc<dyn SecretStore>,
    api_base: String,
}

impl TorBoxProvider {
    pub fn new(client: Client, secrets: Arc<dyn SecretStore>) -> Self {
        Self::with_base(client, secrets, "https://api.torbox.app/v1/api")
    }

    pub fn with_base(
        client: Client,
        secrets: Arc<dyn SecretStore>,
        api_base: impl Into<String>,
    ) -> Self {
        Self {
            client,
            secrets,
            api_base: api_base.into().trim_end_matches('/').to_owned(),
        }
    }

    async fn token(&self) -> VesselResult<String> {
        require_secret(&self.secrets, TORBOX_SECRET_KEY, "TorBox").await
    }

    async fn request_dl(&self, token: &str, torrent_id: u64, file_id: Option<u64>) -> VesselResult<String> {
        let mut url = format!(
            "{}/torrents/requestdl?token={}&torrent_id={torrent_id}&zip_link=false",
            self.api_base,
            url::form_urlencoded::byte_serialize(token.as_bytes()).collect::<String>()
        );
        if let Some(fid) = file_id {
            url.push_str(&format!("&file_id={fid}"));
        }
        let resp = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| map_http_err("TorBox", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("TorBox", status, &body));
        }

        #[derive(Deserialize)]
        struct DlResp {
            data: Option<String>,
        }
        let parsed: DlResp = parse_json_response("TorBox", "requestdl", &body)?;
        parsed
            .data
            .filter(|s| !s.trim().is_empty())
            .ok_or_else(|| VesselError::Network("TorBox requestdl returned empty URL".into()))
    }
}

#[derive(Debug, Deserialize)]
struct TbFile {
    id: u64,
    name: String,
    #[serde(default)]
    short_name: Option<String>,
    #[serde(default)]
    size: u64,
}

#[derive(Debug, Deserialize)]
struct TbTorrent {
    id: u64,
    #[serde(default)]
    hash: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    progress: f32,
    #[serde(default)]
    download_speed: Option<u64>,
    #[serde(default)]
    seeds: Option<u32>,
    #[serde(default)]
    eta: Option<u64>,
    #[serde(default)]
    download_present: bool,
    #[serde(default)]
    download_finished: bool,
    #[serde(default)]
    download_state: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    files: Vec<TbFile>,
}

fn map_tb_status(t: &TbTorrent) -> DebridCloudStatus {
    if t.download_present || t.download_finished {
        return DebridCloudStatus::Ready;
    }
    match t
        .download_state
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "downloading" | "metadl" => DebridCloudStatus::Downloading,
        "uploading" | "seeding" | "cached" => DebridCloudStatus::Uploading,
        "queued" | "checking" => DebridCloudStatus::Queued,
        "error" | "stalled (no seeds)" => DebridCloudStatus::Error,
        _ => DebridCloudStatus::Queued,
    }
}

#[async_trait]
impl DebridProvider for TorBoxProvider {
    fn target_id(&self) -> &str {
        DebridProviderKind::TorBox.target_id()
    }

    fn display_name(&self) -> &str {
        DebridProviderKind::TorBox.display_name()
    }

    fn is_builtin(&self) -> bool {
        true
    }

    fn supports_bulk_cache_check(&self) -> bool {
        true
    }

    async fn account_status(&self) -> VesselResult<DebridAccountStatus> {
        let token = match self.token().await {
            Ok(t) => t,
            Err(_) => {
                return Ok(DebridAccountStatus {
                    target: self.target_id().into(),
                    label: self.display_name().into(),
                    is_builtin: true,
                    supports_bulk_cache_check: true,
                    enabled: false,
                    authenticated: false,
                    username: None,
                    email: None,
                    is_premium: false,
                    expires_at: None,
                    points_or_quota: None,
                    error: None,
                });
            }
        };

        let resp = self
            .client
            .get(format!("{}/user/me", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("TorBox", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Ok(DebridAccountStatus {
                target: self.target_id().into(),
                label: self.display_name().into(),
                is_builtin: true,
                supports_bulk_cache_check: true,
                enabled: true,
                authenticated: false,
                username: None,
                email: None,
                is_premium: false,
                expires_at: None,
                points_or_quota: None,
                error: Some(map_status_err("TorBox", status, &body).to_string()),
            });
        }

        #[derive(Deserialize)]
        struct TbUserData {
            email: Option<String>,
            plan: Option<u32>,
            premium_expires_at: Option<String>,
        }
        #[derive(Deserialize)]
        struct TbUserResp {
            data: Option<TbUserData>,
        }
        let parsed: TbUserResp = parse_json_response("TorBox", "user/me", &body)?;
        let data = parsed.data;
        let plan = data.as_ref().and_then(|d| d.plan).unwrap_or(0);

        Ok(DebridAccountStatus {
            target: self.target_id().into(),
            label: self.display_name().into(),
            is_builtin: true,
            supports_bulk_cache_check: true,
            enabled: true,
            authenticated: true,
            username: data.as_ref().and_then(|d| d.email.clone()),
            email: data.as_ref().and_then(|d| d.email.clone()),
            is_premium: plan > 0,
            expires_at: data.as_ref().and_then(|d| d.premium_expires_at.clone()),
            points_or_quota: Some(format!("Plan {plan}")),
            error: None,
        })
    }

    async fn check_cached(&self, hashes: &[String]) -> VesselResult<HashMap<String, bool>> {
        let mut result = HashMap::new();
        if hashes.is_empty() {
            return Ok(result);
        }
        let token = self.token().await?;

        for chunk in hashes.chunks(50) {
            let joined = chunk.join(",");
            let url = format!("{}/torrents/checkcached?hash={joined}&format=list", self.api_base);
            let resp = self
                .client
                .get(&url)
                .bearer_auth(&token)
                .send()
                .await
                .map_err(|e| map_http_err("TorBox", e))?;
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(map_status_err("TorBox", status, &body));
            }

            #[derive(Deserialize)]
            struct CachedItem {
                hash: String,
            }
            #[derive(Deserialize)]
            struct CacheResp {
                data: Option<Vec<CachedItem>>,
            }
            let parsed: CacheResp = parse_json_response("TorBox", "checkcached", &body)?;
            for h in chunk {
                result.insert(h.to_ascii_lowercase(), false);
            }
            if let Some(items) = parsed.data {
                for item in items {
                    result.insert(item.hash.to_ascii_lowercase(), true);
                }
            }
        }

        Ok(result)
    }

    async fn resolve_magnet(
        &self,
        magnet_or_hash: &str,
        target: &TorrentTarget,
        allow_uncached_queue: bool,
    ) -> VesselResult<DebridResolveOutcome> {
        let token = self.token().await?;
        let create_resp = self
            .client
            .post(format!("{}/torrents/createtorrent", self.api_base))
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encode_form(&[("magnet", magnet_or_hash)]))
            .send()
            .await
            .map_err(|e| map_http_err("TorBox", e))?;
        let status = create_resp.status();
        let body = create_resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("TorBox", status, &body));
        }

        #[derive(Deserialize)]
        struct CreateData {
            torrent_id: u64,
        }
        #[derive(Deserialize)]
        struct CreateResp {
            data: Option<CreateData>,
        }
        let created: CreateResp = parse_json_response("TorBox", "createtorrent", &body)?;
        let torrent_id = created
            .data
            .map(|d| d.torrent_id)
            .ok_or_else(|| VesselError::Network("TorBox createtorrent missing torrent_id".into()))?;

        let list_resp = self
            .client
            .get(format!("{}/torrents/mylist?id={torrent_id}", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("TorBox", e))?;
        let list_status = list_resp.status();
        let list_body = list_resp.text().await.unwrap_or_default();
        if !list_status.is_success() {
            return Err(map_status_err("TorBox", list_status, &list_body));
        }

        #[derive(Deserialize)]
        struct MyListResp {
            data: Option<TbTorrent>,
        }
        let parsed: MyListResp = parse_json_response("TorBox", "mylist", &list_body)?;
        let torrent = parsed
            .data
            .ok_or_else(|| VesselError::NotFound(format!("TorBox torrent #{torrent_id} not found")))?;

        if torrent.download_present || torrent.download_finished {
            let candidates: Vec<TorrentFileCandidate> = torrent
                .files
                .iter()
                .enumerate()
                .map(|(index, f)| TorrentFileCandidate {
                    id: f.id.to_string(),
                    index,
                    path: f.short_name.clone().unwrap_or_else(|| f.name.clone()),
                    size_bytes: f.size,
                })
                .collect();
            let picked = select_best_file(&candidates, target);
            let file_id = picked.and_then(|c| c.id.parse::<u64>().ok());
            let dl_url = self.request_dl(&token, torrent.id, file_id).await?;
            return Ok(DebridResolveOutcome::Ready(ResolvedHttpLink {
                url: dl_url,
                headers: Default::default(),
                filename: picked.map(|p| p.path.clone()).or(Some(torrent.name)),
                size_bytes: picked.map(|p| p.size_bytes).or(torrent.size),
                expires_at: None,
            }));
        }

        let cloud_status = map_tb_status(&torrent);
        if !allow_uncached_queue {
            let _ = self.delete_cloud(&torrent_id.to_string()).await;
            return Ok(DebridResolveOutcome::PendingCloud {
                torrent_id: None,
                status: cloud_status,
                progress_pct: torrent.progress * 100.0,
                speed_bps: torrent.download_speed,
                seeders: torrent.seeds,
            });
        }

        Ok(DebridResolveOutcome::PendingCloud {
            torrent_id: Some(torrent_id.to_string()),
            status: cloud_status,
            progress_pct: torrent.progress * 100.0,
            speed_bps: torrent.download_speed,
            seeders: torrent.seeds,
        })
    }

    async fn list_cloud(&self) -> VesselResult<Vec<DebridCloudItem>> {
        let token = self.token().await?;
        let resp = self
            .client
            .get(format!("{}/torrents/mylist", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("TorBox", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("TorBox", status, &body));
        }

        #[derive(Deserialize)]
        struct ListResp {
            data: Option<Vec<TbTorrent>>,
        }
        let parsed: ListResp = parse_json_response("TorBox", "mylist", &body)?;

        Ok(parsed
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|t| {
                let st = map_tb_status(&t);
                let pct = if t.progress <= 1.0 {
                    t.progress * 100.0
                } else {
                    t.progress
                };
                let downloaded_bytes = t
                    .size
                    .map(|total| ((total as f64) * (pct.clamp(0.0, 100.0) as f64 / 100.0)) as u64);
                let links = if st == DebridCloudStatus::Ready {
                    vec![format!("torbox://torrent/{}", t.id)]
                } else {
                    Vec::new()
                };
                DebridCloudItem {
                    id: t.id.to_string(),
                    target: self.target_id().into(),
                    provider_label: self.display_name().into(),
                    filename: t.name,
                    info_hash: t.hash,
                    size_bytes: t.size,
                    downloaded_bytes,
                    status: st,
                    progress_pct: pct,
                    speed_bps: t.download_speed,
                    seeders: t.seeds,
                    eta_seconds: t.eta,
                    links,
                    added_at: t.created_at,
                }
            })
            .collect())
    }

    async fn delete_cloud(&self, torrent_id: &str) -> VesselResult<()> {
        let token = self.token().await?;
        let id_num: u64 = torrent_id
            .trim()
            .parse()
            .map_err(|_| VesselError::InvalidInput("invalid TorBox torrent id".into()))?;
        let resp = self
            .client
            .post(format!("{}/torrents/controltorrent", self.api_base))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "torrent_id": id_num,
                "operation": "delete",
            }))
            .send()
            .await
            .map_err(|e| map_http_err("TorBox", e))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(map_status_err("TorBox", status, &body));
        }
        Ok(())
    }

    async fn unrestrict_link(&self, link: &str) -> VesselResult<ResolvedHttpLink> {
        let token = self.token().await?;
        if let Some(rest) = link.strip_prefix("torbox://torrent/") {
            let id_num: u64 = rest
                .trim()
                .parse()
                .map_err(|_| VesselError::InvalidInput("invalid torbox:// link".into()))?;
            let dl_url = self.request_dl(&token, id_num, None).await?;
            return Ok(ResolvedHttpLink {
                url: dl_url,
                headers: Default::default(),
                filename: None,
                size_bytes: None,
                expires_at: None,
            });
        }
        Ok(ResolvedHttpLink {
            url: link.to_owned(),
            headers: Default::default(),
            filename: None,
            size_bytes: None,
            expires_at: None,
        })
    }
}
