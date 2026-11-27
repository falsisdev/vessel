//! Premiumize REST API v1 provider.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use vessel_domain::{
    DebridAccountStatus, DebridCloudItem, DebridCloudStatus, DebridProvider, DebridProviderKind,
    DebridResolveOutcome, ResolvedHttpLink, SecretStore, TorrentTarget, VesselResult,
};

use crate::episode_matcher::{TorrentFileCandidate, select_best_file};
use crate::provider::{encode_form, map_http_err, map_status_err, parse_json_response, require_secret};

pub const PREMIUMIZE_SECRET_KEY: &str = "debrid.premiumize.api_key";

#[derive(Clone)]
pub struct PremiumizeProvider {
    client: Client,
    secrets: Arc<dyn SecretStore>,
    api_base: String,
}

impl PremiumizeProvider {
    pub fn new(client: Client, secrets: Arc<dyn SecretStore>) -> Self {
        Self::with_base(client, secrets, "https://www.premiumize.me/api")
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
        require_secret(&self.secrets, PREMIUMIZE_SECRET_KEY, "Premiumize").await
    }
}

#[async_trait]
impl DebridProvider for PremiumizeProvider {
    fn target_id(&self) -> &str {
        DebridProviderKind::Premiumize.target_id()
    }

    fn display_name(&self) -> &str {
        DebridProviderKind::Premiumize.display_name()
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
            .get(format!("{}/account/info", self.api_base))
            .query(&[("apikey", token.as_str())])
            .send()
            .await
            .map_err(|e| map_http_err("Premiumize", e))?;
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
                error: Some(map_status_err("Premiumize", status, &body).to_string()),
            });
        }

        #[derive(Deserialize)]
        struct PmAccount {
            status: String,
            customer_id: Option<serde_json::Value>,
            premium_until: Option<u64>,
            limit_used: Option<f64>,
        }
        let parsed: PmAccount = parse_json_response("Premiumize", "account", &body)?;
        let authed = parsed.status.eq_ignore_ascii_case("success");

        Ok(DebridAccountStatus {
            target: self.target_id().into(),
            label: self.display_name().into(),
            is_builtin: true,
            supports_bulk_cache_check: true,
            enabled: true,
            authenticated: authed,
            username: parsed.customer_id.map(|v| v.to_string()),
            email: None,
            is_premium: parsed.premium_until.unwrap_or(0) > 0,
            expires_at: parsed.premium_until.map(|t| t.to_string()),
            points_or_quota: parsed.limit_used.map(|u| format!("{:.0}% used", u * 100.0)),
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
            let mut form: Vec<(String, String)> = vec![("apikey".into(), token.clone())];
            for h in chunk {
                form.push(("items[]".into(), h.clone()));
            }
            let resp = self
                .client
                .post(format!("{}/cache/check", self.api_base))
                .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(encode_form(&form))
                .send()
                .await
                .map_err(|e| map_http_err("Premiumize", e))?;
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            if !status.is_success() {
                return Err(map_status_err("Premiumize", status, &body));
            }

            #[derive(Deserialize)]
            struct CacheResp {
                #[serde(default)]
                response: Vec<bool>,
            }
            let parsed: CacheResp = parse_json_response("Premiumize", "cache", &body)?;
            for (h, cached) in chunk.iter().zip(parsed.response.into_iter()) {
                result.insert(h.to_ascii_lowercase(), cached);
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
        let direct_resp = self
            .client
            .post(format!("{}/transfer/directdl", self.api_base))
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encode_form(&[("apikey", token.as_str()), ("src", magnet_or_hash)]))
            .send()
            .await
            .map_err(|e| map_http_err("Premiumize", e))?;
        let status = direct_resp.status();
        let body = direct_resp.text().await.unwrap_or_default();
        if status.is_success() {
            #[derive(Deserialize)]
            struct PmContent {
                path: String,
                #[serde(default)]
                size: u64,
                link: Option<String>,
                stream_link: Option<String>,
            }
            #[derive(Deserialize)]
            struct DirectResp {
                status: String,
                #[serde(default)]
                content: Vec<PmContent>,
            }
            if let Ok(parsed) = serde_json::from_str::<DirectResp>(&body)
                && parsed.status.eq_ignore_ascii_case("success")
                && !parsed.content.is_empty()
            {
                let candidates: Vec<TorrentFileCandidate> = parsed
                    .content
                    .iter()
                    .enumerate()
                    .map(|(index, c)| TorrentFileCandidate {
                        id: index.to_string(),
                        index,
                        path: c.path.clone(),
                        size_bytes: c.size,
                    })
                    .collect();
                let picked_idx = select_best_file(&candidates, target)
                    .map(|c| c.index)
                    .unwrap_or(0);
                let item = &parsed.content[picked_idx];
                if let Some(url) = item.link.as_ref().or(item.stream_link.as_ref()) {
                    return Ok(DebridResolveOutcome::Ready(ResolvedHttpLink {
                        url: url.clone(),
                        headers: Default::default(),
                        filename: Some(item.path.clone()),
                        size_bytes: Some(item.size),
                        expires_at: None,
                    }));
                }
            }
        }

        if !allow_uncached_queue {
            return Ok(DebridResolveOutcome::PendingCloud {
                torrent_id: None,
                status: DebridCloudStatus::Queued,
                progress_pct: 0.0,
                speed_bps: None,
                seeders: None,
            });
        }

        let create_resp = self
            .client
            .post(format!("{}/transfer/create", self.api_base))
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encode_form(&[("apikey", token.as_str()), ("src", magnet_or_hash)]))
            .send()
            .await
            .map_err(|e| map_http_err("Premiumize", e))?;
        let c_status = create_resp.status();
        let c_body = create_resp.text().await.unwrap_or_default();
        if !c_status.is_success() {
            return Err(map_status_err("Premiumize", c_status, &c_body));
        }

        #[derive(Deserialize)]
        struct CreateResp {
            id: Option<String>,
        }
        let created: CreateResp = parse_json_response("Premiumize", "create", &c_body)?;

        Ok(DebridResolveOutcome::PendingCloud {
            torrent_id: created.id,
            status: DebridCloudStatus::Downloading,
            progress_pct: 0.0,
            speed_bps: None,
            seeders: None,
        })
    }

    async fn list_cloud(&self) -> VesselResult<Vec<DebridCloudItem>> {
        let token = self.token().await?;
        let resp = self
            .client
            .get(format!("{}/transfer/list", self.api_base))
            .query(&[("apikey", token.as_str())])
            .send()
            .await
            .map_err(|e| map_http_err("Premiumize", e))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(map_status_err("Premiumize", status, &body));
        }

        #[derive(Deserialize)]
        struct PmTransfer {
            id: String,
            name: String,
            status: String,
            #[serde(default)]
            progress: f32,
            src: Option<String>,
        }
        #[derive(Deserialize)]
        struct ListResp {
            #[serde(default)]
            transfers: Vec<PmTransfer>,
        }
        let parsed: ListResp = parse_json_response("Premiumize", "list", &body)?;

        Ok(parsed
            .transfers
            .into_iter()
            .map(|t| {
                let st = match t.status.to_ascii_lowercase().as_str() {
                    "finished" | "seeding" => DebridCloudStatus::Ready,
                    "running" | "downloading" => DebridCloudStatus::Downloading,
                    "waiting" | "queued" => DebridCloudStatus::Queued,
                    _ => DebridCloudStatus::Error,
                };
                DebridCloudItem {
                    id: t.id,
                    target: self.target_id().into(),
                    provider_label: self.display_name().into(),
                    filename: t.name,
                    info_hash: t.src.as_deref().and_then(vessel_domain::extract_info_hash),
                    size_bytes: None,
                    downloaded_bytes: None,
                    status: st,
                    progress_pct: if t.progress <= 1.0 {
                        t.progress * 100.0
                    } else {
                        t.progress
                    },
                    speed_bps: None,
                    seeders: None,
                    eta_seconds: None,
                    links: Vec::new(),
                    added_at: None,
                }
            })
            .collect())
    }

    async fn delete_cloud(&self, torrent_id: &str) -> VesselResult<()> {
        let token = self.token().await?;
        let resp = self
            .client
            .post(format!("{}/transfer/delete", self.api_base))
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(encode_form(&[("apikey", token.as_str()), ("id", torrent_id)]))
            .send()
            .await
            .map_err(|e| map_http_err("Premiumize", e))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(map_status_err("Premiumize", status, &body));
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
