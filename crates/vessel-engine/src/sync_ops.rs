//! OAuth 2.0, media tracker scrobbling, and Google Drive cloud sync orchestration on `VesselEngine`.

use std::sync::Arc;

use vessel_domain::{
    CloudSyncStatus, EngineEvent, ExternalIds, LibraryItem, MediaType, OAuthClientConfig,
    OAuthDevicePrompt, OAuthFlowKind, OAuthProviderKind, OAuthStartResult, PlaybackProgress,
    TrackerAccountStatus, TrackerProviderKind, TrackerSyncAction, TrackerSyncStatus, VesselError,
    VesselResult,
};

use crate::VesselEngine;
use crate::drive_sync::CloudSyncResult;

impl VesselEngine {
    /// Stores OAuth 2.0 client credentials (`client_id` / optional `client_secret`) in `SecretStore`.
    pub async fn save_oauth_client_config(
        &self,
        provider: OAuthProviderKind,
        config: OAuthClientConfig,
    ) -> VesselResult<()> {
        self.oauth.save_client_config(provider, config).await
    }

    /// Returns `true` when `provider` has saved OAuth client credentials.
    pub async fn has_oauth_client_config(&self, provider: OAuthProviderKind) -> VesselResult<bool> {
        Ok(self.oauth.get_client_config(provider).await?.is_some())
    }

    /// Starts a Desktop loopback HTTP OAuth + PKCE browser login session and refreshes profile on finish.
    pub async fn start_oauth_loopback_session(
        &self,
        provider: OAuthProviderKind,
    ) -> VesselResult<OAuthStartResult> {
        let (start, rx) = self.oauth.start_loopback_pkce_session(provider).await?;
        let trackers = Arc::clone(&self.trackers);
        let drive_sync = Arc::clone(&self.drive_sync);
        let sink = Arc::clone(&self.event_sink);

        tokio::spawn(async move {
            match rx.await {
                Ok(Ok(_bundle)) => {
                    post_auth_refresh(provider, &trackers, &drive_sync).await;
                    sink.emit(EngineEvent::OAuthCompleted {
                        provider,
                        success: true,
                        error: None,
                    });
                }
                Ok(Err(err)) => {
                    sink.emit(EngineEvent::OAuthCompleted {
                        provider,
                        success: false,
                        error: Some(err.to_string()),
                    });
                }
                Err(_) => {}
            }
        });

        Ok(start)
    }

    /// Starts an Android App Link / browser OAuth flow for `provider`.
    pub async fn start_oauth_browser_flow(
        &self,
        provider: OAuthProviderKind,
        flow: OAuthFlowKind,
        redirect_uri: &str,
    ) -> VesselResult<OAuthStartResult> {
        self.oauth
            .start_browser_flow(provider, flow, redirect_uri)
            .await
    }

    /// Completes an Android `vessel://oauth/<provider>?code=...&state=...` redirect URI.
    pub async fn complete_oauth_callback_uri(
        &self,
        callback_uri: &str,
    ) -> VesselResult<OAuthProviderKind> {
        let provider = parse_oauth_provider_from_uri(callback_uri)?;
        let res = self.oauth.complete_callback_uri(callback_uri).await;
        match res {
            Ok(_) => {
                post_auth_refresh(provider, &self.trackers, &self.drive_sync).await;
                self.event_sink.emit(EngineEvent::OAuthCompleted {
                    provider,
                    success: true,
                    error: None,
                });
                Ok(provider)
            }
            Err(err) => {
                self.event_sink.emit(EngineEvent::OAuthCompleted {
                    provider,
                    success: false,
                    error: Some(err.to_string()),
                });
                Err(err)
            }
        }
    }

    /// Starts an OAuth 2.0 Device Code flow (TV / QR login).
    pub async fn start_oauth_device_code(
        &self,
        provider: OAuthProviderKind,
    ) -> VesselResult<OAuthDevicePrompt> {
        self.oauth.start_device_code_flow(provider).await
    }

    /// Polls an OAuth 2.0 Device Code once; returns `true` when authorization completes.
    pub async fn poll_oauth_device_code(
        &self,
        provider: OAuthProviderKind,
        device_code: &str,
    ) -> VesselResult<bool> {
        let Some(_bundle) = self.oauth.poll_device_code(provider, device_code).await? else {
            return Ok(false);
        };
        post_auth_refresh(provider, &self.trackers, &self.drive_sync).await;
        self.event_sink.emit(EngineEvent::OAuthCompleted {
            provider,
            success: true,
            error: None,
        });
        Ok(true)
    }

    /// Lists connection and sync status for all 5 media trackers.
    pub async fn list_tracker_accounts(&self) -> VesselResult<Vec<TrackerAccountStatus>> {
        self.trackers.list_accounts_status().await
    }

    /// Updates auto-scrobble / library sync preferences for a tracker.
    pub async fn set_tracker_preferences(
        &self,
        provider: TrackerProviderKind,
        auto_scrobble: bool,
        sync_library: bool,
    ) -> VesselResult<()> {
        self.trackers
            .set_preferences(provider, auto_scrobble, sync_library)
            .await
    }

    /// Disconnects a tracker account and deletes its OAuth tokens & sync queue.
    pub async fn disconnect_tracker(&self, provider: TrackerProviderKind) -> VesselResult<()> {
        self.trackers.disconnect(provider).await
    }

    /// Retries any queued offline tracker scrobble/library actions.
    pub async fn flush_tracker_queue(&self) -> VesselResult<usize> {
        self.trackers.flush_queue().await
    }

    /// Reads current Google Drive cloud sync status and tombstone count.
    pub async fn get_cloud_sync_status(&self) -> VesselResult<CloudSyncStatus> {
        self.drive_sync.status().await
    }

    /// Enables or disables automatic background Google Drive sync.
    pub async fn set_cloud_sync_auto(&self, enabled: bool) -> VesselResult<()> {
        self.drive_sync.set_auto_sync(enabled).await
    }

    /// Executes an immediate two-way LWW + tombstone sync with Google Drive `appDataFolder`.
    pub async fn sync_google_drive_now(&self) -> VesselResult<CloudSyncResult> {
        self.drive_sync.sync_now().await
    }

    /// Disconnects Google Drive sync and deletes its OAuth tokens.
    pub async fn disconnect_google_drive(&self) -> VesselResult<()> {
        self.drive_sync.disconnect().await
    }

    pub(crate) fn spawn_progress_tracker_dispatch(&self, progress: &PlaybackProgress) {
        if progress.media_type_is_live() {
            return;
        }
        let pct = if progress.duration_ms > 0 {
            ((progress.position_ms as f64 / progress.duration_ms as f64) * 100.0) as f32
        } else {
            0.0
        };
        if !progress.completed && pct < 1.0 {
            return;
        }

        let trackers = Arc::clone(&self.trackers);
        let storage = Arc::clone(&self.storage);
        let canonical = progress.canonical_id.clone();
        let plugin_id = progress.media.plugin_id.as_str().to_owned();
        let native_id = progress.media.native_id.clone();
        let title = progress
            .title
            .clone()
            .unwrap_or_else(|| progress.media.native_id.clone());
        let season = progress.season;
        let episode = progress.episode;
        let absolute_episode = progress.absolute_episode;
        let completed = progress.completed;

        tokio::spawn(async move {
            let mut ids = parse_external_ids_from_canonical(canonical.as_deref());
            let meta_key = format!("episodes:{plugin_id}:{native_id}");
            let mut is_anime = ids.anilist.is_some() || ids.mal.is_some();
            let mut media_type = MediaType::Series;
            if let Ok(Some(raw)) = storage.get_metadata_cache(&meta_key).await
                && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw)
            {
                if val.get("type").and_then(|v| v.as_str()) == Some("movie") {
                    media_type = MediaType::Movie;
                }
                if let Some(ext) = val.get("externalIds")
                    && let Ok(parsed) = serde_json::from_value::<ExternalIds>(ext.clone())
                {
                    merge_external_ids(&mut ids, &parsed);
                    is_anime = is_anime || ids.anilist.is_some() || ids.mal.is_some();
                }
            }
            if ids.is_empty() {
                return;
            }
            let action = TrackerSyncAction::Scrobble {
                external_ids: ids,
                title,
                media_type,
                is_anime,
                season,
                episode,
                absolute_episode,
                progress_pct: pct,
                completed,
            };
            trackers.dispatch_or_enqueue(&action).await;
        });
    }

    pub(crate) fn spawn_library_tracker_dispatch(&self, item: &LibraryItem) {
        if item.media_type == MediaType::Live {
            return;
        }
        let ids = parse_external_ids_from_canonical(item.canonical_id.as_deref());
        if ids.is_empty() {
            return;
        }
        let trackers = Arc::clone(&self.trackers);
        let action = TrackerSyncAction::SetLibraryStatus {
            external_ids: ids,
            title: item.title.clone(),
            media_type: item.media_type,
            is_anime: item.is_anime,
            status: TrackerSyncStatus::from(item.status),
        };
        tokio::spawn(async move {
            trackers.dispatch_or_enqueue(&action).await;
        });
    }
}

trait ProgressLiveExt {
    fn media_type_is_live(&self) -> bool;
}

impl ProgressLiveExt for PlaybackProgress {
    fn media_type_is_live(&self) -> bool {
        self.stream_id.starts_with("tv:")
            || self.stream_id.starts_with("tv_")
            || self.stream_id.starts_with("iptv_")
            || self.stream_id.starts_with("vns::tv::")
    }
}

async fn post_auth_refresh(
    provider: OAuthProviderKind,
    trackers: &vessel_trackers::TrackerManager,
    drive_sync: &crate::drive_sync::GoogleDriveSyncClient,
) {
    match provider {
        OAuthProviderKind::GoogleDrive => {
            let _ = drive_sync.refresh_account_email().await;
        }
        OAuthProviderKind::Trakt => {
            let _ = trackers.refresh_profile(TrackerProviderKind::Trakt).await;
        }
        OAuthProviderKind::Anilist => {
            let _ = trackers.refresh_profile(TrackerProviderKind::Anilist).await;
        }
        OAuthProviderKind::Simkl => {
            let _ = trackers.refresh_profile(TrackerProviderKind::Simkl).await;
        }
        OAuthProviderKind::MyAnimeList => {
            let _ = trackers
                .refresh_profile(TrackerProviderKind::MyAnimeList)
                .await;
        }
        OAuthProviderKind::Kitsu => {
            let _ = trackers.refresh_profile(TrackerProviderKind::Kitsu).await;
        }
    }
}

fn parse_oauth_provider_from_uri(uri: &str) -> VesselResult<OAuthProviderKind> {
    let parsed = url::Url::parse(uri)
        .map_err(|err| VesselError::InvalidInput(format!("invalid oauth callback uri: {err}")))?;
    let candidate = parsed
        .path_segments()
        .and_then(|mut s| s.next())
        .filter(|seg| !seg.is_empty())
        .or_else(|| parsed.host_str());
    match candidate.unwrap_or_default().to_ascii_lowercase().as_str() {
        "google_drive" | "google-drive" | "gdrive" | "drive" => Ok(OAuthProviderKind::GoogleDrive),
        "trakt" => Ok(OAuthProviderKind::Trakt),
        "simkl" => Ok(OAuthProviderKind::Simkl),
        "anilist" => Ok(OAuthProviderKind::Anilist),
        "myanimelist" | "mal" => Ok(OAuthProviderKind::MyAnimeList),
        "kitsu" => Ok(OAuthProviderKind::Kitsu),
        other => Err(VesselError::InvalidInput(format!(
            "unknown OAuth provider in callback URI: '{other}'"
        ))),
    }
}

fn merge_external_ids(dst: &mut ExternalIds, src: &ExternalIds) {
    dst.tmdb = dst.tmdb.or(src.tmdb);
    if dst.imdb.is_none() {
        dst.imdb.clone_from(&src.imdb);
    }
    dst.tvdb = dst.tvdb.or(src.tvdb);
    dst.anilist = dst.anilist.or(src.anilist);
    dst.kitsu = dst.kitsu.or(src.kitsu);
    dst.mal = dst.mal.or(src.mal);
    dst.simkl = dst.simkl.or(src.simkl);
}

fn parse_external_ids_from_canonical(canonical_id: Option<&str>) -> ExternalIds {
    let mut ids = ExternalIds::default();
    let Some(raw) = canonical_id else {
        return ids;
    };
    if let Some(rest) = raw.strip_prefix("imdb:") {
        ids.imdb = Some(rest.to_owned());
    } else if let Some(rest) = raw.strip_prefix("tmdb:movie:") {
        ids.tmdb = rest.parse().ok();
    } else if let Some(rest) = raw.strip_prefix("tmdb:tv:") {
        ids.tmdb = rest.parse().ok();
    } else if let Some(rest) = raw.strip_prefix("anilist:") {
        ids.anilist = rest.parse().ok();
    } else if let Some(rest) = raw.strip_prefix("mal:") {
        ids.mal = rest.parse().ok();
    } else if let Some(rest) = raw.strip_prefix("kitsu:") {
        ids.kitsu = rest.parse().ok();
    } else if let Some(rest) = raw.strip_prefix("simkl:") {
        ids.simkl = rest.parse().ok();
    }
    ids
}
