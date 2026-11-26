//! Pure-Rust core orchestrator (`VesselEngine`) binding plugins, storage, proxy, and playback.

pub mod builtins;
pub mod pool;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use vessel_domain::{
    ContentProvider, DomainGrantSource, DownloadStatus, EngineEvent, EventSink, ExternalIds,
    HomeFeed, LibraryItem, LibraryStatus, LinkedMediaSource, LogLevel, MediaDetail, MediaRef,
    MediaType, PlaybackProgress, PlaybackRequest, PlayerBackend, PluginCapability, PluginId,
    RepoPluginEntry, SearchPage, SecretStore, SkipSegment, StoragePort, StreamCandidate,
    StreamKind, StreamResult, SubtitleCandidate, SubtitleFormat, SubtitleProvider, SubtitleQuery,
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
    BackupImportMode, ContinueWatchingEntry, DownloadKind, NewIptvChannel, SqliteSecretStore,
    SqliteStorage, StorageActor, StoredDownload, StoredEpgNowNext, StoredEpgProgram,
    StoredIptvChannel, StoredIptvKind, StoredIptvProfile, StoredPlugin,
};

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

fn subtitle_ext(format: SubtitleFormat) -> &'static str {
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
            SourceResolver::new().with_default_registries(),
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
        self.install_package_with_source(package, repo_id, source_type, None, local_path)
            .await
    }

    /// Persists a `LoadedPackage` along with its origin `source_url`, preserving existing settings,
    /// user-approved domains, and enabled state on in-place updates.
    pub async fn install_package_with_source(
        &self,
        package: LoadedPackage,
        repo_id: Option<String>,
        source_type: &str,
        source_url: Option<String>,
        local_path: &str,
    ) -> VesselResult<()> {
        package.manifest.validate()?;
        let plugin_id = package.manifest.id.clone();
        let existing = self.storage.get_plugin(&plugin_id).await?;
        let preserve_enabled = existing.as_ref().map(|p| p.enabled).unwrap_or(true);
        if existing.is_some() {
            self.clear_plugin_caches(&plugin_id).await;
        }

        let stored = StoredPlugin {
            manifest: package.manifest.clone(),
            repo_id: repo_id.or_else(|| existing.and_then(|p| p.repo_id)),
            source_type: source_type.to_owned(),
            source_url,
            local_path: local_path.to_owned(),
            sha256: Some(package.sha256.clone()),
            enabled: preserve_enabled,
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
                preserve_enabled,
            )
            .await?;

        if let Ok(settings) = self.get_plugin_settings(&plugin_id).await {
            let _ = self.pool.set_settings(&plugin_id, settings).await;
        }

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
                let origin_url = if raw_input.trim().is_empty() {
                    resolved_url.clone()
                } else {
                    resolved_url.clone()
                };
                let dev_path = Path::new(&resolved_url);
                if dev_path.is_dir() {
                    self.install_package_with_source(
                        package,
                        inferred_repo,
                        "dev_dir",
                        Some(origin_url),
                        &resolved_url,
                    )
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
                    self.install_package_with_source(
                        package,
                        inferred_repo,
                        "package",
                        Some(origin_url),
                        &local_path,
                    )
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

    /// Ensures built-in first-party plugins are persisted in SQLite and registered in the pool,
    /// preserving the user's enabled toggle, settings, and granted domains across app updates.
    pub async fn ensure_builtin_plugins(&self) -> VesselResult<usize> {
        let packages = builtins::builtin_packages()?;
        let mut count = 0usize;
        for pkg in packages {
            let plugin_id = pkg.manifest.id.clone();
            let local_path = format!("builtin://{}", plugin_id.as_str());
            self.install_package_with_source(
                pkg,
                Some("builtin".to_owned()),
                builtins::BUILTIN_SOURCE_TYPE,
                None,
                &local_path,
            )
            .await?;
            count += 1;
        }
        Ok(count)
    }

    /// Reloads all persisted plugins from SQLite and their local files/directories into the pool.
    pub async fn bootstrap_plugins(&self) -> VesselResult<usize> {
        let mut loaded_count = self.ensure_builtin_plugins().await?;
        let plugins = self.storage.list_plugins().await?;
        for stored in plugins {
            if stored.source_type == builtins::BUILTIN_SOURCE_TYPE {
                continue;
            }
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
        let existing = self.storage.get_plugin(plugin_id).await?;
        if existing
            .as_ref()
            .is_some_and(|p| p.source_type == builtins::BUILTIN_SOURCE_TYPE)
        {
            return Err(VesselError::InvalidInput(format!(
                "built-in plugin '{}' cannot be uninstalled; disable it instead",
                plugin_id.as_str()
            )));
        }
        self.clear_plugin_caches(plugin_id).await;
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

    async fn resolve_repo_entries_cached<'a>(
        &self,
        repo_key: &str,
        cache: &'a mut HashMap<String, Result<Vec<RepoPluginEntry>, String>>,
    ) -> Result<&'a [RepoPluginEntry], String> {
        if !cache.contains_key(repo_key) {
            let fetched = match self.source_resolver.resolve(repo_key).await {
                Ok(ResolvedSource::Repository { repo, .. }) => Ok(repo.plugins),
                Ok(ResolvedSource::Package { .. }) => Err(format!(
                    "source '{repo_key}' resolved to a package instead of a repo"
                )),
                Err(err) => Err(err.to_string()),
            };
            cache.insert(repo_key.to_owned(), fetched);
        }
        match cache.get(repo_key) {
            Some(Ok(list)) => Ok(list.as_slice()),
            Some(Err(err)) => Err(err.clone()),
            None => Err("missing cached repo".into()),
        }
    }

    /// Checks all installed plugins for newer versions without modifying any settings or domain grants.
    pub async fn check_plugin_updates(&self) -> VesselResult<Vec<PluginUpdateInfo>> {
        let installed = self.storage.list_plugins().await?;
        let mut repo_cache: HashMap<String, Result<Vec<RepoPluginEntry>, String>> = HashMap::new();
        let mut out = Vec::with_capacity(installed.len());

        for plugin in installed {
            let pid = plugin.manifest.id.as_str().to_owned();
            let current_version = plugin.manifest.version.clone();

            if plugin.source_type == builtins::BUILTIN_SOURCE_TYPE {
                out.push(PluginUpdateInfo {
                    plugin_id: pid,
                    current_version: current_version.clone(),
                    latest_version: Some(current_version),
                    update_available: false,
                    source_url: None,
                    error: None,
                });
                continue;
            }

            if let Some(repo_key) = infer_repo_source_for_plugin(&plugin) {
                match self
                    .resolve_repo_entries_cached(&repo_key, &mut repo_cache)
                    .await
                {
                    Ok(entries) => {
                        if let Some(entry) = entries.iter().find(|e| e.id.as_str() == pid) {
                            let source_url = entry
                                .download_url
                                .clone()
                                .or_else(|| entry.source_path.clone())
                                .or_else(|| plugin.source_url.clone());
                            let update_available =
                                is_newer_version(&current_version, &entry.version);
                            out.push(PluginUpdateInfo {
                                plugin_id: pid,
                                current_version,
                                latest_version: Some(entry.version.clone()),
                                update_available,
                                source_url,
                                error: None,
                            });
                            continue;
                        }
                    }
                    Err(err) => {
                        if plugin.source_url.is_none() {
                            out.push(PluginUpdateInfo {
                                plugin_id: pid,
                                current_version,
                                latest_version: None,
                                update_available: false,
                                source_url: None,
                                error: Some(err),
                            });
                            continue;
                        }
                    }
                }
            }

            if let Some(ref src) = plugin.source_url {
                match self.source_resolver.resolve(src).await {
                    Ok(ResolvedSource::Package { package, .. }) => {
                        let latest = package.manifest.version;
                        let update_available = is_newer_version(&current_version, &latest);
                        out.push(PluginUpdateInfo {
                            plugin_id: pid,
                            current_version,
                            latest_version: Some(latest),
                            update_available,
                            source_url: Some(src.clone()),
                            error: None,
                        });
                    }
                    Ok(ResolvedSource::Repository { repo, .. }) => {
                        if let Some(entry) = repo.plugins.iter().find(|e| e.id.as_str() == pid) {
                            let target = entry
                                .download_url
                                .clone()
                                .or_else(|| entry.source_path.clone())
                                .or_else(|| Some(src.clone()));
                            let update_available =
                                is_newer_version(&current_version, &entry.version);
                            out.push(PluginUpdateInfo {
                                plugin_id: pid,
                                current_version,
                                latest_version: Some(entry.version.clone()),
                                update_available,
                                source_url: target,
                                error: None,
                            });
                        } else {
                            out.push(PluginUpdateInfo {
                                plugin_id: pid,
                                current_version,
                                latest_version: None,
                                update_available: false,
                                source_url: Some(src.clone()),
                                error: None,
                            });
                        }
                    }
                    Err(err) => {
                        out.push(PluginUpdateInfo {
                            plugin_id: pid,
                            current_version,
                            latest_version: None,
                            update_available: false,
                            source_url: Some(src.clone()),
                            error: Some(err.to_string()),
                        });
                    }
                }
            } else {
                out.push(PluginUpdateInfo {
                    plugin_id: pid,
                    current_version,
                    latest_version: None,
                    update_available: false,
                    source_url: None,
                    error: None,
                });
            }
        }

        Ok(out)
    }

    /// Updates a single installed plugin in-place, preserving user settings, domain grants, and enabled state.
    pub async fn update_plugin(
        &self,
        plugins_dir: &Path,
        plugin_id: &PluginId,
    ) -> VesselResult<(PluginId, SourceTrustBadge, String)> {
        let Some(stored) = self.storage.get_plugin(plugin_id).await? else {
            return Err(VesselError::NotFound(format!(
                "plugin '{}'",
                plugin_id.as_str()
            )));
        };

        if stored.source_type == builtins::BUILTIN_SOURCE_TYPE {
            return Err(VesselError::InvalidInput(format!(
                "built-in plugin '{}' updates automatically with Vessel",
                plugin_id.as_str()
            )));
        }

        let mut target_source = if stored.source_type == "dev_dir" {
            Some(stored.local_path.clone())
        } else {
            None
        };

        if target_source.is_none()
            && let Some(repo_key) = infer_repo_source_for_plugin(&stored)
        {
            let mut cache = HashMap::new();
            if let Ok(entries) = self
                .resolve_repo_entries_cached(&repo_key, &mut cache)
                .await
                && let Some(entry) = entries.iter().find(|e| e.id == *plugin_id)
            {
                target_source = entry
                    .download_url
                    .clone()
                    .or_else(|| entry.source_path.clone());
            }
        }

        if target_source.is_none() {
            target_source = stored.source_url.clone();
        }

        let Some(raw_target) = target_source.filter(|s| !s.trim().is_empty()) else {
            return Err(VesselError::InvalidInput(format!(
                "plugin '{}' has no recorded update source; reinstall once from a repository or URL",
                plugin_id.as_str()
            )));
        };

        let (id, badge) = self
            .install_package_persisted_with_repo(plugins_dir, &raw_target, stored.repo_id)
            .await?;
        let new_version = self
            .storage
            .get_plugin(&id)
            .await?
            .map(|p| p.manifest.version)
            .unwrap_or(stored.manifest.version);

        Ok((id, badge, new_version))
    }

    /// Checks and updates all outdated plugins in-place.
    pub async fn update_all_plugins(&self, plugins_dir: &Path) -> VesselResult<BatchUpdateResult> {
        let checks = self.check_plugin_updates().await?;
        let checked = checks.len();
        let mut updated = Vec::new();
        let mut failed = Vec::new();

        for item in checks {
            if !item.update_available {
                continue;
            }
            let Some(target) = item.source_url else {
                continue;
            };
            let Ok(pid) = PluginId::new(&item.plugin_id) else {
                continue;
            };
            let existing_repo = self.storage.get_plugin(&pid).await?.and_then(|p| p.repo_id);
            match self
                .install_package_persisted_with_repo(plugins_dir, &target, existing_repo)
                .await
            {
                Ok((id, badge)) => {
                    let version = self
                        .storage
                        .get_plugin(&id)
                        .await?
                        .map(|p| p.manifest.version)
                        .or(item.latest_version)
                        .unwrap_or(item.current_version);
                    updated.push(UpdatedPluginInfo {
                        plugin_id: id.to_string(),
                        version,
                        badge: format!("{badge:?}").to_ascii_lowercase(),
                    });
                }
                Err(err) => {
                    failed.push(BatchUpdateFailure {
                        plugin_id: item.plugin_id,
                        error: err.to_string(),
                    });
                }
            }
        }

        Ok(BatchUpdateResult {
            checked,
            updated,
            failed,
        })
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

    /// Resolves playable stream candidates from `media.plugin_id` plus any persisted linked providers.
    pub async fn streams_with_linked(
        &self,
        media: &MediaRef,
        episode_id: &str,
        season: u32,
        episode: u32,
        absolute_episode: Option<u32>,
        is_movie: bool,
    ) -> VesselResult<StreamResult> {
        let plugins_list = self.list_plugins().await.unwrap_or_default();
        let name_map: HashMap<String, String> = plugins_list
            .into_iter()
            .map(|p| (p.manifest.id.as_str().to_owned(), p.manifest.name))
            .collect();
        let primary_name = name_map
            .get(media.plugin_id.as_str())
            .cloned()
            .unwrap_or_else(|| media.plugin_id.as_str().to_owned());

        let linked = self.get_linked_sources(media).await.unwrap_or_default();
        let primary_res = self.streams(&media.plugin_id, episode_id).await;

        let mut merged = StreamResult::default();
        let mut primary_err: Option<VesselError> = None;

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
                if linked.is_empty() {
                    return Err(err);
                }
                primary_err = Some(err);
            }
        }

        if linked.is_empty() {
            return Ok(merged);
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
                let key = format!("meta3:{}:{}:{lang}", m.plugin_id.as_str(), m.native_id);
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
                let username = profile.username.as_deref().unwrap_or("");
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

    /// Persists drag-and-drop download queue priority ordering and dispatches queued jobs.
    pub async fn reorder_downloads(&self, ordered_ids: &[String]) -> VesselResult<()> {
        self.downloader.reorder(ordered_ids).await
    }

    /// Registers a completed offline download folder on the loopback HTTP proxy for playback.
    pub async fn prepare_local_download_playback(
        &self,
        download_id: &str,
    ) -> VesselResult<LocalPlaybackBundle> {
        let record = self.downloader.get(download_id).await?.ok_or_else(|| {
            VesselError::NotFound(format!("download '{download_id}' does not exist"))
        })?;
        if record.status != DownloadStatus::Completed {
            return Err(VesselError::InvalidInput(format!(
                "download '{download_id}' is not completed yet"
            )));
        }
        let media_path = record.local_media_path.as_deref().ok_or_else(|| {
            VesselError::NotFound(format!("download '{download_id}' has no local media path"))
        })?;
        let root_dir = PathBuf::from(&record.local_dir);
        let media_buf = PathBuf::from(media_path);
        if !tokio::fs::try_exists(&media_buf).await.unwrap_or(false) {
            return Err(VesselError::NotFound(format!(
                "local downloaded file is missing at '{}'",
                media_buf.display()
            )));
        }

        let rel_media = media_buf
            .strip_prefix(&root_dir)
            .ok()
            .or_else(|| media_buf.file_name().map(Path::new))
            .ok_or_else(|| VesselError::Storage("invalid local media path".into()))?;
        let rel_str = rel_media.to_string_lossy().replace('\\', "/");

        let base_url = self.proxy.register_local_dir(&root_dir)?;
        let url = format!("{base_url}/{rel_str}");

        let kind = match record.kind {
            DownloadKind::Hls => StreamKind::Hls,
            DownloadKind::Direct => match media_buf
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_ascii_lowercase())
                .as_deref()
            {
                Some("mkv") => StreamKind::Mkv,
                Some("webm") => StreamKind::Webm,
                Some("ts") => StreamKind::MpegTs,
                Some("m3u8") => StreamKind::Hls,
                _ => StreamKind::Mp4,
            },
        };

        let mut subtitles = Vec::new();
        for (idx, track) in record.subtitles.into_iter().enumerate() {
            let ext = subtitle_ext(track.format);
            let filename = format!("sub_{idx}.{ext}");
            if tokio::fs::try_exists(root_dir.join(&filename))
                .await
                .unwrap_or(false)
            {
                subtitles.push(SubtitleTrack {
                    id: track.id,
                    source: SubtitleSource::Url {
                        url: format!("{base_url}/{filename}"),
                        headers: BTreeMap::new(),
                    },
                    lang: track.lang,
                    label: track.label,
                    format: track.format,
                    is_default: track.is_default,
                });
            } else if matches!(track.source, SubtitleSource::InlineContent { .. }) {
                subtitles.push(track);
            }
        }

        Ok(LocalPlaybackBundle {
            download_id: record.id,
            url,
            kind,
            subtitles,
        })
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

fn apply_canonical_override_hint(detail: &mut MediaDetail, canonical_id: &str) {
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
        "meta3:{}:{}:{}",
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

fn infer_repo_source_for_plugin(stored: &StoredPlugin) -> Option<String> {
    if let Some(ref rid) = stored.repo_id {
        let trimmed = rid.trim();
        let lower = trimmed.to_ascii_lowercase();
        if lower == "anthology (vessel)" || lower == "anthology-vessel" || lower == "tr" {
            return Some("anthology".into());
        }
        if lower == "anthology" {
            return Some("nuvio".into());
        }
        if trimmed.contains('/') && !trimmed.contains(' ') {
            return Some(format!("gh:{trimmed}"));
        }
    }
    if let Some(ref src) = stored.source_url {
        if src.contains("/falsisdev/anthology-vessel/") {
            return Some("anthology".into());
        }
        if src.contains("/falsisdev/anthology/") {
            return Some("nuvio".into());
        }
    }
    None
}

fn parse_numeric_version(raw: &str) -> Vec<u64> {
    let clean = raw.trim().trim_start_matches(['v', 'V']);
    let core = clean.split(['-', '+']).next().unwrap_or(clean);
    core.split('.')
        .filter_map(|seg| seg.trim().parse::<u64>().ok())
        .collect()
}

fn is_newer_version(current: &str, latest: &str) -> bool {
    let cur_clean = current.trim().trim_start_matches(['v', 'V']);
    let lat_clean = latest.trim().trim_start_matches(['v', 'V']);
    if cur_clean == lat_clean || lat_clean.is_empty() {
        return false;
    }
    let a = parse_numeric_version(cur_clean);
    let b = parse_numeric_version(lat_clean);
    if !a.is_empty() && !b.is_empty() {
        let max_len = a.len().max(b.len());
        for i in 0..max_len {
            let av = *a.get(i).unwrap_or(&0);
            let bv = *b.get(i).unwrap_or(&0);
            if bv > av {
                return true;
            }
            if bv < av {
                return false;
            }
        }
        return false;
    }
    lat_clean != cur_clean
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

        // 2. Series search (Breaking Bad S01E01 tt0903747 -> Addic7ed + TVSubtitles + OpenSubtitles v3)
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

        engine.shutdown().await;
    }
}
