//! Debrid-Link REST API v2 provider.

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

pub const DEBRID_LINK_SECRET_KEY: &str = "debrid.debrid_link.api_key";

#[derive(Clone)]
pub struct DebridLinkProvider {
    client: Client,
    secrets: Arc<dyn SecretStore>,
    api_base: String,
}

impl DebridLinkProvider {
    pub fn new(client: Client, secrets: Arc<dyn SecretStore>) -> Self {
        Self::with_base(client, secrets, "https://debrid-link.com/api/v2")
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
        require_secret(&self.secrets, DEBRID_LINK_SECRET_KEY, "Debrid-Link").await
    }
}

#[derive(Debug, Deserialize)]
struct DlFile {
    id: String,
    name: String,
    #[serde(default)]
    size: u64,
    #[serde(default, rename = "downloadUrl")]
    download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DlTorrent {
    id: String,
    name: String,
    #[serde(default, rename = "hashString")]
    hash_string: Option<String>,
    #[serde(default, rename = "totalSize")]
    total_size: Option<u64>,
    #[serde(default, rename = "downloadPercent")]
    download_percent: f32,
    #[serde(default, rename = "downloadSpeed")]
    download_speed: Option<u64>,
    #[serde(default, rename = "peersConnected")]
    peers_connected: Option<u32>,
    #[serde(default)]
    files: Vec<DlFile>,
}

#[async_trait]
impl DebridProvider for DebridLinkProvider {
    fn target_id(&self) -> &str {
        DebridProviderKind::DebridLink.target_id()
    }

    fn display_name(&self) -> &str {
        DebridProviderKind::DebridLink.display_name()
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
            .get(format!("{}/account/infos", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("Debrid-Link", e))?;
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
                error: Some(map_status_err("Debrid-Link", status, &body).to_string()),
            });
        }

        #[derive(Deserialize)]
        struct DlInfo {
            username: Option<String>,
            email: Option<String>,
            #[serde(rename = "accountType")]
            account_type: Option<u32>,
            #[serde(rename = "premiumLeft")]
            premium_left: Option<u64>,
        }
        #[derive(Deserialize)]
        struct InfoResp {
            value: Option<DlInfo>,
        }
        let parsed: InfoResp = parse_json_response("Debrid-Link", "infos", &body)?;
        let val = parsed.value;
        let is_premium = val
            .as_ref()
            .is_some_and(|v| v.account_type.unwrap_or(0) > 0 || v.premium_left.unwrap_or(0) > 0);

        Ok(DebridAccountStatus {
            target: self.target_id().into(),
            label: self.display_name().into(),
            is_builtin: true,
            supports_bulk_cache_check: true,
            enabled: true,
            authenticated: val.is_some(),
            username: val.as_ref().and_then(|v| v.username.clone()),
            email: val.as_ref().and_then(|v| v.email.clone()),
            is_premium,
            expires_at: val
                .as_ref()
                .and_then(|v| v.premium_left)
                .map(|secs| format!("+{secs}s")),
            points_or_quota: None,
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
            let resp = self
                .client
                .get(format!("{}/seedbox/cached?url={joined}", self.api_base))
                .bearer_auth(&token)
                .send()
                .await
                .map_err(|e| map_http_err("Debrid-Link", e))?;
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(map_status_err("Debrid-Link", status, &body));
            }

            #[derive(Deserialize)]
            struct CacheResp {
                value: Option<HashMap<String, serde_json::Value>>,
            }
            let parsed: CacheResp = parse_json_response("Debrid-Link", "cached", &body)?;
            for h in chunk {
                result.insert(h.to_ascii_lowercase(), false);
            }
            if let Some(map) = parsed.value {
                for (k, _) in map {
                    result.insert(k.to_ascii_lowercase(), true);
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
        let resp = self
            .client
            .post(format!("{}/seedbox/add", self.api_base))
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encode_form(&[("url", magnet_or_hash), ("async", "true")]))
            .send()
            .await
            .map_err(|e| map_http_err("Debrid-Link", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("Debrid-Link", status, &body));
        }

        #[derive(Deserialize)]
        struct AddResp {
            value: Option<DlTorrent>,
        }
        let parsed: AddResp = parse_json_response("Debrid-Link", "add", &body)?;
        let torrent = parsed
            .value
            .ok_or_else(|| VesselError::Network("Debrid-Link add missing value".into()))?;

        if torrent.download_percent >= 100.0 && !torrent.files.is_empty() {
            let candidates: Vec<TorrentFileCandidate> = torrent
                .files
                .iter()
                .enumerate()
                .map(|(index, f)| TorrentFileCandidate {
                    id: f.id.clone(),
                    index,
                    path: f.name.clone(),
                    size_bytes: f.size,
                })
                .collect();
            let picked_idx = select_best_file(&candidates, target)
                .map(|c| c.index)
                .unwrap_or(0);
            let file = &torrent.files[picked_idx];
            if let Some(ref dl_url) = file.download_url {
                return Ok(DebridResolveOutcome::Ready(ResolvedHttpLink {
                    url: dl_url.clone(),
                    headers: Default::default(),
                    filename: Some(file.name.clone()),
                    size_bytes: Some(file.size),
                    expires_at: None,
                }));
            }
        }

        if !allow_uncached_queue {
            let _ = self.delete_cloud(&torrent.id).await;
            return Ok(DebridResolveOutcome::PendingCloud {
                torrent_id: None,
                status: DebridCloudStatus::Downloading,
                progress_pct: torrent.download_percent,
                speed_bps: torrent.download_speed,
                seeders: torrent.peers_connected,
            });
        }

        Ok(DebridResolveOutcome::PendingCloud {
            torrent_id: Some(torrent.id),
            status: DebridCloudStatus::Downloading,
            progress_pct: torrent.download_percent,
            speed_bps: torrent.download_speed,
            seeders: torrent.peers_connected,
        })
    }

    async fn list_cloud(&self) -> VesselResult<Vec<DebridCloudItem>> {
        let token = self.token().await?;
        let resp = self
            .client
            .get(format!("{}/seedbox/list", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("Debrid-Link", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("Debrid-Link", status, &body));
        }

        #[derive(Deserialize)]
        struct ListResp {
            #[serde(default)]
            value: Vec<DlTorrent>,
        }
        let parsed: ListResp = parse_json_response("Debrid-Link", "list", &body)?;

        Ok(parsed
            .value
            .into_iter()
            .map(|t| {
                let ready = t.download_percent >= 100.0;
                let links = if ready {
                    t.files
                        .into_iter()
                        .filter_map(|f| f.download_url)
                        .collect()
                } else {
                    Vec::new()
                };
                let downloaded_bytes = t.total_size.map(|total| {
                    ((total as f64) * (t.download_percent.clamp(0.0, 100.0) as f64 / 100.0)) as u64
                });
                DebridCloudItem {
                    id: t.id,
                    target: self.target_id().into(),
                    provider_label: self.display_name().into(),
                    filename: t.name,
                    info_hash: t.hash_string,
                    size_bytes: t.total_size,
                    downloaded_bytes,
                    status: if ready {
                        DebridCloudStatus::Ready
                    } else {
                        DebridCloudStatus::Downloading
                    },
                    progress_pct: t.download_percent,
                    speed_bps: t.download_speed,
                    seeders: t.peers_connected,
                    eta_seconds: None,
                    links,
                    added_at: None,
                }
            })
            .collect())
    }

    async fn delete_cloud(&self, torrent_id: &str) -> VesselResult<()> {
        let token = self.token().await?;
        let resp = self
            .client
            .delete(format!("{}/seedbox/{torrent_id}/remove", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("Debrid-Link", e))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(map_status_err("Debrid-Link", status, &body));
        }
        Ok(())
    }

    async fn unrestrict_link(&self, link: &str) -> VesselResult<ResolvedHttpLink> {
        Ok(ResolvedHttpLink {
            url: link.to_owned(),
            headers: Default::default(),
            filename: None,
            size_bytes: None,
            expires_at: None,
        })
    }
}
