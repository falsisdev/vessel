//! In-process TypeScript type-stripping (`oxc`) and `source_hash` compilation cache.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;
use std::sync::RwLock;

use oxc::allocator::Allocator;
use oxc::codegen::Codegen;
use oxc::diagnostics::Diagnostics;
use oxc::parser::Parser;
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;
use oxc::transformer::{TransformOptions, Transformer};
use sha2::{Digest, Sha256};
use vessel_domain::{PluginId, VesselError, VesselResult};

/// Result of compiling (or retrieving from cache) a TypeScript/JavaScript source module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledScript {
    /// Deterministic SHA-256 hex digest of the original source text.
    pub source_hash: String,
    /// JavaScript code ready for QuickJS evaluation.
    pub code: String,
    /// `true` when `code` was served from `TsCompileCache` without invoking `oxc`.
    pub cache_hit: bool,
}

/// Computes the canonical SHA-256 hex digest used as `ts_compile_cache.source_hash`.
pub fn hash_source(source: &str) -> String {
    let digest = Sha256::digest(source.as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn source_type_for_filename(filename: &str) -> SourceType {
    SourceType::from_path(filename).unwrap_or_else(|_| SourceType::ts())
}

fn format_diagnostics(stage: &str, diagnostics: &Diagnostics) -> String {
    let details = diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    if details.is_empty() {
        format!("{stage} failed")
    } else {
        format!("{stage}: {details}")
    }
}

fn plugin_rt_error(plugin_id: &PluginId, message: impl Into<String>) -> VesselError {
    VesselError::PluginRuntime {
        plugin_id: plugin_id.to_string(),
        message: message.into(),
    }
}

/// Parses and strips TypeScript type annotations into executable ES module JavaScript via `oxc`.
///
/// Plugin authors should avoid non-erasable TypeScript constructs (`enum`, `namespace`, and
/// constructor parameter properties) so source modules remain portable across runtimes.
pub fn strip_typescript(
    plugin_id: &PluginId,
    filename: &str,
    source: &str,
) -> VesselResult<String> {
    let allocator = Allocator::default();
    let source_type = source_type_for_filename(filename);

    let parser_ret = Parser::new(&allocator, source, source_type).parse();
    if parser_ret.fatal_error || parser_ret.diagnostics.has_errors() {
        return Err(plugin_rt_error(
            plugin_id,
            format_diagnostics("typescript parse error", &parser_ret.diagnostics),
        ));
    }

    let mut program = parser_ret.program;
    let semantic_ret = SemanticBuilder::new()
        .with_excess_capacity(2.0)
        .with_enum_eval(true)
        .with_check_syntax_error(true)
        .build(&program);
    if semantic_ret.diagnostics.has_errors() {
        return Err(plugin_rt_error(
            plugin_id,
            format_diagnostics("typescript semantic error", &semantic_ret.diagnostics),
        ));
    }

    let scoping = semantic_ret.semantic.into_scoping();
    let transform_options = TransformOptions::default();
    let transformer_ret = Transformer::new(&allocator, Path::new(filename), &transform_options)
        .build_with_scoping(scoping, &mut program);
    if transformer_ret.diagnostics.has_errors() {
        return Err(plugin_rt_error(
            plugin_id,
            format_diagnostics("typescript transform error", &transformer_ret.diagnostics),
        ));
    }

    Ok(Codegen::new().build(&program).code)
}

/// Thread-safe in-memory compilation cache keyed by `source_hash`.
#[derive(Debug, Default)]
pub struct TsCompileCache {
    entries: RwLock<HashMap<String, String>>,
}

impl TsCompileCache {
    /// Creates an empty in-memory compilation cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds the cache with preloaded `(source_hash, compiled_js)` pairs (e.g. from SQLite).
    pub fn with_entries(entries: impl IntoIterator<Item = (String, String)>) -> Self {
        Self {
            entries: RwLock::new(entries.into_iter().collect()),
        }
    }

    /// Looks up a previously compiled module by `source_hash`.
    pub fn get(&self, source_hash: &str) -> VesselResult<Option<String>> {
        let guard = self
            .entries
            .read()
            .map_err(|err| VesselError::Storage(format!("ts compile cache read lock: {err}")))?;
        Ok(guard.get(source_hash).cloned())
    }

    /// Inserts or updates a compiled module keyed by `source_hash`.
    pub fn insert(&self, source_hash: String, compiled_js: String) -> VesselResult<()> {
        let mut guard = self
            .entries
            .write()
            .map_err(|err| VesselError::Storage(format!("ts compile cache write lock: {err}")))?;
        guard.insert(source_hash, compiled_js);
        Ok(())
    }

    /// Number of cached modules currently held in memory.
    pub fn len(&self) -> usize {
        self.entries.read().map(|guard| guard.len()).unwrap_or(0)
    }

    /// Returns `true` when no compiled scripts are cached.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns cached JavaScript for `source` if present, or compiles via `oxc` and caches it.
    pub fn compile(
        &self,
        plugin_id: &PluginId,
        filename: &str,
        source: &str,
    ) -> VesselResult<CompiledScript> {
        self.compile_with(plugin_id, filename, source, strip_typescript)
    }

    fn compile_with<F>(
        &self,
        plugin_id: &PluginId,
        filename: &str,
        source: &str,
        compiler: F,
    ) -> VesselResult<CompiledScript>
    where
        F: FnOnce(&PluginId, &str, &str) -> VesselResult<String>,
    {
        let source_hash = hash_source(source);
        if let Some(cached) = self.get(&source_hash)? {
            return Ok(CompiledScript {
                source_hash,
                code: cached,
                cache_hit: true,
            });
        }

        let code = compiler(plugin_id, filename, source)?;
        self.insert(source_hash.clone(), code.clone())?;
        Ok(CompiledScript {
            source_hash,
            code,
            cache_hit: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use vessel_domain::{PluginId, VesselError};

    use super::{TsCompileCache, hash_source, strip_typescript};

    fn anthology_id() -> PluginId {
        PluginId::new("anthology").expect("valid plugin id")
    }

    #[test]
    fn strips_typescript_types_and_preserves_runtime_js() {
        let plugin_id = anthology_id();
        let ts_source = r#"
            import type { MediaDetail } from "vessel/sdk";
            import { httpGet } from "vessel/http";

            export interface EpisodeItem {
                id: string;
                title: string;
                episodeNumber: number;
            }

            export type SearchResponse = EpisodeItem[];

            export async function searchCatalog<T extends EpisodeItem>(
                query: string,
                limit: number = 20,
            ): Promise<T[]> {
                const payload = (await httpGet(`/search?q=${query}&limit=${limit}`)) as unknown;
                return payload as T[];
            }
        "#;

        let js = strip_typescript(&plugin_id, "src/index.ts", ts_source)
            .expect("strip typescript module");

        assert!(js.contains("import { httpGet } from \"vessel/http\";"));
        assert!(!js.contains("MediaDetail"));
        assert!(!js.contains("interface EpisodeItem"));
        assert!(!js.contains("type SearchResponse"));
        assert!(!js.contains(": string"));
        assert!(!js.contains(": number"));
        assert!(!js.contains(": Promise"));
        assert!(!js.contains("as unknown"));
        assert!(!js.contains("as T[]"));
        assert!(js.contains("export async function searchCatalog(query, limit = 20)"));
    }

    #[test]
    fn rejects_invalid_typescript_syntax_with_plugin_runtime_error() {
        let plugin_id = anthology_id();
        let bad_ts = "export const broken: string = ;";

        let err = strip_typescript(&plugin_id, "index.ts", bad_ts)
            .expect_err("reject invalid typescript syntax");

        match &err {
            VesselError::PluginRuntime {
                plugin_id: id,
                message,
            } => {
                assert_eq!(id, "anthology");
                assert!(message.contains("typescript"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
        assert!(
            err.to_string()
                .starts_with("[VesselPluginRuntimeError]: plugin 'anthology' error:")
        );
    }

    #[test]
    fn caches_compiled_js_by_source_hash_without_recompiling() {
        let plugin_id = anthology_id();
        let cache = TsCompileCache::new();
        let source = "export const greet = (name: string): string => `hi ${name}`;";
        let expected_hash = hash_source(source);

        assert!(cache.is_empty());

        let calls = AtomicUsize::new(0);
        let first = cache
            .compile_with(&plugin_id, "index.ts", source, |id, file, src| {
                calls.fetch_add(1, Ordering::SeqCst);
                strip_typescript(id, file, src)
            })
            .expect("first compile");

        assert!(!first.cache_hit);
        assert_eq!(first.source_hash, expected_hash);
        assert!(
            first
                .code
                .contains("export const greet = (name) => `hi ${name}`;")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(cache.len(), 1);

        let second = cache
            .compile_with(&plugin_id, "index.ts", source, |id, file, src| {
                calls.fetch_add(1, Ordering::SeqCst);
                strip_typescript(id, file, src)
            })
            .expect("second compile should hit cache");

        assert!(second.cache_hit);
        assert_eq!(second.source_hash, first.source_hash);
        assert_eq!(second.code, first.code);
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // Changing source content invalidates the cache key via a new SHA-256 hash.
        let modified = "export const greet = (name: string): string => `hello ${name}`;";
        let third = cache
            .compile(&plugin_id, "index.ts", modified)
            .expect("compile modified source");
        assert!(!third.cache_hit);
        assert_ne!(third.source_hash, expected_hash);
        assert_eq!(cache.len(), 2);
    }
}
