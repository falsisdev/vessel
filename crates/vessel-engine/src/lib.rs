//! Pure-Rust core orchestrator (`VesselEngine`) binding plugins, storage, proxy, and playback.

pub mod pool;

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use vessel_domain::{
    ContentProvider, DomainGrantSource, EngineEvent, EventSink, ExternalIds, HomeFeed, LibraryItem,
    LibraryStatus, LogLevel, MediaDetail, MediaRef, MediaType, PlaybackProgress, PlaybackRequest,
    PlayerBackend, PluginId, SecretStore, StoragePort, StreamCandidate, StreamResult,
    SubtitleSource, SubtitleTrack, UiPreferences, VesselError, VesselPaths, VesselResult,
};
use vessel_iptv::{IptvClient, channel_to_stream_candidate, xtream::xtream_xmltv_url};
use vessel_metadata::{
    CanonicalMetadata, MetaEndpoints, MetadataService, merge_detail_with_override,
};
use vessel_plugin_rt::{
    LoadedPackage, ResolvedSource, SourceResolver, SourceTrustBadge, TsCompileCache,
    load_plugin_dir, load_plugin_package, pack_plugin_package,
};
use vessel_proxy::ProxyServer;
use vessel_storage::{
    BackupImportMode, ContinueWatchingEntry, NewIptvChannel, SqliteSecretStore, SqliteStorage,
    StorageActor, StoredDownload, StoredEpgNowNext, StoredEpgProgram, StoredIptvChannel,
    StoredIptvKind, StoredIptvProfile, StoredPlugin,
};

pub use pool::{DEFAULT_IDLE_TTL, DEFAULT_MAX_WARM_PLUGINS, PluginPool};
pub use vessel_downloader::{DownloadManager, DownloadRequest};
pub use vessel_iptv::{EpgNowNext, EpgProgram, IptvChannel, IptvKind, IptvProfile};

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
    secrets: Arc<SqliteSecretStore>,
    proxy: Arc<ProxyServer>,
    player: Arc<dyn PlayerBackend>,
    event_sink: Arc<dyn EventSink>,
    ts_cache: Arc<TsCompileCache>,
    source_resolver: SourceResolver,
    metadata: MetadataService,
    iptv: IptvClient,
    downloader: DownloadManager,
    pool: Arc<PluginPool>,
    always_proxy: AtomicBool,
    metadata_enabled: AtomicBool,
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
        let secrets = Arc::new(SqliteSecretStore::new(actor.clone()));
        let storage = Arc::new(SqliteStorage::new(actor));
        let proxy = Arc::new(ProxyServer::start().await?);
        Self::from_parts_with_downloads(
            storage,
            secrets,
            proxy,
            player,
            event_sink,
            SourceResolver::new(),
            paths.downloads_dir(),
            always_proxy,
        )
        .await
    }

    /// Constructs `VesselEngine` from already-initialized storage, proxy, and player components.
    pub async fn from_parts(
        storage: Arc<SqliteStorage>,
        secrets: Arc<SqliteSecretStore>,
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
        secrets: Arc<SqliteSecretStore>,
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
            pool,
            always_proxy: AtomicBool::new(always_proxy),
            metadata_enabled: AtomicBool::new(true),
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
    pub fn secrets(&self) -> &Arc<SqliteSecretStore> {
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

    /// Current `Always Proxy` routing override setting.
    pub fn always_proxy(&self) -> bool {
        self.always_proxy.load(Ordering::Acquire)
    }

    /// Updates whether plain `.mp4`/`.mkv` streams are also forced through the local proxy.
    pub fn set_always_proxy(&self, enabled: bool) {
        self.always_proxy.store(enabled, Ordering::Release);
    }

    /// Resolves any repository or plugin source (`https://`, `gh:`, `gl:`, shortcode, `vessel://`).
    pub async fn resolve_source(&self, raw_input: &str) -> VesselResult<ResolvedSource> {
        self.source_resolver.resolve(raw_input).await
    }

    /// Persists a `LoadedPackage`, records manifest-declared domains, and registers it in the pool.
    pub async fn install_package(
        &self,
        package: LoadedPackage,
        repo_id: Option<String>,
        source_type: &str,
        local_path: &str,
    ) -> VesselResult<()> {
        package.manifest.validate()?;
        let plugin_id = package.manifest.id.clone();

        let stored = StoredPlugin {
            manifest: package.manifest.clone(),
            repo_id,
            source_type: source_type.to_owned(),
            local_path: local_path.to_owned(),
            sha256: Some(package.sha256.clone()),
            enabled: true,
            allow_dynamic_domains: package.manifest.allow_any_https,
            installed_at: "1970-01-01T00:00:00Z".into(),
            updated_at: "1970-01-01T00:00:00Z".into(),
        };
        // Persist timestamp consistently using SQLite CURRENT / now when upserting domain grants below.
        self.storage.upsert_plugin(stored).await?;

        for domain in &package.manifest.domains {
            let trimmed = domain.trim();
            if !trimmed.is_empty() {
                self.storage
                    .grant_domain(&plugin_id, trimmed, DomainGrantSource::Manifest)
                    .await?;
            }
        }

        let extra_domains = self.storage.list_allowed_domains(&plugin_id).await?;
        self.pool
            .register(
                package.manifest,
                Arc::new(package.bundle),
                extra_domains,
                true,
            )
            .await?;

        Ok(())
    }

    /// Resolves `raw_input` and installs a single `.plugin.vessel` or open directory package directly.
    pub async fn install_package_from_source(&self, raw_input: &str) -> VesselResult<PluginId> {
        match self.source_resolver.resolve(raw_input).await? {
            ResolvedSource::Package {
                resolved_url,
                package,
                ..
            } => {
                let id = package.manifest.id.clone();
                self.install_package(package, None, "source", &resolved_url)
                    .await?;
                Ok(id)
            }
            ResolvedSource::Repository { repo, .. } => Err(VesselError::InvalidInput(format!(
                "source '{}' resolved to repository '{}' rather than a single plugin package",
                raw_input, repo.name
            ))),
        }
    }

    /// Approves an additional HTTPS domain for `plugin_id` (optionally persisting permanently in SQLite).
    pub async fn grant_plugin_domain(
        &self,
        plugin_id: &PluginId,
        domain: &str,
        permanent: bool,
    ) -> VesselResult<()> {
        self.pool.allow_domain(plugin_id, domain).await?;
        if permanent {
            self.storage
                .grant_domain(plugin_id, domain, DomainGrantSource::UserPermanent)
                .await?;
        }
        Ok(())
    }

    /// Lists all persisted plugins installed in SQLite.
    pub async fn list_plugins(&self) -> VesselResult<Vec<StoredPlugin>> {
        self.storage.list_plugins().await
    }

    /// Lists active (enabled) plugin IDs currently registered in the pool.
    pub async fn enabled_plugin_ids(&self) -> Vec<PluginId> {
        self.pool.enabled_plugin_ids().await
    }

    /// Lists approved HTTPS domains for `plugin_id`.
    pub async fn list_allowed_domains(&self, plugin_id: &PluginId) -> VesselResult<Vec<String>> {
        self.storage.list_allowed_domains(plugin_id).await
    }

    /// Enables or disables an installed plugin across SQLite and the warm QuickJS pool.
    pub async fn set_plugin_enabled(
        &self,
        plugin_id: &PluginId,
        enabled: bool,
    ) -> VesselResult<()> {
        self.clear_plugin_caches(plugin_id).await;
        self.storage.set_plugin_enabled(plugin_id, enabled).await?;
        self.pool.set_enabled(plugin_id, enabled).await?;
        Ok(())
    }

    /// Resolves `raw_input` and persists the package into `plugins_dir` (or tracks local Dev Mode dir),
    /// tagging an optional `repo_id` group label when installed from a repository.
    pub async fn install_package_persisted_with_repo(
        &self,
        plugins_dir: &Path,
        raw_input: &str,
        repo_id: Option<String>,
    ) -> VesselResult<(PluginId, SourceTrustBadge)> {
        let inferred_repo = repo_id.or_else(|| infer_repo_group(raw_input));
        match self.source_resolver.resolve(raw_input).await? {
            ResolvedSource::Package {
                resolved_url,
                package,
                badge,
            } => {
                let id = package.manifest.id.clone();
                let dev_path = Path::new(&resolved_url);
                if dev_path.is_dir() {
                    self.install_package(package, inferred_repo, "dev_dir", &resolved_url)
                        .await?;
                } else {
                    std::fs::create_dir_all(plugins_dir).map_err(|err| {
                        VesselError::Storage(format!("create plugins directory: {err}"))
                    })?;
                    let pkg_path = plugins_dir.join(format!("{}.plugin.vessel", id.as_str()));
                    let bytes = pack_plugin_package(
                        &package.manifest,
                        package.bundle.files(),
                        package.icon_png.as_deref(),
                    )?;
                    std::fs::write(&pkg_path, bytes).map_err(|err| {
                        VesselError::Storage(format!("write plugin archive: {err}"))
                    })?;
                    let local_path = pkg_path.to_string_lossy().into_owned();
                    self.install_package(package, inferred_repo, "package", &local_path)
                        .await?;
                }
                Ok((id, badge))
            }
            ResolvedSource::Repository { repo, .. } => Err(VesselError::InvalidInput(format!(
                "source '{}' resolved to repository '{}' rather than a single plugin package",
                raw_input, repo.name
            ))),
        }
    }

    /// Resolves `raw_input` and persists the package into `plugins_dir` (or tracks local Dev Mode dir).
    pub async fn install_package_persisted(
        &self,
        plugins_dir: &Path,
        raw_input: &str,
    ) -> VesselResult<(PluginId, SourceTrustBadge)> {
        self.install_package_persisted_with_repo(plugins_dir, raw_input, None)
            .await
    }

    /// Reloads all persisted plugins from SQLite and their local files/directories into the pool.
    pub async fn bootstrap_plugins(&self) -> VesselResult<usize> {
        let plugins = self.storage.list_plugins().await?;
        let mut loaded_count = 0usize;
        for stored in plugins {
            let path = PathBuf::from(&stored.local_path);
            let pkg_res = if stored.source_type == "dev_dir" || path.is_dir() {
                load_plugin_dir(&path)
            } else {
                std::fs::read(&path)
                    .map_err(|err| {
                        VesselError::Storage(format!(
                            "read plugin archive '{}': {err}",
                            path.display()
                        ))
                    })
                    .and_then(|bytes| load_plugin_package(&bytes, None))
            };

            match pkg_res {
                Ok(pkg) => {
                    let plugin_id = pkg.manifest.id.clone();
                    let extra_domains = self.storage.list_allowed_domains(&plugin_id).await?;
                    if let Err(err) = self
                        .pool
                        .register(
                            pkg.manifest,
                            Arc::new(pkg.bundle),
                            extra_domains,
                            stored.enabled,
                        )
                        .await
                    {
                        self.event_sink.emit(EngineEvent::PluginLog {
                            plugin_id,
                            level: LogLevel::Warn,
                            message: format!("Failed registering plugin on startup: {err}"),
                        });
                    } else {
                        if let Ok(settings) = self.get_plugin_settings(&plugin_id).await {
                            let _ = self.pool.set_settings(&plugin_id, settings).await;
                        }
                        loaded_count += 1;
                    }
                }
                Err(err) => {
                    self.event_sink.emit(EngineEvent::PluginLog {
                        plugin_id: stored.manifest.id,
                        level: LogLevel::Warn,
                        message: format!("Failed loading plugin on startup: {err}"),
                    });
                }
            }
        }
        Ok(loaded_count)
    }

    /// Uninstalls `plugin_id`, removing packaged archives (never deleting Dev Mode folders) and DB rows.
    pub async fn uninstall_plugin(&self, plugin_id: &PluginId) -> VesselResult<()> {
        self.clear_plugin_caches(plugin_id).await;
        let existing = self.storage.get_plugin(plugin_id).await?;
        self.pool.unregister(plugin_id).await;
        if let Some(stored) = existing
            && stored.source_type != "dev_dir"
        {
            let path = Path::new(&stored.local_path);
            if path.is_file() {
                let _ = std::fs::remove_file(path);
            }
        }
        self.storage.remove_plugin(plugin_id).await
    }

    /// Reads declarative modular UI preferences (`ui_preferences`) from SQLite.
    pub async fn get_ui_preferences(&self) -> VesselResult<UiPreferences> {
        self.storage.get_ui_preferences().await
    }

    /// Persists declarative modular UI preferences (`ui_preferences`) into SQLite.
    pub async fn set_ui_preferences(&self, prefs: &UiPreferences) -> VesselResult<()> {
        self.storage.set_ui_preferences(prefs).await
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
        self.storage.upsert_library(item).await
    }

    /// Removes a media reference from the user library.
    pub async fn delete_library_item(&self, media: &MediaRef) -> VesselResult<()> {
        self.storage.delete_library_item(media).await
    }

    /// Exports a `.backup.vessel` JSON snapshot.
    pub async fn export_backup_json(&self, include_secrets: bool) -> VesselResult<String> {
        self.storage.export_backup_json(include_secrets).await
    }

    /// Imports a `.backup.vessel` JSON snapshot using `SmartMerge` or `ResetAndImport`.
    pub async fn import_backup_json(&self, json: &str, mode: BackupImportMode) -> VesselResult<()> {
        self.storage.import_backup_json(json, mode).await
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
    pub async fn cancel_active_search(&self) {
        let prev = { self.active_search.lock().await.take() };
        if let Some((_, token)) = prev {
            token.cancel();
            self.pool.cancel_inflight().await;
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
        self.cancel_active_search().await;
        self.pool.reset_cancellations().await;

        let search_id = next_unique_id("search");
        let token = CancellationToken::new();
        {
            let mut active = self.active_search.lock().await;
            *active = Some((search_id.clone(), token.clone()));
        }

        let plugin_ids = self.pool.enabled_plugin_ids().await;
        let query_owned = query.to_owned();
        let pool = Arc::clone(&self.pool);
        let sink = Arc::clone(&self.event_sink);
        let active_search = Arc::clone(&self.active_search);
        let token_clone = token.clone();
        let sid = search_id.clone();

        let done = tokio::spawn(async move {
            let mut set = JoinSet::new();

            for plugin_id in plugin_ids {
                let pool = Arc::clone(&pool);
                let sink = Arc::clone(&sink);
                let token = token_clone.clone();
                let q = query_owned.clone();
                let f = filters.clone();
                let sid_inner = sid.clone();

                set.spawn(async move {
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
                        provider.sandbox().cancel();
                        return;
                    }

                    let watcher_token = token.clone();
                    let watcher_provider = Arc::clone(&provider);
                    let cancel_watcher = tokio::spawn(async move {
                        watcher_token.cancelled().await;
                        watcher_provider.sandbox().cancel();
                    });

                    let outcome = provider.search(&q, &f, page).await;
                    cancel_watcher.abort();

                    if token.is_cancelled() {
                        return;
                    }

                    match outcome {
                        Ok(search_page) => {
                            sink.emit(EngineEvent::SearchPartialResult {
                                search_id: sid_inner,
                                plugin_id,
                                items: search_page.items,
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

    /// Fetches home feed shelves from a single plugin (serving fresh entries from a 3-min RAM cache).
    pub async fn home_feed(
        &self,
        plugin_id: &PluginId,
        cursor: Option<&str>,
    ) -> VesselResult<HomeFeed> {
        let key = (plugin_id.clone(), cursor.map(str::to_owned));
        let now = Instant::now();
        {
            let cache = self.home_feed_cache.lock().await;
            if let Some((cached_at, feed)) = cache.get(&key)
                && now.saturating_duration_since(*cached_at) < HOME_FEED_CACHE_TTL
            {
                return Ok(feed.clone());
            }
        }

        let provider = self.pool.get_or_load(plugin_id).await?;
        let feed = provider.home_feed(cursor).await?;

        let mut cache = self.home_feed_cache.lock().await;
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
                &storage, &secrets, &metadata, &sink, &pid, &raw_clone, &lang, enabled,
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
            .await
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

    /// Routes `candidate` through Smart Proxy, merges subtitles, resolves resume offset,
    /// and dispatches `PlaybackRequest` to `PlayerBackend::load`.
    pub async fn play_candidate(&self, req: PlayCandidateRequest) -> VesselResult<String> {
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
        self.storage.upsert_progress(progress).await
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
    pub async fn upsert_iptv_profile(
        &self,
        profile: IptvProfile,
        password: Option<&str>,
    ) -> VesselResult<StoredIptvProfile> {
        let secret_ref = match password {
            Some(pass) => {
                let key = format!("iptv:{}:password", profile.id);
                self.secrets.set(&key, pass).await?;
                Some(key)
            }
            None => profile.secret_ref,
        };
        let stored = StoredIptvProfile {
            id: profile.id,
            name: profile.name,
            kind: match profile.kind {
                IptvKind::Xtream => StoredIptvKind::Xtream,
                IptvKind::M3u => StoredIptvKind::M3u,
            },
            server_url: profile.server_url,
            username: profile.username,
            secret_ref,
            epg_url: profile.epg_url,
            user_agent: profile.user_agent,
            last_synced_at: profile.last_synced_at,
        };
        self.storage.upsert_iptv_profile(&stored).await?;
        Ok(stored)
    }

    /// Lists configured IPTV profiles.
    pub async fn list_iptv_profiles(&self) -> VesselResult<Vec<StoredIptvProfile>> {
        self.storage.list_iptv_profiles().await
    }

    /// Deletes an IPTV profile, its channels/EPG (via cascade), and any stored password secret.
    pub async fn delete_iptv_profile(&self, profile_id: &str) -> VesselResult<()> {
        let profiles = self.storage.list_iptv_profiles().await?;
        if let Some(target) = profiles.into_iter().find(|p| p.id == profile_id)
            && let Some(secret_ref) = target.secret_ref
        {
            self.secrets.delete(&secret_ref).await?;
        }
        self.storage.delete_iptv_profile(profile_id).await
    }

    /// Syncs channels for `profile_id` from its M3U or Xtream Codes source, preserving favorites.
    pub async fn sync_iptv_profile(&self, profile_id: &str) -> VesselResult<usize> {
        let profile = self
            .storage
            .list_iptv_profiles()
            .await?
            .into_iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| {
                VesselError::NotFound(format!("IPTV profile '{profile_id}' not found"))
            })?;

        let channels = match profile.kind {
            StoredIptvKind::M3u => {
                self.iptv
                    .fetch_m3u(&profile.server_url, profile.user_agent.as_deref())
                    .await?
            }
            StoredIptvKind::Xtream => {
                let username = profile.username.as_deref().ok_or_else(|| {
                    VesselError::InvalidInput("Xtream profile missing username".into())
                })?;
                let password = match profile.secret_ref.as_deref() {
                    Some(key) => self.secrets.get(key).await?.unwrap_or_default(),
                    None => String::new(),
                };
                self.iptv
                    .fetch_xtream_channels(
                        &profile.server_url,
                        username,
                        &password,
                        profile.user_agent.as_deref(),
                    )
                    .await?
            }
        };

        let converted: Vec<NewIptvChannel> = channels
            .into_iter()
            .map(|ch| NewIptvChannel {
                stream_id: ch.stream_id,
                name: ch.name,
                group_title: ch.group_title,
                logo_url: ch.logo_url,
                stream_url: ch.stream_url,
                tvg_id: ch.tvg_id,
            })
            .collect();

        self.storage
            .replace_iptv_channels(profile_id, converted)
            .await
    }

    /// Downloads and stores XMLTV EPG entries for `profile_id`.
    pub async fn sync_iptv_epg(&self, profile_id: &str) -> VesselResult<usize> {
        let profile = self
            .storage
            .list_iptv_profiles()
            .await?
            .into_iter()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| {
                VesselError::NotFound(format!("IPTV profile '{profile_id}' not found"))
            })?;

        let epg_url = match profile.epg_url {
            Some(url) if !url.trim().is_empty() => url,
            _ if profile.kind == StoredIptvKind::Xtream => {
                let username = profile.username.as_deref().unwrap_or_default();
                let password = match profile.secret_ref.as_deref() {
                    Some(key) => self.secrets.get(key).await?.unwrap_or_default(),
                    None => String::new(),
                };
                xtream_xmltv_url(&profile.server_url, username, &password)?
            }
            _ => return Ok(0),
        };

        let programs = self
            .iptv
            .fetch_xmltv_epg(&epg_url, profile.user_agent.as_deref())
            .await?;
        let stored: Vec<StoredEpgProgram> = programs
            .into_iter()
            .map(|p| StoredEpgProgram {
                channel_tvg_id: p.channel_tvg_id,
                title: p.title,
                description: p.description,
                start_ts: p.start_ts,
                end_ts: p.end_ts,
            })
            .collect();
        self.storage.replace_epg_programs(profile_id, stored).await
    }

    /// Searches synced IPTV channels using SQLite FTS5 prefix matching and optional filters.
    pub async fn search_iptv_channels(
        &self,
        profile_id: Option<&str>,
        query: Option<&str>,
        group_title: Option<&str>,
        favorites_only: bool,
        limit: usize,
    ) -> VesselResult<Vec<StoredIptvChannel>> {
        self.storage
            .search_iptv_channels(profile_id, query, group_title, favorites_only, limit)
            .await
    }

    /// Lists sorted IPTV category group names for `profile_id`.
    pub async fn list_iptv_groups(&self, profile_id: &str) -> VesselResult<Vec<String>> {
        self.storage.list_iptv_groups(profile_id).await
    }

    /// Marks or unmarks an IPTV channel row as favorite.
    pub async fn set_iptv_favorite(&self, channel_id: i64, is_favorite: bool) -> VesselResult<()> {
        self.storage
            .set_iptv_favorite(channel_id, is_favorite)
            .await
    }

    /// Deletes expired XMLTV EPG entries ending at or before `before_ts`.
    pub async fn prune_expired_epg(&self, before_ts: i64) -> VesselResult<usize> {
        self.storage.prune_expired_epg(before_ts).await
    }

    /// Returns active (`now`) and upcoming (`next`) programs for a channel `tvg_id`.
    pub async fn get_epg_now_next(
        &self,
        profile_id: &str,
        channel_tvg_id: &str,
        now_ts: i64,
    ) -> VesselResult<EpgNowNext> {
        let StoredEpgNowNext { now, next } = self
            .storage
            .get_epg_now_next(profile_id, channel_tvg_id, now_ts)
            .await?;
        Ok(EpgNowNext {
            now: now.map(|p| EpgProgram {
                channel_tvg_id: p.channel_tvg_id,
                title: p.title,
                description: p.description,
                start_ts: p.start_ts,
                end_ts: p.end_ts,
            }),
            next: next.map(|p| EpgProgram {
                channel_tvg_id: p.channel_tvg_id,
                title: p.title,
                description: p.description,
                start_ts: p.start_ts,
                end_ts: p.end_ts,
            }),
        })
    }

    /// Routes an IPTV live channel through Smart Proxy (preserving custom `User-Agent`) and plays it.
    pub async fn play_iptv_channel(
        &self,
        channel: &StoredIptvChannel,
        profile_user_agent: Option<&str>,
    ) -> VesselResult<String> {
        let iptv_ch = IptvChannel {
            stream_id: channel.stream_id.clone(),
            name: channel.name.clone(),
            group_title: channel.group_title.clone(),
            logo_url: channel.logo_url.clone(),
            stream_url: channel.stream_url.clone(),
            tvg_id: channel.tvg_id.clone(),
            user_agent: None,
        };
        let candidate = channel_to_stream_candidate(&iptv_ch, profile_user_agent);
        let media = MediaRef::new(PluginId::new("iptv")?, channel.stream_id.clone())?;
        self.play_candidate(PlayCandidateRequest {
            media,
            canonical_id: None,
            season: 0,
            episode: 0,
            absolute_episode: None,
            media_type: MediaType::Movie,
            candidate,
            extra_subtitles: Vec::new(),
            start_position_ms: Some(0),
            title: Some(channel.name.clone()),
        })
        .await
    }

    /// Enqueues an offline media download (bounded to `MAX_CONCURRENT_DOWNLOADS = 2`).
    pub async fn enqueue_download(&self, req: DownloadRequest) -> VesselResult<StoredDownload> {
        self.downloader.enqueue(req).await
    }

    /// Pauses an active or queued download while preserving partial files / segment progress.
    pub async fn pause_download(&self, download_id: &str) -> VesselResult<()> {
        self.downloader.pause(download_id).await
    }

    /// Resumes a paused or failed download from its SQLite checkpoint or HTTP `Range` offset.
    pub async fn resume_download(&self, download_id: &str) -> VesselResult<StoredDownload> {
        self.downloader.resume(download_id).await
    }

    /// Cancels, removes local media/subtitles folder, and deletes DB state for `download_id`.
    pub async fn delete_download(&self, download_id: &str) -> VesselResult<()> {
        self.downloader.delete(download_id).await
    }

    /// Lists all tracked offline downloads.
    pub async fn list_downloads(&self) -> VesselResult<Vec<StoredDownload>> {
        self.downloader.list().await
    }

    /// Reads one download record by ID.
    pub async fn get_download(&self, download_id: &str) -> VesselResult<Option<StoredDownload>> {
        self.downloader.get(download_id).await
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

fn apply_canonical_override_hint(detail: &mut MediaDetail, canonical_id: &str) {
    detail.external_ids = ExternalIds::default();
    if let Some(rest) = canonical_id.strip_prefix("anilist:") {
        if let Ok(id) = rest.parse::<u64>() {
            detail.external_ids.anilist = Some(id);
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
    secrets: &SqliteSecretStore,
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
        "meta:{}:{}:{}",
        plugin_id.as_str(),
        raw.native_id,
        preferred_lang.to_ascii_lowercase()
    );
    if let Some(cached_json) = storage.get_metadata_cache(&cache_key).await?
        && let Ok(cached_canon) = serde_json::from_str::<CanonicalMetadata>(&cached_json)
    {
        let merged = merge_detail_with_override(raw, &cached_canon, preferred_lang, has_override);
        sink.emit(EngineEvent::MetadataEnriched {
            plugin_id: plugin_id.clone(),
            native_id: raw.native_id.clone(),
            detail: Box::new(merged.clone()),
        });
        return Ok(Some(merged));
    }

    let tmdb_key = secrets.get(TMDB_API_KEY_SECRET).await?;
    let outcome = metadata
        .enrich_detail(
            &query_detail,
            preferred_lang,
            tmdb_key.as_deref(),
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
        sink.emit(EngineEvent::MetadataEnriched {
            plugin_id: plugin_id.clone(),
            native_id: raw.native_id.clone(),
            detail: Box::new(merged.clone()),
        });
        return Ok(Some(merged));
    }

    Ok(None)
}

fn infer_repo_group(raw_input: &str) -> Option<String> {
    let clean = raw_input
        .trim()
        .strip_prefix("nuvio-plugin://")
        .unwrap_or(raw_input.trim());
    if let Some(rest) = clean.strip_prefix("gh:") {
        let mut parts = rest.split('/');
        if let (Some(owner), Some(repo)) = (parts.next(), parts.next()) {
            let repo_clean = repo.split('@').next().unwrap_or(repo);
            return Some(format!("{owner}/{repo_clean}"));
        }
    }
    if let Some(after_raw) = clean.strip_prefix("https://raw.githubusercontent.com/") {
        let mut parts = after_raw.split('/');
        if let (Some(owner), Some(repo)) = (parts.next(), parts.next()) {
            return Some(format!("{owner}/{repo}"));
        }
    }
    if clean.starts_with("stremio://") || clean.ends_with("/manifest.json") {
        return Some("Stremio & External".into());
    }
    None
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
        EngineEvent, EventSink, MediaRef, MediaType, PlaybackProgress, PlaybackRequest,
        PlayerBackend, PluginId, PluginManifest, StreamFlag, StreamKind, SubtitleTrack,
        VesselError, VesselResult,
    };
    use vessel_plugin_rt::{LoadedPackage, PluginBundle, SourceResolver, sha256_hex};
    use vessel_proxy::ProxyServer;
    use vessel_storage::{SqliteSecretStore, SqliteStorage, StorageActor};

    use super::{PlayCandidateRequest, VesselEngine};

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
            SourceResolver::new(),
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
            SourceResolver::new(),
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
}
