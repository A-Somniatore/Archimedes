//! Tokio runtime management for FFI
//!
//! Provides a managed Tokio runtime that can be safely used from FFI calls.

use std::sync::OnceLock;
use tokio::runtime::Runtime;

/// Global runtime instance
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Initialize or get the global Tokio runtime
///
/// This creates a multi-threaded runtime suitable for handling concurrent requests.
/// The runtime is lazily initialized on first use.
pub(crate) fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("archimedes-ffi")
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

/// Run a future on the runtime, blocking the current thread
///
/// This is used for FFI functions that need to execute async code.
pub(crate) fn block_on<F: std::future::Future>(future: F) -> F::Output {
    get_runtime().block_on(future)
}

/// Spawn a future on the runtime without blocking
///
/// Returns a handle that can be used to await the result later.
pub(crate) fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    get_runtime().spawn(future)
}

/// Runtime configuration options
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Number of worker threads (0 = auto-detect)
    pub worker_threads: usize,
    /// Stack size for worker threads in bytes
    pub thread_stack_size: usize,
    /// Thread name prefix
    pub thread_name: String,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            worker_threads: 0, // Auto-detect
            thread_stack_size: 2 * 1024 * 1024, // 2MB
            thread_name: "archimedes-ffi".to_string(),
        }
    }
}

/// Build a custom runtime with the given configuration
///
/// Note: This creates a new runtime, not the global one.
pub fn build_runtime(config: &RuntimeConfig) -> std::io::Result<Runtime> {
    let mut builder = tokio::runtime::Builder::new_multi_thread();
    builder
        .enable_all()
        .thread_name(&config.thread_name)
        .thread_stack_size(config.thread_stack_size);

    if config.worker_threads > 0 {
        builder.worker_threads(config.worker_threads);
    }

    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_runtime() {
        let rt1 = get_runtime();
        let rt2 = get_runtime();
        // Same runtime instance
        assert!(std::ptr::eq(rt1, rt2));
    }

    #[test]
    fn test_block_on() {
        let result = block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_block_on_async_operation() {
        let result = block_on(async {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            "completed"
        });
        assert_eq!(result, "completed");
    }

    #[test]
    fn test_spawn_and_await() {
        block_on(async {
            let handle = spawn(async { 100 });
            let result = handle.await.unwrap();
            assert_eq!(result, 100);
        });
    }

    #[test]
    fn test_build_custom_runtime() {
        let config = RuntimeConfig {
            worker_threads: 2,
            thread_stack_size: 1024 * 1024,
            thread_name: "test-runtime".to_string(),
        };

        let rt = build_runtime(&config).unwrap();
        let result = rt.block_on(async { "custom runtime" });
        assert_eq!(result, "custom runtime");
    }

    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default();
        assert_eq!(config.worker_threads, 0);
        assert_eq!(config.thread_stack_size, 2 * 1024 * 1024);
        assert_eq!(config.thread_name, "archimedes-ffi");
    }
}
