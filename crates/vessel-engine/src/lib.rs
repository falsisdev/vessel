//! Pure-Rust core orchestrator (`VesselEngine`) binding plugins, storage, proxy, and playback.

pub mod builtins;
mod debrid_ops;
mod download_ops;
pub mod drive_sync;
mod iptv_ops;
mod plugins_ops;
pub mod pool;
pub mod sync_ops;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use vessel_auth::OAuthManager;
use vessel_debrid::DebridManager;
use vessel_domain::{
    ContentProvider, DebridResolution, EngineEvent, EventSink, ExternalIds, HomeFeed, LibraryItem,
    LibraryStatus, LinkedMediaSource, LogLevel, MediaDetail, MediaRef, MediaType,
    PlaybackProgress, PlaybackRequest, PlaybackStats, PlayerBackend, PluginCapability, PluginId,
    ResolveDebridRequest, SearchPage, SecretStore, SkipSegment, StoragePort, StreamCandidate,
    StreamKind, StreamResult, SubtitleCandidate, SubtitleFormat, SubtitleProvider, SubtitleQuery,
    SubtitleSource, SubtitleTrack, UiPreferences, VesselError, VesselPaths, VesselResult,
};
use vessel_iptv::IptvClient;
use vessel_metadata::{
    CanonicalMetadata, MetaEndpoints, MetadataService, merge_detail_with_override,
};
use vessel_plugin_rt::{SourceResolver, TsCompileCache};
use vessel_proxy::ProxyServer;
use vessel_storage::{BackupImportMode, ContinueWatchingEntry, SqliteSecretStore, SqliteStorage, StorageActor};
use vessel_trackers::TrackerManager;

use drive_sync::GoogleDriveSyncClient;
pub use drive_sync::{CloudSyncResult, SyncMergeStatsReport};
pub use pool::{DEFAULT_IDLE_TTL, DEFAULT_MAX_WARM_PLUGINS, PluginPool};
pub use vessel_downloader::{DownloadManager, DownloadRequest};
pub use vessel_iptv::{EpgNowNext, EpgProgram, IptvChannel, IptvKind, IptvProfile};
pub use vessel_metadata::{
    MetadataProviderFilter, MetadataSearchHit, MetadataSearchResponse, SkipQuery,
};

/// Local playback bundle resolved through the loopback file server (`http://127.0.0.1:<port>/local/<token>/...`).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPlaybackBundle {
    pub download_id: String,
    pub url: String,
    pub kind: StreamKind,
    pub subtitles: Vec<SubtitleTrack>,
}

/// Update availability status for an installed plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginUpdateInfo {
    pub plugin_id: String,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub source_url: Option<String>,
    pub error: Option<String>,
}

/// Summary of one plugin updated in-place.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatedPluginInfo {
    pub plugin_id: String,
    pub version: String,
    pub badge: String,
}

/// Failure record for one plugin during batch update.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateFailure {
    pub plugin_id: String,
    pub error: String,
}

/// Single subtitle candidate matched with the plugin that provided it.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleSearchHit {
    pub plugin_id: String,
    pub plugin_name: String,
    pub candidate: SubtitleCandidate,
}

/// Non-fatal per-plugin error collected during parallel external subtitle search.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleProviderError {
    pub plugin_id: String,
    pub plugin_name: String,
    pub error: String,
}

/// Combined response returned by `VesselEngine::search_subtitles`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitleSearchOutcome {
    pub hits: Vec<SubtitleSearchHit>,
    pub errors: Vec<SubtitleProviderError>,
}

/// Summary of an enabled subtitle-capable plugin.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubtitlePluginInfo {
    pub plugin_id: String,
    pub name: String,
}

/// Result of updating all outdated plugins in-place.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateResult {
    pub checked: usize,
    pub updated: Vec<UpdatedPluginInfo>,
    pub failed: Vec<BatchUpdateFailure>,
}

pub(crate) fn subtitle_ext(format: SubtitleFormat) -> &'static str {
    match format {
        SubtitleFormat::Srt => "srt",
        SubtitleFormat::Ass => "ass",
        SubtitleFormat::Ssa => "ssa",
        SubtitleFormat::Vtt => "vtt",
    }
}

async fn persist_episode_snapshot(
    storage: &SqliteStorage,
    plugin_id: &PluginId,
    detail: &MediaDetail,
) {
    let media_type = match detail.media_type {
        MediaType::Movie => "movie",
        MediaType::Series => "series",
        MediaType::Live => "live",
    };
    let episodes: Vec<serde_json::Value> = detail
        .episodes
        .iter()
        .map(|ep| {
            serde_json::json!({
                "streamId": ep.stream_id,
                "season": ep.season,
                "episode": ep.episode,
                "absoluteEpisode": ep.absolute_episode,
                "title": ep.title,
            })
        })
        .collect();
    let payload = serde_json::json!({
        "title": detail.title,
        "poster": detail.poster,
        "backdrop": detail.backdrop,
        "type": media_type,
        "episodes": episodes,
    });
    let cache_key = format!("episodes:{}:{}", plugin_id.as_str(), detail.native_id);
    let _ = storage
        .upsert_metadata_cache(&cache_key, &payload.to_string(), "2099-01-01T00:00:00Z")
        .await;
}

static SEQ_ID: AtomicU64 = AtomicU64::new(1);

fn next_unique_id(prefix: &str) -> String {
    let mut entropy = [0u8; 8];
    let _ = getrandom::fill(&mut entropy);
    let seq = SEQ_ID.fetch_add(1, Ordering::Relaxed);
    let mut hex = String::with_capacity(entropy.len() * 2);
    for byte in entropy {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    format!("{prefix}-{seq}-{hex}")
}

/// Secret key name used when a user optionally configures their own TMDB v3 API key.
pub const TMDB_API_KEY_SECRET: &str = "tmdb_api_key";
/// Secret key name used when a user optionally configures their own Simkl Client ID.
pub const SIMKL_CLIENT_ID_SECRET: &str = "simkl_client_id";

/// Parameters for resolving a `StreamCandidate` and starting playback cleanly.
#[derive(Debug, Clone)]
pub struct PlayCandidateRequest {
    pub media: MediaRef,
    pub canonical_id: Option<String>,
    pub season: u32,
    pub episode: u32,
    pub absolute_episode: Option<u32>,
    pub media_type: MediaType,
    pub candidate: StreamCandidate,
    pub extra_subtitles: Vec<SubtitleTrack>,
    /// Explicit start offset; when `None`, resumes from saved `watch_progress` if incomplete.
    pub start_position_ms: Option<u64>,
    pub title: Option<String>,
}

/// Handle returned by [`VesselEngine::search_all`] for cancellation and test synchronization.
#[derive(Debug)]
pub struct SearchHandle {
    pub search_id: String,
    pub token: CancellationToken,
    pub done: JoinHandle<()>,
}

const HOME_FEED_CACHE_TTL: Duration = Duration::from_secs(180);
const DETAILS_CACHE_TTL: Duration = Duration::from_secs(300);
const MAX_HOME_FEED_CACHE_ENTRIES: usize = 24;
const MAX_DETAILS_CACHE_ENTRIES: usize = 32;

type HomeFeedCacheMap = HashMap<(PluginId, Option<String>), (Instant, HomeFeed)>;
type DetailsCacheMap = HashMap<(PluginId, String), (Instant, MediaDetail)>;

/// Pure core facade orchestrating storage, proxy, isolated QuickJS plugins, and `PlayerBackend`.
pub struct VesselEngine {
    storage: Arc<SqliteStorage>,
    secrets: Arc<dyn SecretStore>,
    proxy: Arc<ProxyServer>,
    player: Arc<dyn PlayerBackend>,
    event_sink: Arc<dyn EventSink>,
    ts_cache: Arc<TsCompileCache>,
    source_resolver: SourceResolver,
    metadata: MetadataService,
    iptv: IptvClient,
    downloader: DownloadManager,
    debrid: Arc<DebridManager>,
    oauth: Arc<OAuthManager>,
    trackers: Arc<TrackerManager>,
    drive_sync: Arc<GoogleDriveSyncClient>,
    pool: Arc<PluginPool>,
    always_proxy: AtomicBool,
    metadata_enabled: AtomicBool,
    last_stream_is_debrid: AtomicBool,
    last_stream_size_bytes: AtomicU64,
    active_search: Arc<Mutex<Option<(String, CancellationToken)>>>,
    home_feed_cache: Arc<Mutex<HomeFeedCacheMap>>,
    details_cache: Arc<Mutex<DetailsCacheMap>>,
}

impl std::fmt::Debug for VesselEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VesselEngine")
            .field("proxy_base_url", &self.proxy.base_url())
            .field("always_proxy", &self.always_proxy.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl VesselEngine {
    /// Provisions storage under `paths`, starts the local proxy server, warms TS compile cache,
    /// and binds `player` + `event_sink`.
    pub async fn open(
        paths: &VesselPaths,
        player: Arc<dyn PlayerBackend>,
        event_sink: Arc<dyn EventSink>,
        always_proxy: bool,
    ) -> VesselResult<Self> {
        let actor = StorageActor::open_with_paths(paths).await?;
        let secrets: Arc<dyn SecretStore> = Arc::new(SqliteSecretStore::new(actor.clone()));
        Self::open_with_actor_and_secrets(paths, actor, secrets, player, event_sink, always_proxy)
            .await
    }

    /// Provisions `VesselEngine` using an already-opened `StorageActor` and platform `SecretStore`
    /// (e.g. `OsKeyringSecretStore` on desktop or `AndroidKeyStore` via FFI on Android).
    pub async fn open_with_actor_and_secrets(
        paths: &VesselPaths,
        actor: StorageActor,
        secrets: Arc<dyn SecretStore>,
        player: Arc<dyn PlayerBackend>,
        event_sink: Arc<dyn EventSink>,
        always_proxy: bool,
    ) -> VesselResult<Self> {
        let storage = Arc::new(SqliteStorage::new(actor));
        let proxy = Arc::new(ProxyServer::start().await?);
        Self::from_parts_with_downloads(
            storage,
            secrets,
            proxy,
            player,
            event_sink,
            SourceResolver::new().with_default_registries(),
            paths.downloads_dir(),
            always_proxy,
        )
        .await
    }

    /// Constructs `VesselEngine` from already-initialized storage, proxy, and player components.
    pub async fn from_parts(
        storage: Arc<SqliteStorage>,
        secrets: Arc<dyn SecretStore>,
        proxy: Arc<ProxyServer>,
        player: Arc<dyn PlayerBackend>,
        event_sink: Arc<dyn EventSink>,
        source_resolver: SourceResolver,
        always_proxy: bool,
    ) -> VesselResult<Self> {
        let downloads_dir = std::env::temp_dir().join("vessel-downloads");
        Self::from_parts_with_downloads(
            storage,
            secrets,
            proxy,
            player,
            event_sink,
            source_resolver,
            downloads_dir,
            always_proxy,
        )
        .await
    }

    /// Constructs `VesselEngine` with an explicit offline downloads directory.
    #[allow(clippy::too_many_arguments)]
    pub async fn from_parts_with_downloads(
        storage: Arc<SqliteStorage>,
        secrets: Arc<dyn SecretStore>,
        proxy: Arc<ProxyServer>,
        player: Arc<dyn PlayerBackend>,
        event_sink: Arc<dyn EventSink>,
        source_resolver: SourceResolver,
        downloads_dir: PathBuf,
        always_proxy: bool,
    ) -> VesselResult<Self> {
        let cached_ts = storage.load_ts_compile_cache().await?;
        let ts_cache = Arc::new(TsCompileCache::with_entries(cached_ts));
        let storage_port: Arc<dyn StoragePort> = storage.clone();
        let pool = Arc::new(PluginPool::with_defaults(
            storage_port,
            Some(Arc::clone(&event_sink)),
            Arc::clone(&ts_cache),
        ));
        let downloader =
            DownloadManager::new(Arc::clone(&storage), Arc::clone(&event_sink), downloads_dir);
        let debrid = Arc::new(DebridManager::new(Arc::clone(&secrets))?);
        let oauth = Arc::new(OAuthManager::new(Arc::clone(&secrets))?);
        let trackers = Arc::new(TrackerManager::new(Arc::clone(&oauth), Arc::clone(&storage))?);
        let drive_sync = Arc::new(GoogleDriveSyncClient::new(
            Arc::clone(&oauth),
            Arc::clone(&storage),
        )?);

        Ok(Self {
            storage,
            secrets,
            proxy,
            player,
            event_sink,
            ts_cache,
            source_resolver,
            metadata: MetadataService::default(),
            iptv: IptvClient::new(),
            downloader,
            debrid,
            oauth,
            trackers,
            drive_sync,
            pool,
            always_proxy: AtomicBool::new(always_proxy),
            metadata_enabled: AtomicBool::new(true),
            last_stream_is_debrid: AtomicBool::new(false),
            last_stream_size_bytes: AtomicU64::new(0),
            active_search: Arc::new(Mutex::new(None)),
            home_feed_cache: Arc::new(Mutex::new(HashMap::new())),
            details_cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Replaces metadata service endpoints (useful for tests or mirror overrides).
    pub fn set_metadata_endpoints(&mut self, endpoints: MetaEndpoints) {
        self.metadata = MetadataService::new(endpoints);
    }

    /// Returns whether background external metadata enrichment (AniList/TMDB/Cinemeta) is active.
    pub fn metadata_enabled(&self) -> bool {
        self.metadata_enabled.load(Ordering::Acquire)
    }

    /// Privacy toggle disabling/enabling all outbound metadata enrichment requests.
    pub fn set_metadata_enabled(&self, enabled: bool) {
        self.metadata_enabled.store(enabled, Ordering::Release);
    }

    /// Borrows the SQLite persistence repository.
    pub fn storage(&self) -> &Arc<SqliteStorage> {
        &self.storage
    }

    /// Borrows the secret credential store.
    pub fn secrets(&self) -> &Arc<dyn SecretStore> {
        &self.secrets
    }

    /// Borrows the running local HTTP/HLS proxy server.
    pub fn proxy(&self) -> &Arc<ProxyServer> {
        &self.proxy
    }

    /// Borrows the active platform `PlayerBackend`.
    pub fn player(&self) -> &Arc<dyn PlayerBackend> {
        &self.player
    }

    /// Borrows the warm QuickJS LRU pool.
    pub fn pool(&self) -> &Arc<PluginPool> {
        &self.pool
    }

    /// Shared in-memory TypeScript compilation cache.
    pub fn ts_cache(&self) -> &Arc<TsCompileCache> {
        &self.ts_cache
    }

    /// Current offline download base directory.
    pub fn downloads_dir(&self) -> PathBuf {
        self.downloader.downloads_root()
    }

    /// Updates the offline download base directory for future downloads.
    pub fn set_downloads_dir(&self, path: impl Into<PathBuf>) -> VesselResult<()> {
        self.downloader.set_downloads_root(path)
    }

    /// Current `Always Proxy` routing override setting.
    pub fn always_proxy(&self) -> bool {
        self.always_proxy.load(Ordering::Acquire)
    }

    /// Updates whether plain `.mp4`/`.mkv` streams are also forced through the local proxy.
    pub fn set_always_proxy(&self, enabled: bool) {
        self.always_proxy.store(enabled, Ordering::Release);
    }

    /// Resolves any repository or plugin source (`https://`, `gh:`, `gl:`, shortcode, `vessel://`).
    /// Reads declarative modular UI preferences (`ui_preferences`) from SQLite.
    pub async fn get_ui_preferences(&self) -> VesselResult<UiPreferences> {
        self.storage.get_ui_preferences().await
    }

    /// Persists declarative modular UI preferences (`ui_preferences`) into SQLite.
    pub async fn set_ui_preferences(&self, prefs: &UiPreferences) -> VesselResult<()> {
        self.storage.set_ui_preferences(prefs).await
    }

    /// Reads an auxiliary app-level setting from SQLite `settings` (e.g. `app_setting:*`).
    pub async fn get_app_setting(&self, key: &str) -> VesselResult<Option<String>> {
        let trimmed = key.trim();
        if trimmed.is_empty() {
            return Err(VesselError::InvalidInput(
                "setting key cannot be empty".into(),
            ));
        }
        self.storage.get_setting_raw(trimmed).await
    }

    /// Persists an auxiliary app-level setting into SQLite `settings` so it survives upgrades and backups.
    pub async fn set_app_setting(&self, key: &str, value_json: &str) -> VesselResult<()> {
        let trimmed = key.trim();
        if trimmed.is_empty() || trimmed == "ui_preferences" {
            return Err(VesselError::InvalidInput(
                "invalid app setting key".into(),
            ));
        }
        self.storage.set_setting_raw(trimmed, value_json).await
    }

    /// Lists library items optionally filtered by `LibraryStatus`.
    pub async fn list_library(
        &self,
        status: Option<LibraryStatus>,
    ) -> VesselResult<Vec<LibraryItem>> {
        self.storage.list_library_items(status).await
    }

    /// Inserts or updates a user library entry.
    pub async fn upsert_library(&self, item: &LibraryItem) -> VesselResult<()> {
        self.storage.upsert_library(item).await?;
        self.spawn_library_tracker_dispatch(item);
        Ok(())
    }

    /// Removes a media reference from the user library.
    pub async fn delete_library_item(&self, media: &MediaRef) -> VesselResult<()> {
        self.storage.delete_library_item(media).await
    }

    /// Exports a `.backup.vessel` JSON snapshot.
    pub async fn export_backup_json(&self, include_secrets: bool) -> VesselResult<String> {
        self.storage
            .export_backup_json_with_secrets(include_secrets, self.secrets.as_ref())
            .await
    }

    /// Imports a `.backup.vessel` JSON snapshot using `SmartMerge` or `ResetAndImport`.
    pub async fn import_backup_json(&self, json: &str, mode: BackupImportMode) -> VesselResult<()> {
        self.storage
            .import_backup_json_with_secrets(json, mode, self.secrets.as_ref())
            .await
    }

    /// Reads combined user settings (`settings` + `SecretStore` secret fields) for `plugin_id`.
    pub async fn get_plugin_settings(
        &self,
        plugin_id: &PluginId,
    ) -> VesselResult<BTreeMap<String, serde_json::Value>> {
        let stored = self.storage.get_plugin(plugin_id).await?.ok_or_else(|| {
            VesselError::NotFound(format!("plugin '{}' is not installed", plugin_id.as_str()))
        })?;
        let key = format!("plugin_settings:{}", plugin_id.as_str());
        let mut map: BTreeMap<String, serde_json::Value> = self
            .storage
            .get_json_setting(&key)
            .await?
            .unwrap_or_default();

        for field in &stored.manifest.settings_schema {
            if field.field_type == vessel_domain::SettingFieldType::Secret {
                let sec_key = format!("plugin:{}:{}", plugin_id.as_str(), field.key);
                if let Some(val) = self.secrets.get(&sec_key).await? {
                    map.insert(field.key.clone(), serde_json::Value::String(val));
                }
            }
        }
        Ok(map)
    }

    /// Persists user settings for `plugin_id` (`Secret` fields into `SecretStore`) and applies them live.
    pub async fn set_plugin_settings(
        &self,
        plugin_id: &PluginId,
        mut values: BTreeMap<String, serde_json::Value>,
    ) -> VesselResult<()> {
        let stored = self.storage.get_plugin(plugin_id).await?.ok_or_else(|| {
            VesselError::NotFound(format!("plugin '{}' is not installed", plugin_id.as_str()))
        })?;

        let mut public_map = BTreeMap::new();
        for field in &stored.manifest.settings_schema {
            if field.field_type == vessel_domain::SettingFieldType::Secret {
                let sec_key = format!("plugin:{}:{}", plugin_id.as_str(), field.key);
                if let Some(val) = values.remove(&field.key) {
                    let s = val.as_str().unwrap_or_default().to_owned();
                    if s.is_empty() {
                        self.secrets.delete(&sec_key).await?;
                    } else {
                        self.secrets.set(&sec_key, &s).await?;
                    }
                }
            }
        }
        for (k, v) in values {
            public_map.insert(k, v);
        }

        let key = format!("plugin_settings:{}", plugin_id.as_str());
        self.storage.set_json_setting(&key, &public_map).await?;

        let combined = self.get_plugin_settings(plugin_id).await?;
        let _ = self.pool.set_settings(plugin_id, combined).await;
        Ok(())
    }

    /// Cancels any currently running multi-plugin search.
    /// Cancels any currently running streaming search so newly typed queries start cleanly.
    pub async fn cancel_active_search(&self) {
        let prev = { self.active_search.lock().await.take() };
        if let Some((_, token)) = prev {
            token.cancel();
        }
    }

    /// Runs streaming parallel search across all enabled plugins using `CancellationToken`.
    ///
    /// Cancels any previous search immediately and streams partial results through `EventSink`.
    pub async fn search_all(
        &self,
        query: &str,
        filters: BTreeMap<String, String>,
        page: u32,
    ) -> SearchHandle {
        self.search_selected(query, filters, page, None).await
    }

    /// Runs a streaming search across either all enabled plugins or an explicit `plugin_ids` subset.
    pub async fn search_selected(
        &self,
        query: &str,
        filters: BTreeMap<String, String>,
        page: u32,
        target_plugins: Option<Vec<PluginId>>,
    ) -> SearchHandle {
        self.cancel_active_search().await;

        let search_id = next_unique_id("search");
        let token = CancellationToken::new();
        {
            let mut active = self.active_search.lock().await;
            *active = Some((search_id.clone(), token.clone()));
        }

        let enabled = self
            .pool
            .enabled_plugin_ids_with_any(&[PluginCapability::Search, PluginCapability::Vod])
            .await;
        let plugin_ids: Vec<PluginId> = match target_plugins {
            Some(requested) if !requested.is_empty() => enabled
                .into_iter()
                .filter(|id| requested.contains(id))
                .collect(),
            _ => enabled,
        };
        let query_owned = query.to_owned();
        let pool = Arc::clone(&self.pool);
        let sink = Arc::clone(&self.event_sink);
        let active_search = Arc::clone(&self.active_search);
        let token_clone = token.clone();
        let sid = search_id.clone();

        let done = tokio::spawn(async move {
            // Cap concurrent plugin searches to 4 so the 6-slot QuickJS LRU pool never thrashes evictions.
            let sem = Arc::new(tokio::sync::Semaphore::new(4));
            let mut set = JoinSet::new();

            for plugin_id in plugin_ids {
                let pool = Arc::clone(&pool);
                let sink = Arc::clone(&sink);
                let token = token_clone.clone();
                let q = query_owned.clone();
                let f = filters.clone();
                let sid_inner = sid.clone();
                let sem = Arc::clone(&sem);

                set.spawn(async move {
                    if token.is_cancelled() {
                        return;
                    }
                    let _permit = tokio::select! {
                        biased;
                        () = token.cancelled() => return,
                        res = sem.acquire_owned() => match res {
                            Ok(permit) => permit,
                            Err(_) => return,
                        },
                    };
                    if token.is_cancelled() {
                        return;
                    }
                    let provider = match tokio::select! {
                        biased;
                        () = token.cancelled() => return,
                        res = pool.get_or_load(&plugin_id) => res,
                    } {
                        Ok(p) => p,
                        Err(_) => return,
                    };

                    if token.is_cancelled() {
                        return;
                    }

                    let call_cancel = provider.sandbox().new_call_cancellation();
                    let watcher_token = token.clone();
                    let watcher_cancel = call_cancel.clone();
                    let cancel_watcher = tokio::spawn(async move {
                        watcher_token.cancelled().await;
                        watcher_cancel.cancel();
                    });

                    let outcome = provider
                        .search_cancellable(&q, &f, page, &call_cancel)
                        .await;
                    cancel_watcher.abort();

                    if token.is_cancelled() {
                        return;
                    }

                    match outcome {
                        Ok(search_page) => {
                            let filtered = filter_search_items(
                                search_page.items,
                                &q,
                                f.get("mediaType").map(String::as_str),
                            );
                            sink.emit(EngineEvent::SearchPartialResult {
                                search_id: sid_inner,
                                plugin_id,
                                items: filtered,
                            });
                        }
                        Err(VesselError::DomainBlocked {
                            plugin_id: blocked_id,
                            domain,
                        }) => {
                            if let Ok(parsed_id) = PluginId::new(blocked_id) {
                                sink.emit(EngineEvent::DomainPermissionRequested {
                                    request_id: next_unique_id("perm"),
                                    plugin_id: parsed_id,
                                    domain,
                                });
                            }
                        }
                        Err(_) => {}
                    }
                });
            }

            while set.join_next().await.is_some() {}

            sink.emit(EngineEvent::SearchCompleted {
                search_id: sid.clone(),
            });

            let mut guard = active_search.lock().await;
            if guard.as_ref().is_some_and(|(cur_id, _)| cur_id == &sid) {
                *guard = None;
            }
        });

        SearchHandle {
            search_id,
            token,
            done,
        }
    }

    /// Clears in-memory `home_feed` and `details` caches (used when user hits manual Refresh).
    pub async fn clear_runtime_caches(&self) {
        self.home_feed_cache.lock().await.clear();
        self.details_cache.lock().await.clear();
    }

    async fn clear_plugin_caches(&self, plugin_id: &PluginId) {
        self.home_feed_cache
            .lock()
            .await
            .retain(|(pid, _), _| pid != plugin_id);
        self.details_cache
            .lock()
            .await
            .retain(|(pid, _), _| pid != plugin_id);
    }

    async fn fetch_home_feed_cached(
        pool: &PluginPool,
        home_feed_cache: &Mutex<HomeFeedCacheMap>,
        plugin_id: &PluginId,
        cursor: Option<&str>,
    ) -> VesselResult<HomeFeed> {
        let key = (plugin_id.clone(), cursor.map(str::to_owned));
        let now = Instant::now();
        {
            let cache = home_feed_cache.lock().await;
            if let Some((cached_at, feed)) = cache.get(&key)
                && now.saturating_duration_since(*cached_at) < HOME_FEED_CACHE_TTL
            {
                return Ok(feed.clone());
            }
        }

        let provider = pool.get_or_load(plugin_id).await?;
        let feed = provider.home_feed(cursor).await?;

        let mut cache = home_feed_cache.lock().await;
        if cache.len() >= MAX_HOME_FEED_CACHE_ENTRIES {
            let cutoff = Instant::now();
            cache.retain(|_, (ts, _)| cutoff.saturating_duration_since(*ts) < HOME_FEED_CACHE_TTL);
            if cache.len() >= MAX_HOME_FEED_CACHE_ENTRIES
                && let Some(oldest) = cache
                    .iter()
                    .min_by_key(|(_, (ts, _))| *ts)
                    .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest);
            }
        }
        cache.insert(key, (Instant::now(), feed.clone()));
        Ok(feed)
    }

    /// Fetches home feed shelves from a single plugin (serving fresh entries from a 3-min RAM cache).
    pub async fn home_feed(
        &self,
        plugin_id: &PluginId,
        cursor: Option<&str>,
    ) -> VesselResult<HomeFeed> {
        Self::fetch_home_feed_cached(&self.pool, &self.home_feed_cache, plugin_id, cursor).await
    }

    /// Streams home feed shelves per plugin via `EngineEvent::HomeFeedPartialResult` / `HomeFeedCompleted`.
    pub async fn start_home_feeds_stream(
        &self,
        target_plugins: Option<Vec<PluginId>>,
    ) -> VesselResult<String> {
        let request_id = next_unique_id("home");
        let plugins = self.list_plugins().await?;
        let filter_set: Option<HashSet<PluginId>> =
            target_plugins.map(|ids| ids.into_iter().collect());

        let targets: Vec<(PluginId, String)> = plugins
            .into_iter()
            .filter(|p| {
                p.enabled
                    && p.manifest
                        .capabilities
                        .iter()
                        .any(|cap| matches!(cap, PluginCapability::Vod | PluginCapability::LiveTv))
                    && filter_set
                        .as_ref()
                        .is_none_or(|allowed| allowed.contains(&p.manifest.id))
            })
            .map(|p| (p.manifest.id, p.manifest.name))
            .collect();

        if targets.is_empty() {
            self.event_sink.emit(EngineEvent::HomeFeedCompleted {
                request_id: request_id.clone(),
            });
            return Ok(request_id);
        }

        let pool = Arc::clone(&self.pool);
        let cache = Arc::clone(&self.home_feed_cache);
        let sink = Arc::clone(&self.event_sink);
        let rid = request_id.clone();

        tokio::spawn(async move {
            let sem = Arc::new(tokio::sync::Semaphore::new(6));
            let mut set = JoinSet::new();

            for (pid, pname) in targets {
                let pool = Arc::clone(&pool);
                let cache = Arc::clone(&cache);
                let sink = Arc::clone(&sink);
                let sem = Arc::clone(&sem);
                let rid_inner = rid.clone();

                set.spawn(async move {
                    let _permit = sem.acquire_owned().await;
                    match Self::fetch_home_feed_cached(&pool, &cache, &pid, None).await {
                        Ok(feed) => {
                            sink.emit(EngineEvent::HomeFeedPartialResult {
                                request_id: rid_inner,
                                plugin_id: pid,
                                plugin_name: pname,
                                sections: feed.sections,
                                error: None,
                            });
                        }
                        Err(VesselError::DomainBlocked {
                            plugin_id: blocked_id,
                            domain,
                        }) => {
                            if let Ok(parsed_id) = PluginId::new(&blocked_id) {
                                sink.emit(EngineEvent::DomainPermissionRequested {
                                    request_id: next_unique_id("perm"),
                                    plugin_id: parsed_id,
                                    domain: domain.clone(),
                                });
                            }
                            sink.emit(EngineEvent::HomeFeedPartialResult {
                                request_id: rid_inner,
                                plugin_id: pid,
                                plugin_name: pname,
                                sections: Vec::new(),
                                error: Some(format!(
                                    "[VesselDomainBlockedError]: domain '{domain}' requires permission"
                                )),
                            });
                        }
                        Err(err) => {
                            sink.emit(EngineEvent::HomeFeedPartialResult {
                                request_id: rid_inner,
                                plugin_id: pid,
                                plugin_name: pname,
                                sections: Vec::new(),
                                error: Some(err.to_string()),
                            });
                        }
                    }
                });
            }

            while set.join_next().await.is_some() {}
            sink.emit(EngineEvent::HomeFeedCompleted { request_id: rid });
        });

        Ok(request_id)
    }

    /// Fetches full item metadata & episode tree from `plugin_id` (with a 5-min RAM cache).
    pub async fn details(
        &self,
        plugin_id: &PluginId,
        native_id: &str,
    ) -> VesselResult<MediaDetail> {
        let key = (plugin_id.clone(), native_id.to_owned());
        let now = Instant::now();
        {
            let cache = self.details_cache.lock().await;
            if let Some((cached_at, detail)) = cache.get(&key)
                && now.saturating_duration_since(*cached_at) < DETAILS_CACHE_TTL
            {
                return Ok(detail.clone());
            }
        }

        let provider = self.pool.get_or_load(plugin_id).await?;
        let detail = provider.details(native_id).await?;

        let mut cache = self.details_cache.lock().await;
        if cache.len() >= MAX_DETAILS_CACHE_ENTRIES {
            let cutoff = Instant::now();
            cache.retain(|_, (ts, _)| cutoff.saturating_duration_since(*ts) < DETAILS_CACHE_TTL);
            if cache.len() >= MAX_DETAILS_CACHE_ENTRIES
                && let Some(oldest) = cache
                    .iter()
                    .min_by_key(|(_, (ts, _))| *ts)
                    .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest);
            }
        }
        cache.insert(key, (Instant::now(), detail.clone()));
        Ok(detail)
    }

    /// Returns the plugin's immediate `MediaDetail` (0 ms UI latency) and runs background
    /// enrichment (`AniList` / `TMDB` / `Cinemeta`), emitting `EngineEvent::MetadataEnriched`.
    pub async fn details_with_enrichment(
        &self,
        plugin_id: &PluginId,
        native_id: &str,
        preferred_lang: &str,
    ) -> VesselResult<(MediaDetail, JoinHandle<()>)> {
        let raw = self.details(plugin_id, native_id).await?;
        persist_episode_snapshot(&self.storage, plugin_id, &raw).await;
        let storage = Arc::clone(&self.storage);
        let secrets = Arc::clone(&self.secrets);
        let metadata = self.metadata.clone();
        let sink = Arc::clone(&self.event_sink);
        let enabled = self.metadata_enabled();
        let pid = plugin_id.clone();
        let raw_clone = raw.clone();
        let lang = preferred_lang.to_owned();

        let task = tokio::spawn(async move {
            if let Err(err) = run_enrichment_and_emit(
                &storage,
                secrets.as_ref(),
                &metadata,
                &sink,
                &pid,
                &raw_clone,
                &lang,
                enabled,
            )
            .await
            {
                sink.emit(EngineEvent::PluginLog {
                    plugin_id: pid,
                    level: LogLevel::Warn,
                    message: format!("Metadata enrichment error: {err}"),
                });
            }
        });

        Ok((raw, task))
    }

    /// Persists a user manual canonical metadata binding or disables enrichment for `media`.
    pub async fn set_metadata_override(
        &self,
        media: &MediaRef,
        canonical_id: Option<&str>,
        is_disabled: bool,
    ) -> VesselResult<()> {
        if let Some(cid) = canonical_id
            && cid.starts_with("tmdb:")
        {
            let has_key = self
                .secrets
                .get(TMDB_API_KEY_SECRET)
                .await?
                .is_some_and(|k| !k.trim().is_empty());
            if !has_key {
                return Err(VesselError::InvalidInput(
                    "TMDB IDs require a TMDB API key in Settings (or use IMDb / AniList ID without a key)"
                        .into(),
                ));
            }
        }
        self.storage
            .set_metadata_override(media, canonical_id, is_disabled)
            .await?;
        self.storage
            .invalidate_metadata_cache(&media.plugin_id, &media.native_id)
            .await?;
        self.clear_plugin_caches(&media.plugin_id).await;
        Ok(())
    }

    /// Searches a single plugin directly without fanning out across all plugins.
    pub async fn search_plugin_sync(
        &self,
        plugin_id: &PluginId,
        query: &str,
        page: u32,
    ) -> VesselResult<SearchPage> {
        let provider = self.pool.get_or_load(plugin_id).await?;
        provider.search(query, &BTreeMap::new(), page.max(1)).await
    }

    /// Searches across TMDB, IMDb, Simkl, AniList, MyAnimeList, and Kitsu for interactive Fix Match.
    pub async fn search_metadata(
        &self,
        query: &str,
        provider: MetadataProviderFilter,
        preferred_lang: &str,
    ) -> VesselResult<MetadataSearchResponse> {
        let tmdb_key = self.secrets.get(TMDB_API_KEY_SECRET).await?;
        let simkl_key = self.secrets.get(SIMKL_CLIENT_ID_SECRET).await?;
        Ok(self
            .metadata
            .search_interactive(
                query,
                provider,
                preferred_lang,
                tmdb_key.as_deref(),
                simkl_key.as_deref(),
            )
            .await)
    }

    fn linked_sources_key(media: &MediaRef) -> String {
        format!(
            "linked_sources:{}:{}",
            media.plugin_id.as_str(),
            media.native_id
        )
    }

    /// Returns user-linked secondary stream providers saved for `media`.
    pub async fn get_linked_sources(
        &self,
        media: &MediaRef,
    ) -> VesselResult<Vec<LinkedMediaSource>> {
        let key = Self::linked_sources_key(media);
        Ok(self
            .storage
            .get_json_setting::<Vec<LinkedMediaSource>>(&key)
            .await?
            .unwrap_or_default())
    }

    /// Persists user-linked secondary stream providers for `media` on this device.
    pub async fn set_linked_sources(
        &self,
        media: &MediaRef,
        sources: Vec<LinkedMediaSource>,
    ) -> VesselResult<Vec<LinkedMediaSource>> {
        let mut deduped: Vec<LinkedMediaSource> = Vec::new();
        for src in sources {
            if src.plugin_id == media.plugin_id && src.native_id == media.native_id {
                continue;
            }
            if let Some(pos) = deduped.iter().position(|x| x.plugin_id == src.plugin_id) {
                deduped[pos] = src;
            } else {
                deduped.push(src);
            }
        }
        let key = Self::linked_sources_key(media);
        self.storage.set_json_setting(&key, &deduped).await?;
        Ok(deduped)
    }

    /// Resolves playable stream candidates from `media.plugin_id` plus any persisted linked providers
    /// and automatic `PluginCapability::Stream` addons (e.g. Torrentio) when IMDb ID is known.
    pub async fn streams_with_linked(
        &self,
        media: &MediaRef,
        episode_id: &str,
        season: u32,
        episode: u32,
        absolute_episode: Option<u32>,
        is_movie: bool,
    ) -> VesselResult<StreamResult> {
        self.streams_with_external_ids(
            media,
            episode_id,
            season,
            episode,
            absolute_episode,
            is_movie,
            None,
        )
        .await
    }

    /// Resolves playable stream candidates from `media.plugin_id`, linked providers, and automatic
    /// `PluginCapability::Stream` addons using `external_ids` (or cached detail metadata).
    #[allow(clippy::too_many_arguments)]
    pub async fn streams_with_external_ids(
        &self,
        media: &MediaRef,
        episode_id: &str,
        season: u32,
        episode: u32,
        absolute_episode: Option<u32>,
        is_movie: bool,
        external_ids: Option<&ExternalIds>,
    ) -> VesselResult<StreamResult> {
        let plugins_list = self.list_plugins().await.unwrap_or_default();
        let name_map: HashMap<String, String> = plugins_list
            .iter()
            .map(|p| (p.manifest.id.as_str().to_owned(), p.manifest.name.clone()))
            .collect();
        let primary_name = name_map
            .get(media.plugin_id.as_str())
            .cloned()
            .unwrap_or_else(|| media.plugin_id.as_str().to_owned());

        let linked = self.get_linked_sources(media).await.unwrap_or_default();
        let is_virtual_meta =
            media.plugin_id.as_str() == "vessel.metadata" || episode_id.starts_with("virtual::");

        let mut merged = StreamResult::default();
        let mut primary_err: Option<VesselError> = None;

        if !is_virtual_meta {
            let primary_res = self.streams(&media.plugin_id, episode_id).await;
            match primary_res {
                Ok(mut res) => {
                    for cand in &mut res.streams {
                        cand.plugin_id = Some(media.plugin_id.as_str().to_owned());
                        cand.plugin_name = Some(primary_name.clone());
                    }
                    merged.streams.extend(res.streams);
                    merged.subtitles.extend(res.subtitles);
                    merged.skip_segments.extend(res.skip_segments);
                }
                Err(err) => {
                    primary_err = Some(err);
                }
            }
        }

        // Determine IMDb / Kitsu ID from argument, cached detail, or virtual streamId.
        let mut resolved_imdb = external_ids
            .and_then(|e| e.imdb.clone())
            .filter(|s| !s.trim().is_empty());
        if resolved_imdb.is_none()
            && let Some(raw_imdb) = episode_id
                .split("::")
                .find(|part| part.starts_with("tt") && part[2..].chars().all(|c| c.is_ascii_digit()))
        {
            resolved_imdb = Some(raw_imdb.to_owned());
        }
        if resolved_imdb.is_none() {
            let guard = self.details_cache.lock().await;
            if let Some((_, cached_det)) = guard.get(&(media.plugin_id.clone(), media.native_id.clone())) {
                resolved_imdb = cached_det.external_ids.imdb.clone();
            }
        }

        // Collect automatic `PluginCapability::Stream` addons (such as Torrentio / MediaFusion).
        let stream_addons: Vec<(PluginId, String)> = if let Some(ref imdb) = resolved_imdb {
            let linked_ids: HashSet<&str> = linked.iter().map(|l| l.plugin_id.as_str()).collect();
            let auto_disabled: HashSet<String> = self
                .storage
                .get_json_setting::<Vec<String>>("stream_addons.disabled")
                .await
                .ok()
                .flatten()
                .unwrap_or_default()
                .into_iter()
                .collect();
            let _ = imdb;
            plugins_list
                .iter()
                .filter(|p| {
                    p.enabled
                        && p.manifest.id != media.plugin_id
                        && p.manifest.capabilities.contains(&PluginCapability::Stream)
                        && !linked_ids.contains(p.manifest.id.as_str())
                        && !auto_disabled.contains(p.manifest.id.as_str())
                })
                .map(|p| (p.manifest.id.clone(), p.manifest.name.clone()))
                .collect()
        } else {
            Vec::new()
        };

        if primary_err.is_some() && linked.is_empty() && stream_addons.is_empty() {
            return Err(primary_err.expect("checked is_some"));
        }

        let mut set = JoinSet::new();
        for link in linked {
            let link_pid = link.plugin_id.clone();
            let link_nid = link.native_id.clone();
            let link_name = link
                .plugin_name
                .clone()
                .or_else(|| name_map.get(link_pid.as_str()).cloned())
                .unwrap_or_else(|| link_pid.as_str().to_owned());
            let pool = Arc::clone(&self.pool);
            let details_cache = Arc::clone(&self.details_cache);
            set.spawn(async move {
                let cached = {
                    let guard = details_cache.lock().await;
                    let key = (link_pid.clone(), link_nid.clone());
                    guard
                        .get(&key)
                        .filter(|(ts, _)| ts.elapsed() < DETAILS_CACHE_TTL)
                        .map(|(_, d)| d.clone())
                };
                let provider = pool.get_or_load(&link_pid).await.ok()?;
                let detail = match cached {
                    Some(d) => d,
                    None => {
                        let fetched = provider.details(&link_nid).await.ok()?;
                        let mut guard = details_cache.lock().await;
                        guard.insert(
                            (link_pid.clone(), link_nid.clone()),
                            (Instant::now(), fetched.clone()),
                        );
                        fetched
                    }
                };
                let synthetic_stream_id;
                let target_stream_id = if let Some(ep) = match_linked_episode(
                    &detail.episodes,
                    season,
                    episode,
                    absolute_episode,
                    is_movie,
                ) {
                    ep.stream_id.as_str()
                } else if let Some(only_ep) = detail
                    .episodes
                    .first()
                    .filter(|_| detail.episodes.len() == 1)
                    && let Some(raw_rest) = only_ep.stream_id.strip_prefix("vns::")
                    && let Some((mtype, rest_after_type)) = raw_rest.split_once("::")
                    && let Some((_, rest_after_s)) = rest_after_type.split_once("::")
                    && let Some((_, raw_id)) = rest_after_s.split_once("::")
                {
                    synthetic_stream_id = format!("vns::{mtype}::{season}::{episode}::{raw_id}");
                    synthetic_stream_id.as_str()
                } else {
                    return None;
                };
                let mut sr = provider.streams(target_stream_id).await.ok()?;
                for cand in &mut sr.streams {
                    cand.plugin_id = Some(link_pid.as_str().to_owned());
                    cand.plugin_name = Some(link_name.clone());
                }
                Some(sr)
            });
        }

        if let Some(imdb) = resolved_imdb {
            for (addon_pid, addon_name) in stream_addons {
                let pool = Arc::clone(&self.pool);
                let target_stream_id = if is_movie {
                    format!("movie::{imdb}")
                } else {
                    format!("series::{imdb}:{season}:{episode}")
                };
                set.spawn(async move {
                    let provider = pool.get_or_load(&addon_pid).await.ok()?;
                    let mut sr = provider.streams(&target_stream_id).await.ok()?;
                    for cand in &mut sr.streams {
                        cand.plugin_id = Some(addon_pid.as_str().to_owned());
                        cand.plugin_name = Some(addon_name.clone());
                        if let Some(ref mut tor) = cand.torrent
                            && !is_movie
                        {
                            if tor.season.is_none() {
                                tor.season = Some(season);
                            }
                            if tor.episode.is_none() {
                                tor.episode = Some(episode);
                            }
                        }
                    }
                    Some(sr)
                });
            }
        }

        while let Some(res) = set.join_next().await {
            if let Ok(Some(sr)) = res {
                merged.streams.extend(sr.streams);
                merged.subtitles.extend(sr.subtitles);
                merged.skip_segments.extend(sr.skip_segments);
            }
        }

        if merged.streams.is_empty()
            && let Some(err) = primary_err
        {
            return Err(err);
        }

        // Automatically probe bulk Debrid cache status for any returned torrent candidates.
        if merged.streams.iter().any(StreamCandidate::is_torrent)
            && let Ok(prefs) = self.get_debrid_preferences().await
        {
            self.debrid.probe_cache(&mut merged.streams, &prefs).await;
            if prefs.only_cached {
                merged.streams.retain(|c| {
                    !c.is_torrent()
                        || c.debrid_cache
                            .as_ref()
                            .is_some_and(|info| info.state != vessel_domain::DebridCacheState::Uncached)
                });
            }
        }

        Ok(merged)
    }

    /// Resolves playable stream candidates for an episode or movie from `plugin_id`.
    pub async fn streams(
        &self,
        plugin_id: &PluginId,
        stream_id: &str,
    ) -> VesselResult<StreamResult> {
        let provider = self.pool.get_or_load(plugin_id).await?;
        provider.streams(stream_id).await
    }

    fn route_subtitle_track(&self, mut track: SubtitleTrack) -> VesselResult<SubtitleTrack> {
        if let SubtitleSource::Url {
            ref url,
            ref headers,
        } = track.source
            && (!headers.is_empty() || self.always_proxy())
        {
            let proxied = self.proxy.register_stream(url.clone(), headers.clone())?;
            track.source = SubtitleSource::Url {
                url: proxied,
                headers: BTreeMap::new(),
            };
        }
        Ok(track)
    }

    /// Resolves a playback URL through Smart Proxy (injecting headers / rewriting HLS) without launching `mpv`.
    pub fn prepare_playback_url(&self, candidate: &StreamCandidate) -> VesselResult<String> {
        self.proxy
            .resolve_playback_url(candidate, self.always_proxy())
    }

    /// Routes `candidate` through Debrid (if torrent) + Smart Proxy, merges subtitles, resolves resume offset,
    /// and dispatches `PlaybackRequest` to `PlayerBackend::load`.
    pub async fn play_candidate(&self, mut req: PlayCandidateRequest) -> VesselResult<String> {
        let mut is_debrid = false;
        let debrid_size: u64;
        if req.candidate.is_torrent() {
            let res = self
                .resolve_torrent_candidate(ResolveDebridRequest {
                    url: req.candidate.url.clone(),
                    torrent: req.candidate.torrent.clone(),
                    preferred_target: req
                        .candidate
                        .debrid_cache
                        .as_ref()
                        .map(|c| c.target.clone()),
                    allow_uncached_queue: false,
                    season: Some(req.season),
                    episode: Some(req.episode),
                })
                .await?;
            match res {
                DebridResolution::Ready {
                    url,
                    headers,
                    size_bytes,
                    ..
                } => {
                    is_debrid = true;
                    debrid_size = size_bytes
                        .or_else(|| req.candidate.torrent.as_ref().and_then(|t| t.size_bytes))
                        .unwrap_or(0);
                    req.candidate.url = url.clone();
                    req.candidate.kind = infer_stream_kind_from_url(&url);
                    req.candidate.headers = headers;
                }
                DebridResolution::NeedsConfirmation {
                    provider_label,
                    progress_pct,
                    ..
                } => {
                    return Err(VesselError::InvalidInput(format!(
                        "Torrent is not cached on {provider_label} ({progress_pct:.0}%). Queue it to Debrid cloud first."
                    )));
                }
                DebridResolution::NoProviderConfigured { message } => {
                    return Err(VesselError::InvalidInput(message));
                }
            }
        } else {
            debrid_size = req
                .candidate
                .torrent
                .as_ref()
                .and_then(|t| t.size_bytes)
                .unwrap_or(0);
        }

        self.last_stream_is_debrid
            .store(is_debrid, Ordering::Release);
        self.last_stream_size_bytes
            .store(debrid_size, Ordering::Release);

        let always_proxy = self.always_proxy();
        let resolved_url = self
            .proxy
            .resolve_playback_url(&req.candidate, always_proxy)?;
        let proxied = resolved_url != req.candidate.url;

        let mut merged_subtitles =
            Vec::with_capacity(req.candidate.subtitles.len() + req.extra_subtitles.len());
        for track in req
            .candidate
            .subtitles
            .into_iter()
            .chain(req.extra_subtitles)
        {
            merged_subtitles.push(self.route_subtitle_track(track)?);
        }

        let start_position_ms = match req.start_position_ms {
            Some(pos) => pos,
            None => {
                if let Some(direct) = self
                    .storage
                    .get_progress(&req.media, req.season, req.episode)
                    .await?
                {
                    if direct.completed {
                        0
                    } else {
                        direct.position_ms
                    }
                } else if let (Some(canonical_id), Some(abs_ep)) =
                    (req.canonical_id.as_deref(), req.absolute_episode)
                {
                    self.storage
                        .get_watch_progress_by_canonical(canonical_id, abs_ep)
                        .await?
                        .filter(|p| !p.completed)
                        .map_or(0, |p| p.position_ms)
                } else {
                    0
                }
            }
        };

        let headers = if proxied {
            BTreeMap::new()
        } else {
            req.candidate.headers
        };

        self.player
            .load(PlaybackRequest {
                url: resolved_url.clone(),
                headers,
                media_type: req.media_type,
                start_position_ms,
                subtitles: merged_subtitles,
                title: req.title,
            })
            .await?;

        Ok(resolved_url)
    }

    /// Persists playback progress into SQLite (`watch_progress`).
    pub async fn record_progress(&self, progress: &PlaybackProgress) -> VesselResult<()> {
        self.storage.upsert_progress(progress).await?;
        self.spawn_progress_tracker_dispatch(progress);
        Ok(())
    }

    /// Reads saved watch progress for a specific `(plugin_id, native_id, season, episode)`.
    pub async fn get_watch_progress(
        &self,
        media: &MediaRef,
        season: u32,
        episode: u32,
    ) -> VesselResult<Option<PlaybackProgress>> {
        self.storage.get_progress(media, season, episode).await
    }

    /// Lists recently watched incomplete items for the Home "Continue Watching" shelf.
    pub async fn list_continue_watching(
        &self,
        limit: usize,
    ) -> VesselResult<Vec<ContinueWatchingEntry>> {
        self.storage.list_continue_watching(limit).await
    }

    /// Lists all watch progress rows for `(plugin_id, native_id)` ordered by `(season, episode)`.
    pub async fn list_media_watch_progress(
        &self,
        media: &MediaRef,
    ) -> VesselResult<Vec<PlaybackProgress>> {
        self.storage.list_media_watch_progress(media).await
    }

    /// Lists recent watch progress rows across all media (completed and active).
    pub async fn list_all_watch_progress(
        &self,
        limit: usize,
    ) -> VesselResult<Vec<PlaybackProgress>> {
        self.storage.list_all_watch_progress(limit).await
    }

    /// Removes a title's watch progress from Continue Watching.
    pub async fn delete_watch_progress(
        &self,
        plugin_id: &PluginId,
        native_id: &str,
    ) -> VesselResult<()> {
        self.storage
            .delete_watch_progress(plugin_id, native_id)
            .await
    }

    /// Resolves validated skip intervals (`intro`, `recap`, `credits`, `preview`) from plugin
    /// candidates or external databases (`TheIntroDB`, `IntroDB`, `AniSkip`).
    pub async fn get_skip_segments(
        &self,
        media: Option<&MediaRef>,
        mut query: SkipQuery,
    ) -> Vec<SkipSegment> {
        if let Some(m) = media
            && query.external_ids.tmdb.is_none()
            && query.external_ids.imdb.is_none()
            && query.external_ids.mal.is_none()
            && query.external_ids.anilist.is_none()
        {
            for lang in [
                "en", "tr", "es", "de", "fr", "it", "ru", "ja", "pt", "zh", "ar", "fa",
            ] {
                let key = format!("meta4:{}:{}:{lang}", m.plugin_id.as_str(), m.native_id);
                if let Ok(Some(json_str)) = self.storage.get_metadata_cache(&key).await
                    && let Ok(canon) = serde_json::from_str::<CanonicalMetadata>(&json_str)
                {
                    query.external_ids = canon.external_ids;
                    break;
                }
            }
        }
        self.metadata
            .resolve_skip_segments(&query, self.metadata_enabled())
            .await
    }

    /// Lists all enabled subtitle-capable plugins registered in the pool.
    pub async fn list_subtitle_plugins(&self) -> Vec<SubtitlePluginInfo> {
        self.pool
            .enabled_subtitle_plugins()
            .await
            .into_iter()
            .map(|(id, name)| SubtitlePluginInfo {
                plugin_id: id.to_string(),
                name,
            })
            .collect()
    }

    /// Searches enabled subtitle-capable plugins in parallel for external subtitles matching `query`.
    pub async fn search_subtitles(
        &self,
        query: SubtitleQuery,
        target_plugins: Option<Vec<PluginId>>,
    ) -> VesselResult<SubtitleSearchOutcome> {
        let available = self.pool.enabled_subtitle_plugins().await;
        let explicit_targets = target_plugins
            .as_ref()
            .is_some_and(|list| !list.is_empty());
        let filter_set: Option<HashSet<PluginId>> = target_plugins
            .filter(|list| !list.is_empty())
            .map(|list| list.into_iter().collect());

        let targets: Vec<(PluginId, String)> = available
            .into_iter()
            .filter(|(id, _)| {
                filter_set
                    .as_ref()
                    .is_none_or(|allowed| allowed.contains(id))
            })
            .collect();

        if targets.is_empty() {
            return Ok(SubtitleSearchOutcome::default());
        }

        let pool = Arc::clone(&self.pool);
        let sink = Arc::clone(&self.event_sink);
        let sem = Arc::new(tokio::sync::Semaphore::new(4));
        let mut set = JoinSet::new();

        for (pid, pname) in targets {
            let pool = Arc::clone(&pool);
            let sink = Arc::clone(&sink);
            let sem = Arc::clone(&sem);
            let q = query.clone();
            set.spawn(async move {
                let _permit = sem.acquire_owned().await;
                let provider = match pool.get_or_load(&pid).await {
                    Ok(p) => p,
                    Err(err) => {
                        return Some((pid, pname, Err(err)));
                    }
                };
                if !provider.supports_subtitle_search() {
                    pool.set_subtitle_search_supported(&pid, false).await;
                    if !explicit_targets {
                        return None;
                    }
                }
                let res = SubtitleProvider::search_subtitles(provider.as_ref(), &q).await;
                if let Err(VesselError::DomainBlocked {
                    plugin_id: ref blocked_id,
                    ref domain,
                }) = res
                    && let Ok(parsed_id) = PluginId::new(blocked_id)
                {
                    sink.emit(EngineEvent::DomainPermissionRequested {
                        request_id: next_unique_id("perm"),
                        plugin_id: parsed_id,
                        domain: domain.clone(),
                    });
                }
                Some((pid, pname, res))
            });
        }

        let mut outcome = SubtitleSearchOutcome::default();
        while let Some(joined) = set.join_next().await {
            let Ok(Some((pid, pname, res))) = joined else {
                continue;
            };
            match res {
                Ok(candidates) => {
                    for candidate in candidates {
                        outcome.hits.push(SubtitleSearchHit {
                            plugin_id: pid.as_str().to_owned(),
                            plugin_name: pname.clone(),
                            candidate,
                        });
                    }
                }
                Err(err) => {
                    outcome.errors.push(SubtitleProviderError {
                        plugin_id: pid.as_str().to_owned(),
                        plugin_name: pname,
                        error: err.to_string(),
                    });
                }
            }
        }

        Ok(outcome)
    }

    /// Resolves an external subtitle candidate from `plugin_id` into a playback-ready `SubtitleTrack`.
    pub async fn resolve_subtitle(
        &self,
        plugin_id: &PluginId,
        subtitle_id: &str,
        lang: &str,
        label: &str,
    ) -> VesselResult<SubtitleTrack> {
        let provider = self.pool.get_or_load(plugin_id).await?;
        let resolved =
            match SubtitleProvider::resolve_subtitle(provider.as_ref(), subtitle_id).await {
                Ok(r) => r,
                Err(VesselError::DomainBlocked {
                    plugin_id: ref blocked_id,
                    ref domain,
                }) => {
                    if let Ok(parsed_id) = PluginId::new(blocked_id) {
                        self.event_sink
                            .emit(EngineEvent::DomainPermissionRequested {
                                request_id: next_unique_id("perm"),
                                plugin_id: parsed_id,
                                domain: domain.clone(),
                            });
                    }
                    return Err(VesselError::DomainBlocked {
                        plugin_id: blocked_id.clone(),
                        domain: domain.clone(),
                    });
                }
                Err(err) => return Err(err),
            };
        let track = SubtitleTrack {
            id: Some(format!("{}:{subtitle_id}", plugin_id.as_str())),
            source: resolved.source,
            lang: lang.to_owned(),
            label: label.to_owned(),
            format: resolved.format,
            is_default: false,
        };
        self.route_subtitle_track(track)
    }

    /// Adds an external or inline subtitle track to the active player session.
    pub async fn add_subtitle(&self, track: SubtitleTrack) -> VesselResult<()> {
        let routed = self.route_subtitle_track(track)?;
        self.player.add_subtitle(routed).await
    }

    /// Adjusts active subtitle delay in milliseconds (`+` delays, `-` advances).
    pub async fn set_subtitle_delay(&self, delay_ms: i64) -> VesselResult<()> {
        self.player.set_subtitle_delay(delay_ms).await
    }

    /// Resumes active playback backend.
    pub async fn play(&self) -> VesselResult<()> {
        self.player.play().await
    }

    /// Pauses active playback backend.
    pub async fn pause(&self) -> VesselResult<()> {
        self.player.pause().await
    }

    /// Seeks playback position in milliseconds.
    pub async fn seek(&self, position_ms: u64) -> VesselResult<()> {
        self.player.seek(position_ms).await
    }

    /// Stops playback and cleans up temporary subtitle resources.
    pub async fn stop(&self) -> VesselResult<()> {
        self.player.stop().await
    }

    /// Saves or updates an IPTV profile, storing any optional Xtream password in `SecretStore`.

    /// Combines local proxy throughput and active `PlayerBackend` telemetry for the Stats HUD.
    pub async fn get_playback_stats(&self) -> PlaybackStats {
        let (proxy_speed, proxy_total) = self.proxy.throughput_stats();
        let mut stats = self.player.stats().await.unwrap_or_default();
        if proxy_speed > 0 {
            stats.network_speed_bps = proxy_speed;
        }
        if proxy_total > stats.total_bytes_transferred {
            stats.total_bytes_transferred = proxy_total;
        }
        if stats.file_size_bytes.is_none() {
            let sz = self.last_stream_size_bytes.load(Ordering::Acquire);
            if sz > 0 {
                stats.file_size_bytes = Some(sz);
            }
        }
        stats.is_debrid_stream = self.last_stream_is_debrid.load(Ordering::Acquire);
        stats
    }

    /// Evicts warm QuickJS runtimes idle beyond TTL (`3 min`).
    pub async fn evict_idle_plugins(&self) -> usize {
        self.pool.evict_idle().await
    }

    /// Shuts down warm QuickJS plugins gracefully.
    pub async fn shutdown(&self) {
        self.cancel_active_search().await;
        self.pool.shutdown_all().await;
    }
}

fn infer_stream_kind_from_url(url: &str) -> StreamKind {
    let lower = url.trim().to_ascii_lowercase();
    if lower.contains(".m3u8") {
        StreamKind::Hls
    } else if lower.contains(".mkv") {
        StreamKind::Mkv
    } else if lower.contains(".mp4") {
        StreamKind::Mp4
    } else if lower.contains(".webm") {
        StreamKind::Webm
    } else {
        StreamKind::Auto
    }
}

fn match_linked_episode(
    episodes: &[vessel_domain::Episode],
    season: u32,
    episode: u32,
    absolute_episode: Option<u32>,
    is_movie: bool,
) -> Option<&vessel_domain::Episode> {
    if episodes.is_empty() {
        return None;
    }
    if is_movie {
        return episodes.first();
    }
    episodes
        .iter()
        .find(|e| e.season == season && e.episode == episode)
        .or_else(|| {
            let abs = absolute_episode?;
            episodes
                .iter()
                .find(|e| e.absolute_episode == Some(abs) || (e.season == 1 && e.episode == abs))
        })
        .or_else(|| episodes.iter().find(|e| e.episode == episode))
}

pub(crate) fn apply_canonical_override_hint(detail: &mut MediaDetail, canonical_id: &str) {
    detail.external_ids = ExternalIds::default();
    if let Some(rest) = canonical_id.strip_prefix("anilist:") {
        if let Ok(id) = rest.parse::<u64>() {
            detail.external_ids.anilist = Some(id);
            detail.is_anime = true;
        }
    } else if let Some(rest) = canonical_id.strip_prefix("mal:") {
        if let Ok(id) = rest.parse::<u64>() {
            detail.external_ids.mal = Some(id);
            detail.is_anime = true;
        }
    } else if let Some(rest) = canonical_id.strip_prefix("kitsu:") {
        if let Ok(id) = rest.parse::<u64>() {
            detail.external_ids.kitsu = Some(id);
            detail.is_anime = true;
        }
    } else if let Some(rest) = canonical_id.strip_prefix("simkl:") {
        if let Ok(id) = rest.parse::<u64>() {
            detail.external_ids.simkl = Some(id);
        }
    } else if let Some(rest) = canonical_id.strip_prefix("tmdb:movie:") {
        if let Ok(id) = rest.parse::<u64>() {
            detail.external_ids.tmdb = Some(id);
            detail.media_type = MediaType::Movie;
        }
    } else if let Some(rest) = canonical_id.strip_prefix("tmdb:tv:") {
        if let Ok(id) = rest.parse::<u64>() {
            detail.external_ids.tmdb = Some(id);
            detail.media_type = MediaType::Series;
        }
    } else if let Some(rest) = canonical_id.strip_prefix("imdb:") {
        detail.external_ids.imdb = Some(rest.to_owned());
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_enrichment_and_emit(
    storage: &SqliteStorage,
    secrets: &dyn SecretStore,
    metadata: &MetadataService,
    sink: &Arc<dyn EventSink>,
    plugin_id: &PluginId,
    raw: &MediaDetail,
    preferred_lang: &str,
    metadata_enabled: bool,
) -> VesselResult<Option<MediaDetail>> {
    if !metadata_enabled {
        return Ok(None);
    }

    let media_ref = MediaRef::new(plugin_id.clone(), raw.native_id.clone())?;
    let mut query_detail = raw.clone();
    let mut has_override = false;

    if let Some(override_entry) = storage.get_metadata_override(&media_ref).await? {
        if override_entry.is_disabled {
            return Ok(None);
        }
        if let Some(ref cid) = override_entry.canonical_id {
            apply_canonical_override_hint(&mut query_detail, cid);
            has_override = true;
        }
    }

    let cache_key = format!(
        "meta4:{}:{}:{}",
        plugin_id.as_str(),
        raw.native_id,
        preferred_lang.to_ascii_lowercase()
    );
    if let Some(cached_json) = storage.get_metadata_cache(&cache_key).await?
        && let Ok(cached_canon) = serde_json::from_str::<CanonicalMetadata>(&cached_json)
    {
        let merged = merge_detail_with_override(raw, &cached_canon, preferred_lang, has_override);
        persist_episode_snapshot(storage, plugin_id, &merged).await;
        sink.emit(EngineEvent::MetadataEnriched {
            plugin_id: plugin_id.clone(),
            native_id: raw.native_id.clone(),
            detail: Box::new(merged.clone()),
        });
        return Ok(Some(merged));
    }

    let tmdb_key = secrets.get(TMDB_API_KEY_SECRET).await?;
    let simkl_key = secrets.get(SIMKL_CLIENT_ID_SECRET).await?;
    let outcome = metadata
        .enrich_detail_with_keys(
            &query_detail,
            preferred_lang,
            tmdb_key.as_deref(),
            simkl_key.as_deref(),
            metadata_enabled,
        )
        .await?;

    if let Some(ref canonical) = outcome.canonical {
        if let Ok(json) = serde_json::to_string(canonical) {
            let _ = storage
                .upsert_metadata_cache(&cache_key, &json, "2099-01-01T00:00:00Z")
                .await;
        }
        let merged = merge_detail_with_override(raw, canonical, preferred_lang, has_override);
        persist_episode_snapshot(storage, plugin_id, &merged).await;
        sink.emit(EngineEvent::MetadataEnriched {
            plugin_id: plugin_id.clone(),
            native_id: raw.native_id.clone(),
            detail: Box::new(merged.clone()),
        });
        return Ok(Some(merged));
    }

    Ok(None)
}

fn filter_search_items(
    items: Vec<vessel_domain::MediaSummary>,
    query: &str,
    media_type_filter: Option<&str>,
) -> Vec<vessel_domain::MediaSummary> {
    let type_filter = media_type_filter
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());

    let after_type: Vec<vessel_domain::MediaSummary> = items
        .into_iter()
        .filter(|item| match type_filter.as_deref() {
            Some("movie") => item.media_type == MediaType::Movie,
            Some("anime") => item.is_anime,
            Some("series") => item.media_type == MediaType::Series && !item.is_anime,
            _ => true,
        })
        .collect();

    let clean_q = query.trim().to_ascii_lowercase();
    if clean_q.is_empty() || after_type.len() < 6 {
        return after_type;
    }

    let tokens: Vec<&str> = clean_q
        .split_whitespace()
        .filter(|tok| tok.len() >= 2)
        .collect();
    let matches_query = |title: &str| -> bool {
        let lc = title.to_ascii_lowercase();
        if lc.contains(&clean_q) {
            return true;
        }
        !tokens.is_empty() && tokens.iter().all(|tok| lc.contains(tok))
    };

    let hits = after_type
        .iter()
        .filter(|item| matches_query(&item.title))
        .count();
    if hits == 0 || (hits * 100) / after_type.len() < 40 {
        after_type
            .into_iter()
            .filter(|item| matches_query(&item.title))
            .collect()
    } else {
        after_type
    }
}


#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use axum::Router;
    use axum::http::header::REFERER;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;
    use tokio::net::TcpListener;
    use vessel_domain::{
        EngineEvent, EventSink, ExternalIds, LinkedMediaSource, MediaRef, MediaType,
        PlaybackProgress, PlaybackRequest, PlayerBackend, PluginId, PluginManifest, StreamFlag,
        StreamKind, SubtitleQuery, SubtitleSource, SubtitleTrack, VesselError, VesselResult,
    };
    use vessel_plugin_rt::{LoadedPackage, PluginBundle, SourceResolver, sha256_hex};
    use vessel_proxy::ProxyServer;
    use vessel_storage::{SqliteSecretStore, SqliteStorage, StorageActor};

    use super::{PlayCandidateRequest, VesselEngine, builtins};

    #[derive(Default)]
    struct MockPlayer {
        loaded: Mutex<Vec<PlaybackRequest>>,
    }

    #[async_trait]
    impl PlayerBackend for MockPlayer {
        async fn load(&self, request: PlaybackRequest) -> VesselResult<()> {
            self.loaded.lock().expect("lock loaded").push(request);
            Ok(())
        }
        async fn add_subtitle(&self, _track: SubtitleTrack) -> VesselResult<()> {
            Ok(())
        }
        async fn set_subtitle_delay(&self, _delay_ms: i64) -> VesselResult<()> {
            Ok(())
        }
        async fn play(&self) -> VesselResult<()> {
            Ok(())
        }
        async fn pause(&self) -> VesselResult<()> {
            Ok(())
        }
        async fn seek(&self, _position_ms: u64) -> VesselResult<()> {
            Ok(())
        }
        async fn stop(&self) -> VesselResult<()> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct EventRecorder {
        events: Mutex<Vec<EngineEvent>>,
    }

    impl EventSink for EventRecorder {
        fn emit(&self, event: EngineEvent) {
            self.events.lock().expect("lock events").push(event);
        }
    }

    fn build_plugin_package(id: &str, index_ts: &str) -> LoadedPackage {
        let manifest: PluginManifest = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "id": id,
            "name": format!("Plugin {id}"),
            "version": "1.0.0",
            "entry": "index.ts",
            "capabilities": ["vod", "search"],
            "domains": ["anthology.example"]
        }))
        .expect("manifest");
        let bundle = PluginBundle::new([("index.ts", index_ts)]).expect("bundle");
        LoadedPackage {
            manifest,
            bundle,
            icon_png: None,
            sha256: sha256_hex(index_ts.as_bytes()),
        }
    }

    async fn spawn_protected_media_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock media");
        let addr = listener.local_addr().expect("addr");
        let app = Router::new().route(
            "/video.mp4",
            get(|headers: HeaderMap| async move {
                let referer = headers.get(REFERER).and_then(|v| v.to_str().ok());
                if referer == Some("https://anthology.example") {
                    (StatusCode::OK, "video-bytes")
                } else {
                    (StatusCode::FORBIDDEN, "missing-referer")
                }
            }),
        );
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        format!("http://{addr}")
    }

    async fn test_engine() -> (VesselEngine, Arc<MockPlayer>, Arc<EventRecorder>) {
        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let secrets = Arc::new(SqliteSecretStore::new(actor.clone()));
        let storage = Arc::new(SqliteStorage::new(actor));
        let proxy = Arc::new(ProxyServer::start().await.expect("start proxy"));
        let player = Arc::new(MockPlayer::default());
        let recorder = Arc::new(EventRecorder::default());
        let engine = VesselEngine::from_parts(
            storage,
            secrets,
            proxy,
            player.clone(),
            recorder.clone(),
            SourceResolver::new().with_default_registries(),
            false,
        )
        .await
        .expect("create engine");
        (engine, player, recorder)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn streams_parallel_search_surfaces_domain_prompt_and_supports_cancellation() {
        let (engine, _, recorder) = test_engine().await;

        let fast_ts = r#"
            export default {
                async search(query: string) {
                    return {
                        items: [{
                            pluginId: "anthology",
                            nativeId: "/anime/1",
                            type: "series",
                            isAnime: true,
                            title: `Result for ${query}`,
                        }],
                        hasMore: false
                    };
                }
            };
        "#;
        let blocked_ts = r#"
            import { http } from "vessel";
            export default {
                async search() {
                    await http.get("https://mirror.anthology.example/search");
                    return { items: [], hasMore: false };
                }
            };
        "#;
        let slow_ts = r#"
            export default {
                async search() {
                    while (true) {}
                }
            };
        "#;

        engine
            .install_package(
                build_plugin_package("anthology", fast_ts),
                None,
                "local",
                "/tmp/a",
            )
            .await
            .expect("install fast plugin");
        engine
            .install_package(
                build_plugin_package("needs-perm", blocked_ts),
                None,
                "local",
                "/tmp/b",
            )
            .await
            .expect("install blocked plugin");

        let handle = engine.search_all("gintama", BTreeMap::new(), 1).await;
        handle.done.await.expect("join search");

        {
            let events = recorder.events.lock().expect("lock events");
            assert!(events.iter().any(|ev| matches!(
                ev,
                EngineEvent::SearchPartialResult { plugin_id, items, .. }
                    if plugin_id.as_str() == "anthology" && items.len() == 1 && items[0].title == "Result for gintama"
            )));
            assert!(events.iter().any(|ev| matches!(
                ev,
                EngineEvent::DomainPermissionRequested { plugin_id, domain, .. }
                    if plugin_id.as_str() == "needs-perm" && domain == "mirror.anthology.example"
            )));
            assert!(events.iter().any(|ev| matches!(
                ev,
                EngineEvent::SearchCompleted { search_id } if search_id == &handle.search_id
            )));
        }

        // Verify CancellationToken aborts a tight infinite loop cleanly.
        engine
            .install_package(
                build_plugin_package("infinite-loop", slow_ts),
                None,
                "local",
                "/tmp/c",
            )
            .await
            .expect("install slow plugin");

        let cancelled_handle = engine.search_all("cancel-me", BTreeMap::new(), 1).await;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        engine.cancel_active_search().await;
        cancelled_handle.done.await.expect("join cancelled search");

        engine.shutdown().await;
    }

    #[tokio::test]
    async fn orchestrates_ts_streams_smart_proxy_resume_and_progress_persistence() {
        let media_base = spawn_protected_media_server().await;
        let (engine, player, _) = test_engine().await;

        let plugin_ts = &format!(
            r#"
            export default {{
                async getDetails(nativeId: string) {{
                    return {{
                        pluginId: "anthology",
                        nativeId,
                        type: "series",
                        isAnime: true,
                        title: "Gintama",
                        episodes: [{{
                            streamId: "ep-50",
                            season: 1,
                            episode: 50,
                            absoluteEpisode: 50,
                        }}],
                    }};
                }},
                async getStreams(streamId: string) {{
                    return {{
                        streams: [{{
                            url: "{media_base}/video.mp4",
                            kind: "mp4",
                            label: "Protected MP4",
                            flags: ["requires_proxy"],
                            headers: {{ Referer: "https://anthology.example" }}
                        }}],
                        subtitles: [],
                    }};
                }},
            }};
            "#
        );

        engine
            .install_package(
                build_plugin_package("anthology", plugin_ts),
                None,
                "local",
                "/tmp/anthology",
            )
            .await
            .expect("install anthology");

        let anthology = PluginId::new("anthology").expect("id");
        let detail = engine
            .details(&anthology, "/anime/gintama")
            .await
            .expect("details");
        assert_eq!(detail.episodes[0].absolute_episode, Some(50));

        let stream_res = engine.streams(&anthology, "ep-50").await.expect("streams");
        assert!(
            stream_res.streams[0]
                .flags
                .contains(&StreamFlag::RequiresProxy)
        );
        assert_eq!(stream_res.streams[0].kind, StreamKind::Mp4);

        let media = MediaRef::new(anthology, "/anime/gintama").expect("media ref");
        // Seed prior watch progress at 42_000 ms so play_candidate resumes automatically.
        let progress = PlaybackProgress {
            media: media.clone(),
            stream_id: "ep-50".into(),
            canonical_id: Some("anilist:918".into()),
            season: 1,
            episode: 50,
            absolute_episode: Some(50),
            title: None,
            poster_url: None,
            backdrop_url: None,
            episode_title: None,
            position_ms: 42_000,
            duration_ms: 1_440_000,
            completed: false,
            last_watched_at: "2026-11-23T10:00:00Z".into(),
        };
        engine
            .record_progress(&progress)
            .await
            .expect("save watch progress");

        let playback_url = engine
            .play_candidate(PlayCandidateRequest {
                media: media.clone(),
                canonical_id: Some("anilist:918".into()),
                season: 1,
                episode: 50,
                absolute_episode: Some(50),
                media_type: MediaType::Series,
                candidate: stream_res.streams[0].clone(),
                extra_subtitles: Vec::new(),
                start_position_ms: None,
                title: Some("Gintama — Episode 50".into()),
            })
            .await
            .expect("play candidate through Smart Proxy");

        assert!(playback_url.starts_with(&format!("{}/s/", engine.proxy().base_url())));
        let body = reqwest::get(&playback_url)
            .await
            .expect("fetch via proxy")
            .text()
            .await
            .expect("read proxied body");
        assert_eq!(body, "video-bytes");

        {
            let loaded = player.loaded.lock().expect("lock loaded");
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].start_position_ms, 42_000);
            assert!(loaded[0].headers.is_empty());
            assert_eq!(loaded[0].url, playback_url);
        }

        // Verify grant_plugin_domain persists to SQLite when permanent=true.
        engine
            .grant_plugin_domain(&media.plugin_id, "cdn2.anthology.example", true)
            .await
            .expect("grant domain");
        assert!(
            engine
                .storage()
                .is_domain_allowed(&media.plugin_id, "cdn2.anthology.example")
                .await
                .expect("check allowed domain")
        );

        engine.shutdown().await;
    }

    #[tokio::test]
    async fn ts_compile_cache_persists_through_sqlite_storage() {
        let actor = StorageActor::open_in_memory().await.expect("actor");
        let storage = SqliteStorage::new(actor);
        storage
            .upsert_ts_compile_cache("abc123", "export const x = 1;")
            .await
            .expect("upsert cache");
        let entries = storage.load_ts_compile_cache().await.expect("load cache");
        assert_eq!(
            entries,
            vec![("abc123".into(), "export const x = 1;".into())]
        );
        assert!(
            VesselError::Timeout
                .to_string()
                .contains("[VesselTimeoutError]")
        );
    }

    #[tokio::test]
    async fn cancel_active_search_does_not_poison_subsequent_details_or_subtitle_search() {
        let (engine, _player, _recorder) = test_engine().await;
        let plugin_ts = r#"
            export default {
                async search(query: string) {
                    const start = Date.now();
                    while (Date.now() - start < 250) {}
                    return { items: [{ pluginId: "anthology", nativeId: "n1", type: "movie", title: query }], hasMore: false };
                },
                async getDetails(nativeId: string) {
                    return {
                        pluginId: "anthology",
                        nativeId,
                        type: "movie",
                        title: "Recovered After Search Cancel",
                        episodes: [{ streamId: "s1", season: 1, episode: 1 }]
                    };
                }
            };
        "#;
        let mut pkg = build_plugin_package("anthology", plugin_ts);
        pkg.manifest.capabilities = vec![
            vessel_domain::PluginCapability::Vod,
            vessel_domain::PluginCapability::Search,
            vessel_domain::PluginCapability::Subtitles,
        ];
        engine
            .install_package(pkg, None, "test", "https://anthology.example/pkg.vessel")
            .await
            .expect("install anthology plugin");

        // Warm plugin and cancel an active search mid-flight.
        let handle = engine
            .search_all("matrix", std::collections::BTreeMap::new(), 1)
            .await;
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        engine.cancel_active_search().await;
        let _ = handle.done.await;

        // Subsequent details() on the exact same plugin must succeed without VesselCancelledError.
        let detail = engine
            .details(&PluginId::new("anthology").expect("id"), "n1")
            .await
            .expect("details after search cancel");
        assert_eq!(detail.title, "Recovered After Search Cancel");

        // Subtitle search skips VOD plugins without searchSubtitles() instead of returning errors.
        let outcome = engine
            .search_subtitles(
                SubtitleQuery {
                    title: "Matrix".into(),
                    original_title: None,
                    year: Some(1999),
                    media_type: MediaType::Movie,
                    season: None,
                    episode: None,
                    external_ids: ExternalIds::default(),
                    preferred_languages: vec!["tr".into()],
                },
                None,
            )
            .await
            .expect("subtitle search");
        assert!(
            outcome.errors.is_empty(),
            "unexpected subtitle errors: {:?}",
            outcome.errors
        );

        engine.shutdown().await;
    }

    #[tokio::test]
    async fn enriches_details_in_background_and_resumes_across_plugins_by_absolute_episode() {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock meta");
        let addr = listener.local_addr().expect("addr");
        let base = format!("http://{addr}");

        let app = Router::new().route(
            "/anilist",
            axum::routing::post(|| async {
                axum::Json(serde_json::json!({
                    "data": {
                        "Media": {
                            "id": 918,
                            "idMal": 918,
                            "title": { "romaji": "Gintama", "english": "Gintama", "native": "銀魂" },
                            "description": "Odd Jobs Gin.",
                            "seasonYear": 2006,
                            "averageScore": 89.0,
                            "genres": ["Action", "Comedy"],
                            "coverImage": { "extraLarge": "https://cdn.anilist.example/918-hd.jpg" },
                            "episodes": 201
                        }
                    }
                }))
            }),
        );
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let (mut engine, player, recorder) = test_engine().await;
        engine.set_metadata_endpoints(vessel_metadata::MetaEndpoints {
            anilist_graphql: format!("{base}/anilist"),
            tmdb_api_base: format!("{base}/tmdb"),
            tmdb_image_base: "https://image.tmdb.org/t/p".into(),
            cinemeta_base: format!("{base}/cinemeta"),
            ..vessel_metadata::MetaEndpoints::default()
        });

        let plugin_a_ts = r#"
            export default {
                async getDetails(nativeId: string) {
                    return {
                        pluginId: "anthology",
                        nativeId,
                        type: "series",
                        isAnime: true,
                        title: "Gintama",
                        year: 2006,
                        episodes: [{ streamId: "a-s2e1", season: 2, episode: 1, absoluteEpisode: 50 }]
                    };
                }
            };
        "#;
        engine
            .install_package(
                build_plugin_package("anthology", plugin_a_ts),
                None,
                "local",
                "/tmp/a",
            )
            .await
            .expect("install anthology");

        let anthology = PluginId::new("anthology").expect("id a");
        let (raw, enrich_task) = engine
            .details_with_enrichment(&anthology, "/gintama", "en")
            .await
            .expect("details");
        assert_eq!(raw.poster, None);
        enrich_task.await.expect("join enrich");

        {
            let events = recorder.events.lock().expect("lock events");
            assert!(events.iter().any(|ev| matches!(
                ev,
                EngineEvent::MetadataEnriched { detail, .. }
                    if detail.poster.as_deref() == Some("https://cdn.anilist.example/918-hd.jpg")
            )));
        }

        let media_a = MediaRef::new(anthology, "/gintama").expect("media a");
        engine
            .record_progress(&PlaybackProgress {
                media: media_a.clone(),
                stream_id: "a-s2e1".into(),
                canonical_id: Some("anilist:918".into()),
                season: 2,
                episode: 1,
                absolute_episode: Some(50),
                title: None,
                poster_url: None,
                backdrop_url: None,
                episode_title: None,
                position_ms: 123_456,
                duration_ms: 1_440_000,
                completed: false,
                last_watched_at: "2026-11-23T11:00:00Z".into(),
            })
            .await
            .expect("record progress on plugin a");

        let anizium = PluginId::new("anizium").expect("id b");
        let media_b = MediaRef::new(anizium, "/anizium/gintama").expect("media b");
        engine
            .play_candidate(PlayCandidateRequest {
                media: media_b,
                canonical_id: Some("anilist:918".into()),
                season: 1,
                episode: 50,
                absolute_episode: Some(50),
                media_type: MediaType::Series,
                candidate: vessel_domain::StreamCandidate {
                    url: "https://cdn.anizium.example/50.mp4".into(),
                    kind: StreamKind::Mp4,
                    quality: Some("1080p".into()),
                    label: "Anizium Direct".into(),
                    language: Some("tr".into()),
                    audio_type: None,
                    priority: None,
                    is_default: true,
                    flags: Vec::new(),
                    headers: BTreeMap::new(),
                    needs_extractor: false,
                    subtitles: Vec::new(),
                    skip_segments: Vec::new(),
                    plugin_id: None,
                    plugin_name: None,
                    torrent: None,
                    debrid_cache: None,
                },
                extra_subtitles: Vec::new(),
                start_position_ms: None,
                title: Some("Gintama — 50".into()),
            })
            .await
            .expect("play on plugin b");

        {
            let loaded = player.loaded.lock().expect("lock loaded");
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].start_position_ms, 123_456);
            assert_eq!(loaded[0].url, "https://cdn.anizium.example/50.mp4");
        }

        engine
            .set_metadata_override(&media_a, None, true)
            .await
            .expect("disable metadata override");
        let prev_count = recorder.events.lock().expect("lock").len();
        let (_, disabled_task) = engine
            .details_with_enrichment(&media_a.plugin_id, "/gintama", "en")
            .await
            .expect("details after disable");
        disabled_task.await.expect("join disabled");
        assert_eq!(recorder.events.lock().expect("lock").len(), prev_count);

        engine.shutdown().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn engine_syncs_iptv_searches_fts5_plays_via_proxy_and_completes_download() {
        use std::time::{Duration, SystemTime, UNIX_EPOCH};
        use vessel_domain::DownloadStatus;
        use vessel_downloader::DownloadRequest;
        use vessel_iptv::{IptvKind, IptvProfile};
        use vessel_storage::DownloadKind;

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let base_url = format!("http://{addr}");
        let m3u_base = base_url.clone();

        let app = Router::new()
            .route(
                "/playlist.m3u",
                get(move || async move {
                    format!(
                        "#EXTM3U\n#EXTINF:-1 tvg-id=\"ch.one\" group-title=\"News\",Anthology News HD\n{m3u_base}/live/news.m3u8\n"
                    )
                }),
            )
            .route(
                "/epg.xml",
                get(|| async {
                    r#"<tv><programme start="20261123120000 +0000" stop="20261123130000 +0000" channel="ch.one"><title>Noon Live</title></programme></tv>"#
                }),
            )
            .route(
                "/live/news.m3u8",
                get(|headers: HeaderMap| async move {
                    let ua = headers
                        .get(axum::http::header::USER_AGENT)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");
                    if ua == "VesselCustomUA/3.0" {
                        (StatusCode::OK, "#EXTM3U\n#EXTINF:4,\nseg.ts\n")
                    } else {
                        (StatusCode::FORBIDDEN, "bad ua")
                    }
                }),
            )
            .route("/offline.mp4", get(|| async { b"OFFLINE_MP4_BYTES".to_vec() }));

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dl_root = std::env::temp_dir().join(format!("vessel-engine-dl-{nanos}"));

        let actor = StorageActor::open_in_memory().await.expect("open actor");
        let secrets = Arc::new(SqliteSecretStore::new(actor.clone()));
        let storage = Arc::new(SqliteStorage::new(actor));
        let proxy = Arc::new(ProxyServer::start().await.expect("start proxy"));
        let player = Arc::new(MockPlayer::default());
        let recorder = Arc::new(EventRecorder::default());
        let engine = VesselEngine::from_parts_with_downloads(
            storage,
            secrets,
            proxy,
            player.clone(),
            recorder,
            SourceResolver::new().with_default_registries(),
            dl_root.clone(),
            false,
        )
        .await
        .expect("engine");

        engine
            .upsert_iptv_profile(
                IptvProfile {
                    id: "prof-1".into(),
                    name: "Anthology TV".into(),
                    kind: IptvKind::M3u,
                    server_url: format!("{base_url}/playlist.m3u"),
                    username: None,
                    secret_ref: None,
                    epg_url: Some(format!("{base_url}/epg.xml")),
                    user_agent: Some("VesselCustomUA/3.0".into()),
                    last_synced_at: None,
                },
                None,
            )
            .await
            .expect("upsert iptv profile");

        let synced_ch = engine.sync_iptv_profile("prof-1").await.expect("sync iptv");
        assert_eq!(synced_ch, 1);
        let synced_epg = engine.sync_iptv_epg("prof-1").await.expect("sync epg");
        assert_eq!(synced_epg, 1);

        let hits = engine
            .search_iptv_channels(Some("prof-1"), Some("antho"), None, false, 10)
            .await
            .expect("fts5 search");
        assert_eq!(hits.len(), 1);

        let proxied_url = engine
            .play_iptv_channel(&hits[0], Some("VesselCustomUA/3.0"))
            .await
            .expect("play iptv");
        assert!(proxied_url.starts_with("http://127.0.0.1:"));

        // Verify the player's proxied URL actually forwards our custom User-Agent.
        let body = reqwest::get(&proxied_url)
            .await
            .expect("fetch proxied")
            .text()
            .await
            .expect("proxied text");
        assert!(body.contains("#EXTM3U"));

        let now_ts = vessel_iptv::xmltv::parse_xmltv_timestamp("20261123121500 +0000").expect("ts");
        let nn = engine
            .get_epg_now_next("prof-1", "ch.one", now_ts)
            .await
            .expect("now next");
        assert_eq!(nn.now.as_ref().map(|p| p.title.as_str()), Some("Noon Live"));

        let dl = engine
            .enqueue_download(DownloadRequest {
                id: Some("dl-engine-1".into()),
                plugin_id: "anthology".into(),
                native_id: "offline-1".into(),
                title: "Anthology Offline".into(),
                season: None,
                episode: None,
                kind: DownloadKind::Direct,
                source_url: format!("{base_url}/offline.mp4"),
                headers: BTreeMap::new(),
                stream_label: Some("Offline MP4".into()),
                subtitles: Vec::new(),
            })
            .await
            .expect("enqueue download");

        let mut completed = None;
        for _ in 0..80 {
            if let Some(cur) = engine.get_download(&dl.id).await.expect("get dl")
                && cur.status == DownloadStatus::Completed
            {
                completed = Some(cur);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let completed = completed.expect("download finished");
        let saved = tokio::fs::read(completed.local_media_path.expect("path"))
            .await
            .expect("read downloaded file");
        assert_eq!(saved, b"OFFLINE_MP4_BYTES");

        engine
            .delete_download(&dl.id)
            .await
            .expect("delete download");
        let _ = tokio::fs::remove_dir_all(&dl_root).await;
        engine.shutdown().await;
    }

    #[tokio::test]
    async fn linked_sources_persist_and_merge_streams_across_plugins() {
        let (engine, _, _) = test_engine().await;
        let anthology_ts = r#"
            export default {
                async getDetails(id) {
                    return {
                        pluginId: "anthology",
                        nativeId: id,
                        type: "series",
                        isAnime: true,
                        title: "Gintama",
                        episodes: [{ streamId: "ant-ep-1", season: 1, episode: 1 }]
                    };
                },
                async getStreams(sid) {
                    return {
                        streams: [{ url: `https://cdn.anthology.example/${sid}.m3u8`, kind: "hls", label: "Anthology 1080p", isDefault: true }]
                    };
                }
            };
        "#;
        let mirror_ts = r#"
            export default {
                async search(query) {
                    return {
                        items: [{ nativeId: "mir-gintama", type: "series", isAnime: true, title: `Mirror: ${query}` }],
                        hasMore: false
                    };
                },
                async getDetails(id) {
                    return {
                        pluginId: "mirror-src",
                        nativeId: id,
                        type: "series",
                        isAnime: true,
                        title: "Gintama",
                        episodes: [{ streamId: "mir-ep-1", season: 1, episode: 1 }]
                    };
                },
                async getStreams(sid) {
                    return {
                        streams: [{ url: `https://cdn.mirror.example/${sid}.mp4`, kind: "mp4", label: "Mirror 720p" }]
                    };
                }
            };
        "#;

        engine
            .install_package(
                build_plugin_package("anthology", anthology_ts),
                None,
                "local",
                "/tmp/linked-a",
            )
            .await
            .expect("install anthology");
        engine
            .install_package(
                build_plugin_package("mirror-src", mirror_ts),
                None,
                "local",
                "/tmp/linked-b",
            )
            .await
            .expect("install mirror-src");

        let anthology = PluginId::new("anthology").expect("id a");
        let mirror = PluginId::new("mirror-src").expect("id b");
        let search_page = engine
            .search_plugin_sync(&mirror, "Gintama", 1)
            .await
            .expect("search single mirror plugin");
        assert_eq!(search_page.items.len(), 1);
        assert_eq!(search_page.items[0].native_id, "mir-gintama");

        let media = MediaRef::new(anthology, "/anime/gintama").expect("media");
        let saved = engine
            .set_linked_sources(
                &media,
                vec![LinkedMediaSource {
                    plugin_id: mirror.clone(),
                    plugin_name: Some("Mirror Source".into()),
                    native_id: "mir-gintama".into(),
                    title: "Gintama".into(),
                    poster: None,
                }],
            )
            .await
            .expect("save linked sources");
        assert_eq!(saved.len(), 1);

        let loaded = engine
            .get_linked_sources(&media)
            .await
            .expect("load linked");
        assert_eq!(loaded, saved);

        let merged = engine
            .streams_with_linked(&media, "ant-ep-1", 1, 1, None, false)
            .await
            .expect("resolve merged streams");
        assert_eq!(merged.streams.len(), 2);
        assert!(
            merged
                .streams
                .iter()
                .any(|s| s.plugin_id.as_deref() == Some("anthology"))
        );
        assert!(
            merged
                .streams
                .iter()
                .any(|s| s.plugin_id.as_deref() == Some("mirror-src"))
        );

        engine.shutdown().await;
    }

    #[tokio::test]
    async fn streams_home_feeds_per_plugin_and_completes() {
        let (engine, _, recorder) = test_engine().await;
        let alpha_ts = r#"
            export default {
                async getHomeFeed() {
                    return {
                        sections: [{
                            id: "sec-a",
                            title: "Alpha Section",
                            items: [{ nativeId: "a-1", type: "series", title: "Alpha Show" }]
                        }]
                    };
                }
            };
        "#;
        let beta_ts = r#"
            export default {
                async getHomeFeed() {
                    return {
                        sections: [{
                            id: "sec-b",
                            title: "Beta Section",
                            items: [{ nativeId: "b-1", type: "movie", title: "Beta Film" }]
                        }]
                    };
                }
            };
        "#;

        engine
            .install_package(
                build_plugin_package("anthology", alpha_ts),
                None,
                "local",
                "/tmp/stream-a",
            )
            .await
            .expect("install alpha");
        engine
            .install_package(
                build_plugin_package("beta-src", beta_ts),
                None,
                "local",
                "/tmp/stream-b",
            )
            .await
            .expect("install beta");

        let request_id = engine
            .start_home_feeds_stream(None)
            .await
            .expect("start home feeds stream");

        for _ in 0..80 {
            let done = {
                let guard = recorder.events.lock().expect("lock events");
                guard.iter().any(|ev| {
                    matches!(ev, EngineEvent::HomeFeedCompleted { request_id: rid } if rid == &request_id)
                })
            };
            if done {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(15)).await;
        }

        let events = recorder.events.lock().expect("lock events").clone();
        let partials: Vec<String> = events
            .iter()
            .filter_map(|ev| match ev {
                EngineEvent::HomeFeedPartialResult {
                    request_id: rid,
                    plugin_id,
                    sections,
                    ..
                } if rid == &request_id && !sections.is_empty() => {
                    Some(plugin_id.as_str().to_owned())
                }
                _ => None,
            })
            .collect();
        assert_eq!(partials.len(), 2);
        assert!(partials.contains(&"anthology".to_owned()));
        assert!(partials.contains(&"beta-src".to_owned()));
        assert!(events.iter().any(|ev| {
            matches!(ev, EngineEvent::HomeFeedCompleted { request_id: rid } if rid == &request_id)
        }));

        engine.shutdown().await;
    }

    #[tokio::test]
    async fn builtin_subtitles_container_bootstraps_persists_and_blocks_uninstall() {
        let (engine, _, recorder) = test_engine().await;
        let loaded = engine
            .bootstrap_plugins()
            .await
            .expect("bootstrap plugins including builtins");
        assert!(loaded >= 1);

        let list = engine.list_plugins().await.expect("list plugins");
        let container = list
            .iter()
            .find(|p| p.manifest.id.as_str() == builtins::SUBTITLES_CONTAINER_ID)
            .expect("subtitles-container present");
        assert_eq!(container.source_type, builtins::BUILTIN_SOURCE_TYPE);
        assert!(container.enabled);

        let container_id = PluginId::new(builtins::SUBTITLES_CONTAINER_ID).expect("valid id");
        let err = engine
            .uninstall_plugin(&container_id)
            .await
            .expect_err("refuse uninstalling built-in plugin");
        assert!(matches!(err, VesselError::InvalidInput(_)));

        assert_eq!(engine.list_subtitle_plugins().await.len(), 1);

        // User can disable and re-enable built-in plugin cleanly.
        engine
            .set_plugin_enabled(&container_id, false)
            .await
            .expect("disable subtitles-container");
        assert!(engine.list_subtitle_plugins().await.is_empty());

        engine
            .set_plugin_enabled(&container_id, true)
            .await
            .expect("re-enable subtitles-container");
        assert_eq!(engine.list_subtitle_plugins().await.len(), 1);

        // Home feed stream ignores subtitle-only plugins without throwing getHomeFeed error.
        let req_id = engine
            .start_home_feeds_stream(None)
            .await
            .expect("start home feed stream");
        for _ in 0..40 {
            let done = {
                let guard = recorder.events.lock().expect("lock events");
                guard.iter().any(|ev| {
                    matches!(ev, EngineEvent::HomeFeedCompleted { request_id } if request_id == &req_id)
                })
            };
            if done {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let has_sub_err = recorder
            .events
            .lock()
            .expect("lock events")
            .iter()
            .any(|ev| {
                matches!(
                    ev,
                    EngineEvent::HomeFeedPartialResult { plugin_id, error: Some(_), .. }
                        if plugin_id.as_str() == builtins::SUBTITLES_CONTAINER_ID
                )
            });
        assert!(
            !has_sub_err,
            "subtitle-only plugin must not run getHomeFeed"
        );

        engine.shutdown().await;
    }

    #[tokio::test]
    #[ignore = "Live network subtitle container smoke test"]
    async fn live_smoke_builtin_subtitles_container() {
        let (engine, _, _) = test_engine().await;
        engine.bootstrap_plugins().await.expect("bootstrap");

        // 1. Movie search (The Matrix tt0133093 -> OpenSubtitles v3 + TurkceAltyazi + YIFY)
        let movie_outcome = engine
            .search_subtitles(
                SubtitleQuery {
                    title: "The Matrix".into(),
                    original_title: Some("The Matrix".into()),
                    year: Some(1999),
                    media_type: MediaType::Movie,
                    season: None,
                    episode: None,
                    external_ids: ExternalIds {
                        imdb: Some("tt0133093".into()),
                        ..Default::default()
                    },
                    preferred_languages: vec!["tr".into(), "en".into()],
                },
                None,
            )
            .await
            .expect("movie search");
        eprintln!(
            "MOVIE HITS={} ERRORS={:?} SOURCES={:?}",
            movie_outcome.hits.len(),
            movie_outcome.errors,
            movie_outcome
                .hits
                .iter()
                .map(|h| h.candidate.source_name.as_deref().unwrap_or("?"))
                .collect::<Vec<_>>()
        );
        assert!(
            !movie_outcome.hits.is_empty(),
            "expected hits for The Matrix; errors={:?}",
            movie_outcome.errors
        );

        let container_id = PluginId::new(builtins::SUBTITLES_CONTAINER_ID).expect("valid id");
        if let Some(ta_hit) = movie_outcome
            .hits
            .iter()
            .find(|h| h.candidate.source_name.as_deref() == Some("TurkceAltyazi"))
        {
            let resolved = engine
                .resolve_subtitle(
                    &container_id,
                    &ta_hit.candidate.id,
                    &ta_hit.candidate.lang,
                    &ta_hit.candidate.label,
                )
                .await
                .expect("resolve TurkceAltyazi subtitle");
            match resolved.source {
                SubtitleSource::InlineContent { content } => {
                    assert!(content.contains("-->"), "expected valid SRT cues");
                }
                other => panic!("expected inline_content for TA, got {other:?}"),
            }
        }

        // 2. Series search (Breaking Bad S01E01 tt0903747 -> OpenSubtitles + SubDL + TurkceAltyazi + Addic7ed + TVSubtitles)
        let series_outcome = engine
            .search_subtitles(
                SubtitleQuery {
                    title: "Breaking Bad".into(),
                    original_title: Some("Breaking Bad".into()),
                    year: Some(2008),
                    media_type: MediaType::Series,
                    season: Some(1),
                    episode: Some(1),
                    external_ids: ExternalIds {
                        imdb: Some("tt0903747".into()),
                        ..Default::default()
                    },
                    preferred_languages: vec!["en".into(), "tr".into()],
                },
                None,
            )
            .await
            .expect("series search");
        eprintln!(
            "SERIES HITS={} ERRORS={:?} SOURCES={:?}",
            series_outcome.hits.len(),
            series_outcome.errors,
            series_outcome
                .hits
                .iter()
                .map(|h| h.candidate.source_name.as_deref().unwrap_or("?"))
                .collect::<Vec<_>>()
        );
        assert!(
            !series_outcome.hits.is_empty(),
            "expected hits for Breaking Bad; errors={:?}",
            series_outcome.errors
        );

        // Verify resolving OpenSubtitles (.gz) and SubDL (.zip) works end-to-end.
        if let Some(os_hit) = movie_outcome
            .hits
            .iter()
            .find(|h| h.candidate.source_name.as_deref() == Some("OpenSubtitles"))
        {
            let resolved = engine
                .resolve_subtitle(
                    &container_id,
                    &os_hit.candidate.id,
                    &os_hit.candidate.lang,
                    &os_hit.candidate.label,
                )
                .await
                .expect("resolve OpenSubtitles subtitle");
            match resolved.source {
                SubtitleSource::InlineContent { content } => {
                    assert!(content.contains("-->"), "expected valid OS cues");
                }
                SubtitleSource::Url { url, .. } => {
                    assert!(url.starts_with("https://"), "expected valid OS URL");
                }
            }
        }
        if let Some(subdl_hit) = series_outcome
            .hits
            .iter()
            .find(|h| h.candidate.source_name.as_deref() == Some("SubDL"))
        {
            let resolved = engine
                .resolve_subtitle(
                    &container_id,
                    &subdl_hit.candidate.id,
                    &subdl_hit.candidate.lang,
                    &subdl_hit.candidate.label,
                )
                .await
                .expect("resolve SubDL subtitle");
            match resolved.source {
                SubtitleSource::InlineContent { content } => {
                    assert!(content.contains("-->"), "expected valid SubDL cues");
                }
                other => panic!("expected inline_content for SubDL, got {other:?}"),
            }
        }

        // 3. Turkish-titled movie without IMDb ID ("Esaretin Bedeli" -> auto-enriches tt0111161)
        let tr_no_imdb = engine
            .search_subtitles(
                SubtitleQuery {
                    title: "Esaretin Bedeli".into(),
                    original_title: None,
                    year: Some(1994),
                    media_type: MediaType::Movie,
                    season: None,
                    episode: None,
                    external_ids: ExternalIds::default(),
                    preferred_languages: vec!["tr".into(), "en".into()],
                },
                None,
            )
            .await
            .expect("Turkish title without imdb search");
        eprintln!(
            "TR_NO_IMDB HITS={} ERRORS={:?} SOURCES={:?}",
            tr_no_imdb.hits.len(),
            tr_no_imdb.errors,
            tr_no_imdb
                .hits
                .iter()
                .map(|h| h.candidate.source_name.as_deref().unwrap_or("?"))
                .collect::<Vec<_>>()
        );
        assert!(
            !tr_no_imdb.hits.is_empty(),
            "expected hits for Esaretin Bedeli without IMDb ID"
        );

        // 4. Non-existent title must NOT return unrelated sidebar subtitles from TurkceAltyazi.
        let miss_outcome = engine
            .search_subtitles(
                SubtitleQuery {
                    title: "xyznonexistentvesseltest998877".into(),
                    original_title: None,
                    year: None,
                    media_type: MediaType::Series,
                    season: Some(1),
                    episode: Some(1),
                    external_ids: ExternalIds::default(),
                    preferred_languages: vec!["tr".into()],
                },
                None,
            )
            .await
            .expect("miss search");
        assert!(
            miss_outcome.hits.is_empty(),
            "expected 0 hits for nonexistent title, got {:?}",
            miss_outcome
                .hits
                .iter()
                .map(|h| (&h.candidate.source_name, &h.candidate.label))
                .collect::<Vec<_>>()
        );

        engine.shutdown().await;
    }

    #[tokio::test]
    async fn resolves_torrentio_stream_addon_via_community_debrid_and_virtual_metadata() {
        let app = Router::new()
            .route(
                "/cinemeta/meta/movie/tt0903747.json",
                get(|| async {
                    axum::Json(serde_json::json!({
                        "meta": {
                            "id": "tt0903747",
                            "type": "movie",
                            "name": "El Camino Test",
                            "releaseInfo": "2019",
                            "poster": "https://cinemeta.example/poster.jpg"
                        }
                    }))
                }),
            )
            .route(
                "/torrentio/stream/movie/tt0903747.json",
                get(|| async {
                    axum::Json(serde_json::json!({
                        "streams": [
                            {
                                "name": "Torrentio\n1080p",
                                "title": "El.Camino.2019.1080p.WEB-DL.mkv\n👤 150 💾 2.5 GB ⚙️ YTS",
                                "infoHash": "1111111111111111111111111111111111111111",
                                "fileIdx": 0
                            }
                        ]
                    }))
                }),
            );

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let (mut engine, player, _sink) = test_engine().await;
        engine.set_metadata_endpoints(crate::MetaEndpoints {
            cinemeta_base: format!("http://{addr}/cinemeta"),
            ..crate::MetaEndpoints::default()
        });

        // 1. Install a Stream-only addon (`PluginCapability::Stream`) simulating Torrentio output.
        let torrentio_manifest: vessel_domain::PluginManifest = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "id": "com.stremio.torrentio.test",
            "name": "Torrentio Test",
            "version": "1.0.0",
            "entry": "index.ts",
            "capabilities": ["stream"]
        }))
        .unwrap();
        let torrentio_bundle = PluginBundle::new([(
            "index.ts",
            r#"
            export default {
                async getStreams(streamId: string) {
                    if (!streamId.includes("tt0903747")) return { streams: [], subtitles: [] };
                    return {
                        streams: [{
                            url: "magnet:?xt=urn:btih:1111111111111111111111111111111111111111",
                            kind: "torrent",
                            quality: "1080p",
                            label: "El.Camino.2019.1080p.WEB-DL.mkv • 👤 150 • 💾 2.5 GB",
                            torrent: {
                                infoHash: "1111111111111111111111111111111111111111",
                                fileIndex: 0,
                                seeders: 150,
                                sizeBytes: 2684354560
                            }
                        }],
                        subtitles: []
                    };
                }
            };
            "#,
        )])
        .unwrap();
        engine
            .install_package(
                LoadedPackage {
                    manifest: torrentio_manifest,
                    bundle: torrentio_bundle,
                    icon_png: None,
                    sha256: "tor-sha".into(),
                },
                None,
                "stremio_addon",
                "mem://torrentio",
            )
            .await
            .expect("install torrentio");

        // 2. Install a Community Debrid plugin (`PluginCapability::Debrid`).
        let debrid_manifest: vessel_domain::PluginManifest = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "id": "community.seedr",
            "name": "Community Seedr",
            "version": "1.0.0",
            "entry": "index.ts",
            "capabilities": ["debrid"]
        }))
        .unwrap();
        let debrid_bundle = PluginBundle::new([(
            "index.ts",
            r#"
            import { definePlugin } from "vessel";
            export default definePlugin({
                async getDebridAccountStatus() {
                    return { authenticated: true, username: "seedr-test", isPremium: true };
                },
                async checkDebridCached(hashes: string[]) {
                    const out: Record<string, boolean> = {};
                    for (const h of hashes) out[h] = true;
                    return out;
                },
                async resolveDebridMagnet(req) {
                    return {
                        outcome: "ready",
                        url: `https://cdn.seedr.example/${req.infoHash}.mkv`,
                        filename: "El.Camino.2019.1080p.WEB-DL.mkv",
                        sizeBytes: 2684354560
                    };
                }
            });
            "#,
        )])
        .unwrap();
        engine
            .install_package(
                LoadedPackage {
                    manifest: debrid_manifest,
                    bundle: debrid_bundle,
                    icon_png: None,
                    sha256: "seedr-sha".into(),
                },
                None,
                "local_dev",
                "mem://seedr",
            )
            .await
            .expect("install community debrid");

        // 3. Open virtual Cinemeta detail without any catalog plugin -> streams auto-aggregate Torrentio + probe cache!
        let detail = engine
            .get_virtual_metadata_detail("imdb:tt0903747", Some("El Camino"), Some(MediaType::Movie), "en")
            .await
            .expect("virtual detail");
        assert_eq!(detail.external_ids.imdb.as_deref(), Some("tt0903747"));

        let media = MediaRef::new(PluginId::new("vessel.metadata").unwrap(), detail.native_id.clone()).unwrap();
        let sr = engine
            .streams_with_external_ids(
                &media,
                &detail.episodes[0].stream_id,
                1,
                1,
                None,
                true,
                Some(&detail.external_ids),
            )
            .await
            .expect("streams");
        assert_eq!(sr.streams.len(), 1);
        assert_eq!(sr.streams[0].kind, StreamKind::Torrent);
        assert_eq!(
            sr.streams[0].debrid_cache.as_ref().map(|c| c.state),
            Some(vessel_domain::DebridCacheState::Cached)
        );

        // 4. Playing the torrent candidate automatically resolves it via Community Debrid!
        let played_url = engine
            .play_candidate(PlayCandidateRequest {
                media,
                canonical_id: Some("imdb:tt0903747".into()),
                season: 1,
                episode: 1,
                absolute_episode: None,
                media_type: MediaType::Movie,
                candidate: sr.streams[0].clone(),
                extra_subtitles: Vec::new(),
                start_position_ms: Some(0),
                title: Some("El Camino".into()),
            })
            .await
            .expect("play torrent candidate");
        assert!(played_url.contains("cdn.seedr.example/1111111111111111111111111111111111111111.mkv"));
        {
            let loaded = player.loaded.lock().unwrap();
            assert_eq!(loaded.len(), 1);
        }
        let stats = engine.get_playback_stats().await;
        assert!(stats.is_debrid_stream);
        assert_eq!(stats.file_size_bytes, Some(2684354560));

        engine.shutdown().await;
    }
}
