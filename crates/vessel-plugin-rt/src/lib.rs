//! Sandboxed QuickJS plugin runtime (`oxc` TS stripping, `ModuleLoader`, `vessel.*` SDK, SSRF guards).

pub mod module_loader;
pub mod provider;
pub mod sandbox;
pub mod sdk;
pub mod security;
pub mod ts_strip;

pub use module_loader::{PluginBundle, VESSEL_SDK_MODULE_ID};
pub use provider::QuickJsContentProvider;
pub use sandbox::{
    DEFAULT_EXECUTION_TIMEOUT, DEFAULT_MAX_STACK_SIZE_BYTES, DEFAULT_MEMORY_LIMIT_BYTES,
    PluginSandbox, SandboxLimits, SandboxMemoryStats,
};
pub use sdk::HostServices;
pub use sdk::unpack::unpack_dean_edwards;
pub use security::{DomainPolicy, is_forbidden_ip};
pub use ts_strip::{CompiledScript, TsCompileCache, hash_source, strip_typescript};
