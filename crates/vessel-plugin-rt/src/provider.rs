//! `QuickJsContentProvider` bridging a sandboxed plugin module to `vessel_domain::ContentProvider`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rquickjs::{CatchResultExt, Function, Module, Object, Promise, Value};
use serde::Serialize;
use serde::de::DeserializeOwned;
use vessel_domain::{
    ContentProvider, Episode, HomeFeed, MediaDetail, PluginId, PluginManifest, SearchPage,
    StreamResult, VesselError, VesselResult,
};

use crate::module_loader::{PluginBundle, PluginLoader, PluginResolver};
use crate::sandbox::{PluginSandbox, SandboxLimits};
use crate::sdk::{HostServices, install_sdk};

const STREAM_EXECUTION_TIMEOUT: Duration = Duration::from_secs(25);

const PROVIDER_DISPATCH_JS: &str = r#"
globalThis.__vessel_call = async (method, args) => {
    const target = globalThis.__vessel_provider;
    if (!target || typeof target[method] !== "function") {
        throw new Error(`plugin does not implement '${method}()'`);
    }
    const result = await target[method](...(Array.isArray(args) ? args : []));
    return JSON.stringify(result === undefined ? null : result);
};
"#;

/// Stateful QuickJS plugin instance implementing `vessel_domain::ContentProvider`.
pub struct QuickJsContentProvider {
    plugin_id: PluginId,
    manifest: PluginManifest,
    sandbox: PluginSandbox,
    host: Arc<HostServices>,
}

impl std::fmt::Debug for QuickJsContentProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QuickJsContentProvider")
            .field("plugin_id", &self.plugin_id)
            .field("sandbox", &self.sandbox)
            .finish_non_exhaustive()
    }
}

impl QuickJsContentProvider {
    /// Provisions an isolated QuickJS runtime, installs `vessel.*` bindings, and loads `manifest.entry`.
    pub async fn load(
        manifest: PluginManifest,
        bundle: Arc<PluginBundle>,
        host: Arc<HostServices>,
        limits: SandboxLimits,
    ) -> VesselResult<Self> {
        manifest.validate()?;
        let plugin_id = manifest.id.clone();
        let entry = bundle.resolve_entry(&manifest.entry)?;
        let import_specifier = format!("./{entry}");

        let sandbox = PluginSandbox::new(plugin_id.clone(), limits).await?;
        sandbox
            .set_loader(
                PluginResolver::new(Arc::clone(&bundle)),
                PluginLoader::new(
                    plugin_id.clone(),
                    Arc::clone(&bundle),
                    Arc::clone(&host.ts_cache),
                ),
            )
            .await;

        sandbox
            .run_in_context(limits.execution_timeout, async |ctx| {
                install_sdk(&ctx, Arc::clone(&host))
                    .catch(&ctx)
                    .map_err(|err| err.to_string())?;

                ctx.eval::<(), _>(PROVIDER_DISPATCH_JS)
                    .catch(&ctx)
                    .map_err(|err| err.to_string())?;

                let promise = Module::import(&ctx, import_specifier)
                    .catch(&ctx)
                    .map_err(|err| err.to_string())?;
                let namespace: Object<'_> = promise
                    .into_future()
                    .await
                    .catch(&ctx)
                    .map_err(|err| err.to_string())?;

                let default_export: Value<'_> = namespace
                    .get("default")
                    .catch(&ctx)
                    .map_err(|err| err.to_string())?;
                let provider_obj = if default_export.is_object() {
                    default_export
                        .into_object()
                        .ok_or_else(|| "default export must be an object".to_owned())?
                } else {
                    namespace
                };

                ctx.globals()
                    .set("__vessel_provider", provider_obj)
                    .catch(&ctx)
                    .map_err(|err| err.to_string())?;

                Ok(())
            })
            .await?;

        Ok(Self {
            plugin_id,
            manifest,
            sandbox,
            host,
        })
    }

    /// Borrows this provider's host services (e.g. to grant dynamic domain permissions).
    pub fn host(&self) -> &Arc<HostServices> {
        &self.host
    }

    /// Borrows the underlying QuickJS sandbox (for cancellation or memory diagnostics).
    pub fn sandbox(&self) -> &PluginSandbox {
        &self.sandbox
    }

    /// Gracefully shuts down the underlying QuickJS runtime.
    pub async fn shutdown(self) {
        self.sandbox.shutdown().await;
    }

    async fn call_json<A, R>(&self, method: &str, args: &A, timeout: Duration) -> VesselResult<R>
    where
        A: Serialize + Sync,
        R: DeserializeOwned,
    {
        let method_name = method.to_owned();
        let args_json = serde_json::to_string(args).map_err(|err| {
            VesselError::InvalidInput(format!("serialize arguments for '{method}': {err}"))
        })?;

        let raw_json = self
            .sandbox
            .run_in_context(timeout, async |ctx| {
                let args_val = ctx
                    .json_parse(args_json)
                    .catch(&ctx)
                    .map_err(|err| err.to_string())?;
                let call_fn: Function<'_> = ctx
                    .globals()
                    .get("__vessel_call")
                    .catch(&ctx)
                    .map_err(|err| err.to_string())?;
                let promise: Promise<'_> = call_fn
                    .call((method_name, args_val))
                    .catch(&ctx)
                    .map_err(|err| err.to_string())?;
                promise
                    .into_future::<String>()
                    .await
                    .catch(&ctx)
                    .map_err(|err| err.to_string())
            })
            .await?;

        let parsed =
            serde_json::from_str::<R>(&raw_json).map_err(|err| VesselError::PluginRuntime {
                plugin_id: self.plugin_id.to_string(),
                message: format!("invalid return payload from '{method}': {err}"),
            })?;

        // Reclaim transient cheerio/DOM objects promptly so QuickJS RAM stays lean.
        let mem = self.sandbox.memory_stats().await;
        if mem.memory_used_size > 6 * 1024 * 1024 {
            self.sandbox.run_gc().await;
        }

        Ok(parsed)
    }
}

#[async_trait]
impl ContentProvider for QuickJsContentProvider {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn home_feed(&self, cursor: Option<&str>) -> VesselResult<HomeFeed> {
        let timeout = self.sandbox.limits().execution_timeout;
        self.call_json("getHomeFeed", &(cursor,), timeout).await
    }

    async fn search(
        &self,
        query: &str,
        filters: &BTreeMap<String, String>,
        page: u32,
    ) -> VesselResult<SearchPage> {
        let timeout = self.sandbox.limits().execution_timeout;
        self.call_json("search", &(query, filters, page), timeout)
            .await
    }

    async fn details(&self, native_id: &str) -> VesselResult<MediaDetail> {
        let timeout = self.sandbox.limits().execution_timeout;
        self.call_json("getDetails", &(native_id,), timeout).await
    }

    async fn season_episodes(
        &self,
        native_id: &str,
        season_id: &str,
    ) -> VesselResult<Vec<Episode>> {
        let timeout = self.sandbox.limits().execution_timeout;
        self.call_json("getSeasonEpisodes", &(native_id, season_id), timeout)
            .await
    }

    async fn streams(&self, stream_id: &str) -> VesselResult<StreamResult> {
        let timeout = self
            .sandbox
            .limits()
            .execution_timeout
            .max(STREAM_EXECUTION_TIMEOUT);
        self.call_json("getStreams", &(stream_id,), timeout).await
    }
}

#[cfg(test)]
pub(crate) fn provider_test_storage() -> std::sync::Arc<dyn vessel_domain::StoragePort> {
    std::sync::Arc::new(tests::InMemoryTestStorage::default())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use axum::Router;
    use axum::http::StatusCode;
    use axum::response::{Html as AxumHtml, IntoResponse};
    use axum::routing::get;
    use tokio::net::TcpListener;
    use vessel_domain::{
        ContentProvider, DomainGrantSource, EngineEvent, EventSink, LibraryItem, LibraryStatus,
        MediaRef, MediaType, PlaybackProgress, PluginId, PluginManifest, StoragePort, VesselError,
        VesselResult,
    };

    use super::QuickJsContentProvider;
    use crate::{DomainPolicy, HostServices, PluginBundle, SandboxLimits, TsCompileCache};

    #[derive(Default)]
    pub(super) struct InMemoryTestStorage {
        kv: Mutex<BTreeMap<(String, String), String>>,
        settings: Mutex<BTreeMap<String, String>>,
    }

    #[async_trait]
    impl StoragePort for InMemoryTestStorage {
        async fn get_setting(&self, key: &str) -> VesselResult<Option<String>> {
            let guard = self.settings.lock().expect("lock settings");
            Ok(guard.get(key).cloned())
        }

        async fn set_setting(&self, key: &str, value_json: &str) -> VesselResult<()> {
            let mut guard = self.settings.lock().expect("lock settings");
            guard.insert(key.to_owned(), value_json.to_owned());
            Ok(())
        }

        async fn get_plugin_kv(
            &self,
            plugin_id: &PluginId,
            key: &str,
        ) -> VesselResult<Option<String>> {
            let guard = self.kv.lock().expect("lock kv");
            Ok(guard
                .get(&(plugin_id.as_str().to_owned(), key.to_owned()))
                .cloned())
        }

        async fn set_plugin_kv(
            &self,
            plugin_id: &PluginId,
            key: &str,
            value: &str,
        ) -> VesselResult<()> {
            let mut guard = self.kv.lock().expect("lock kv");
            guard.insert(
                (plugin_id.as_str().to_owned(), key.to_owned()),
                value.to_owned(),
            );
            Ok(())
        }

        async fn remove_plugin_kv(&self, plugin_id: &PluginId, key: &str) -> VesselResult<()> {
            let mut guard = self.kv.lock().expect("lock kv");
            guard.remove(&(plugin_id.as_str().to_owned(), key.to_owned()));
            Ok(())
        }

        async fn is_domain_allowed(
            &self,
            _plugin_id: &PluginId,
            _domain: &str,
        ) -> VesselResult<bool> {
            Ok(false)
        }

        async fn grant_domain(
            &self,
            _plugin_id: &PluginId,
            _domain: &str,
            _source: DomainGrantSource,
        ) -> VesselResult<()> {
            Ok(())
        }

        async fn get_progress(
            &self,
            _media: &MediaRef,
            _season: u32,
            _episode: u32,
        ) -> VesselResult<Option<PlaybackProgress>> {
            Ok(None)
        }

        async fn upsert_progress(&self, _progress: &PlaybackProgress) -> VesselResult<()> {
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

    #[derive(Default)]
    struct LogSink {
        events: Mutex<Vec<EngineEvent>>,
    }

    impl EventSink for LogSink {
        fn emit(&self, event: EngineEvent) {
            self.events.lock().expect("lock events").push(event);
        }
    }

    async fn spawn_mock_site() -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock site");
        let addr = listener.local_addr().expect("mock site addr");

        let app = Router::new()
            .route(
                "/search",
                get(|| async {
                    AxumHtml(
                        r#"
                        <html><body>
                            <div class="card" data-id="/anime/sample-1">
                                <h2 class="title">Anthology Chronicle</h2>
                                <span class="year">2025</span>
                            </div>
                        </body></html>
                        "#,
                    )
                }),
            )
            .route(
                "/streams",
                get(|| async {
                    let packed = r#"eval(function(p,a,c,k,e,d){while(c--)if(k[c])p=p.replace(new RegExp('\\b'+c.toString(a)+'\\b','g'),k[c]);return p}('0 1="2://3.4/5.6";',7,7,'const|url|https|cdn|anthology|master|m3u8'.split('|'),0,{}))"#;
                    (StatusCode::OK, packed).into_response()
                }),
            );

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        format!("http://{addr}")
    }

    #[tokio::test]
    async fn loads_ts_plugin_executes_sdk_bridges_and_enforces_domain_policy() {
        let mock_base = spawn_mock_site().await;
        let plugin_id = PluginId::new("anthology").expect("valid id");
        let manifest: PluginManifest = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "id": "anthology",
            "name": "Anthology",
            "version": "0.1.0",
            "entry": "src/index.ts",
            "capabilities": ["vod"],
            "domains": ["127.0.0.1"]
        }))
        .expect("parse manifest");

        let bundle = Arc::new(
            PluginBundle::new([
                (
                    "src/utils/parser.ts",
                    r#"
                    import { html } from "vessel";

                    export interface ParsedCard {
                        nativeId: string;
                        title: string;
                        year: number;
                    }

                    export function parseSearchHtml(raw: string): ParsedCard[] {
                        const doc = html.parse(raw);
                        return doc.select(".card").map((el) => ({
                            nativeId: el.attr("data-id") || "",
                            title: el.selectOne(".title")?.text() || "",
                            year: Number(el.selectOne(".year")?.text() || "0"),
                        }));
                    }
                    "#,
                ),
                (
                    "src/index.ts",
                    &format!(
                        r#"
                        import {{ http, crypto, storage, unpack, evalWithEnv, definePlugin }} from "vessel";
                        import {{ parseSearchHtml }} from "./utils/parser";

                        const BASE_URL = "{mock_base}";

                        export default definePlugin({{
                            async getHomeFeed() {{
                                console.log("loading home feed");
                                return {{ sections: [] }};
                            }},

                            async search(query: string) {{
                                if (query === "blocked-domain") {{
                                    await http.get("https://unapproved.anthology.example/api");
                                }}
                                if (query === "ssrf") {{
                                    await http.get("https://169.254.169.254/latest/meta-data/");
                                }}

                                await storage.set("lastQuery", query);
                                const saved = await storage.get<string>("lastQuery");
                                const token = await storage.getSetting<string>("apiToken");
                                const digest = crypto.sha256(`${{saved}}:${{token ?? ""}}`);
                                console.info("search digest", digest.slice(0, 8));

                                const resp = await http.get(`${{BASE_URL}}/search`);
                                const cards = parseSearchHtml(resp.text());
                                return {{
                                    items: cards.map((c) => ({{
                                        pluginId: "anthology",
                                        nativeId: c.nativeId,
                                        type: "series",
                                        isAnime: true,
                                        title: c.title,
                                        year: c.year,
                                    }})),
                                    hasMore: false,
                                }};
                            }},

                            async getDetails(nativeId: string) {{
                                return {{
                                    pluginId: "anthology",
                                    nativeId,
                                    type: "series",
                                    isAnime: true,
                                    title: "Anthology Chronicle",
                                    episodes: [{{
                                        streamId: "ep-1",
                                        season: 1,
                                        episode: 1,
                                    }}],
                                }};
                            }},

                            async getStreams(streamId: string) {{
                                const resp = await http.get(`${{BASE_URL}}/streams`);
                                const unpacked = unpack(resp.text());
                                const extracted = await evalWithEnv<string>(
                                    unpacked + "\nreturn url + '?ua=' + encodeURIComponent(navigator.userAgent) + '&id=" + streamId + "';",
                                    {{ url: "https://player.anthology.example/embed", userAgent: "VesselTest/1.0" }}
                                );
                                return {{
                                    streams: [{{
                                        url: extracted,
                                        kind: "hls",
                                        label: "Anthology CDN",
                                    }}],
                                    subtitles: [],
                                }};
                            }},
                        }});
                        "#
                    ),
                ),
            ])
            .expect("build plugin bundle"),
        );

        let policy = Arc::new(
            DomainPolicy::new(plugin_id.clone(), vec!["127.0.0.1".into()], false)
                .with_loopback_for_tests(),
        );
        let storage = Arc::new(InMemoryTestStorage::default());
        let sink = Arc::new(LogSink::default());
        let ts_cache = Arc::new(TsCompileCache::new());

        let host = Arc::new(
            HostServices::new(
                plugin_id.clone(),
                policy,
                storage,
                Some(sink.clone()),
                ts_cache.clone(),
            )
            .expect("create host services"),
        );
        let mut settings = BTreeMap::new();
        settings.insert("apiToken".into(), serde_json::json!("secret-123"));
        host.set_settings(settings).expect("set plugin settings");

        let provider = QuickJsContentProvider::load(
            manifest,
            bundle,
            Arc::clone(&host),
            SandboxLimits::default(),
        )
        .await
        .expect("load anthology TS provider");

        assert_eq!(ts_cache.len(), 2);

        let search_page = provider
            .search("chronicle", &BTreeMap::new(), 1)
            .await
            .expect("run TS search");
        assert_eq!(search_page.items.len(), 1);
        assert_eq!(search_page.items[0].title, "Anthology Chronicle");
        assert_eq!(search_page.items[0].media_type, MediaType::Series);
        assert!(search_page.items[0].is_anime);

        let streams = provider.streams("ep-1").await.expect("run TS getStreams");
        assert_eq!(streams.streams.len(), 1);
        assert_eq!(
            streams.streams[0].url,
            "https://cdn.anthology/master.m3u8?ua=VesselTest%2F1.0&id=ep-1"
        );

        let blocked_err = provider
            .search("blocked-domain", &BTreeMap::new(), 1)
            .await
            .expect_err("reject unapproved domain");
        assert_eq!(
            blocked_err,
            VesselError::DomainBlocked {
                plugin_id: "anthology".into(),
                domain: "unapproved.anthology.example".into(),
            }
        );

        let ssrf_err = provider
            .search("ssrf", &BTreeMap::new(), 1)
            .await
            .expect_err("reject SSRF");
        assert!(matches!(ssrf_err, VesselError::PluginRuntime { .. }));

        {
            let logs = sink.events.lock().expect("lock log events");
            assert!(!logs.is_empty());
        }

        provider.shutdown().await;
    }
}
