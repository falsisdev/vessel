//! Host SDK bridge (`vessel.*`), Web standard polyfills, and sandboxed logging.

pub mod compat_js;
pub mod crypto;
pub mod html;
pub mod http;
pub mod storage;
pub mod unpack;

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use rquickjs::prelude::Async;
use rquickjs::{CatchResultExt, Ctx, Function, IntoJs, Object, Result as QjsResult, Value};
use vessel_domain::{
    EngineEvent, EventSink, LogLevel, PluginId, StoragePort, VesselError, VesselResult,
};

use crate::security::DomainPolicy;
use crate::ts_strip::TsCompileCache;

/// Converts a host `Result<T, String>` into a resolved JS value or a thrown JS `Error`.
///
/// Keeping host async closures `'static` (with zero captured `Ctx<'js>`) avoids Rust variance /
/// lifetime conflicts and lets `rquickjs` bind errors cleanly on promise settlement.
pub(crate) struct HostResult<T>(pub Result<T, String>);

impl<'js, T: IntoJs<'js>> IntoJs<'js> for HostResult<T> {
    fn into_js(self, ctx: &Ctx<'js>) -> QjsResult<Value<'js>> {
        match self.0 {
            Ok(val) => val.into_js(ctx),
            Err(msg) => {
                let js_msg = rquickjs::String::from_str(ctx.clone(), &msg)?.into_value();
                let err_ctor: Function<'js> = ctx.globals().get("Error")?;
                let err_obj: Value<'js> = err_ctor.call((js_msg,))?;
                Err(ctx.throw(err_obj))
            }
        }
    }
}

/// Minimal Web standard polyfills (`atob`/`btoa`, `TextEncoder`/`TextDecoder`, `URL`/`URLSearchParams`)
/// not natively included in bare QuickJS contexts.
pub(crate) const POLYFILLS_JS: &str = r##"
(() => {
    if (typeof globalThis.atob !== "function") {
        globalThis.atob = (input) => {
            const bytes = globalThis.vessel.crypto.base64DecodeBytes(String(input));
            let out = "";
            for (let i = 0; i < bytes.length; i++) {
                out += String.fromCharCode(bytes[i]);
            }
            return out;
        };
    }
    if (typeof globalThis.btoa !== "function") {
        globalThis.btoa = (input) => {
            const str = String(input);
            const u8 = new Uint8Array(str.length);
            for (let i = 0; i < str.length; i++) {
                u8[i] = str.charCodeAt(i) & 0xff;
            }
            return globalThis.vessel.crypto.base64Encode(u8);
        };
    }
    if (typeof globalThis.TextEncoder !== "function") {
        globalThis.TextEncoder = class TextEncoder {
            encode(input = "") {
                const b64 = globalThis.vessel.crypto.base64Encode(String(input));
                return globalThis.vessel.crypto.base64DecodeBytes(b64);
            }
        };
    }
    if (typeof globalThis.TextDecoder !== "function") {
        globalThis.TextDecoder = class TextDecoder {
            decode(bytes = new Uint8Array()) {
                const u8 = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
                const b64 = globalThis.vessel.crypto.base64Encode(u8);
                return globalThis.vessel.crypto.base64Decode(b64);
            }
        };
    }
    if (typeof globalThis.URLSearchParams !== "function") {
        globalThis.URLSearchParams = class URLSearchParams {
            constructor(init = "") {
                this._entries = [];
                if (typeof init === "string") {
                    const clean = init.startsWith("?") ? init.slice(1) : init;
                    if (clean) {
                        for (const pair of clean.split("&")) {
                            const idx = pair.indexOf("=");
                            const k = idx >= 0 ? pair.slice(0, idx) : pair;
                            const v = idx >= 0 ? pair.slice(idx + 1) : "";
                            this._entries.push([
                                decodeURIComponent(k.replace(/\+/g, " ")),
                                decodeURIComponent(v.replace(/\+/g, " ")),
                            ]);
                        }
                    }
                } else if (init && typeof init === "object") {
                    for (const [k, v] of Object.entries(init)) {
                        this._entries.push([String(k), String(v)]);
                    }
                }
            }
            append(k, v) { this._entries.push([String(k), String(v)]); }
            set(k, v) {
                const key = String(k);
                this._entries = this._entries.filter(([ek]) => ek !== key);
                this._entries.push([key, String(v)]);
            }
            get(k) {
                const found = this._entries.find(([ek]) => ek === String(k));
                return found ? found[1] : null;
            }
            getAll(k) {
                return this._entries.filter(([ek]) => ek === String(k)).map(([, v]) => v);
            }
            has(k) { return this._entries.some(([ek]) => ek === String(k)); }
            delete(k) { this._entries = this._entries.filter(([ek]) => ek !== String(k)); }
            entries() { return this._entries[Symbol.iterator](); }
            [Symbol.iterator]() { return this.entries(); }
            toString() {
                return this._entries
                    .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
                    .join("&");
            }
        };
    }
    if (typeof globalThis.URL !== "function") {
        globalThis.URL = class URL {
            constructor(input, base) {
                let raw = String(input);
                if (base !== undefined && !/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(raw)) {
                    const b = new URL(String(base));
                    if (raw.startsWith("//")) {
                        raw = b.protocol + raw;
                    } else if (raw.startsWith("/")) {
                        raw = b.origin + raw;
                    } else if (raw.startsWith("?")) {
                        raw = b.origin + b.pathname + raw;
                    } else if (raw.startsWith("#")) {
                        raw = b.origin + b.pathname + b.search + raw;
                    } else {
                        const baseDir = b.pathname.replace(/\/[^/]*$/, "/");
                        raw = b.origin + baseDir + raw;
                    }
                }
                const match = /^([a-zA-Z][a-zA-Z0-9+.-]*:)\/\/([^/?#]+)?([^?#]*)(\?[^#]*)?(#.*)?$/.exec(raw);
                if (!match) {
                    throw new TypeError(`Invalid URL: ${input}`);
                }
                this.protocol = match[1].toLowerCase();
                this.host = match[2] || "";
                const colonIdx = this.host.lastIndexOf(":");
                const bracketEnd = this.host.lastIndexOf("]");
                if (colonIdx > bracketEnd) {
                    this.hostname = this.host.slice(0, colonIdx);
                    this.port = this.host.slice(colonIdx + 1);
                } else {
                    this.hostname = this.host;
                    this.port = "";
                }
                this.pathname = match[3] || "/";
                this.search = match[4] || "";
                this.hash = match[5] || "";
                this.origin = `${this.protocol}//${this.host}`;
                this.searchParams = new URLSearchParams(this.search);
                this.href = `${this.origin}${this.pathname}${this.search}${this.hash}`;
            }
            toString() {
                const q = this.searchParams.toString();
                const search = q ? `?${q}` : "";
                return `${this.origin}${this.pathname}${search}${this.hash}`;
            }
        };
    }
    for (const alias of ["window", "global", "self"]) {
        if (typeof globalThis[alias] === "undefined") {
            Object.defineProperty(globalThis, alias, {
                get: () => globalThis,
                set: (val) => {
                    Object.defineProperty(globalThis, alias, {
                        value: val,
                        writable: true,
                        configurable: true,
                        enumerable: true,
                    });
                },
                configurable: true,
            });
        }
    }
    if (typeof globalThis.queueMicrotask !== "function") {
        globalThis.queueMicrotask = (fn) => Promise.resolve().then(() => fn());
    }
    if (typeof globalThis.process !== "object" || !globalThis.process) {
        globalThis.process = {
            env: {},
            version: "v20.0.0",
            platform: "linux",
            nextTick: (fn, ...args) => Promise.resolve().then(() => fn(...args)),
        };
    }
})();
"##;

const CONSOLE_BRIDGE_JS: &str = r#"
(() => {
    const formatArg = (arg) => {
        if (typeof arg === "string") return arg;
        try { return JSON.stringify(arg); } catch { return String(arg); }
    };
    const emit = (level, args) => {
        if (typeof globalThis.__vessel_log === "function") {
            globalThis.__vessel_log(level, args.map(formatArg).join(" "));
        }
    };
    globalThis.console = {
        log: (...args) => emit("info", args),
        info: (...args) => emit("info", args),
        warn: (...args) => emit("warn", args),
        error: (...args) => emit("error", args),
        debug: (...args) => emit("debug", args),
    };
})();
"#;

/// Shared host capabilities bound into a plugin's sandboxed `vessel.*` namespace.
pub struct HostServices {
    pub plugin_id: PluginId,
    pub domain_policy: Arc<DomainPolicy>,
    pub storage: Arc<dyn StoragePort>,
    pub event_sink: Option<Arc<dyn EventSink>>,
    pub settings: RwLock<BTreeMap<String, serde_json::Value>>,
    pub http_client: reqwest::Client,
    pub ts_cache: Arc<TsCompileCache>,
}

impl std::fmt::Debug for HostServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostServices")
            .field("plugin_id", &self.plugin_id)
            .field("domain_policy", &self.domain_policy)
            .finish_non_exhaustive()
    }
}

impl HostServices {
    /// Builds `HostServices` using a process-wide shared redirect-safe `reqwest::Client` (`Policy::none()`).
    pub fn new(
        plugin_id: PluginId,
        domain_policy: Arc<DomainPolicy>,
        storage: Arc<dyn StoragePort>,
        event_sink: Option<Arc<dyn EventSink>>,
        ts_cache: Arc<TsCompileCache>,
    ) -> VesselResult<Self> {
        static SHARED_HTTP_CLIENT: std::sync::OnceLock<Result<reqwest::Client, String>> =
            std::sync::OnceLock::new();
        let http_client = SHARED_HTTP_CLIENT
            .get_or_init(|| {
                vessel_domain::http_client_builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .map_err(|err| format!("build plugin http client: {err}"))
            })
            .clone()
            .map_err(VesselError::Network)?;
        Ok(Self {
            plugin_id,
            domain_policy,
            storage,
            event_sink,
            settings: RwLock::new(BTreeMap::new()),
            http_client,
            ts_cache,
        })
    }

    /// Replaces user-configured plugin settings readable via `vessel.storage.getSetting`.
    pub fn set_settings(&self, settings: BTreeMap<String, serde_json::Value>) -> VesselResult<()> {
        let mut guard = self
            .settings
            .write()
            .map_err(|err| VesselError::Storage(format!("settings write lock: {err}")))?;
        *guard = settings;
        Ok(())
    }

    fn emit_log(&self, level: &str, message: String) {
        if let Some(ref sink) = self.event_sink {
            let parsed_level = match level {
                "debug" => LogLevel::Debug,
                "warn" => LogLevel::Warn,
                "error" => LogLevel::Error,
                _ => LogLevel::Info,
            };
            sink.emit(EngineEvent::PluginLog {
                plugin_id: self.plugin_id.clone(),
                level: parsed_level,
                message,
            });
        }
    }
}

fn define_plugin<'js>(plugin: Value<'js>) -> Value<'js> {
    plugin
}

fn attach_console<'js>(ctx: &Ctx<'js>, host: &Arc<HostServices>) -> QjsResult<()> {
    let log_host = Arc::clone(host);
    ctx.globals().set(
        "__vessel_log",
        Function::new(ctx.clone(), move |level: String, message: String| {
            log_host.emit_log(&level, message);
        })?,
    )?;
    ctx.eval::<(), _>(CONSOLE_BRIDGE_JS)
        .catch(ctx)
        .map_err(|err| err.throw(ctx))?;
    Ok(())
}

/// Installs `globalThis.vessel`, `console`, and Web standard polyfills into a QuickJS `Ctx`.
pub(crate) fn install_sdk<'js>(ctx: &Ctx<'js>, host: Arc<HostServices>) -> QjsResult<()> {
    let vessel_obj = Object::new(ctx.clone())?;
    vessel_obj.set("definePlugin", Function::new(ctx.clone(), define_plugin)?)?;
    vessel_obj.set(
        "__sleep",
        Function::new(
            ctx.clone(),
            Async(|ms: u64| async move {
                tokio::time::sleep(std::time::Duration::from_millis(ms.min(60_000))).await;
            }),
        )?,
    )?;
    ctx.globals().set("vessel", vessel_obj.clone())?;

    http::attach_http(ctx, Arc::clone(&host), &vessel_obj)?;
    html::attach_html(ctx, &vessel_obj)?;
    crypto::attach_crypto(ctx, &vessel_obj)?;
    storage::attach_storage(ctx, Arc::clone(&host), &vessel_obj)?;
    unpack::attach_unpack_and_env(ctx, &host.plugin_id, &vessel_obj)?;

    attach_console(ctx, &host)?;

    ctx.eval::<(), _>(POLYFILLS_JS)
        .catch(ctx)
        .map_err(|err| err.throw(ctx))?;

    ctx.eval::<(), _>(compat_js::COMPAT_BRIDGE_JS)
        .catch(ctx)
        .map_err(|err| err.throw(ctx))?;

    Ok(())
}
