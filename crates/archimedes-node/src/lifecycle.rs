//! Lifecycle hook management for TypeScript bindings.
//!
//! Provides startup and shutdown hooks that run when the server starts
//! and stops respectively.
//!
//! ## Example (TypeScript)
//!
//! ```typescript
//! import { Archimedes, Config } from '@archimedes/node';
//!
//! const app = new Archimedes(config);
//!
//! // Register startup hooks
//! app.onStartup(async () => {
//!   console.log('Connecting to database...');
//!   await db.connect();
//! });
//!
//! app.onStartup(async () => {
//!   console.log('Loading cache...');
//!   await cache.warmup();
//! }, { name: 'cache_warmup' });
//!
//! // Register shutdown hooks
//! app.onShutdown(async () => {
//!   console.log('Closing database connection...');
//!   await db.close();
//! });
//!
//! await app.listen(8080);
//! ```

use napi_derive::napi;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A lifecycle hook entry with optional name.
#[derive(Clone)]
#[allow(dead_code)] // Fields used for future hook invocation
pub struct LifecycleHookEntry {
    /// Optional name for the hook (for debugging)
    pub name: Option<String>,
    /// The hook function placeholder (actual JS function stored separately)
    pub registered: bool,
}

/// Manages application lifecycle hooks.
///
/// Lifecycle hooks are executed in order:
/// - Startup hooks run in registration order when the server starts
/// - Shutdown hooks run in reverse registration order when the server stops
#[napi]
#[derive(Clone)]
pub struct Lifecycle {
    startup_hooks: Arc<RwLock<Vec<LifecycleHookEntry>>>,
    shutdown_hooks: Arc<RwLock<Vec<LifecycleHookEntry>>>,
    startup_names: Arc<RwLock<Vec<String>>>,
    shutdown_names: Arc<RwLock<Vec<String>>>,
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl Lifecycle {
    /// Create a new lifecycle manager.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            startup_hooks: Arc::new(RwLock::new(Vec::new())),
            shutdown_hooks: Arc::new(RwLock::new(Vec::new())),
            startup_names: Arc::new(RwLock::new(Vec::new())),
            shutdown_names: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a startup hook.
    ///
    /// Startup hooks are executed in registration order when the server starts.
    ///
    /// ## Arguments
    ///
    /// * `name` - Optional name for debugging and logging
    ///
    /// ## Returns
    ///
    /// The index of the registered hook (for reference)
    #[napi]
    pub async fn add_startup(&self, name: Option<String>) -> u32 {
        let mut hooks = self.startup_hooks.write().await;
        let mut names = self.startup_names.write().await;

        let index = hooks.len() as u32;
        hooks.push(LifecycleHookEntry {
            name: name.clone(),
            registered: true,
        });

        let display_name = name.unwrap_or_else(|| format!("startup_hook_{}", index));
        names.push(display_name);

        index
    }

    /// Register a shutdown hook.
    ///
    /// Shutdown hooks are executed in reverse registration order (LIFO)
    /// when the server stops.
    ///
    /// ## Arguments
    ///
    /// * `name` - Optional name for debugging and logging
    ///
    /// ## Returns
    ///
    /// The index of the registered hook (for reference)
    #[napi]
    pub async fn add_shutdown(&self, name: Option<String>) -> u32 {
        let mut hooks = self.shutdown_hooks.write().await;
        let mut names = self.shutdown_names.write().await;

        let index = hooks.len() as u32;
        hooks.push(LifecycleHookEntry {
            name: name.clone(),
            registered: true,
        });

        let display_name = name.unwrap_or_else(|| format!("shutdown_hook_{}", index));
        names.push(display_name);

        index
    }

    /// Get the number of registered startup hooks.
    #[napi]
    pub async fn startup_count(&self) -> u32 {
        self.startup_hooks.read().await.len() as u32
    }

    /// Get the number of registered shutdown hooks.
    #[napi]
    pub async fn shutdown_count(&self) -> u32 {
        self.shutdown_hooks.read().await.len() as u32
    }

    /// Get all startup hook names in execution order.
    #[napi]
    pub async fn startup_names(&self) -> Vec<String> {
        self.startup_names.read().await.clone()
    }

    /// Get all shutdown hook names in execution order (reversed).
    #[napi]
    pub async fn shutdown_names(&self) -> Vec<String> {
        let names = self.shutdown_names.read().await;
        names.iter().rev().cloned().collect()
    }

    /// Clear all hooks (for testing).
    #[napi]
    pub async fn clear(&self) {
        self.startup_hooks.write().await.clear();
        self.shutdown_hooks.write().await.clear();
        self.startup_names.write().await.clear();
        self.shutdown_names.write().await.clear();
    }

    /// Check if any startup hooks are registered.
    #[napi]
    pub async fn has_startup_hooks(&self) -> bool {
        !self.startup_hooks.read().await.is_empty()
    }

    /// Check if any shutdown hooks are registered.
    #[napi]
    pub async fn has_shutdown_hooks(&self) -> bool {
        !self.shutdown_hooks.read().await.is_empty()
    }
}

/// Configuration for a lifecycle hook.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct LifecycleHookOptions {
    /// Optional name for the hook
    pub name: Option<String>,
    /// Optional timeout in milliseconds
    pub timeout_ms: Option<u32>,
}

/// Create lifecycle hook options.
#[napi]
pub fn create_lifecycle_hook_options(
    name: Option<String>,
    timeout_ms: Option<u32>,
) -> LifecycleHookOptions {
    LifecycleHookOptions { name, timeout_ms }
}

/// Result of running lifecycle hooks.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct LifecycleResult {
    /// Whether all hooks succeeded
    pub success: bool,
    /// Number of hooks executed
    pub executed: u32,
    /// Names of hooks that failed (if any)
    pub failed: Vec<String>,
    /// Total duration in milliseconds
    pub duration_ms: u32,
}

/// Create a lifecycle result.
#[napi]
pub fn create_lifecycle_result(
    success: bool,
    executed: u32,
    failed: Vec<String>,
    duration_ms: u32,
) -> LifecycleResult {
    LifecycleResult {
        success,
        executed,
        failed,
        duration_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lifecycle_creation() {
        let lifecycle = Lifecycle::new();
        assert_eq!(lifecycle.startup_count().await, 0);
        assert_eq!(lifecycle.shutdown_count().await, 0);
    }

    #[tokio::test]
    async fn test_add_startup_hook() {
        let lifecycle = Lifecycle::new();

        let idx = lifecycle.add_startup(Some("db_connect".to_string())).await;
        assert_eq!(idx, 0);
        assert_eq!(lifecycle.startup_count().await, 1);

        let names = lifecycle.startup_names().await;
        assert!(names.contains(&"db_connect".to_string()));
    }

    #[tokio::test]
    async fn test_add_shutdown_hook() {
        let lifecycle = Lifecycle::new();

        let idx = lifecycle
            .add_shutdown(Some("db_disconnect".to_string()))
            .await;
        assert_eq!(idx, 0);
        assert_eq!(lifecycle.shutdown_count().await, 1);
    }

    #[tokio::test]
    async fn test_default_hook_names() {
        let lifecycle = Lifecycle::new();

        lifecycle.add_startup(None).await;
        lifecycle.add_startup(None).await;

        let names = lifecycle.startup_names().await;
        assert_eq!(names.len(), 2);
        assert!(names[0].contains("startup_hook_0"));
        assert!(names[1].contains("startup_hook_1"));
    }

    #[tokio::test]
    async fn test_shutdown_names_reversed() {
        let lifecycle = Lifecycle::new();

        lifecycle.add_shutdown(Some("first".to_string())).await;
        lifecycle.add_shutdown(Some("second".to_string())).await;
        lifecycle.add_shutdown(Some("third".to_string())).await;

        let names = lifecycle.shutdown_names().await;
        assert_eq!(names, vec!["third", "second", "first"]);
    }

    #[tokio::test]
    async fn test_has_hooks() {
        let lifecycle = Lifecycle::new();

        assert!(!lifecycle.has_startup_hooks().await);
        assert!(!lifecycle.has_shutdown_hooks().await);

        lifecycle.add_startup(None).await;
        assert!(lifecycle.has_startup_hooks().await);

        lifecycle.add_shutdown(None).await;
        assert!(lifecycle.has_shutdown_hooks().await);
    }

    #[tokio::test]
    async fn test_clear_hooks() {
        let lifecycle = Lifecycle::new();

        lifecycle.add_startup(None).await;
        lifecycle.add_startup(None).await;
        lifecycle.add_shutdown(None).await;

        assert_eq!(lifecycle.startup_count().await, 2);
        assert_eq!(lifecycle.shutdown_count().await, 1);

        lifecycle.clear().await;

        assert_eq!(lifecycle.startup_count().await, 0);
        assert_eq!(lifecycle.shutdown_count().await, 0);
    }

    #[test]
    fn test_lifecycle_hook_options() {
        let opts = create_lifecycle_hook_options(Some("test".to_string()), Some(5000));
        assert_eq!(opts.name, Some("test".to_string()));
        assert_eq!(opts.timeout_ms, Some(5000));
    }

    #[test]
    fn test_lifecycle_result() {
        let result = create_lifecycle_result(true, 3, vec![], 150);
        assert!(result.success);
        assert_eq!(result.executed, 3);
        assert!(result.failed.is_empty());
        assert_eq!(result.duration_ms, 150);
    }

    #[test]
    fn test_lifecycle_result_with_failures() {
        let result = create_lifecycle_result(
            false,
            3,
            vec!["hook1".to_string(), "hook2".to_string()],
            500,
        );
        assert!(!result.success);
        assert_eq!(result.failed.len(), 2);
    }
}
