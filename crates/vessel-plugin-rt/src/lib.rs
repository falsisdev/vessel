//! Sandboxed plugin runtime (`oxc` TypeScript stripping, `rquickjs` sandbox, and upcoming SDK).

pub mod sandbox;
pub mod ts_strip;

pub use sandbox::{
    DEFAULT_EXECUTION_TIMEOUT, DEFAULT_MAX_STACK_SIZE_BYTES, DEFAULT_MEMORY_LIMIT_BYTES,
    PluginSandbox, SandboxLimits, SandboxMemoryStats,
};
pub use ts_strip::{CompiledScript, TsCompileCache, hash_source, strip_typescript};
