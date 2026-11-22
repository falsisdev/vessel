//! Sandboxed QuickJS (`AsyncRuntime`) lifecycle, memory ceiling, and interrupt watchdog.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rquickjs::{AsyncContext, AsyncRuntime, CatchResultExt, FromJs};
use vessel_domain::{PluginId, VesselError, VesselResult};

/// Default per-plugin QuickJS heap ceiling (32 MiB).
pub const DEFAULT_MEMORY_LIMIT_BYTES: usize = 32 * 1024 * 1024;

/// Default per-plugin QuickJS call stack limit (512 KiB).
pub const DEFAULT_MAX_STACK_SIZE_BYTES: usize = 512 * 1024;

/// Default per-call JS execution timeout (`15s`).
pub const DEFAULT_EXECUTION_TIMEOUT: Duration = Duration::from_secs(15);

/// Resource limits enforced on a single isolated plugin QuickJS runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxLimits {
    pub memory_limit_bytes: usize,
    pub max_stack_size_bytes: usize,
    pub execution_timeout: Duration,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            memory_limit_bytes: DEFAULT_MEMORY_LIMIT_BYTES,
            max_stack_size_bytes: DEFAULT_MAX_STACK_SIZE_BYTES,
            execution_timeout: DEFAULT_EXECUTION_TIMEOUT,
        }
    }
}

/// Snapshot of runtime memory usage reported by QuickJS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxMemoryStats {
    pub malloc_size: usize,
    pub malloc_limit: usize,
    pub memory_used_size: usize,
}

struct InterruptState {
    epoch: Instant,
    /// Relative nanosecond deadline from `epoch`; `0` means no active execution deadline.
    deadline_ns: AtomicU64,
    timed_out: AtomicBool,
    cancelled: AtomicBool,
}

impl InterruptState {
    fn new() -> Self {
        Self {
            epoch: Instant::now(),
            deadline_ns: AtomicU64::new(0),
            timed_out: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
        }
    }

    fn arm_timeout(&self, timeout: Duration) {
        self.timed_out.store(false, Ordering::Release);
        let now_ns = u64::try_from(self.epoch.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let timeout_ns = u64::try_from(timeout.as_nanos()).unwrap_or(u64::MAX);
        // Reserve `0` as the disarmed sentinel so even `Duration::ZERO` triggers immediately.
        let deadline = now_ns.saturating_add(timeout_ns).max(1);
        self.deadline_ns.store(deadline, Ordering::Release);
    }

    fn disarm(&self) {
        self.deadline_ns.store(0, Ordering::Release);
    }

    fn should_interrupt(&self) -> bool {
        if self.cancelled.load(Ordering::Acquire) {
            return true;
        }
        let deadline = self.deadline_ns.load(Ordering::Acquire);
        if deadline != 0 {
            let now_ns = u64::try_from(self.epoch.elapsed().as_nanos()).unwrap_or(u64::MAX);
            if now_ns >= deadline {
                self.timed_out.store(true, Ordering::Release);
                return true;
            }
        }
        false
    }
}

struct ExecutionGuard<'a>(&'a InterruptState);

impl Drop for ExecutionGuard<'_> {
    fn drop(&mut self) {
        self.0.disarm();
    }
}

fn map_qjs_err(plugin_id: &PluginId, err: rquickjs::Error) -> VesselError {
    VesselError::PluginRuntime {
        plugin_id: plugin_id.to_string(),
        message: err.to_string(),
    }
}

/// Isolated `rquickjs` sandbox owning its own `AsyncRuntime` and `AsyncContext`.
///
/// Each active plugin runs inside its own `PluginSandbox` so slow or misbehaving scripts cannot
/// block other plugins or exhaust host memory.
pub struct PluginSandbox {
    plugin_id: PluginId,
    limits: SandboxLimits,
    runtime: AsyncRuntime,
    context: AsyncContext,
    interrupt: Arc<InterruptState>,
}

impl fmt::Debug for PluginSandbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PluginSandbox")
            .field("plugin_id", &self.plugin_id)
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl PluginSandbox {
    /// Provisions a new isolated QuickJS runtime and full ES context with enforced resource limits.
    pub async fn new(plugin_id: PluginId, limits: SandboxLimits) -> VesselResult<Self> {
        let runtime = AsyncRuntime::new().map_err(|err| map_qjs_err(&plugin_id, err))?;
        runtime.set_memory_limit(limits.memory_limit_bytes).await;
        runtime
            .set_max_stack_size(limits.max_stack_size_bytes)
            .await;

        let interrupt = Arc::new(InterruptState::new());
        let interrupt_hook = Arc::clone(&interrupt);
        runtime
            .set_interrupt_handler(Some(Box::new(move || interrupt_hook.should_interrupt())))
            .await;

        let context = AsyncContext::full(&runtime)
            .await
            .map_err(|err| map_qjs_err(&plugin_id, err))?;

        Ok(Self {
            plugin_id,
            limits,
            runtime,
            context,
            interrupt,
        })
    }

    /// Provisions a new sandbox using Vessel's default 32 MiB / 15s limits.
    pub async fn with_defaults(plugin_id: PluginId) -> VesselResult<Self> {
        Self::new(plugin_id, SandboxLimits::default()).await
    }

    /// Identifier of the plugin isolated inside this sandbox.
    pub fn plugin_id(&self) -> &PluginId {
        &self.plugin_id
    }

    /// Active memory, stack, and execution timeout configuration.
    pub fn limits(&self) -> SandboxLimits {
        self.limits
    }

    /// Signals any in-flight JS execution to abort at the next QuickJS interrupt checkpoint.
    pub fn cancel(&self) {
        self.interrupt.cancelled.store(true, Ordering::Release);
    }

    /// Clears a previous cancellation signal before starting a fresh operation.
    pub fn reset_cancel(&self) {
        self.interrupt.cancelled.store(false, Ordering::Release);
    }

    /// Returns a snapshot of QuickJS memory allocation counters for this runtime.
    pub async fn memory_stats(&self) -> SandboxMemoryStats {
        let usage = self.runtime.memory_usage().await;
        SandboxMemoryStats {
            malloc_size: usize::try_from(usage.malloc_size.max(0)).unwrap_or(usize::MAX),
            malloc_limit: usize::try_from(usage.malloc_limit.max(0)).unwrap_or(usize::MAX),
            memory_used_size: usize::try_from(usage.memory_used_size.max(0)).unwrap_or(usize::MAX),
        }
    }

    /// Triggers QuickJS cyclic garbage collection inside this runtime.
    pub async fn run_gc(&self) {
        self.runtime.run_gc().await;
    }

    /// Evaluates a JavaScript snippet under this sandbox's timeout and memory constraints.
    pub async fn eval<T>(&self, script: &str) -> VesselResult<T>
    where
        T: for<'js> FromJs<'js> + Send + 'static,
    {
        self.eval_with_timeout(script, self.limits.execution_timeout)
            .await
    }

    /// Evaluates a JavaScript snippet with a caller-supplied per-call timeout override.
    pub async fn eval_with_timeout<T>(&self, script: &str, timeout: Duration) -> VesselResult<T>
    where
        T: for<'js> FromJs<'js> + Send + 'static,
    {
        if self.interrupt.cancelled.load(Ordering::Acquire) {
            return Err(VesselError::Cancelled);
        }

        self.interrupt.arm_timeout(timeout);
        let _guard = ExecutionGuard(&self.interrupt);
        let source = script.to_owned();

        let eval_future = self.context.async_with(async |ctx| {
            ctx.eval::<T, _>(source)
                .catch(&ctx)
                .map_err(|err| err.to_string())
        });

        let outcome = tokio::time::timeout(timeout, eval_future).await;
        match outcome {
            Err(_) => Err(VesselError::Timeout),
            Ok(Ok(value)) => Ok(value),
            Ok(Err(message)) => {
                if self.interrupt.timed_out.load(Ordering::Acquire) {
                    Err(VesselError::Timeout)
                } else if self.interrupt.cancelled.load(Ordering::Acquire) {
                    Err(VesselError::Cancelled)
                } else {
                    Err(VesselError::PluginRuntime {
                        plugin_id: self.plugin_id.to_string(),
                        message,
                    })
                }
            }
        }
    }

    /// Drains remaining pending QuickJS jobs and tears down the context before runtime drop.
    pub async fn shutdown(self) {
        let Self {
            context, runtime, ..
        } = self;
        drop(context);
        runtime.idle().await;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use vessel_domain::{PluginId, VesselError};

    use super::{
        DEFAULT_EXECUTION_TIMEOUT, DEFAULT_MEMORY_LIMIT_BYTES, PluginSandbox, SandboxLimits,
    };
    use crate::strip_typescript;

    fn anthology_id() -> PluginId {
        PluginId::new("anthology").expect("valid plugin id")
    }

    #[tokio::test]
    async fn executes_hello_vessel_and_typescript_output_in_isolated_sandbox() {
        let plugin_id = anthology_id();
        let sandbox = PluginSandbox::with_defaults(plugin_id.clone())
            .await
            .expect("create plugin sandbox");

        assert_eq!(
            sandbox.limits().memory_limit_bytes,
            DEFAULT_MEMORY_LIMIT_BYTES
        );
        assert_eq!(
            sandbox.limits().execution_timeout,
            DEFAULT_EXECUTION_TIMEOUT
        );

        let mem = sandbox.memory_stats().await;
        assert_eq!(mem.malloc_limit, DEFAULT_MEMORY_LIMIT_BYTES);

        let sum: i32 = sandbox.eval("1 + 1").await.expect("eval arithmetic");
        assert_eq!(sum, 2);

        let ts_source = r#"
            const greet = (target: string): string => `Hello ${target}`;
            greet("Vessel");
        "#;
        let compiled_js =
            strip_typescript(&plugin_id, "hello.ts", ts_source).expect("strip hello.ts");
        let greeting: String = sandbox.eval(&compiled_js).await.expect("eval greeting");
        assert_eq!(greeting, "Hello Vessel");

        // Built-in ES globals work without exposing OS/filesystem/process globals.
        let safe_globals: bool = sandbox
            .eval(
                "typeof std === 'undefined' && typeof os === 'undefined' && typeof process === 'undefined'",
            )
            .await
            .expect("check sandbox globals");
        assert!(safe_globals);

        sandbox.shutdown().await;
    }

    #[tokio::test]
    async fn enforces_memory_limit_when_plugin_allocates_excessive_ram() {
        let limits = SandboxLimits {
            memory_limit_bytes: 512 * 1024,
            ..SandboxLimits::default()
        };
        let sandbox = PluginSandbox::new(anthology_id(), limits)
            .await
            .expect("create low-memory sandbox");

        let err = sandbox
            .eval::<usize>(
                r#"
                const chunks = [];
                for (let i = 0; i < 64; i++) {
                    chunks.push("x".repeat(65536));
                }
                chunks.length;
                "#,
            )
            .await
            .expect_err("reject allocation exceeding 512 KiB limit");

        assert!(
            matches!(err, VesselError::PluginRuntime { ref plugin_id, .. } if plugin_id == "anthology"),
            "unexpected error: {err:?}"
        );

        sandbox.run_gc().await;
        sandbox.shutdown().await;
    }

    #[tokio::test]
    async fn interrupts_infinite_loop_on_execution_timeout() {
        let limits = SandboxLimits {
            execution_timeout: Duration::from_millis(40),
            ..SandboxLimits::default()
        };
        let sandbox = PluginSandbox::new(anthology_id(), limits)
            .await
            .expect("create short-timeout sandbox");

        let err = sandbox
            .eval::<()>("while (true) {}")
            .await
            .expect_err("interrupt infinite loop");
        assert_eq!(err, VesselError::Timeout);
        assert_eq!(err.to_string(), "[VesselTimeoutError]: request timed out");

        // Runtime remains usable cleanly after a timeout interrupt.
        let recovered: i32 = sandbox.eval("40 + 2").await.expect("recover after timeout");
        assert_eq!(recovered, 42);

        sandbox.shutdown().await;
    }

    #[tokio::test]
    async fn honours_explicit_cancellation_and_reset() {
        let sandbox = PluginSandbox::with_defaults(anthology_id())
            .await
            .expect("create sandbox");

        sandbox.cancel();
        let err = sandbox
            .eval::<i32>("1 + 1")
            .await
            .expect_err("cancel before eval");
        assert_eq!(err, VesselError::Cancelled);

        sandbox.reset_cancel();
        let value: String = sandbox
            .eval("'anthology-ok'")
            .await
            .expect("eval after reset_cancel");
        assert_eq!(value, "anthology-ok");

        sandbox.shutdown().await;
    }
}
