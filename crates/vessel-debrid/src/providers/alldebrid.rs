//! AllDebrid REST API v4 provider.

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

pub const ALLDEBRID_SECRET_KEY: &str = "debrid.alldebrid.api_key";

#[derive(Clone)]
pub struct AllDebridProvider {
    client: Client,
    secrets: Arc<dyn SecretStore>,
    api_base: String,
}

impl AllDebridProvider {
    pub fn new(client: Client, secrets: Arc<dyn SecretStore>) -> Self {
        Self::with_base(client, secrets, "https://api.alldebrid.com/v4")
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
        require_secret(&self.secrets, ALLDEBRID_SECRET_KEY, "AllDebrid").await
    }
}

#[derive(Debug, Deserialize)]
struct AdMagnetStatus {
    id: u64,
    filename: String,
    #[serde(default)]
    hash: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    #[serde(default)]
    downloaded: Option<u64>,
    #[serde(default, rename = "downloadSpeed")]
    download_speed: Option<u64>,
    #[serde(default)]
    seeders: Option<u32>,
    #[serde(default)]
    status: String,
    #[serde(default, rename = "statusCode")]
    status_code: Option<u32>,
    #[serde(default)]
    links: Vec<AdLinkEntry>,
}

#[derive(Debug, Deserialize)]
struct AdLinkEntry {
    link: String,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    size: u64,
}

fn map_ad_status(status_code: Option<u32>, status: &str) -> DebridCloudStatus {
    match status_code {
        Some(4) => DebridCloudStatus::Ready,
        Some(0) => DebridCloudStatus::Queued,
        Some(1) | Some(2) => DebridCloudStatus::Downloading,
        Some(3) => DebridCloudStatus::Uploading,
        Some(_) => DebridCloudStatus::Error,
        None => {
            if status.eq_ignore_ascii_case("ready") {
                DebridCloudStatus::Ready
            } else {
                DebridCloudStatus::Downloading
            }
        }
    }
}

#[async_trait]
impl DebridProvider for AllDebridProvider {
    fn target_id(&self) -> &str {
        DebridProviderKind::AllDebrid.target_id()
    }

    fn display_name(&self) -> &str {
        DebridProviderKind::AllDebrid.display_name()
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
            .get(format!("{}/user?agent=Vessel", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("AllDebrid", e))?;
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
                error: Some(map_status_err("AllDebrid", status, &body).to_string()),
            });
        }

        #[derive(Deserialize)]
        struct AdUser {
            username: Option<String>,
            email: Option<String>,
            #[serde(default, rename = "isPremium")]
            is_premium: bool,
            #[serde(rename = "premiumUntil")]
            premium_until: Option<u64>,
            #[serde(rename = "fidelityPoints")]
            fidelity_points: Option<u64>,
        }
        #[derive(Deserialize)]
        struct AdUserData {
            user: AdUser,
        }
        #[derive(Deserialize)]
        struct AdResp {
            data: Option<AdUserData>,
        }
        let parsed: AdResp = parse_json_response("AllDebrid", "user", &body)?;
        let user = parsed.data.map(|d| d.user);

        Ok(DebridAccountStatus {
            target: self.target_id().into(),
            label: self.display_name().into(),
            is_builtin: true,
            supports_bulk_cache_check: true,
            enabled: true,
            authenticated: user.is_some(),
            username: user.as_ref().and_then(|u| u.username.clone()),
            email: user.as_ref().and_then(|u| u.email.clone()),
            is_premium: user.as_ref().is_some_and(|u| u.is_premium),
            expires_at: user
                .as_ref()
                .and_then(|u| u.premium_until)
                .map(|ts| ts.to_string()),
            points_or_quota: user
                .as_ref()
                .and_then(|u| u.fidelity_points)
                .map(|p| format!("{p} pts")),
            error: None,
        })
    }

    async fn check_cached(&self, hashes: &[String]) -> VesselResult<HashMap<String, bool>> {
        let mut result = HashMap::new();
        if hashes.is_empty() {
            return Ok(result);
        }
        let token = self.token().await?;

        for chunk in hashes.chunks(40) {
            let mut params = vec![("agent".to_owned(), "Vessel".to_owned())];
            for h in chunk {
                params.push(("magnets[]".to_owned(), h.clone()));
            }
            let resp = self
                .client
                .post(format!("{}/magnet/instant", self.api_base))
                .bearer_auth(&token)
                .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(encode_form(&params))
                .send()
                .await
                .map_err(|e| map_http_err("AllDebrid", e))?;
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(map_status_err("AllDebrid", status, &body));
            }

            #[derive(Deserialize)]
            struct InstantMagnet {
                hash: Option<String>,
                magnet: Option<String>,
                #[serde(default)]
                instant: bool,
            }
            #[derive(Deserialize)]
            struct InstantData {
                #[serde(default)]
                magnets: Vec<InstantMagnet>,
            }
            #[derive(Deserialize)]
            struct InstantResp {
                data: Option<InstantData>,
            }
            let parsed: InstantResp = parse_json_response("AllDebrid", "instant", &body)?;
            for h in chunk {
                result.insert(h.to_ascii_lowercase(), false);
            }
            if let Some(data) = parsed.data {
                for m in data.magnets {
                    if let Some(h) = m.hash.or(m.magnet) {
                        result.insert(h.to_ascii_lowercase(), m.instant);
                    }
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
            .post(format!("{}/magnet/upload?agent=Vessel", self.api_base))
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encode_form(&[("magnets[]", magnet_or_hash)]))
            .send()
            .await
            .map_err(|e| map_http_err("AllDebrid", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("AllDebrid", status, &body));
        }

        #[derive(Deserialize)]
        struct UploadedMagnet {
            id: u64,
        }
        #[derive(Deserialize)]
        struct UploadData {
            #[serde(default)]
            magnets: Vec<UploadedMagnet>,
        }
        #[derive(Deserialize)]
        struct UploadResp {
            data: Option<UploadData>,
        }
        let uploaded: UploadResp = parse_json_response("AllDebrid", "upload", &body)?;
        let magnet_id = uploaded
            .data
            .and_then(|d| d.magnets.into_iter().next())
            .map(|m| m.id)
            .ok_or_else(|| VesselError::Network("AllDebrid upload missing magnet id".into()))?;

        let status_resp = self
            .client
            .get(format!("{}/magnet/status?agent=Vessel&id={magnet_id}", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("AllDebrid", e))?;
        let st_code = status_resp.status();
        let st_body = status_resp.text().await.unwrap_or_default();
        if !st_code.is_success() {
            return Err(map_status_err("AllDebrid", st_code, &st_body));
        }

        #[derive(Deserialize)]
        struct StatusData {
            magnets: AdMagnetStatus,
        }
        #[derive(Deserialize)]
        struct StatusResp {
            data: Option<StatusData>,
        }
        let parsed: StatusResp = parse_json_response("AllDebrid", "magnet/status", &st_body)?;
        let mag = parsed
            .data
            .map(|d| d.magnets)
            .ok_or_else(|| VesselError::NotFound("AllDebrid magnet status missing".into()))?;

        let cloud_status = map_ad_status(mag.status_code, &mag.status);
        if cloud_status == DebridCloudStatus::Ready && !mag.links.is_empty() {
            let candidates: Vec<TorrentFileCandidate> = mag
                .links
                .iter()
                .enumerate()
                .map(|(index, l)| TorrentFileCandidate {
                    id: l.link.clone(),
                    index,
                    path: l.filename.clone(),
                    size_bytes: l.size,
                })
                .collect();
            let picked = select_best_file(&candidates, target)
                .map(|c| c.id.as_str())
                .unwrap_or_else(|| mag.links[0].link.as_str());
            let unlocked = self.unrestrict_link(picked).await?;
            return Ok(DebridResolveOutcome::Ready(unlocked));
        }

        let pct = match (mag.size, mag.downloaded) {
            (Some(total), Some(done)) if total > 0 => (done as f32 / total as f32) * 100.0,
            _ => 0.0,
        };
        if !allow_uncached_queue {
            let _ = self.delete_cloud(&magnet_id.to_string()).await;
            return Ok(DebridResolveOutcome::PendingCloud {
                torrent_id: None,
                status: cloud_status,
                progress_pct: pct,
                speed_bps: mag.download_speed,
                seeders: mag.seeders,
            });
        }

        Ok(DebridResolveOutcome::PendingCloud {
            torrent_id: Some(magnet_id.to_string()),
            status: cloud_status,
            progress_pct: pct,
            speed_bps: mag.download_speed,
            seeders: mag.seeders,
        })
    }

    async fn list_cloud(&self) -> VesselResult<Vec<DebridCloudItem>> {
        let token = self.token().await?;
        let resp = self
            .client
            .get(format!("{}/magnet/status?agent=Vessel", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("AllDebrid", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("AllDebrid", status, &body));
        }

        #[derive(Deserialize)]
        struct ListData {
            #[serde(default)]
            magnets: Vec<AdMagnetStatus>,
        }
        #[derive(Deserialize)]
        struct ListResp {
            data: Option<ListData>,
        }
        let parsed: ListResp = parse_json_response("AllDebrid", "list", &body)?;

        Ok(parsed
            .data
            .map(|d| d.magnets)
            .unwrap_or_default()
            .into_iter()
            .map(|m| {
                let st = map_ad_status(m.status_code, &m.status);
                let pct = match (m.size, m.downloaded) {
                    (Some(total), Some(done)) if total > 0 => (done as f32 / total as f32) * 100.0,
                    _ if st == DebridCloudStatus::Ready => 100.0,
                    _ => 0.0,
                };
                let eta_seconds = match (m.size, m.downloaded, m.download_speed) {
                    (Some(total), Some(done), Some(speed)) if speed > 0 && total > done => {
                        Some((total - done) / speed)
                    }
                    _ => None,
                };
                DebridCloudItem {
                    id: m.id.to_string(),
                    target: self.target_id().into(),
                    provider_label: self.display_name().into(),
                    filename: m.filename,
                    info_hash: m.hash,
                    size_bytes: m.size,
                    downloaded_bytes: m.downloaded,
                    status: st,
                    progress_pct: pct,
                    speed_bps: m.download_speed,
                    seeders: m.seeders,
                    eta_seconds,
                    links: m.links.into_iter().map(|l| l.link).collect(),
                    added_at: None,
                }
            })
            .collect())
    }

    async fn delete_cloud(&self, torrent_id: &str) -> VesselResult<()> {
        let token = self.token().await?;
        let resp = self
            .client
            .get(format!("{}/magnet/delete?agent=Vessel&id={torrent_id}", self.api_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("AllDebrid", e))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(map_status_err("AllDebrid", status, &body));
        }
        Ok(())
    }

    async fn unrestrict_link(&self, link: &str) -> VesselResult<ResolvedHttpLink> {
        let token = self.token().await?;
        let encoded: String = url::form_urlencoded::byte_serialize(link.as_bytes()).collect();
        let resp = self
            .client
            .get(format!(
                "{}/link/unlock?agent=Vessel&link={encoded}",
                self.api_base
            ))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("AllDebrid", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("AllDebrid", status, &body));
        }

        #[derive(Deserialize)]
        struct UnlockData {
            link: String,
            filename: Option<String>,
            filesize: Option<u64>,
        }
        #[derive(Deserialize)]
        struct UnlockResp {
            data: Option<UnlockData>,
        }
        let parsed: UnlockResp = parse_json_response("AllDebrid", "link/unlock", &body)?;
        let data = parsed
            .data
            .ok_or_else(|| VesselError::Network("AllDebrid link/unlock missing data".into()))?;

        Ok(ResolvedHttpLink {
            url: data.link,
            headers: Default::default(),
            filename: data.filename,
            size_bytes: data.filesize,
            expires_at: None,
        })
    }
}
