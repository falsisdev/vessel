//! Safe in-bundle ES module resolver & loader (`"vessel"` SDK + `.ts`/`.js` via `oxc`).

use std::collections::BTreeMap;
use std::sync::Arc;

use rquickjs::loader::{ImportAttributes, Loader, Resolver};
use rquickjs::{Ctx, Error as QjsError, Module, module::Declared};
use vessel_domain::{PluginId, VesselError, VesselResult};

use crate::ts_strip::TsCompileCache;

/// Built-in specifier exported to plugin ES modules (`import { http, html } from "vessel"`).
pub const VESSEL_SDK_MODULE_ID: &str = "vessel";

const VESSEL_MODULE_SHIM: &str = r#"
const v = globalThis.vessel;
if (!v) {
    throw new Error("Vessel runtime host bindings were not initialized");
}
export const http = v.http;
export const html = v.html;
export const crypto = v.crypto;
export const storage = v.storage;
export const unpack = v.unpack;
export const evalWithEnv = v.evalWithEnv;
export const definePlugin = v.definePlugin;
export default v;
"#;

const RESOLVE_CANDIDATE_SUFFIXES: &[&str] = &[
    "",
    ".ts",
    ".mts",
    ".js",
    ".mjs",
    "/index.ts",
    "/index.mts",
    "/index.js",
    "/index.mjs",
];

fn normalize_path(raw: &str) -> VesselResult<String> {
    if raw.starts_with('/') || raw.starts_with('\\') {
        return Err(VesselError::InvalidInput(format!(
            "absolute module paths are not allowed: '{raw}'"
        )));
    }

    let mut stack: Vec<&str> = Vec::new();
    for segment in raw.split(['/', '\\']) {
        match segment {
            "" | "." => {}
            ".." => {
                if stack.pop().is_none() {
                    return Err(VesselError::InvalidInput(format!(
                        "module path escapes plugin bundle root: '{raw}'"
                    )));
                }
            }
            part => stack.push(part),
        }
    }

    if stack.is_empty() {
        return Err(VesselError::InvalidInput(format!(
            "module path cannot be empty: '{raw}'"
        )));
    }

    Ok(stack.join("/"))
}

fn is_typescript_path(path: &str) -> bool {
    path.ends_with(".ts") || path.ends_with(".mts") || path.ends_with(".tsx")
}

/// Read-only archive of source files belonging to a single plugin bundle.
#[derive(Debug, Clone, Default)]
pub struct PluginBundle {
    files: BTreeMap<String, String>,
}

impl PluginBundle {
    /// Creates a bundle from `(relative_path, source)` entries, rejecting escaping paths.
    pub fn new<K, V>(entries: impl IntoIterator<Item = (K, V)>) -> VesselResult<Self>
    where
        K: AsRef<str>,
        V: Into<String>,
    {
        let mut files = BTreeMap::new();
        for (raw_path, source) in entries {
            let normalized = normalize_path(raw_path.as_ref())?;
            files.insert(normalized, source.into());
        }
        Ok(Self { files })
    }

    /// Reads raw source code for an exact normalized bundle path.
    pub fn get(&self, normalized_path: &str) -> Option<&str> {
        self.files.get(normalized_path).map(String::as_str)
    }

    /// Resolves a manifest entry path (e.g. `"src/index.ts"` or `"./index.ts"`).
    pub fn resolve_entry(&self, entry: &str) -> VesselResult<String> {
        let base_normalized = normalize_path(entry)?;
        self.match_candidate(&base_normalized).ok_or_else(|| {
            VesselError::NotFound(format!("plugin entry module '{entry}' not found in bundle"))
        })
    }

    /// Resolves an `import` specifier originating from `base` inside the bundle.
    pub fn resolve_import(&self, base: &str, specifier: &str) -> VesselResult<String> {
        if specifier == VESSEL_SDK_MODULE_ID {
            return Ok(VESSEL_SDK_MODULE_ID.to_owned());
        }

        if !specifier.starts_with("./") && !specifier.starts_with("../") {
            return Err(VesselError::InvalidInput(format!(
                "unsupported external or bare module import '{specifier}'; bundle npm dependencies or use 'vessel'"
            )));
        }

        let parent_dir = match base.rsplit_once('/') {
            Some((dir, _)) => dir,
            None => "",
        };
        let combined = if parent_dir.is_empty() {
            specifier.to_owned()
        } else {
            format!("{parent_dir}/{specifier}")
        };
        let normalized = normalize_path(&combined)?;
        self.match_candidate(&normalized).ok_or_else(|| {
            VesselError::NotFound(format!(
                "module '{specifier}' imported from '{base}' not found in bundle"
            ))
        })
    }

    fn match_candidate(&self, normalized_stem: &str) -> Option<String> {
        for suffix in RESOLVE_CANDIDATE_SUFFIXES {
            let candidate = if suffix.is_empty() {
                normalized_stem.to_owned()
            } else {
                format!("{normalized_stem}{suffix}")
            };
            if self.files.contains_key(&candidate) {
                return Some(candidate);
            }
        }
        None
    }
}

/// QuickJS `Resolver` restricted to `"vessel"` and intra-bundle relative paths.
pub(crate) struct PluginResolver {
    bundle: Arc<PluginBundle>,
}

impl PluginResolver {
    pub fn new(bundle: Arc<PluginBundle>) -> Self {
        Self { bundle }
    }
}

impl Resolver for PluginResolver {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        self.bundle
            .resolve_import(base, name)
            .map_err(|err| QjsError::new_resolving_message(base, name, err.to_string()))
    }
}

/// QuickJS `Loader` compiling `.ts`/`.mts` modules transparently via `oxc` + `TsCompileCache`.
pub(crate) struct PluginLoader {
    plugin_id: PluginId,
    bundle: Arc<PluginBundle>,
    ts_cache: Arc<TsCompileCache>,
}

impl PluginLoader {
    pub fn new(
        plugin_id: PluginId,
        bundle: Arc<PluginBundle>,
        ts_cache: Arc<TsCompileCache>,
    ) -> Self {
        Self {
            plugin_id,
            bundle,
            ts_cache,
        }
    }
}

impl Loader for PluginLoader {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        _attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, Declared>> {
        if name == VESSEL_SDK_MODULE_ID {
            return Module::declare(ctx.clone(), VESSEL_SDK_MODULE_ID, VESSEL_MODULE_SHIM);
        }

        let Some(source) = self.bundle.get(name) else {
            return Err(QjsError::new_loading(name));
        };

        let js_source = if is_typescript_path(name) {
            self.ts_cache
                .compile(&self.plugin_id, name, source)
                .map(|compiled| compiled.code)
                .map_err(|err| QjsError::new_loading_message(name, err.to_string()))?
        } else {
            source.to_owned()
        };

        Module::declare(ctx.clone(), name, js_source)
    }
}

#[cfg(test)]
mod tests {
    use vessel_domain::VesselError;

    use super::PluginBundle;

    #[test]
    fn resolves_relative_ts_and_js_modules_while_blocking_escapes() {
        let bundle = PluginBundle::new([
            (
                "src/index.ts",
                "import { parseItem } from './utils/parser';",
            ),
            ("src/utils/parser.ts", "export const parseItem = () => 1;"),
            ("src/sub/index.js", "export const ok = true;"),
        ])
        .expect("build bundle");

        assert_eq!(
            bundle.resolve_entry("./src/index.ts").expect("entry"),
            "src/index.ts"
        );
        assert_eq!(
            bundle
                .resolve_import("src/index.ts", "vessel")
                .expect("vessel"),
            "vessel"
        );
        assert_eq!(
            bundle
                .resolve_import("src/index.ts", "./utils/parser")
                .expect("relative ts without ext"),
            "src/utils/parser.ts"
        );
        assert_eq!(
            bundle
                .resolve_import("src/utils/parser.ts", "../sub")
                .expect("directory index"),
            "src/sub/index.js"
        );

        let escape_err = bundle
            .resolve_import("src/index.ts", "../../etc/passwd")
            .expect_err("reject bundle escape");
        assert!(matches!(escape_err, VesselError::InvalidInput(_)));

        let bare_err = bundle
            .resolve_import("src/index.ts", "axios")
            .expect_err("reject unbundled npm package");
        assert!(matches!(bare_err, VesselError::InvalidInput(_)));
    }
}
