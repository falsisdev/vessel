//! AniList GraphQL v2 tracker adapter (`SaveMediaListEntry` + `Viewer`).

use async_trait::async_trait;
use serde::Deserialize;
use vessel_domain::{
    TrackerProviderKind, TrackerSyncAction, TrackerSyncStatus, VesselError, VesselResult,
    http_client_builder,
};

use crate::{TrackerProfile, TrackerProvider};

pub struct AnilistProvider {
    http: reqwest::Client,
    graphql_url: String,
}

impl AnilistProvider {
    pub fn new() -> VesselResult<Self> {
        Self::with_url("https://graphql.anilist.co")
    }

    pub fn with_url(graphql_url: impl Into<String>) -> VesselResult<Self> {
        let http = http_client_builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|err| VesselError::Network(format!("build anilist http client: {err}")))?;
        Ok(Self {
            http,
            graphql_url: graphql_url.into(),
        })
    }
}

fn map_anilist_status(status: TrackerSyncStatus) -> &'static str {
    match status {
        TrackerSyncStatus::Watching => "CURRENT",
        TrackerSyncStatus::Completed => "COMPLETED",
        TrackerSyncStatus::PlanToWatch => "PLANNING",
        TrackerSyncStatus::OnHold => "PAUSED",
        TrackerSyncStatus::Dropped => "DROPPED",
    }
}

#[async_trait]
impl TrackerProvider for AnilistProvider {
    fn kind(&self) -> TrackerProviderKind {
        TrackerProviderKind::Anilist
    }

    fn is_implemented(&self) -> bool {
        true
    }

    async fn fetch_profile(
        &self,
        access_token: &str,
        _client_id: Option<&str>,
    ) -> VesselResult<TrackerProfile> {
        let query = r#"query { Viewer { id name } }"#;
        let resp = self
            .http
            .post(&self.graphql_url)
            .bearer_auth(access_token)
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("anilist viewer request: {err}")))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|err| VesselError::Network(format!("anilist viewer read: {err}")))?;
        if !status.is_success() {
            return Err(VesselError::Network(format!(
                "anilist viewer HTTP {status}: {}",
                text.trim()
            )));
        }

        #[derive(Deserialize)]
        struct Viewer {
            id: u64,
            name: String,
        }
        #[derive(Deserialize)]
        struct Data {
            #[serde(rename = "Viewer")]
            viewer: Viewer,
        }
        #[derive(Deserialize)]
        struct Envelope {
            data: Data,
        }

        let parsed: Envelope = serde_json::from_str(&text)
            .map_err(|err| VesselError::Network(format!("decode anilist viewer: {err}")))?;
        Ok(TrackerProfile {
            username: parsed.data.viewer.name,
            user_id: Some(parsed.data.viewer.id.to_string()),
        })
    }

    async fn dispatch_action(
        &self,
        access_token: &str,
        _client_id: Option<&str>,
        action: &TrackerSyncAction,
    ) -> VesselResult<()> {
        let (media_id, status_str, progress) = match action {
            TrackerSyncAction::Scrobble {
                external_ids,
                episode,
                absolute_episode,
                progress_pct,
                completed,
                ..
            } => {
                let Some(id) = external_ids.anilist else {
                    return Ok(());
                };
                if !*completed && *progress_pct < 80.0 {
                    return Ok(());
                }
                let ep_num = absolute_episode.unwrap_or(*episode).max(1);
                (id, "CURRENT", Some(ep_num))
            }
            TrackerSyncAction::SetLibraryStatus {
                external_ids,
                status,
                ..
            } => {
                let Some(id) = external_ids.anilist else {
                    return Ok(());
                };
                (id, map_anilist_status(*status), None)
            }
        };

        let mutation = r#"
            mutation ($mediaId: Int!, $status: MediaListStatus, $progress: Int) {
                SaveMediaListEntry(mediaId: $mediaId, status: $status, progress: $progress) {
                    id
                    status
                    progress
                }
            }
        "#;
        let mut vars = serde_json::json!({
            "mediaId": media_id,
            "status": status_str,
        });
        if let Some(ep) = progress {
            vars["progress"] = serde_json::Value::Number(ep.into());
        }

        let resp = self
            .http
            .post(&self.graphql_url)
            .bearer_auth(access_token)
            .json(&serde_json::json!({
                "query": mutation,
                "variables": vars,
            }))
            .send()
            .await
            .map_err(|err| VesselError::Network(format!("anilist mutation: {err}")))?;
        if !resp.status().is_success() {
            let st = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(VesselError::Network(format!(
                "anilist mutation HTTP {st}: {}",
                body.trim()
            )));
        }
        Ok(())
    }
}
