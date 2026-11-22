//! `vessel.storage` isolated per-plugin KV and `settingsSchema` value accessors.

use std::sync::Arc;

use rquickjs::prelude::Async;
use rquickjs::{CatchResultExt, Ctx, Function, Object, Result as QjsResult};

use super::{HostResult, HostServices};

const STORAGE_WRAPPER_JS: &str = r#"
(() => {
    const decodeStored = (raw) => {
        if (raw === null || raw === undefined) return null;
        try { return JSON.parse(raw); } catch { return raw; }
    };
    globalThis.vessel.storage = {
        get: async (key) => decodeStored(await globalThis.__vessel_kv_get(String(key))),
        set: async (key, value) => {
            const encoded = JSON.stringify(value === undefined ? null : value);
            return globalThis.__vessel_kv_set(String(key), encoded);
        },
        remove: async (key) => globalThis.__vessel_kv_remove(String(key)),
        getSetting: async (key) => decodeStored(await globalThis.__vessel_setting_get(String(key))),
    };
})();
"#;

/// Attaches `vessel.storage` (`get`, `set`, `remove`, `getSetting`) to `globalThis.vessel`.
pub(crate) fn attach_storage<'js>(
    ctx: &Ctx<'js>,
    host: Arc<HostServices>,
    _vessel_obj: &Object<'js>,
) -> QjsResult<()> {
    {
        let host = Arc::clone(&host);
        ctx.globals().set(
            "__vessel_kv_get",
            Function::new(
                ctx.clone(),
                Async(move |key: String| {
                    let host = Arc::clone(&host);
                    async move {
                        HostResult(
                            host.storage
                                .get_plugin_kv(&host.plugin_id, &key)
                                .await
                                .map_err(|err| err.to_string()),
                        )
                    }
                }),
            )?,
        )?;
    }

    {
        let host = Arc::clone(&host);
        ctx.globals().set(
            "__vessel_kv_set",
            Function::new(
                ctx.clone(),
                Async(move |key: String, json: String| {
                    let host = Arc::clone(&host);
                    async move {
                        HostResult(
                            host.storage
                                .set_plugin_kv(&host.plugin_id, &key, &json)
                                .await
                                .map_err(|err| err.to_string()),
                        )
                    }
                }),
            )?,
        )?;
    }

    {
        let host = Arc::clone(&host);
        ctx.globals().set(
            "__vessel_kv_remove",
            Function::new(
                ctx.clone(),
                Async(move |key: String| {
                    let host = Arc::clone(&host);
                    async move {
                        HostResult(
                            host.storage
                                .remove_plugin_kv(&host.plugin_id, &key)
                                .await
                                .map_err(|err| err.to_string()),
                        )
                    }
                }),
            )?,
        )?;
    }

    {
        let host = Arc::clone(&host);
        ctx.globals().set(
            "__vessel_setting_get",
            Function::new(
                ctx.clone(),
                Async(move |key: String| {
                    let host = Arc::clone(&host);
                    async move {
                        let out = (|| -> Result<Option<String>, String> {
                            let guard = host
                                .settings
                                .read()
                                .map_err(|err| format!("settings lock: {err}"))?;
                            match guard.get(&key) {
                                Some(val) => serde_json::to_string(val)
                                    .map(Some)
                                    .map_err(|err| err.to_string()),
                                None => Ok(None),
                            }
                        })();
                        HostResult(out)
                    }
                }),
            )?,
        )?;
    }

    ctx.eval::<(), _>(STORAGE_WRAPPER_JS)
        .catch(ctx)
        .map_err(|err| err.throw(ctx))?;

    Ok(())
}
