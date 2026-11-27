//! Real-Debrid REST API v1.0 & OAuth Device Code provider.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use vessel_domain::{
    DebridAccountStatus, DebridCloudItem, DebridCloudStatus, DebridProvider, DebridProviderKind,
    DebridResolveOutcome, RealDebridDeviceAuthPoll, RealDebridDeviceAuthStart, ResolvedHttpLink,
    SecretStore, TorrentTarget, VesselResult,
};

use crate::episode_matcher::{TorrentFileCandidate, select_best_file};
use crate::provider::{encode_form, map_http_err, map_status_err, parse_json_response, require_secret};

pub const REAL_DEBRID_SECRET_KEY: &str = "debrid.real_debrid.api_key";
pub const REAL_DEBRID_CLIENT_ID_KEY: &str = "debrid.real_debrid.oauth_client_id";
pub const REAL_DEBRID_CLIENT_SECRET_KEY: &str = "debrid.real_debrid.oauth_client_secret";
pub const REAL_DEBRID_REFRESH_KEY: &str = "debrid.real_debrid.refresh_token";
const RD_OPEN_SOURCE_CLIENT_ID: &str = "X245A4XAIBGVM";

#[derive(Clone)]
pub struct RealDebridProvider {
    client: Client,
    secrets: Arc<dyn SecretStore>,
    rest_base: String,
    oauth_base: String,
}

impl RealDebridProvider {
    pub fn new(client: Client, secrets: Arc<dyn SecretStore>) -> Self {
        Self::with_bases(
            client,
            secrets,
            "https://api.real-debrid.com/rest/1.0",
            "https://api.real-debrid.com/oauth/v2",
        )
    }

    pub fn with_bases(
        client: Client,
        secrets: Arc<dyn SecretStore>,
        rest_base: impl Into<String>,
        oauth_base: impl Into<String>,
    ) -> Self {
        Self {
            client,
            secrets,
            rest_base: rest_base.into().trim_end_matches('/').to_owned(),
            oauth_base: oauth_base.into().trim_end_matches('/').to_owned(),
        }
    }

    async fn token(&self) -> VesselResult<String> {
        require_secret(&self.secrets, REAL_DEBRID_SECRET_KEY, "Real-Debrid").await
    }

    /// Initiates Real-Debrid OAuth2 Device Code authentication flow.
    pub async fn start_device_auth(&self) -> VesselResult<RealDebridDeviceAuthStart> {
        let url = format!(
            "{}/device/code?client_id={RD_OPEN_SOURCE_CLIENT_ID}&new_credentials=yes",
            self.oauth_base
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| map_http_err("Real-Debrid", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("Real-Debrid", status, &body));
        }

        #[derive(Deserialize)]
        struct Resp {
            device_code: String,
            user_code: String,
            verification_url: String,
            direct_verification_url: Option<String>,
            interval: Option<u64>,
            expires_in: Option<u64>,
        }
        let parsed: Resp = parse_json_response("Real-Debrid", "device/code", &body)?;

        Ok(RealDebridDeviceAuthStart {
            device_code: parsed.device_code,
            user_code: parsed.user_code,
            verification_url: parsed.verification_url,
            direct_verification_url: parsed.direct_verification_url,
            interval_seconds: parsed.interval.unwrap_or(5),
            expires_in_seconds: parsed.expires_in.unwrap_or(600),
        })
    }

    /// Polls Real-Debrid OAuth2 Device Code credentials and exchanges for an access token once approved.
    pub async fn poll_device_auth(&self, device_code: &str) -> VesselResult<RealDebridDeviceAuthPoll> {
        let cred_url = format!(
            "{}/device/credentials?client_id={RD_OPEN_SOURCE_CLIENT_ID}&code={}",
            self.oauth_base,
            url::form_urlencoded::byte_serialize(device_code.trim().as_bytes()).collect::<String>()
        );
        let cred_resp = self
            .client
            .get(&cred_url)
            .send()
            .await
            .map_err(|e| map_http_err("Real-Debrid", e))?;
        let cred_status = cred_resp.status();
        let cred_body = cred_resp.text().await.unwrap_or_default();
        if cred_status.as_u16() == 403 || cred_status.as_u16() == 400 {
            return Ok(RealDebridDeviceAuthPoll {
                authorized: false,
                pending: true,
                username: None,
                error: None,
            });
        }
        if !cred_status.is_success() {
            return Err(map_status_err("Real-Debrid", cred_status, &cred_body));
        }

        #[derive(Deserialize)]
        struct CredResp {
            client_id: String,
            client_secret: String,
        }
        let creds: CredResp = parse_json_response("Real-Debrid", "device/credentials", &cred_body)?;

        let token_url = format!("{}/token", self.oauth_base);
        let form_body = encode_form(&[
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("code", device_code.trim()),
            ("grant_type", "http://oauth.net/grant_type/device/1.0"),
        ]);
        let token_resp = self
            .client
            .post(&token_url)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(form_body)
            .send()
            .await
            .map_err(|e| map_http_err("Real-Debrid", e))?;
        let token_status = token_resp.status();
        let token_body = token_resp.text().await.unwrap_or_default();
        if !token_status.is_success() {
            return Err(map_status_err("Real-Debrid", token_status, &token_body));
        }

        #[derive(Deserialize)]
        struct TokenResp {
            access_token: String,
            refresh_token: Option<String>,
        }
        let tokens: TokenResp = parse_json_response("Real-Debrid", "token", &token_body)?;

        self.secrets
            .set(REAL_DEBRID_SECRET_KEY, &tokens.access_token)
            .await?;
        self.secrets
            .set(REAL_DEBRID_CLIENT_ID_KEY, &creds.client_id)
            .await?;
        self.secrets
            .set(REAL_DEBRID_CLIENT_SECRET_KEY, &creds.client_secret)
            .await?;
        if let Some(refresh) = tokens.refresh_token.as_deref() {
            self.secrets.set(REAL_DEBRID_REFRESH_KEY, refresh).await?;
        }

        let status = self.account_status().await.ok();
        Ok(RealDebridDeviceAuthPoll {
            authorized: true,
            pending: false,
            username: status.and_then(|s| s.username),
            error: None,
        })
    }

    async fn fetch_torrent_info(&self, token: &str, torrent_id: &str) -> VesselResult<RdTorrentInfo> {
        let url = format!("{}/torrents/info/{torrent_id}", self.rest_base);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| map_http_err("Real-Debrid", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("Real-Debrid", status, &body));
        }
        parse_json_response("Real-Debrid", "torrent info", &body)
    }
}

#[derive(Debug, Deserialize)]
struct RdTorrentFile {
    id: u64,
    path: String,
    bytes: u64,
    #[serde(default)]
    selected: u8,
}

#[derive(Debug, Deserialize)]
struct RdTorrentInfo {
    id: String,
    filename: String,
    hash: Option<String>,
    bytes: Option<u64>,
    #[serde(default)]
    progress: f32,
    status: String,
    speed: Option<u64>,
    seeders: Option<u32>,
    added: Option<String>,
    #[serde(default)]
    files: Vec<RdTorrentFile>,
    #[serde(default)]
    links: Vec<String>,
}

fn map_rd_status(raw: &str) -> DebridCloudStatus {
    match raw.to_ascii_lowercase().as_str() {
        "downloaded" => DebridCloudStatus::Ready,
        "downloading" | "compressing" => DebridCloudStatus::Downloading,
        "uploading" => DebridCloudStatus::Uploading,
        "magnet_conversion" | "waiting_files_selection" | "queued" => DebridCloudStatus::Queued,
        _ => DebridCloudStatus::Error,
    }
}

#[async_trait]
impl DebridProvider for RealDebridProvider {
    fn target_id(&self) -> &str {
        DebridProviderKind::RealDebrid.target_id()
    }

    fn display_name(&self) -> &str {
        DebridProviderKind::RealDebrid.display_name()
    }

    fn is_builtin(&self) -> bool {
        true
    }

    fn supports_bulk_cache_check(&self) -> bool {
        false
    }

    async fn account_status(&self) -> VesselResult<DebridAccountStatus> {
        let token = match self.token().await {
            Ok(t) => t,
            Err(_) => {
                return Ok(DebridAccountStatus {
                    target: self.target_id().into(),
                    label: self.display_name().into(),
                    is_builtin: true,
                    supports_bulk_cache_check: false,
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
            .get(format!("{}/user", self.rest_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("Real-Debrid", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Ok(DebridAccountStatus {
                target: self.target_id().into(),
                label: self.display_name().into(),
                is_builtin: true,
                supports_bulk_cache_check: false,
                enabled: true,
                authenticated: false,
                username: None,
                email: None,
                is_premium: false,
                expires_at: None,
                points_or_quota: None,
                error: Some(map_status_err("Real-Debrid", status, &body).to_string()),
            });
        }

        #[derive(Deserialize)]
        struct RdUser {
            username: Option<String>,
            email: Option<String>,
            #[serde(rename = "type")]
            account_type: Option<String>,
            expiration: Option<String>,
            points: Option<u64>,
        }
        let user: RdUser = parse_json_response("Real-Debrid", "user", &body)?;
        let is_premium = user
            .account_type
            .as_deref()
            .is_some_and(|t| t.eq_ignore_ascii_case("premium"));

        Ok(DebridAccountStatus {
            target: self.target_id().into(),
            label: self.display_name().into(),
            is_builtin: true,
            supports_bulk_cache_check: false,
            enabled: true,
            authenticated: true,
            username: user.username,
            email: user.email,
            is_premium,
            expires_at: user.expiration,
            points_or_quota: user.points.map(|p| format!("{p} pts")),
            error: None,
        })
    }

    async fn check_cached(&self, _hashes: &[String]) -> VesselResult<HashMap<String, bool>> {
        Ok(HashMap::new())
    }

    async fn resolve_magnet(
        &self,
        magnet_or_hash: &str,
        target: &TorrentTarget,
        allow_uncached_queue: bool,
    ) -> VesselResult<DebridResolveOutcome> {
        let token = self.token().await?;

        let add_resp = self
            .client
            .post(format!("{}/torrents/addMagnet", self.rest_base))
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encode_form(&[("magnet", magnet_or_hash)]))
            .send()
            .await
            .map_err(|e| map_http_err("Real-Debrid", e))?;
        let add_status = add_resp.status();
        let add_body = add_resp.text().await.unwrap_or_default();
        if !add_status.is_success() {
            return Err(map_status_err("Real-Debrid", add_status, &add_body));
        }

        #[derive(Deserialize)]
        struct AddResp {
            id: String,
        }
        let added: AddResp = parse_json_response("Real-Debrid", "addMagnet", &add_body)?;

        let mut info = self.fetch_torrent_info(&token, &added.id).await?;
        let mut selected_link_idx = 0usize;

        if !info.files.is_empty() {
            let candidates: Vec<TorrentFileCandidate> = info
                .files
                .iter()
                .enumerate()
                .map(|(index, f)| TorrentFileCandidate {
                    id: f.id.to_string(),
                    index,
                    path: f.path.clone(),
                    size_bytes: f.bytes,
                })
                .collect();
            let picked = select_best_file(&candidates, target)
                .map(|c| c.id.clone())
                .unwrap_or_else(|| "all".to_owned());

            let sel_resp = self
                .client
                .post(format!("{}/torrents/selectFiles/{}", self.rest_base, added.id))
                .bearer_auth(&token)
                .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(encode_form(&[("files", picked.as_str())]))
                .send()
                .await
                .map_err(|e| map_http_err("Real-Debrid", e))?;
            let sel_status = sel_resp.status();
            if !sel_status.is_success() {
                let sel_body = sel_resp.text().await.unwrap_or_default();
                let _ = self.delete_cloud(&added.id).await;
                return Err(map_status_err("Real-Debrid", sel_status, &sel_body));
            }

            info = self.fetch_torrent_info(&token, &added.id).await?;
            if let Some(pos) = info
                .files
                .iter()
                .filter(|f| f.selected == 1)
                .position(|f| f.id.to_string() == picked)
            {
                selected_link_idx = pos;
            }
        }

        if info.status.eq_ignore_ascii_case("downloaded")
            && let Some(link) = info
                .links
                .get(selected_link_idx)
                .or_else(|| info.links.first())
        {
            let resolved = self.unrestrict_link(link).await?;
            return Ok(DebridResolveOutcome::Ready(resolved));
        }

        if !allow_uncached_queue {
            let _ = self.delete_cloud(&added.id).await;
            return Ok(DebridResolveOutcome::PendingCloud {
                torrent_id: None,
                status: map_rd_status(&info.status),
                progress_pct: info.progress,
                speed_bps: info.speed,
                seeders: info.seeders,
            });
        }

        Ok(DebridResolveOutcome::PendingCloud {
            torrent_id: Some(added.id),
            status: map_rd_status(&info.status),
            progress_pct: info.progress,
            speed_bps: info.speed,
            seeders: info.seeders,
        })
    }

    async fn list_cloud(&self) -> VesselResult<Vec<DebridCloudItem>> {
        let token = self.token().await?;
        let resp = self
            .client
            .get(format!("{}/torrents?limit=50", self.rest_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("Real-Debrid", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("Real-Debrid", status, &body));
        }

        let items: Vec<RdTorrentInfo> = parse_json_response("Real-Debrid", "torrents", &body)?;

        Ok(items
            .into_iter()
            .map(|t| {
                let size_bytes = t.bytes;
                let downloaded_bytes = size_bytes.map(|total| {
                    ((total as f64) * (t.progress.clamp(0.0, 100.0) as f64 / 100.0)) as u64
                });
                let eta_seconds = match (size_bytes, downloaded_bytes, t.speed) {
                    (Some(total), Some(done), Some(speed)) if speed > 0 && total > done => {
                        Some((total - done) / speed)
                    }
                    _ => None,
                };
                DebridCloudItem {
                    id: t.id,
                    target: self.target_id().into(),
                    provider_label: self.display_name().into(),
                    filename: t.filename,
                    info_hash: t.hash,
                    size_bytes,
                    downloaded_bytes,
                    status: map_rd_status(&t.status),
                    progress_pct: t.progress,
                    speed_bps: t.speed,
                    seeders: t.seeders,
                    eta_seconds,
                    links: t.links,
                    added_at: t.added,
                }
            })
            .collect())
    }

    async fn delete_cloud(&self, torrent_id: &str) -> VesselResult<()> {
        let token = self.token().await?;
        let resp = self
            .client
            .delete(format!("{}/torrents/delete/{torrent_id}", self.rest_base))
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| map_http_err("Real-Debrid", e))?;
        let status = resp.status();
        if !status.is_success() && status.as_u16() != 204 {
            let body = resp.text().await.unwrap_or_default();
            return Err(map_status_err("Real-Debrid", status, &body));
        }
        Ok(())
    }

    async fn unrestrict_link(&self, link: &str) -> VesselResult<ResolvedHttpLink> {
        let token = self.token().await?;
        let resp = self
            .client
            .post(format!("{}/unrestrict/link", self.rest_base))
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encode_form(&[("link", link)]))
            .send()
            .await
            .map_err(|e| map_http_err("Real-Debrid", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("Real-Debrid", status, &body));
        }

        #[derive(Deserialize)]
        struct UnrestrictResp {
            download: String,
            filename: Option<String>,
            filesize: Option<u64>,
        }
        let parsed: UnrestrictResp = parse_json_response("Real-Debrid", "unrestrict", &body)?;

        Ok(ResolvedHttpLink {
            url: parsed.download,
            headers: Default::default(),
            filename: parsed.filename,
            size_bytes: parsed.filesize,
            expires_at: None,
        })
    }
}
