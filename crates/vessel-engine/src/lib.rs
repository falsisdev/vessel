//! Pure-Rust core orchestrator (`VesselEngine`) binding plugins, storage, proxy, and playback.

pub mod pool;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tokio::sync::Mutex;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use vessel_domain::{
    ContentProvider, DomainGrantSource, EngineEvent, EventSink, HomeFeed, MediaDetail, MediaRef,
    MediaType, PlaybackProgress, PlaybackRequest, PlayerBackend, PluginId, StoragePort,
    StreamCandidate, StreamResult, SubtitleSource, SubtitleTrack, VesselError, VesselPaths,
    VesselResult,
};
use vessel_plugin_rt::{LoadedPackage, ResolvedSource, SourceResolver, TsCompileCache};
use vessel_proxy::ProxyServer;
use vessel_storage::{SqliteSecretStore, SqliteStorage, StorageActor, StoredPlugin};

pub use pool::{DEFAULT_IDLE_TTL, DEFAULT_MAX_WARM_PLUGINS, PluginPool};

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

/// Parameters for resolving a `StreamCandidate` and starting playback cleanly.
#[derive(Debug, Clone)]
pub struct PlayCandidateRequest {
    pub media: MediaRef,
    pub season: u32,
    pub episode: u32,
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

/// Pure core facade orchestrating storage, proxy, isolated QuickJS plugins, and `PlayerBackend`.
pub struct VesselEngine {
    storage: Arc<SqliteStorage>,
    secrets: Arc<SqliteSecretStore>,
    proxy: Arc<ProxyServer>,
    player: Arc<dyn PlayerBackend>,
    event_sink: Arc<dyn EventSink>,
    ts_cache: Arc<TsCompileCache>,
    source_resolver: SourceResolver,
    pool: Arc<PluginPool>,
    always_proxy: AtomicBool,
    active_search: Arc<Mutex<Option<(String, CancellationToken)>>>,
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
        Self::from_parts(
            storage,
            secrets,
            proxy,
            player,
            event_sink,
            SourceResolver::new(),
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
        let cached_ts = storage.load_ts_compile_cache().await?;
        let ts_cache = Arc::new(TsCompileCache::with_entries(cached_ts));
        let storage_port: Arc<dyn StoragePort> = storage.clone();
        let pool = Arc::new(PluginPool::with_defaults(
            storage_port,
            Some(Arc::clone(&event_sink)),
            Arc::clone(&ts_cache),
        ));

        Ok(Self {
            storage,
            secrets,
            proxy,
            player,
            event_sink,
            ts_cache,
            source_resolver,
            pool,
            always_proxy: AtomicBool::new(always_proxy),
            active_search: Arc::new(Mutex::new(None)),
        })
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

    /// Fetches home feed shelves from a single plugin.
    pub async fn home_feed(
        &self,
        plugin_id: &PluginId,
        cursor: Option<&str>,
    ) -> VesselResult<HomeFeed> {
        let provider = self.pool.get_or_load(plugin_id).await?;
        provider.home_feed(cursor).await
    }

    /// Fetches full item metadata & episode tree directly from `plugin_id`.
    pub async fn details(
        &self,
        plugin_id: &PluginId,
        native_id: &str,
    ) -> VesselResult<MediaDetail> {
        let provider = self.pool.get_or_load(plugin_id).await?;
        provider.details(native_id).await
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
            None => self
                .storage
                .get_progress(&req.media, req.season, req.episode)
                .await?
                .filter(|p| !p.completed)
                .map_or(0, |p| p.position_ms),
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
                season: 1,
                episode: 50,
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
}
