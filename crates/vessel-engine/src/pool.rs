//! Warm isolated QuickJS plugin LRU pool with idle TTL eviction.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use vessel_domain::{
    EventSink, PluginCapability, PluginId, PluginManifest, StoragePort, VesselError, VesselResult,
};
use vessel_plugin_rt::{
    DomainPolicy, HostServices, PluginBundle, QuickJsContentProvider, SandboxLimits, TsCompileCache,
};

/// Default number of warm isolated QuickJS runtimes kept in RAM simultaneously.
#[cfg(target_os = "android")]
pub const DEFAULT_MAX_WARM_PLUGINS: usize = 8;

/// Default number of warm isolated QuickJS runtimes kept in RAM simultaneously.
#[cfg(not(target_os = "android"))]
pub const DEFAULT_MAX_WARM_PLUGINS: usize = 20;

/// Default inactivity window after which a warm QuickJS runtime is evicted (`5 min`).
pub const DEFAULT_IDLE_TTL: Duration = Duration::from_secs(300);

struct RegisteredPlugin {
    manifest: PluginManifest,
    bundle: Arc<PluginBundle>,
    domain_policy: Arc<DomainPolicy>,
    host: Arc<HostServices>,
    enabled: bool,
    subtitle_search_supported: Option<bool>,
}

fn bundle_might_implement_subtitle_search(bundle: &PluginBundle) -> bool {
    bundle
        .files()
        .values()
        .any(|src| src.contains("searchSubtitles"))
}

struct WarmPlugin {
    provider: Arc<QuickJsContentProvider>,
    last_used: Instant,
}

struct PoolInner {
    registered: BTreeMap<PluginId, RegisteredPlugin>,
    warm: HashMap<PluginId, WarmPlugin>,
}

/// Manages registered plugin bundles and at most `max_warm` active isolated QuickJS sandboxes.
pub struct PluginPool {
    max_warm: usize,
    idle_ttl: Duration,
    sandbox_limits: SandboxLimits,
    storage: Arc<dyn StoragePort>,
    event_sink: Option<Arc<dyn EventSink>>,
    ts_cache: Arc<TsCompileCache>,
    inner: Mutex<PoolInner>,
}

impl std::fmt::Debug for PluginPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginPool")
            .field("max_warm", &self.max_warm)
            .field("idle_ttl", &self.idle_ttl)
            .finish_non_exhaustive()
    }
}

impl PluginPool {
    /// Creates an empty LRU sandbox pool enforcing `max_warm` capacity and `idle_ttl`.
    pub fn new(
        max_warm: usize,
        idle_ttl: Duration,
        sandbox_limits: SandboxLimits,
        storage: Arc<dyn StoragePort>,
        event_sink: Option<Arc<dyn EventSink>>,
        ts_cache: Arc<TsCompileCache>,
    ) -> Self {
        Self {
            max_warm: max_warm.max(1),
            idle_ttl,
            sandbox_limits,
            storage,
            event_sink,
            ts_cache,
            inner: Mutex::new(PoolInner {
                registered: BTreeMap::new(),
                warm: HashMap::new(),
            }),
        }
    }

    /// Creates a pool with `PLAN.md` defaults (`6` plugins, `3 min` idle eviction).
    pub fn with_defaults(
        storage: Arc<dyn StoragePort>,
        event_sink: Option<Arc<dyn EventSink>>,
        ts_cache: Arc<TsCompileCache>,
    ) -> Self {
        Self::new(
            DEFAULT_MAX_WARM_PLUGINS,
            DEFAULT_IDLE_TTL,
            SandboxLimits::default(),
            storage,
            event_sink,
            ts_cache,
        )
    }

    /// Registers (or replaces) a plugin bundle and its domain policy without warming JS yet.
    pub async fn register(
        &self,
        manifest: PluginManifest,
        bundle: Arc<PluginBundle>,
        extra_allowed_domains: Vec<String>,
        enabled: bool,
    ) -> VesselResult<()> {
        manifest.validate()?;
        let plugin_id = manifest.id.clone();

        let mut initial_domains = manifest.domains.clone();
        initial_domains.extend(extra_allowed_domains);
        let domain_policy = Arc::new(DomainPolicy::new(
            plugin_id.clone(),
            initial_domains,
            manifest.allow_any_https,
        ));

        let host = Arc::new(HostServices::new(
            plugin_id.clone(),
            Arc::clone(&domain_policy),
            Arc::clone(&self.storage),
            self.event_sink.clone(),
            Arc::clone(&self.ts_cache),
        )?);

        let subtitle_search_supported = if manifest
            .capabilities
            .contains(&PluginCapability::Subtitles)
        {
            if bundle_might_implement_subtitle_search(&bundle) {
                None
            } else {
                Some(false)
            }
        } else {
            Some(false)
        };

        let old_warm = {
            let mut guard = self.inner.lock().await;
            guard.registered.insert(
                plugin_id.clone(),
                RegisteredPlugin {
                    manifest,
                    bundle,
                    domain_policy,
                    host,
                    enabled,
                    subtitle_search_supported,
                },
            );
            guard.warm.remove(&plugin_id)
        };

        if let Some(evicted) = old_warm {
            shutdown_arc_provider(evicted.provider).await;
        }

        Ok(())
    }

    /// Enables or disables a registered plugin (disabling also shuts down any warm sandbox).
    pub async fn set_enabled(&self, plugin_id: &PluginId, enabled: bool) -> VesselResult<()> {
        let evicted = {
            let mut guard = self.inner.lock().await;
            let Some(reg) = guard.registered.get_mut(plugin_id) else {
                return Err(VesselError::NotFound(format!("plugin '{plugin_id}'")));
            };
            reg.enabled = enabled;
            if !enabled {
                guard.warm.remove(plugin_id)
            } else {
                None
            }
        };

        if let Some(entry) = evicted {
            shutdown_arc_provider(entry.provider).await;
        }
        Ok(())
    }

    /// Dynamically permits `domain` on a registered plugin's active `DomainPolicy`.
    pub async fn allow_domain(&self, plugin_id: &PluginId, domain: &str) -> VesselResult<()> {
        let policy = {
            let guard = self.inner.lock().await;
            let Some(reg) = guard.registered.get(plugin_id) else {
                return Err(VesselError::NotFound(format!("plugin '{plugin_id}'")));
            };
            Arc::clone(&reg.domain_policy)
        };
        policy.allow_domain(domain)
    }

    /// Replaces user-configured settings exposed to `plugin_id` via `vessel.storage.getSetting`.
    pub async fn set_settings(
        &self,
        plugin_id: &PluginId,
        settings: BTreeMap<String, serde_json::Value>,
    ) -> VesselResult<()> {
        let host = {
            let guard = self.inner.lock().await;
            let Some(reg) = guard.registered.get(plugin_id) else {
                return Err(VesselError::NotFound(format!("plugin '{plugin_id}'")));
            };
            Arc::clone(&reg.host)
        };
        host.set_settings(settings)
    }

    /// Lists all currently enabled registered plugin IDs.
    pub async fn enabled_plugin_ids(&self) -> Vec<PluginId> {
        let guard = self.inner.lock().await;
        guard
            .registered
            .iter()
            .filter(|&(_, reg)| reg.enabled)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Lists enabled registered plugin IDs supporting at least one capability in `caps`.
    pub async fn enabled_plugin_ids_with_any(&self, caps: &[PluginCapability]) -> Vec<PluginId> {
        let guard = self.inner.lock().await;
        guard
            .registered
            .iter()
            .filter(|&(_, reg)| {
                reg.enabled
                    && reg
                        .manifest
                        .capabilities
                        .iter()
                        .any(|cap| caps.contains(cap))
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Lists enabled subtitle-capable plugins as `(PluginId, display_name)` pairs.
    pub async fn enabled_subtitle_plugins(&self) -> Vec<(PluginId, String)> {
        let guard = self.inner.lock().await;
        guard
            .registered
            .iter()
            .filter(|&(_, reg)| {
                reg.enabled
                    && reg
                        .manifest
                        .capabilities
                        .contains(&PluginCapability::Subtitles)
                    && reg.subtitle_search_supported != Some(false)
            })
            .map(|(id, reg)| (id.clone(), reg.manifest.name.clone()))
            .collect()
    }

    /// Returns whether `plugin_id` is known to implement `searchSubtitles()`.
    pub async fn supports_subtitle_search(&self, plugin_id: &PluginId) -> Option<bool> {
        let guard = self.inner.lock().await;
        guard
            .registered
            .get(plugin_id)
            .and_then(|reg| reg.subtitle_search_supported)
    }

    /// Records runtime confirmation of whether `plugin_id` implements `searchSubtitles()`.
    pub async fn set_subtitle_search_supported(&self, plugin_id: &PluginId, supported: bool) {
        let mut guard = self.inner.lock().await;
        if let Some(reg) = guard.registered.get_mut(plugin_id) {
            reg.subtitle_search_supported = Some(supported);
        }
    }

    /// Number of currently warm QuickJS plugin runtimes held in memory.
    pub async fn warm_len(&self) -> usize {
        self.inner.lock().await.warm.len()
    }

    /// Signals cancellation (`AtomicBool`) on all currently warm QuickJS sandboxes.
    pub async fn cancel_inflight(&self) {
        let providers: Vec<Arc<QuickJsContentProvider>> = {
            let guard = self.inner.lock().await;
            guard
                .warm
                .values()
                .map(|e| Arc::clone(&e.provider))
                .collect()
        };
        for provider in providers {
            provider.sandbox().cancel();
        }
    }

    /// Clears prior cancellation flags across warm sandboxes before launching a new batch.
    pub async fn reset_cancellations(&self) {
        let providers: Vec<Arc<QuickJsContentProvider>> = {
            let guard = self.inner.lock().await;
            guard
                .warm
                .values()
                .map(|e| Arc::clone(&e.provider))
                .collect()
        };
        for provider in providers {
            provider.sandbox().reset_cancel();
        }
    }

    /// Returns a warm isolated `QuickJsContentProvider` for `plugin_id`, booting it on demand
    /// and evicting the least-recently-used sandbox if `max_warm` would be exceeded.
    pub async fn get_or_load(
        &self,
        plugin_id: &PluginId,
    ) -> VesselResult<Arc<QuickJsContentProvider>> {
        let spec = {
            let mut guard = self.inner.lock().await;
            if let Some(entry) = guard.warm.get_mut(plugin_id) {
                entry.last_used = Instant::now();
                return Ok(Arc::clone(&entry.provider));
            }
            let Some(reg) = guard.registered.get(plugin_id) else {
                return Err(VesselError::NotFound(format!("plugin '{plugin_id}'")));
            };
            if !reg.enabled {
                return Err(VesselError::InvalidInput(format!(
                    "plugin '{plugin_id}' is disabled"
                )));
            }
            (
                reg.manifest.clone(),
                Arc::clone(&reg.bundle),
                Arc::clone(&reg.host),
            )
        };

        let provider = Arc::new(
            QuickJsContentProvider::load(spec.0, spec.1, spec.2, self.sandbox_limits).await?,
        );

        let mut evicted_entries = Vec::new();
        {
            let mut guard = self.inner.lock().await;
            if let Some(existing) = guard.warm.get_mut(plugin_id) {
                existing.last_used = Instant::now();
                let duplicate = Arc::clone(&provider);
                drop(guard);
                shutdown_arc_provider(duplicate).await;
                return existing_or_clone(&self.inner, plugin_id).await;
            }

            while guard.warm.len() >= self.max_warm {
                let oldest_key = guard
                    .warm
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_used)
                    .map(|(k, _)| k.clone());
                match oldest_key {
                    Some(key) => {
                        if let Some(evicted) = guard.warm.remove(&key) {
                            evicted_entries.push(evicted.provider);
                        }
                    }
                    None => break,
                }
            }

            if let Some(reg) = guard.registered.get_mut(plugin_id) {
                reg.subtitle_search_supported = Some(provider.supports_subtitle_search());
            }

            guard.warm.insert(
                plugin_id.clone(),
                WarmPlugin {
                    provider: Arc::clone(&provider),
                    last_used: Instant::now(),
                },
            );
        }

        for old in evicted_entries {
            shutdown_arc_provider(old).await;
        }

        Ok(provider)
    }

    /// Evicts all warm runtimes idle for at least `idle_ttl` and returns the count evicted.
    pub async fn evict_idle(&self) -> usize {
        self.evict_idle_older_than(self.idle_ttl).await
    }

    /// Evicts warm runtimes whose inactivity exceeds `max_idle`.
    pub async fn evict_idle_older_than(&self, max_idle: Duration) -> usize {
        let now = Instant::now();
        let stale: Vec<Arc<QuickJsContentProvider>> = {
            let mut guard = self.inner.lock().await;
            let stale_keys: Vec<PluginId> = guard
                .warm
                .iter()
                .filter(|&(_, entry)| now.saturating_duration_since(entry.last_used) >= max_idle)
                .map(|(id, _)| id.clone())
                .collect();
            stale_keys
                .into_iter()
                .filter_map(|k| guard.warm.remove(&k).map(|e| e.provider))
                .collect()
        };

        let count = stale.len();
        for provider in stale {
            shutdown_arc_provider(provider).await;
        }
        count
    }

    /// Shuts down all warm QuickJS runtimes gracefully.
    pub async fn shutdown_all(&self) {
        let all: Vec<Arc<QuickJsContentProvider>> = {
            let mut guard = self.inner.lock().await;
            guard.warm.drain().map(|(_, e)| e.provider).collect()
        };
        for provider in all {
            shutdown_arc_provider(provider).await;
        }
    }

    /// Removes a plugin from the registry and shuts down any warm sandbox.
    pub async fn unregister(&self, plugin_id: &PluginId) -> bool {
        let (existed, warm) = {
            let mut guard = self.inner.lock().await;
            let existed = guard.registered.remove(plugin_id).is_some();
            let warm = guard.warm.remove(plugin_id);
            (existed, warm)
        };
        if let Some(entry) = warm {
            shutdown_arc_provider(entry.provider).await;
        }
        existed
    }
}

async fn existing_or_clone(
    inner: &Mutex<PoolInner>,
    plugin_id: &PluginId,
) -> VesselResult<Arc<QuickJsContentProvider>> {
    let guard = inner.lock().await;
    guard
        .warm
        .get(plugin_id)
        .map(|e| Arc::clone(&e.provider))
        .ok_or_else(|| VesselError::NotFound(format!("plugin '{plugin_id}'")))
}

async fn shutdown_arc_provider(mut provider: Arc<QuickJsContentProvider>) {
    for _ in 0..150 {
        match Arc::try_unwrap(provider) {
            Ok(owned) => {
                owned.shutdown().await;
                return;
            }
            Err(still_shared) => {
                provider = still_shared;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }
    }
    tokio::spawn(async move {
        for _ in 0..1200 {
            match Arc::try_unwrap(provider) {
                Ok(owned) => {
                    owned.shutdown().await;
                    return;
                }
                Err(still_shared) => {
                    provider = still_shared;
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            }
        }
        // Intentionally avoid raw synchronous Drop on a background QuickJS runtime if still wedged.
        std::mem::forget(provider);
    });
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use async_trait::async_trait;
    use vessel_domain::{
        DomainGrantSource, LibraryItem, LibraryStatus, MediaRef, PlaybackProgress, PluginId,
        PluginManifest, StoragePort, VesselResult,
    };
    use vessel_plugin_rt::{PluginBundle, SandboxLimits, TsCompileCache};

    use super::PluginPool;

    #[derive(Default)]
    struct DummyStorage(Mutex<BTreeMap<String, String>>);

    #[async_trait]
    impl StoragePort for DummyStorage {
        async fn get_setting(&self, _key: &str) -> VesselResult<Option<String>> {
            Ok(None)
        }
        async fn set_setting(&self, _key: &str, _value_json: &str) -> VesselResult<()> {
            Ok(())
        }
        async fn get_plugin_kv(&self, _p: &PluginId, _k: &str) -> VesselResult<Option<String>> {
            Ok(None)
        }
        async fn set_plugin_kv(&self, _p: &PluginId, _k: &str, _v: &str) -> VesselResult<()> {
            Ok(())
        }
        async fn remove_plugin_kv(&self, _p: &PluginId, _k: &str) -> VesselResult<()> {
            Ok(())
        }
        async fn is_domain_allowed(&self, _p: &PluginId, _d: &str) -> VesselResult<bool> {
            Ok(false)
        }
        async fn grant_domain(
            &self,
            _p: &PluginId,
            _d: &str,
            _s: DomainGrantSource,
        ) -> VesselResult<()> {
            let _ = &self.0;
            Ok(())
        }
        async fn get_progress(
            &self,
            _m: &MediaRef,
            _s: u32,
            _e: u32,
        ) -> VesselResult<Option<PlaybackProgress>> {
            Ok(None)
        }
        async fn upsert_progress(&self, _p: &PlaybackProgress) -> VesselResult<()> {
            Ok(())
        }
        async fn list_library(
            &self,
            _status: Option<LibraryStatus>,
        ) -> VesselResult<Vec<LibraryItem>> {
            Ok(Vec::new())
        }
        async fn upsert_library_item(&self, _item: &LibraryItem) -> VesselResult<()> {
            Ok(())
        }
        async fn remove_library_item(&self, _media: &MediaRef) -> VesselResult<()> {
            Ok(())
        }
    }

    fn manifest_for(id: &str) -> PluginManifest {
        serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "id": id,
            "name": id,
            "version": "1.0.0",
            "entry": "index.ts",
            "capabilities": ["vod"]
        }))
        .expect("manifest")
    }

    fn bundle_for() -> Arc<PluginBundle> {
        Arc::new(
            PluginBundle::new([(
                "index.ts",
                "export default { async getHomeFeed() { return { sections: [] }; } };",
            )])
            .expect("bundle"),
        )
    }

    #[tokio::test]
    async fn enforces_lru_capacity_and_idle_ttl_eviction() {
        let storage = Arc::new(DummyStorage::default());
        let ts_cache = Arc::new(TsCompileCache::new());
        let pool = PluginPool::new(
            2,
            Duration::from_millis(30),
            SandboxLimits::default(),
            storage,
            None,
            ts_cache,
        );

        let p1 = PluginId::new("anthology").expect("id1");
        let p2 = PluginId::new("plugin-two").expect("id2");
        let p3 = PluginId::new("plugin-three").expect("id3");

        for id in [&p1, &p2, &p3] {
            pool.register(manifest_for(id.as_str()), bundle_for(), vec![], true)
                .await
                .expect("register");
        }

        let _ = pool.get_or_load(&p1).await.expect("warm p1");
        tokio::time::sleep(Duration::from_millis(5)).await;
        let _ = pool.get_or_load(&p2).await.expect("warm p2");
        assert_eq!(pool.warm_len().await, 2);

        // Touch p1 so p2 becomes least-recently-used before loading p3.
        tokio::time::sleep(Duration::from_millis(5)).await;
        let _ = pool.get_or_load(&p1).await.expect("touch p1");
        let _ = pool.get_or_load(&p3).await.expect("warm p3");
        assert_eq!(pool.warm_len().await, 2);

        tokio::time::sleep(Duration::from_millis(40)).await;
        let evicted = pool.evict_idle().await;
        assert_eq!(evicted, 2);
        assert_eq!(pool.warm_len().await, 0);

        pool.shutdown_all().await;
    }
}
