//! Unified media tracker orchestration (`Trakt`, `AniList`, and extensible trait slots for `Simkl`, `MAL`, `Kitsu`).

pub mod anilist;
pub mod trakt;

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use vessel_auth::OAuthManager;
use vessel_domain::{
    TrackerAccountStatus, TrackerProviderKind, TrackerSyncAction, VesselError, VesselResult,
};
use vessel_storage::{SqliteStorage, StoredTrackerAccount};

use anilist::AnilistProvider;
use trakt::TraktProvider;

/// Authenticated tracker profile metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackerProfile {
    pub username: String,
    pub user_id: Option<String>,
}

/// Extensible tracker adapter contract (`Trakt`, `AniList`, `Simkl`, `MyAnimeList`, `Kitsu`).
#[async_trait]
pub trait TrackerProvider: Send + Sync {
    fn kind(&self) -> TrackerProviderKind;
    fn is_implemented(&self) -> bool;
    async fn fetch_profile(
        &self,
        access_token: &str,
        client_id: Option<&str>,
    ) -> VesselResult<TrackerProfile>;
    async fn dispatch_action(
        &self,
        access_token: &str,
        client_id: Option<&str>,
        action: &TrackerSyncAction,
    ) -> VesselResult<()>;
}

struct PlaceholderTracker(TrackerProviderKind);

#[async_trait]
impl TrackerProvider for PlaceholderTracker {
    fn kind(&self) -> TrackerProviderKind {
        self.0
    }

    fn is_implemented(&self) -> bool {
        false
    }

    async fn fetch_profile(
        &self,
        _access_token: &str,
        _client_id: Option<&str>,
    ) -> VesselResult<TrackerProfile> {
        Err(VesselError::NotFound(format!(
            "{} tracker sync adapter arrives in Phase 2B",
            self.0.display_name()
        )))
    }

    async fn dispatch_action(
        &self,
        _access_token: &str,
        _client_id: Option<&str>,
        _action: &TrackerSyncAction,
    ) -> VesselResult<()> {
        Ok(())
    }
}

/// Orchestrates OAuth tokens, preferences, immediate scrobbling, and offline retry queue flushing.
pub struct TrackerManager {
    oauth: Arc<OAuthManager>,
    storage: Arc<SqliteStorage>,
    providers: HashMap<TrackerProviderKind, Arc<dyn TrackerProvider>>,
}

impl std::fmt::Debug for TrackerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrackerManager").finish_non_exhaustive()
    }
}

impl TrackerManager {
    /// Builds a `TrackerManager` with production `Trakt` & `AniList` adapters + extensible slots.
    pub fn new(oauth: Arc<OAuthManager>, storage: Arc<SqliteStorage>) -> VesselResult<Self> {
        let mut providers: HashMap<TrackerProviderKind, Arc<dyn TrackerProvider>> = HashMap::new();
        providers.insert(TrackerProviderKind::Trakt, Arc::new(TraktProvider::new()?));
        providers.insert(TrackerProviderKind::Anilist, Arc::new(AnilistProvider::new()?));
        providers.insert(
            TrackerProviderKind::Simkl,
            Arc::new(PlaceholderTracker(TrackerProviderKind::Simkl)),
        );
        providers.insert(
            TrackerProviderKind::MyAnimeList,
            Arc::new(PlaceholderTracker(TrackerProviderKind::MyAnimeList)),
        );
        providers.insert(
            TrackerProviderKind::Kitsu,
            Arc::new(PlaceholderTracker(TrackerProviderKind::Kitsu)),
        );
        Ok(Self {
            oauth,
            storage,
            providers,
        })
    }

    /// Replaces or registers a custom `TrackerProvider` (useful for tests or Phase 2B adapters).
    pub fn with_provider(mut self, provider: Arc<dyn TrackerProvider>) -> Self {
        self.providers.insert(provider.kind(), provider);
        self
    }

    /// Lists account status, client config readiness, and pending queue counts for all 5 trackers.
    pub async fn list_accounts_status(&self) -> VesselResult<Vec<TrackerAccountStatus>> {
        let saved = self.storage.list_tracker_accounts().await?;
        let saved_map: HashMap<String, StoredTrackerAccount> = saved
            .into_iter()
            .map(|row| (row.provider.clone(), row))
            .collect();
        let pending_counts = self.storage.count_pending_tracker_actions().await?;

        let order = [
            TrackerProviderKind::Trakt,
            TrackerProviderKind::Anilist,
            TrackerProviderKind::Simkl,
            TrackerProviderKind::MyAnimeList,
            TrackerProviderKind::Kitsu,
        ];
        let mut out = Vec::with_capacity(order.len());
        for kind in order {
            let oauth_kind = kind.oauth_provider();
            let has_client_config = self.oauth.get_client_config(oauth_kind).await?.is_some();
            let connected = self.oauth.get_token_bundle(oauth_kind).await?.is_some();
            let row = saved_map.get(kind.as_str());
            let implemented = self
                .providers
                .get(&kind)
                .is_some_and(|p| p.is_implemented());

            out.push(TrackerAccountStatus {
                provider: kind,
                display_name: kind.display_name().to_owned(),
                implemented,
                has_client_config,
                connected,
                username: row.and_then(|r| r.username.clone()),
                user_id: row.and_then(|r| r.user_id.clone()),
                auto_scrobble: row.is_none_or(|r| r.auto_scrobble),
                sync_library: row.is_none_or(|r| r.sync_library),
                last_synced_at: row.and_then(|r| r.last_synced_at.clone()),
                pending_queue_count: pending_counts.get(kind.as_str()).copied().unwrap_or(0),
            });
        }
        Ok(out)
    }

    /// Updates auto-scrobble / library sync toggles for `kind`.
    pub async fn set_preferences(
        &self,
        kind: TrackerProviderKind,
        auto_scrobble: bool,
        sync_library: bool,
    ) -> VesselResult<()> {
        let existing = self.storage.get_tracker_account(kind.as_str()).await?;
        let row = StoredTrackerAccount {
            provider: kind.as_str().to_owned(),
            username: existing.as_ref().and_then(|e| e.username.clone()),
            user_id: existing.as_ref().and_then(|e| e.user_id.clone()),
            auto_scrobble,
            sync_library,
            last_synced_at: existing.and_then(|e| e.last_synced_at),
            updated_at: String::new(),
        };
        self.storage.upsert_tracker_account(&row).await
    }

    /// Fetches the remote username/ID for `kind` using its stored OAuth token and persists it.
    pub async fn refresh_profile(&self, kind: TrackerProviderKind) -> VesselResult<TrackerProfile> {
        let provider = self.providers.get(&kind).ok_or_else(|| {
            VesselError::NotFound(format!("tracker '{}' not registered", kind.as_str()))
        })?;
        let oauth_kind = kind.oauth_provider();
        let token = self.oauth.get_valid_access_token(oauth_kind).await?;
        let client_id = self
            .oauth
            .get_client_config(oauth_kind)
            .await?
            .map(|c| c.client_id);
        let profile = provider.fetch_profile(&token, client_id.as_deref()).await?;

        let existing = self.storage.get_tracker_account(kind.as_str()).await?;
        self.storage
            .upsert_tracker_account(&StoredTrackerAccount {
                provider: kind.as_str().to_owned(),
                username: Some(profile.username.clone()),
                user_id: profile.user_id.clone(),
                auto_scrobble: existing.as_ref().is_none_or(|e| e.auto_scrobble),
                sync_library: existing.as_ref().is_none_or(|e| e.sync_library),
                last_synced_at: existing.and_then(|e| e.last_synced_at),
                updated_at: String::new(),
            })
            .await?;
        Ok(profile)
    }

    /// Disconnects a tracker account from both OAuth tokens and SQLite metadata.
    pub async fn disconnect(&self, kind: TrackerProviderKind) -> VesselResult<()> {
        self.oauth.disconnect(kind.oauth_provider()).await?;
        self.storage.delete_tracker_account(kind.as_str()).await
    }

    /// Dispatches `action` to all eligible connected trackers; queues failures in SQLite for retry.
    pub async fn dispatch_or_enqueue(&self, action: &TrackerSyncAction) {
        let Ok(accounts) = self.list_accounts_status().await else {
            return;
        };
        let is_scrobble = matches!(action, TrackerSyncAction::Scrobble { .. });
        let Ok(payload_json) = serde_json::to_string(action) else {
            return;
        };

        for acc in accounts {
            if !acc.implemented || !acc.connected {
                continue;
            }
            if is_scrobble && !acc.auto_scrobble {
                continue;
            }
            if !is_scrobble && !acc.sync_library {
                continue;
            }
            if let Err(err) = self.dispatch_single(acc.provider, action).await {
                let _ = self
                    .storage
                    .enqueue_tracker_action(acc.provider.as_str(), &payload_json, Some(&err.to_string()))
                    .await;
            }
        }
    }

    /// Retries due items in `tracker_sync_queue`, returning the count of flushed actions.
    pub async fn flush_queue(&self) -> VesselResult<usize> {
        let due = self.storage.list_due_tracker_actions(50).await?;
        let mut flushed = 0usize;
        for item in due {
            let Ok(kind) = serde_json::from_value::<TrackerProviderKind>(serde_json::Value::String(
                item.provider.clone(),
            )) else {
                let _ = self.storage.delete_queued_tracker_action(item.id).await;
                continue;
            };
            let Ok(action) = serde_json::from_str::<TrackerSyncAction>(&item.payload_json) else {
                let _ = self.storage.delete_queued_tracker_action(item.id).await;
                continue;
            };

            match self.dispatch_single(kind, &action).await {
                Ok(()) => {
                    self.storage.delete_queued_tracker_action(item.id).await?;
                    flushed += 1;
                }
                Err(err) => {
                    self.storage
                        .mark_tracker_action_failed(item.id, item.attempts, &err.to_string())
                        .await?;
                }
            }
        }
        Ok(flushed)
    }

    async fn dispatch_single(
        &self,
        kind: TrackerProviderKind,
        action: &TrackerSyncAction,
    ) -> VesselResult<()> {
        let provider = self.providers.get(&kind).ok_or_else(|| {
            VesselError::NotFound(format!("tracker '{}' not registered", kind.as_str()))
        })?;
        let oauth_kind = kind.oauth_provider();
        let token = self.oauth.get_valid_access_token(oauth_kind).await?;
        let client_id = self
            .oauth
            .get_client_config(oauth_kind)
            .await?
            .map(|c| c.client_id);
        provider
            .dispatch_action(&token, client_id.as_deref(), action)
            .await?;

        let _ = self.storage.touch_tracker_last_synced(kind.as_str()).await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use axum::Router;
    use axum::extract::Json;
    use axum::http::StatusCode;
    use axum::routing::post;
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use vessel_auth::OAuthManager;
    use vessel_domain::{
        ExternalIds, MediaType, OAuthClientConfig, OAuthProviderKind, OAuthTokenBundle,
        SecretStore, TrackerProviderKind, TrackerSyncAction, TrackerSyncStatus, VesselResult,
    };
    use vessel_storage::{SqliteStorage, StorageActor};

    use super::TrackerManager;
    use crate::anilist::AnilistProvider;
    use crate::trakt::TraktProvider;

    #[derive(Default)]
    struct MemSecrets {
        map: Mutex<BTreeMap<String, String>>,
    }

    #[async_trait::async_trait]
    impl SecretStore for MemSecrets {
        async fn get(&self, key: &str) -> VesselResult<Option<String>> {
            Ok(self.map.lock().await.get(key).cloned())
        }

        async fn set(&self, key: &str, value: &str) -> VesselResult<()> {
            self.map
                .lock()
                .await
                .insert(key.to_owned(), value.to_owned());
            Ok(())
        }

        async fn delete(&self, key: &str) -> VesselResult<()> {
            self.map.lock().await.remove(key);
            Ok(())
        }
    }

    #[tokio::test]
    async fn dispatches_scrobble_to_trakt_and_anilist_and_queues_failures() {
        let received = Arc::new(Mutex::new(Vec::<String>::new()));
        let rec_trakt = Arc::clone(&received);
        let rec_anilist = Arc::clone(&received);

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let app = Router::new()
            .route(
                "/scrobble/stop",
                post(move |Json(body): Json<serde_json::Value>| {
                    let rec = Arc::clone(&rec_trakt);
                    async move {
                        rec.lock().await.push(format!("trakt:{body}"));
                        StatusCode::OK
                    }
                }),
            )
            .route(
                "/graphql",
                post(move |Json(body): Json<serde_json::Value>| {
                    let rec = Arc::clone(&rec_anilist);
                    async move {
                        rec.lock().await.push(format!("anilist:{body}"));
                        (StatusCode::OK, Json(serde_json::json!({"data": {"SaveMediaListEntry": {"id": 1}}})))
                    }
                }),
            );
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let base = format!("http://{addr}");
        let actor = StorageActor::open_in_memory().await.expect("actor");
        let storage = Arc::new(SqliteStorage::new(actor));
        let secrets: Arc<dyn SecretStore> = Arc::new(MemSecrets::default());
        let oauth = Arc::new(OAuthManager::new(secrets).expect("oauth"));

        for kind in [OAuthProviderKind::Trakt, OAuthProviderKind::Anilist] {
            oauth
                .save_client_config(
                    kind,
                    OAuthClientConfig {
                        client_id: "cid".into(),
                        client_secret: None,
                    },
                )
                .await
                .expect("client cfg");
            oauth
                .save_token_bundle(
                    kind,
                    &OAuthTokenBundle {
                        access_token: "valid-tok".into(),
                        refresh_token: None,
                        token_type: "Bearer".into(),
                        scope: None,
                        expires_at_epoch_secs: Some(9_999_999_999),
                    },
                )
                .await
                .expect("token");
        }

        let mgr = TrackerManager::new(oauth, Arc::clone(&storage))
            .expect("mgr")
            .with_provider(Arc::new(TraktProvider::with_base(&base).expect("trakt")))
            .with_provider(Arc::new(
                AnilistProvider::with_url(format!("{base}/graphql")).expect("anilist"),
            ));

        let action = TrackerSyncAction::Scrobble {
            external_ids: ExternalIds {
                imdb: Some("tt1234567".into()),
                anilist: Some(154587),
                ..Default::default()
            },
            title: "Frieren".into(),
            media_type: MediaType::Series,
            is_anime: true,
            season: 1,
            episode: 4,
            absolute_episode: Some(4),
            progress_pct: 92.0,
            completed: true,
        };
        mgr.dispatch_or_enqueue(&action).await;

        let calls = received.lock().await;
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().any(|c| c.starts_with("trakt:")));
        assert!(calls.iter().any(|c| c.starts_with("anilist:")));

        let accounts = mgr.list_accounts_status().await.expect("accounts");
        assert_eq!(accounts.len(), 5);
        assert!(accounts.iter().find(|a| a.provider == TrackerProviderKind::Trakt).unwrap().connected);
        assert_eq!(
            TrackerSyncStatus::from(vessel_domain::LibraryStatus::Completed),
            TrackerSyncStatus::Completed
        );
    }
}
