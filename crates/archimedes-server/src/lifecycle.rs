//! Lifecycle hooks for server startup and shutdown.
//!
//! This module provides callback registration for server lifecycle events,
//! similar to `FastAPI`'s lifespan events and Axum's `on_shutdown`.
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_server::{Server, Lifecycle};
//!
//! let lifecycle = Lifecycle::new()
//!     .on_startup(|container| async move {
//!         let db = Database::connect("postgres://...").await?;
//!         container.register(db);
//!         Ok(())
//!     })
//!     .on_shutdown(|container| async move {
//!         if let Some(db) = container.get::<Database>() {
//!             db.close().await;
//!         }
//!         Ok(())
//!     });
//!
//! let server = Server::builder()
//!     .lifecycle(lifecycle)
//!     .build();
//! ```
//!
//! # Execution Order
//!
//! - **Startup hooks**: Run in registration order before server starts accepting connections
//! - **Shutdown hooks**: Run in reverse registration order after server stops accepting connections

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use archimedes_core::di::Container;
use thiserror::Error;

/// Error type for lifecycle hook failures.
#[derive(Error, Debug)]
pub enum LifecycleError {
    /// A startup hook failed.
    #[error("Startup hook failed: {0}")]
    StartupFailed(String),

    /// A shutdown hook failed.
    #[error("Shutdown hook failed: {0}")]
    ShutdownFailed(String),

    /// Generic hook error with source.
    #[error("Lifecycle hook error: {message}")]
    HookError {
        /// Error message
        message: String,
        /// Optional source error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl LifecycleError {
    /// Creates a new hook error with a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self::HookError {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new hook error with a source.
    pub fn with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::HookError {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

/// Result type for lifecycle hooks.
pub type LifecycleResult<T = ()> = Result<T, LifecycleError>;

/// A lifecycle hook callback.
///
/// Takes a mutable reference to the DI container and returns a future
/// that resolves to a result.
pub type LifecycleHook = Arc<
    dyn Fn(&mut Container) -> Pin<Box<dyn Future<Output = LifecycleResult> + Send + '_>>
        + Send
        + Sync,
>;

/// Lifecycle manager for server startup and shutdown hooks.
///
/// Provides a fluent API for registering callbacks that run during
/// server lifecycle events.
///
/// # Example
///
/// ```rust
/// use archimedes_server::Lifecycle;
///
/// let lifecycle = Lifecycle::new()
///     .on_startup(|_container| async { Ok(()) })
///     .on_shutdown(|_container| async { Ok(()) });
/// ```
#[must_use]
pub struct Lifecycle {
    /// Startup hooks (run in order)
    startup_hooks: Vec<(String, LifecycleHook)>,
    /// Shutdown hooks (run in reverse order)
    shutdown_hooks: Vec<(String, LifecycleHook)>,
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Lifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Lifecycle")
            .field("startup_hooks", &self.startup_hooks.len())
            .field("shutdown_hooks", &self.shutdown_hooks.len())
            .finish()
    }
}

impl Lifecycle {
    /// Creates a new empty lifecycle manager.
    pub fn new() -> Self {
        Self {
            startup_hooks: Vec::new(),
            shutdown_hooks: Vec::new(),
        }
    }

    /// Registers a startup hook.
    ///
    /// Startup hooks run before the server starts accepting connections.
    /// They run in registration order.
    ///
    /// # Arguments
    ///
    /// * `hook` - An async function that takes `&mut Container` and returns `LifecycleResult`
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::Lifecycle;
    ///
    /// let lifecycle = Lifecycle::new()
    ///     .on_startup(|container| async move {
    ///         println!("Server starting...");
    ///         Ok(())
    ///     });
    /// ```
    pub fn on_startup<F, Fut>(self, hook: F) -> Self
    where
        F: Fn(&mut Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = LifecycleResult> + Send + 'static,
    {
        let name = format!("startup_{}", self.startup_hooks.len());
        self.on_startup_named(name, hook)
    }

    /// Registers a named startup hook.
    ///
    /// Like `on_startup` but with a custom name for logging/debugging.
    pub fn on_startup_named<F, Fut>(mut self, name: impl Into<String>, hook: F) -> Self
    where
        F: Fn(&mut Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = LifecycleResult> + Send + 'static,
    {
        let hook: LifecycleHook = Arc::new(move |container| Box::pin(hook(container)));
        self.startup_hooks.push((name.into(), hook));
        self
    }

    /// Registers a shutdown hook.
    ///
    /// Shutdown hooks run after the server stops accepting connections.
    /// They run in reverse registration order (LIFO).
    ///
    /// # Arguments
    ///
    /// * `hook` - An async function that takes `&mut Container` and returns `LifecycleResult`
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::Lifecycle;
    ///
    /// let lifecycle = Lifecycle::new()
    ///     .on_shutdown(|container| async move {
    ///         println!("Server shutting down...");
    ///         Ok(())
    ///     });
    /// ```
    pub fn on_shutdown<F, Fut>(self, hook: F) -> Self
    where
        F: Fn(&mut Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = LifecycleResult> + Send + 'static,
    {
        let name = format!("shutdown_{}", self.shutdown_hooks.len());
        self.on_shutdown_named(name, hook)
    }

    /// Registers a named shutdown hook.
    ///
    /// Like `on_shutdown` but with a custom name for logging/debugging.
    pub fn on_shutdown_named<F, Fut>(mut self, name: impl Into<String>, hook: F) -> Self
    where
        F: Fn(&mut Container) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = LifecycleResult> + Send + 'static,
    {
        let hook: LifecycleHook = Arc::new(move |container| Box::pin(hook(container)));
        self.shutdown_hooks.push((name.into(), hook));
        self
    }

    /// Returns the number of startup hooks.
    pub fn startup_hook_count(&self) -> usize {
        self.startup_hooks.len()
    }

    /// Returns the number of shutdown hooks.
    pub fn shutdown_hook_count(&self) -> usize {
        self.shutdown_hooks.len()
    }

    /// Runs all startup hooks in registration order.
    ///
    /// If any hook fails, execution stops and the error is returned.
    ///
    /// # Arguments
    ///
    /// * `container` - The DI container to pass to hooks
    ///
    /// # Errors
    ///
    /// Returns `LifecycleError::StartupFailed` if any hook fails.
    pub async fn run_startup(&self, container: &mut Container) -> LifecycleResult {
        for (name, hook) in &self.startup_hooks {
            tracing::debug!(hook = %name, "Running startup hook");
            match hook(container).await {
                Ok(()) => {
                    tracing::debug!(hook = %name, "Startup hook completed");
                }
                Err(e) => {
                    tracing::error!(hook = %name, error = %e, "Startup hook failed");
                    return Err(LifecycleError::StartupFailed(format!(
                        "Hook '{}' failed: {}",
                        name, e
                    )));
                }
            }
        }
        Ok(())
    }

    /// Runs all shutdown hooks in reverse registration order.
    ///
    /// Unlike startup, shutdown continues even if hooks fail.
    /// All errors are collected and returned.
    ///
    /// # Arguments
    ///
    /// * `container` - The DI container to pass to hooks
    ///
    /// # Errors
    ///
    /// Returns `LifecycleError::ShutdownFailed` if any hooks failed,
    /// with a summary of all failures.
    pub async fn run_shutdown(&self, container: &mut Container) -> LifecycleResult {
        let mut errors: Vec<String> = Vec::new();

        // Run in reverse order (LIFO)
        for (name, hook) in self.shutdown_hooks.iter().rev() {
            tracing::debug!(hook = %name, "Running shutdown hook");
            match hook(container).await {
                Ok(()) => {
                    tracing::debug!(hook = %name, "Shutdown hook completed");
                }
                Err(e) => {
                    tracing::error!(hook = %name, error = %e, "Shutdown hook failed");
                    errors.push(format!("{}: {}", name, e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(LifecycleError::ShutdownFailed(errors.join("; ")))
        }
    }

    /// Merges another lifecycle into this one.
    ///
    /// The other lifecycle's hooks are appended to this one's.
    pub fn merge(mut self, other: Lifecycle) -> Self {
        self.startup_hooks.extend(other.startup_hooks);
        self.shutdown_hooks.extend(other.shutdown_hooks);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_empty_lifecycle() {
        let lifecycle = Lifecycle::new();
        let mut container = Container::new();

        assert!(lifecycle.run_startup(&mut container).await.is_ok());
        assert!(lifecycle.run_shutdown(&mut container).await.is_ok());
    }

    #[tokio::test]
    async fn test_startup_hook() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let lifecycle = Lifecycle::new().on_startup(move |_container| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        let mut container = Container::new();
        lifecycle.run_startup(&mut container).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_shutdown_hook() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let lifecycle = Lifecycle::new().on_shutdown(move |_container| {
            let counter = Arc::clone(&counter_clone);
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        let mut container = Container::new();
        lifecycle.run_shutdown(&mut container).await.unwrap();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_startup_order() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let order1 = Arc::clone(&order);
        let order2 = Arc::clone(&order);
        let order3 = Arc::clone(&order);

        let lifecycle = Lifecycle::new()
            .on_startup(move |_| {
                let order = Arc::clone(&order1);
                async move {
                    order.lock().unwrap().push(1);
                    Ok(())
                }
            })
            .on_startup(move |_| {
                let order = Arc::clone(&order2);
                async move {
                    order.lock().unwrap().push(2);
                    Ok(())
                }
            })
            .on_startup(move |_| {
                let order = Arc::clone(&order3);
                async move {
                    order.lock().unwrap().push(3);
                    Ok(())
                }
            });

        let mut container = Container::new();
        lifecycle.run_startup(&mut container).await.unwrap();

        assert_eq!(*order.lock().unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_shutdown_reverse_order() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let order1 = Arc::clone(&order);
        let order2 = Arc::clone(&order);
        let order3 = Arc::clone(&order);

        let lifecycle = Lifecycle::new()
            .on_shutdown(move |_| {
                let order = Arc::clone(&order1);
                async move {
                    order.lock().unwrap().push(1);
                    Ok(())
                }
            })
            .on_shutdown(move |_| {
                let order = Arc::clone(&order2);
                async move {
                    order.lock().unwrap().push(2);
                    Ok(())
                }
            })
            .on_shutdown(move |_| {
                let order = Arc::clone(&order3);
                async move {
                    order.lock().unwrap().push(3);
                    Ok(())
                }
            });

        let mut container = Container::new();
        lifecycle.run_shutdown(&mut container).await.unwrap();

        // Shutdown runs in reverse order (LIFO)
        assert_eq!(*order.lock().unwrap(), vec![3, 2, 1]);
    }

    #[tokio::test]
    async fn test_startup_stops_on_failure() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let order1 = Arc::clone(&order);
        let order2 = Arc::clone(&order);

        let lifecycle = Lifecycle::new()
            .on_startup(move |_| {
                let order = Arc::clone(&order1);
                async move {
                    order.lock().unwrap().push(1);
                    Ok(())
                }
            })
            .on_startup(move |_| async move { Err(LifecycleError::new("test error")) })
            .on_startup(move |_| {
                let order = Arc::clone(&order2);
                async move {
                    order.lock().unwrap().push(3);
                    Ok(())
                }
            });

        let mut container = Container::new();
        let result = lifecycle.run_startup(&mut container).await;

        assert!(result.is_err());
        // Third hook should not have run
        assert_eq!(*order.lock().unwrap(), vec![1]);
    }

    #[tokio::test]
    async fn test_shutdown_continues_on_failure() {
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let order1 = Arc::clone(&order);
        let order3 = Arc::clone(&order);

        let lifecycle = Lifecycle::new()
            .on_shutdown(move |_| {
                let order = Arc::clone(&order1);
                async move {
                    order.lock().unwrap().push(1);
                    Ok(())
                }
            })
            .on_shutdown(move |_| async move { Err(LifecycleError::new("test error")) })
            .on_shutdown(move |_| {
                let order = Arc::clone(&order3);
                async move {
                    order.lock().unwrap().push(3);
                    Ok(())
                }
            });

        let mut container = Container::new();
        let result = lifecycle.run_shutdown(&mut container).await;

        assert!(result.is_err());
        // All hooks should have run (in reverse order)
        assert_eq!(*order.lock().unwrap(), vec![3, 1]);
    }

    #[tokio::test]
    async fn test_named_hooks() {
        let lifecycle = Lifecycle::new()
            .on_startup_named("database_init", |_| async { Ok(()) })
            .on_shutdown_named("database_close", |_| async { Ok(()) });

        assert_eq!(lifecycle.startup_hook_count(), 1);
        assert_eq!(lifecycle.shutdown_hook_count(), 1);
    }

    #[tokio::test]
    async fn test_merge_lifecycles() {
        let lifecycle1 = Lifecycle::new()
            .on_startup(|_| async { Ok(()) })
            .on_shutdown(|_| async { Ok(()) });

        let lifecycle2 = Lifecycle::new()
            .on_startup(|_| async { Ok(()) })
            .on_shutdown(|_| async { Ok(()) });

        let merged = lifecycle1.merge(lifecycle2);

        assert_eq!(merged.startup_hook_count(), 2);
        assert_eq!(merged.shutdown_hook_count(), 2);
    }

    #[tokio::test]
    async fn test_error_message() {
        let err = LifecycleError::new("test message");
        assert!(err.to_string().contains("test message"));

        let err = LifecycleError::StartupFailed("startup failed".into());
        assert!(err.to_string().contains("startup failed"));

        let err = LifecycleError::ShutdownFailed("shutdown failed".into());
        assert!(err.to_string().contains("shutdown failed"));
    }

    #[tokio::test]
    async fn test_lifecycle_debug() {
        let lifecycle = Lifecycle::new()
            .on_startup(|_| async { Ok(()) })
            .on_shutdown(|_| async { Ok(()) });

        let debug = format!("{:?}", lifecycle);
        assert!(debug.contains("Lifecycle"));
        assert!(debug.contains("startup_hooks"));
        assert!(debug.contains("shutdown_hooks"));
    }
}
