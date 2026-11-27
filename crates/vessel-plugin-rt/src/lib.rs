//! Sandboxed QuickJS plugin runtime (`oxc` TS stripping, `ModuleLoader`, `.plugin.vessel`, `SourceResolver`, `vessel.*` SDK).

pub mod compat;
pub mod module_loader;
pub mod package;
pub mod provider;
pub mod sandbox;
pub mod sdk;
pub mod security;
pub mod source_resolver;
pub mod ts_strip;

pub use module_loader::{PluginBundle, VESSEL_SDK_MODULE_ID};
pub use package::{
    LoadedPackage, MAX_PACKAGE_ENTRY_BYTES, MAX_PACKAGE_UNCOMPRESSED_BYTES,
    collect_relative_imports, load_builtin_package, load_plugin_dir, load_plugin_package,
    pack_plugin_package, verify_sha256,
};
pub use provider::{QuickJsContentProvider, QuickJsDebridProvider};
pub use sandbox::{
    CallCancellation, DEFAULT_EXECUTION_TIMEOUT, DEFAULT_MAX_STACK_SIZE_BYTES,
    DEFAULT_MEMORY_LIMIT_BYTES, PluginSandbox, SandboxLimits, SandboxMemoryStats,
};
pub use sdk::HostServices;
pub use sdk::unpack::unpack_dean_edwards;
pub use security::{DomainPolicy, is_forbidden_ip};
pub use source_resolver::{
    ParsedSource, ResolvedSource, SourceResolver, SourceTrustBadge, badge_for_source,
    parse_source_input,
};
pub use ts_strip::{CompiledScript, TsCompileCache, hash_source, sha256_hex, strip_typescript};
