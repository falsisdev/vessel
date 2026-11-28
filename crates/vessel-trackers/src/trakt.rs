//! Trakt.tv REST v2 tracker adapter (scrobble, history, and watchlist sync).

use async_trait::async_trait;
use serde::Deserialize;
use vessel_domain::{
    ExternalIds, MediaType, TrackerProviderKind, TrackerSyncAction, TrackerSyncStatus,
    VesselError, VesselResult, http_client_builder,
};

use crate::{TrackerProfile, TrackerProvider};

pub struct TraktProvider {
    http: reqwest::Client,
    api_base: String,
}

impl TraktProvider {
    pub fn new() -> VesselResult<Self> {
        Self::with_base("https://api.trakt.tv")
    }

    pub fn with_base(api_base: impl Into<String>) -> VesselResult<Self> {
        let http = http_client_builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|err| VesselError::Network(format!("build trakt http client: {err}")))?;
        Ok(Self {
            http,
            api_base: api_base.into().trim_end_matches('/').to_owned(),
        })
    }
}

fn ids_json(ids: &ExternalIds) -> serde_json::Value {
    let mut obj = serde_json::Map::new();
    if let Some(ref imdb) = ids.imdb {
        obj.insert("imdb".into(), serde_json::Value::String(imdb.clone()));
    }
    if let Some(tmdb) = ids.tmdb {
        obj.insert("tmdb".into(), serde_json::Value::Number(tmdb.into()));
    }
    if let Some(tvdb) = ids.tvdb {
        obj.insert("tvdb".into(), serde_json::Value::Number(tvdb.into()));
    }
    serde_json::Value::Object(obj)
}

#[async_trait]
impl TrackerProvider for TraktProvider {
    fn kind(&self) -> TrackerProviderKind {
        TrackerProviderKind::Trakt
    }

    fn is_implemented(&self) -> bool {
        true
    }

    async fn fetch_profile(
        &self,
        access_token: &str,
        client_id: Option<&str>,
    ) -> VesselResult<TrackerProfile> {
        let cid = client_id.unwrap_or_default();
        let resp = self
            .http
            .get(format!("{}/users/settings", self.api_base))
            .bearer_auth(access_token)
            .header("trakt-api-version", "2")
            .header("trakt-api-key", cid)
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("trakt profile request: {err}")))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|err| VesselError::Network(format!("trakt profile read: {err}")))?;
        if !status.is_success() {
            return Err(VesselError::Network(format!(
                "trakt profile HTTP {status}: {}",
                text.trim()
            )));
        }

        #[derive(Deserialize)]
        struct User {
            username: String,
            ids: Option<UserIds>,
        }
        #[derive(Deserialize)]
        struct UserIds {
            slug: Option<String>,
        }
        #[derive(Deserialize)]
        struct SettingsResp {
            user: User,
        }

        let parsed: SettingsResp = serde_json::from_str(&text)
            .map_err(|err| VesselError::Network(format!("decode trakt profile: {err}")))?;
        Ok(TrackerProfile {
            username: parsed.user.username,
            user_id: parsed.user.ids.and_then(|i| i.slug),
        })
    }

    async fn dispatch_action(
        &self,
        access_token: &str,
        client_id: Option<&str>,
        action: &TrackerSyncAction,
    ) -> VesselResult<()> {
        let cid = client_id.unwrap_or_default();
        match action {
            TrackerSyncAction::Scrobble {
                external_ids,
                title,
                media_type,
                season,
                episode,
                progress_pct,
                completed,
                ..
            } => {
                if external_ids.imdb.is_none()
                    && external_ids.tmdb.is_none()
                    && external_ids.tvdb.is_none()
                {
                    return Ok(());
                }
                let endpoint = if *completed || *progress_pct >= 80.0 {
                    "scrobble/stop"
                } else {
                    "scrobble/pause"
                };
                let ids = ids_json(external_ids);
                let payload = if *media_type == MediaType::Movie {
                    serde_json::json!({
                        "movie": { "title": title, "ids": ids },
                        "progress": (*progress_pct).clamp(0.0, 100.0)
                    })
                } else {
                    serde_json::json!({
                        "show": { "title": title, "ids": ids },
                        "episode": { "season": season, "number": episode },
                        "progress": (*progress_pct).clamp(0.0, 100.0)
                    })
                };
                let resp = self
                    .http
                    .post(format!("{}/{endpoint}", self.api_base))
                    .bearer_auth(access_token)
                    .header("trakt-api-version", "2")
                    .header("trakt-api-key", cid)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|err| VesselError::Network(format!("trakt scrobble: {err}")))?;
                if !resp.status().is_success() {
                    let st = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    return Err(VesselError::Network(format!(
                        "trakt {endpoint} HTTP {st}: {}",
                        body.trim()
                    )));
                }
                Ok(())
            }
            TrackerSyncAction::SetLibraryStatus {
                external_ids,
                title,
                media_type,
                status,
                ..
            } => {
                if external_ids.imdb.is_none()
                    && external_ids.tmdb.is_none()
                    && external_ids.tvdb.is_none()
                {
                    return Ok(());
                }
                let path = match status {
                    TrackerSyncStatus::PlanToWatch => "sync/watchlist",
                    TrackerSyncStatus::Completed => "sync/history",
                    _ => return Ok(()),
                };
                let ids = ids_json(external_ids);
                let payload = if *media_type == MediaType::Movie {
                    serde_json::json!({ "movies": [{ "title": title, "ids": ids }] })
                } else {
                    serde_json::json!({ "shows": [{ "title": title, "ids": ids }] })
                };
                let resp = self
                    .http
                    .post(format!("{}/{path}", self.api_base))
                    .bearer_auth(access_token)
                    .header("trakt-api-version", "2")
                    .header("trakt-api-key", cid)
                    .json(&payload)
                    .send()
                    .await
                    .map_err(|err| VesselError::Network(format!("trakt {path}: {err}")))?;
                if !resp.status().is_success() {
                    let st = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    return Err(VesselError::Network(format!(
                        "trakt {path} HTTP {st}: {}",
                        body.trim()
                    )));
                }
                Ok(())
            }
        }
    }
}
