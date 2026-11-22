//! Sandboxed plugin runtime (`oxc` TypeScript stripping, and upcoming QuickJS + `vessel.*` SDK).

pub mod ts_strip;

pub use ts_strip::{CompiledScript, TsCompileCache, hash_source, strip_typescript};
