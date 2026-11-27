//! Multi-provider Debrid orchestration, TTL link caching, and season-pack matching.

pub mod episode_matcher;
pub mod provider;
pub mod providers;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;
use tokio::task::JoinSet;
use vessel_domain::{
    DebridAccountStatus, DebridCacheInfo, DebridCacheState, DebridCloudItem, DebridPreferences,
    DebridProvider, DebridProviderKind, DebridResolution, DebridResolveOutcome,
    RealDebridDeviceAuthPoll, RealDebridDeviceAuthStart, ResolveDebridRequest, ResolvedHttpLink,
    SecretStore, StreamCandidate, TorrentTarget, VesselError, VesselResult, build_magnet_uri,
    extract_info_hash,
};

use crate::provider::build_debrid_http_client;
use crate::providers::{
    ALLDEBRID_SECRET_KEY, AllDebridProvider, DEBRID_LINK_SECRET_KEY, DebridLinkProvider,
    PREMIUMIZE_SECRET_KEY, PremiumizeProvider, REAL_DEBRID_CLIENT_ID_KEY,
    REAL_DEBRID_CLIENT_SECRET_KEY, REAL_DEBRID_REFRESH_KEY, REAL_DEBRID_SECRET_KEY,
    RealDebridProvider, TORBOX_SECRET_KEY, TorBoxProvider,
};

const RESOLVED_LINK_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct CacheKey {
    target_id: String,
    info_hash: String,
    file_index: Option<u32>,
    season: Option<u32>,
    episode: Option<u32>,
}

#[derive(Debug, Clone)]
struct CachedLink {
    target_id: String,
    provider_label: String,
    link: ResolvedHttpLink,
    cached_at: Instant,
}

/// Orchestrates built-in Rust Debrid providers and community QuickJS Debrid plugins.
pub struct DebridManager {
    secrets: Arc<dyn SecretStore>,
    real_debrid: Arc<RealDebridProvider>,
    builtins: Vec<Arc<dyn DebridProvider>>,
    community: RwLock<Vec<Arc<dyn DebridProvider>>>,
    link_cache: RwLock<HashMap<CacheKey, CachedLink>>,
}

impl DebridManager {
    /// Creates a `DebridManager` with all 5 built-in providers wired to `secrets`.
    pub fn new(secrets: Arc<dyn SecretStore>) -> VesselResult<Self> {
        let client = build_debrid_http_client()?;
        let real_debrid = Arc::new(RealDebridProvider::new(client.clone(), Arc::clone(&secrets)));
        let torbox: Arc<dyn DebridProvider> =
            Arc::new(TorBoxProvider::new(client.clone(), Arc::clone(&secrets)));
        let alldebrid: Arc<dyn DebridProvider> =
            Arc::new(AllDebridProvider::new(client.clone(), Arc::clone(&secrets)));
        let premiumize: Arc<dyn DebridProvider> =
            Arc::new(PremiumizeProvider::new(client.clone(), Arc::clone(&secrets)));
        let debrid_link: Arc<dyn DebridProvider> =
            Arc::new(DebridLinkProvider::new(client, Arc::clone(&secrets)));

        Ok(Self {
            secrets,
            builtins: vec![
                Arc::clone(&real_debrid) as Arc<dyn DebridProvider>,
                torbox,
                alldebrid,
                premiumize,
                debrid_link,
            ],
            real_debrid,
            community: RwLock::new(Vec::new()),
            link_cache: RwLock::new(HashMap::new()),
        })
    }

    /// Creates a `DebridManager` with custom provider instances (used in unit/mock tests).
    pub fn with_providers(
        secrets: Arc<dyn SecretStore>,
        real_debrid: Arc<RealDebridProvider>,
        builtins: Vec<Arc<dyn DebridProvider>>,
    ) -> Self {
        Self {
            secrets,
            real_debrid,
            builtins,
            community: RwLock::new(Vec::new()),
            link_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Replaces the currently active community QuickJS Debrid providers.
    pub async fn set_community_providers(&self, providers: Vec<Arc<dyn DebridProvider>>) {
        let mut guard = self.community.write().await;
        *guard = providers;
    }

    /// Returns the secret store key for a built-in provider's primary API token.
    pub fn secret_key_for_builtin(kind: DebridProviderKind) -> &'static str {
        match kind {
            DebridProviderKind::RealDebrid => REAL_DEBRID_SECRET_KEY,
            DebridProviderKind::TorBox => TORBOX_SECRET_KEY,
            DebridProviderKind::AllDebrid => ALLDEBRID_SECRET_KEY,
            DebridProviderKind::Premiumize => PREMIUMIZE_SECRET_KEY,
            DebridProviderKind::DebridLink => DEBRID_LINK_SECRET_KEY,
        }
    }

    /// Saves a built-in Debrid API key into `SecretStore`.
    pub async fn save_builtin_api_key(
        &self,
        kind: DebridProviderKind,
        api_key: &str,
    ) -> VesselResult<()> {
        let trimmed = api_key.trim();
        if trimmed.is_empty() {
            return Err(VesselError::InvalidInput("Debrid API key cannot be empty".into()));
        }
        self.secrets
            .set(Self::secret_key_for_builtin(kind), trimmed)
            .await?;
        self.link_cache.write().await.clear();
        Ok(())
    }

    /// Deletes stored credentials for a built-in Debrid provider.
    pub async fn remove_builtin_credentials(&self, kind: DebridProviderKind) -> VesselResult<()> {
        self.secrets.delete(Self::secret_key_for_builtin(kind)).await?;
        if kind == DebridProviderKind::RealDebrid {
            let _ = self.secrets.delete(REAL_DEBRID_CLIENT_ID_KEY).await;
            let _ = self.secrets.delete(REAL_DEBRID_CLIENT_SECRET_KEY).await;
            let _ = self.secrets.delete(REAL_DEBRID_REFRESH_KEY).await;
        }
        self.link_cache.write().await.clear();
        Ok(())
    }

    pub async fn start_real_debrid_device_auth(&self) -> VesselResult<RealDebridDeviceAuthStart> {
        self.real_debrid.start_device_auth().await
    }

    pub async fn poll_real_debrid_device_auth(
        &self,
        device_code: &str,
    ) -> VesselResult<RealDebridDeviceAuthPoll> {
        let res = self.real_debrid.poll_device_auth(device_code).await?;
        if res.authorized {
            self.link_cache.write().await.clear();
        }
        Ok(res)
    }

    async fn ordered_providers(&self, prefs: &DebridPreferences) -> Vec<Arc<dyn DebridProvider>> {
        let community = self.community.read().await.clone();
        let mut all_map: HashMap<String, Arc<dyn DebridProvider>> = HashMap::new();
        for p in self.builtins.iter().chain(community.iter()) {
            all_map.insert(p.target_id().to_owned(), Arc::clone(p));
        }

        let mut ordered = Vec::new();
        let mut seen = HashSet::new();
        for id in &prefs.priority_order {
            if let Some(p) = all_map.get(id)
                && seen.insert(id.clone())
            {
                ordered.push(Arc::clone(p));
            }
        }
        for p in self.builtins.iter().chain(community.iter()) {
            if seen.insert(p.target_id().to_owned()) {
                ordered.push(Arc::clone(p));
            }
        }
        ordered
    }

    async fn find_provider(&self, target_id: &str) -> Option<Arc<dyn DebridProvider>> {
        if let Some(p) = self.builtins.iter().find(|p| p.target_id() == target_id) {
            return Some(Arc::clone(p));
        }
        let community = self.community.read().await;
        community
            .iter()
            .find(|p| p.target_id() == target_id)
            .cloned()
    }

    /// Queries account status for all built-in and community Debrid providers in priority order.
    pub async fn accounts_status(
        &self,
        prefs: &DebridPreferences,
    ) -> Vec<DebridAccountStatus> {
        let providers = self.ordered_providers(prefs).await;
        let mut set = JoinSet::new();
        for (idx, p) in providers.into_iter().enumerate() {
            set.spawn(async move {
                let status = p.account_status().await.unwrap_or_else(|err| DebridAccountStatus {
                    target: p.target_id().into(),
                    label: p.display_name().into(),
                    is_builtin: p.is_builtin(),
                    supports_bulk_cache_check: p.supports_bulk_cache_check(),
                    enabled: true,
                    authenticated: false,
                    username: None,
                    email: None,
                    is_premium: false,
                    expires_at: None,
                    points_or_quota: None,
                    error: Some(err.to_string()),
                });
                (idx, status)
            });
        }
        let mut results = Vec::new();
        while let Some(Ok(item)) = set.join_next().await {
            results.push(item);
        }
        results.sort_by_key(|(idx, _)| *idx);
        results.into_iter().map(|(_, s)| s).collect()
    }

    /// Enriches torrent `candidates` with instant cache availability badges (`⚡ Cached` / `☁️ Uncached`).
    pub async fn probe_cache(
        &self,
        candidates: &mut [StreamCandidate],
        prefs: &DebridPreferences,
    ) {
        let mut unique_hashes = Vec::new();
        let mut seen = HashSet::new();
        for cand in candidates.iter() {
            if cand.is_torrent()
                && let Some(hash) = cand.effective_info_hash()
                && seen.insert(hash.clone())
            {
                unique_hashes.push(hash);
            }
        }
        if unique_hashes.is_empty() {
            return;
        }

        let statuses = self.accounts_status(prefs).await;
        let providers = self.ordered_providers(prefs).await;
        let active_bulk: Vec<Arc<dyn DebridProvider>> = providers
            .into_iter()
            .filter(|p| {
                p.supports_bulk_cache_check()
                    && statuses
                        .iter()
                        .any(|s| s.target == p.target_id() && s.enabled && s.authenticated)
            })
            .collect();

        if active_bulk.is_empty() {
            // If only non-bulk provider (e.g. Real-Debrid) is active, mark as Unknown rather than Uncached.
            let has_any = statuses.iter().any(|s| s.enabled && s.authenticated);
            if has_any
                && let Some(first) = statuses.into_iter().find(|s| s.enabled && s.authenticated)
            {
                for cand in candidates.iter_mut() {
                    if cand.is_torrent() && cand.debrid_cache.is_none() {
                        cand.debrid_cache = Some(DebridCacheInfo {
                            target: first.target.clone(),
                            label: first.label.clone(),
                            state: DebridCacheState::Unknown,
                        });
                    }
                }
            }
            return;
        }

        let mut set = JoinSet::new();
        for (idx, p) in active_bulk.into_iter().enumerate() {
            let hashes = unique_hashes.clone();
            set.spawn(async move {
                let map = p.check_cached(&hashes).await.unwrap_or_default();
                (idx, p.target_id().to_owned(), p.display_name().to_owned(), map)
            });
        }

        let mut per_provider = Vec::new();
        while let Some(Ok(item)) = set.join_next().await {
            per_provider.push(item);
        }
        per_provider.sort_by_key(|(idx, _, _, _)| *idx);

        for cand in candidates.iter_mut() {
            if !cand.is_torrent() {
                continue;
            }
            let Some(hash) = cand.effective_info_hash() else {
                continue;
            };
            let mut matched: Option<DebridCacheInfo> = None;
            for (_, target, label, map) in &per_provider {
                if map.get(&hash).copied() == Some(true) {
                    matched = Some(DebridCacheInfo {
                        target: target.clone(),
                        label: label.clone(),
                        state: DebridCacheState::Cached,
                    });
                    break;
                }
            }
            if matched.is_none()
                && let Some((_, target, label, _)) = per_provider.first()
            {
                matched = Some(DebridCacheInfo {
                    target: target.clone(),
                    label: label.clone(),
                    state: DebridCacheState::Uncached,
                });
            }
            cand.debrid_cache = matched;
        }
    }

    /// Resolves a torrent candidate across configured built-in and community Debrid providers.
    pub async fn resolve_candidate(
        &self,
        req: &ResolveDebridRequest,
        prefs: &DebridPreferences,
    ) -> VesselResult<DebridResolution> {
        let info_hash = req
            .torrent
            .as_ref()
            .and_then(|t| t.info_hash.as_deref())
            .and_then(extract_info_hash)
            .or_else(|| extract_info_hash(&req.url))
            .ok_or_else(|| {
                VesselError::InvalidInput("Stream candidate does not contain a valid infoHash or magnet URI".into())
            })?;

        let magnet_uri = if req.url.trim_start().to_ascii_lowercase().starts_with("magnet:?") {
            req.url.clone()
        } else {
            let trackers = req
                .torrent
                .as_ref()
                .map(|t| t.trackers.as_slice())
                .unwrap_or(&[]);
            let name = req.torrent.as_ref().and_then(|t| t.filename.as_deref());
            build_magnet_uri(&info_hash, name, trackers)
        };

        let mut target = req
            .torrent
            .as_ref()
            .map(TorrentTarget::from)
            .unwrap_or_default();
        target.info_hash = Some(info_hash.clone());
        if target.season.is_none() {
            target.season = req.season;
        }
        if target.episode.is_none() {
            target.episode = req.episode;
        }

        let statuses = self.accounts_status(prefs).await;
        let ordered = self.ordered_providers(prefs).await;
        let mut active: Vec<Arc<dyn DebridProvider>> = ordered
            .into_iter()
            .filter(|p| {
                statuses
                    .iter()
                    .any(|s| s.target == p.target_id() && s.enabled && s.authenticated)
            })
            .collect();

        if let Some(ref preferred) = req.preferred_target
            && let Some(pos) = active.iter().position(|p| p.target_id() == preferred)
        {
            let p = active.remove(pos);
            active.insert(0, p);
        }

        if active.is_empty() {
            return Ok(DebridResolution::NoProviderConfigured {
                message: "No Debrid account is configured. Connect Real-Debrid, TorBox, or a community Debrid plugin in Settings > Debrid (native P2P arrives in v0.6.0).".into(),
            });
        }

        // Check TTL cache across active providers first.
        {
            let guard = self.link_cache.read().await;
            for p in &active {
                let key = CacheKey {
                    target_id: p.target_id().to_owned(),
                    info_hash: info_hash.clone(),
                    file_index: target.file_index,
                    season: target.season,
                    episode: target.episode,
                };
                if let Some(entry) = guard.get(&key)
                    && entry.cached_at.elapsed() < RESOLVED_LINK_TTL
                {
                    return Ok(DebridResolution::Ready {
                        target: entry.target_id.clone(),
                        provider_label: entry.provider_label.clone(),
                        url: entry.link.url.clone(),
                        headers: entry.link.headers.clone(),
                        filename: entry.link.filename.clone(),
                        size_bytes: entry.link.size_bytes,
                        expires_at: entry.link.expires_at.clone(),
                    });
                }
            }
        }

        let mut first_pending: Option<DebridResolution> = None;
        let mut last_err: Option<VesselError> = None;

        for p in active {
            match p
                .resolve_magnet(&magnet_uri, &target, req.allow_uncached_queue)
                .await
            {
                Ok(DebridResolveOutcome::Ready(link)) => {
                    let key = CacheKey {
                        target_id: p.target_id().to_owned(),
                        info_hash: info_hash.clone(),
                        file_index: target.file_index,
                        season: target.season,
                        episode: target.episode,
                    };
                    self.link_cache.write().await.insert(
                        key,
                        CachedLink {
                            target_id: p.target_id().to_owned(),
                            provider_label: p.display_name().to_owned(),
                            link: link.clone(),
                            cached_at: Instant::now(),
                        },
                    );
                    return Ok(DebridResolution::Ready {
                        target: p.target_id().to_owned(),
                        provider_label: p.display_name().to_owned(),
                        url: link.url,
                        headers: link.headers,
                        filename: link.filename,
                        size_bytes: link.size_bytes,
                        expires_at: link.expires_at,
                    });
                }
                Ok(DebridResolveOutcome::PendingCloud {
                    torrent_id,
                    status,
                    progress_pct,
                    speed_bps,
                    seeders,
                }) => {
                    let pending = DebridResolution::NeedsConfirmation {
                        target: p.target_id().to_owned(),
                        provider_label: p.display_name().to_owned(),
                        info_hash: info_hash.clone(),
                        cloud_status: status,
                        progress_pct,
                        speed_bps,
                        seeders,
                        torrent_id,
                    };
                    if req.allow_uncached_queue || !prefs.auto_fallback {
                        return Ok(pending);
                    }
                    if first_pending.is_none() {
                        first_pending = Some(pending);
                    }
                }
                Err(err) => {
                    last_err = Some(err);
                    if !prefs.auto_fallback {
                        break;
                    }
                }
            }
        }

        if let Some(pending) = first_pending {
            return Ok(pending);
        }
        Err(last_err.unwrap_or_else(|| VesselError::Network("Failed to resolve torrent via Debrid".into())))
    }

    pub async fn list_cloud(&self, prefs: &DebridPreferences) -> Vec<DebridCloudItem> {
        let statuses = self.accounts_status(prefs).await;
        let providers = self.ordered_providers(prefs).await;
        let active: Vec<Arc<dyn DebridProvider>> = providers
            .into_iter()
            .filter(|p| {
                statuses
                    .iter()
                    .any(|s| s.target == p.target_id() && s.enabled && s.authenticated)
            })
            .collect();

        let mut set = JoinSet::new();
        for p in active {
            set.spawn(async move { p.list_cloud().await.unwrap_or_default() });
        }
        let mut all = Vec::new();
        while let Some(Ok(items)) = set.join_next().await {
            all.extend(items);
        }
        all
    }

    pub async fn delete_cloud(&self, target_id: &str, torrent_id: &str) -> VesselResult<()> {
        let provider = self
            .find_provider(target_id)
            .await
            .ok_or_else(|| VesselError::NotFound(format!("Debrid provider '{target_id}' not found")))?;
        provider.delete_cloud(torrent_id).await
    }

    pub async fn unrestrict_cloud_link(
        &self,
        target_id: &str,
        link: &str,
    ) -> VesselResult<ResolvedHttpLink> {
        let provider = self
            .find_provider(target_id)
            .await
            .ok_or_else(|| VesselError::NotFound(format!("Debrid provider '{target_id}' not found")))?;
        provider.unrestrict_link(link).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use axum::extract::{Path, Query};
    use axum::routing::{delete, get, post};
    use axum::{Json, Router};
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;
    use vessel_domain::{
        DebridCacheState, DebridPreferences, DebridProviderKind, DebridResolution, SecretStore,
        StreamCandidate, StreamKind, TorrentMeta, VesselResult,
    };

    use super::{
        DebridManager, REAL_DEBRID_SECRET_KEY, RealDebridProvider, TORBOX_SECRET_KEY,
        TorBoxProvider, build_debrid_http_client,
    };

    #[derive(Default)]
    struct MemSecrets {
        map: Mutex<HashMap<String, String>>,
    }

    #[async_trait]
    impl SecretStore for MemSecrets {
        async fn get(&self, key: &str) -> VesselResult<Option<String>> {
            Ok(self.map.lock().await.get(key).cloned())
        }
        async fn set(&self, key: &str, value: &str) -> VesselResult<()> {
            self.map.lock().await.insert(key.into(), value.into());
            Ok(())
        }
        async fn delete(&self, key: &str) -> VesselResult<()> {
            self.map.lock().await.remove(key);
            Ok(())
        }
    }

    #[tokio::test]
    async fn resolves_and_caches_with_mock_real_debrid_and_torbox_fallback() {
        let rd_deletes = Arc::new(AtomicUsize::new(0));
        let tb_requests = Arc::new(AtomicUsize::new(0));
        let rd_deletes_srv = Arc::clone(&rd_deletes);
        let tb_requests_srv = Arc::clone(&tb_requests);

        let app = Router::new()
            // Real-Debrid mocks (returns uncached downloading -> must trigger delete when allow_uncached_queue=false)
            .route(
                "/rd/user",
                get(|| async { Json(serde_json::json!({ "username": "falsis", "type": "premium" })) }),
            )
            .route(
                "/rd/torrents/addMagnet",
                post(|| async { Json(serde_json::json!({ "id": "rd-t1", "uri": "x" })) }),
            )
            .route(
                "/rd/torrents/info/{id}",
                get(|Path(_id): Path<String>| async {
                    Json(serde_json::json!({
                        "id": "rd-t1",
                        "filename": "Movie.2026.mkv",
                        "progress": 12.5,
                        "status": "downloading",
                        "speed": 5000000,
                        "seeders": 42,
                        "files": [{ "id": 1, "path": "/Movie.2026.mkv", "bytes": 4000000000u64, "selected": 1 }],
                        "links": []
                    }))
                }),
            )
            .route(
                "/rd/torrents/selectFiles/{id}",
                post(|| async { axum::http::StatusCode::NO_CONTENT }),
            )
            .route(
                "/rd/torrents/delete/{id}",
                delete(move |Path(_id): Path<String>| {
                    let c = Arc::clone(&rd_deletes_srv);
                    async move {
                        c.fetch_add(1, Ordering::SeqCst);
                        axum::http::StatusCode::NO_CONTENT
                    }
                }),
            )
            // TorBox mocks (cached + instant resolve)
            .route(
                "/tb/user/me",
                get(|| async { Json(serde_json::json!({ "data": { "email": "tb@vessel.dev", "plan": 2 } })) }),
            )
            .route(
                "/tb/torrents/checkcached",
                get(|Query(q): Query<HashMap<String, String>>| async move {
                    let h = q.get("hash").cloned().unwrap_or_default();
                    Json(serde_json::json!({ "data": [{ "hash": h }] }))
                }),
            )
            .route(
                "/tb/torrents/createtorrent",
                post(|| async { Json(serde_json::json!({ "data": { "torrent_id": 77 } })) }),
            )
            .route(
                "/tb/torrents/mylist",
                get(|| async {
                    Json(serde_json::json!({
                        "data": {
                            "id": 77,
                            "name": "Movie.2026.mkv",
                            "size": 4000000000u64,
                            "progress": 1.0,
                            "download_present": true,
                            "download_finished": true,
                            "files": [{ "id": 9, "name": "Movie.2026.mkv", "size": 4000000000u64 }]
                        }
                    }))
                }),
            )
            .route(
                "/tb/torrents/requestdl",
                get(move || {
                    let c = Arc::clone(&tb_requests_srv);
                    async move {
                        c.fetch_add(1, Ordering::SeqCst);
                        Json(serde_json::json!({ "data": "https://cdn.torbox.example/movie.mkv" }))
                    }
                }),
            );

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let secrets: Arc<dyn SecretStore> = Arc::new(MemSecrets::default());
        secrets.set(REAL_DEBRID_SECRET_KEY, "rd-key").await.unwrap();
        secrets.set(TORBOX_SECRET_KEY, "tb-key").await.unwrap();

        let client = build_debrid_http_client().unwrap();
        let rd = Arc::new(RealDebridProvider::with_bases(
            client.clone(),
            Arc::clone(&secrets),
            format!("http://{addr}/rd"),
            format!("http://{addr}/rd-oauth"),
        ));
        let tb: Arc<dyn vessel_domain::DebridProvider> = Arc::new(TorBoxProvider::with_base(
            client,
            Arc::clone(&secrets),
            format!("http://{addr}/tb"),
        ));

        let manager = DebridManager::with_providers(
            Arc::clone(&secrets),
            Arc::clone(&rd),
            vec![Arc::clone(&rd) as Arc<dyn vessel_domain::DebridProvider>, tb],
        );

        let prefs = DebridPreferences {
            priority_order: vec![
                DebridProviderKind::RealDebrid.target_id().into(),
                DebridProviderKind::TorBox.target_id().into(),
            ],
            only_cached: false,
            auto_fallback: true,
        };

        let mut cands = vec![StreamCandidate {
            url: "magnet:?xt=urn:btih:ABCDEF0123456789ABCDEF0123456789ABCDEF01".into(),
            kind: StreamKind::Torrent,
            quality: Some("2160p".into()),
            label: "Torrentio 4K".into(),
            language: None,
            audio_type: None,
            priority: None,
            is_default: false,
            flags: Vec::new(),
            headers: Default::default(),
            needs_extractor: false,
            subtitles: Vec::new(),
            skip_segments: Vec::new(),
            plugin_id: None,
            plugin_name: None,
            torrent: Some(TorrentMeta {
                info_hash: Some("abcdef0123456789abcdef0123456789abcdef01".into()),
                ..Default::default()
            }),
            debrid_cache: None,
        }];

        manager.probe_cache(&mut cands, &prefs).await;
        assert_eq!(
            cands[0].debrid_cache.as_ref().map(|c| c.state),
            Some(DebridCacheState::Cached)
        );
        assert_eq!(
            cands[0].debrid_cache.as_ref().map(|c| c.label.as_str()),
            Some("TorBox")
        );

        // First resolve: RD is uncached (auto-cleaned via DELETE), auto-fallback hits TorBox!
        let req = vessel_domain::ResolveDebridRequest {
            url: cands[0].url.clone(),
            torrent: cands[0].torrent.clone(),
            preferred_target: None,
            allow_uncached_queue: false,
            season: None,
            episode: None,
        };
        let res1 = manager.resolve_candidate(&req, &prefs).await.expect("resolve 1");
        assert!(matches!(
            res1,
            DebridResolution::Ready { ref url, .. } if url == "https://cdn.torbox.example/movie.mkv"
        ));
        assert_eq!(rd_deletes.load(Ordering::SeqCst), 1);
        assert_eq!(tb_requests.load(Ordering::SeqCst), 1);

        // Second resolve: served from 15-min TTL cache without hitting TorBox again.
        let res2 = manager.resolve_candidate(&req, &prefs).await.expect("resolve 2");
        assert_eq!(res1, res2);
        assert_eq!(tb_requests.load(Ordering::SeqCst), 1);
    }
}
