//! Lifecycle hooks for Python bindings
//!
//! Provides startup and shutdown hook registration similar to FastAPI:
//!
//! ```python,ignore
//! from archimedes import App
//!
//! app = App(config)
//!
//! @app.on_startup
//! async def startup():
//!     print("Server starting...")
//!     await connect_database()
//!
//! @app.on_shutdown
//! async def shutdown():
//!     print("Server shutting down...")
//!     await close_database()
//!
//! # Named hooks for better logging
//! @app.on_startup("database_connect")
//! async def connect_db():
//!     pass
//!
//! @app.on_shutdown("cache_flush")
//! async def flush_cache():
//!     pass
//! ```

use std::sync::Arc;

use pyo3::prelude::*;
use tokio::sync::RwLock;

use crate::error::PyArchimedesError;

/// A registered lifecycle hook
pub struct LifecycleHookEntry {
    /// Hook name for logging
    pub name: String,
    /// The Python callable
    pub handler: PyObject,
    /// Whether the handler is async
    pub is_async: bool,
}

/// Lifecycle manager for Python applications
///
/// Manages startup and shutdown hooks with async support.
pub struct PyLifecycle {
    /// Startup hooks (run in order)
    startup_hooks: Vec<LifecycleHookEntry>,
    /// Shutdown hooks (run in reverse order)
    shutdown_hooks: Vec<LifecycleHookEntry>,
}

impl PyLifecycle {
    /// Create a new empty lifecycle manager
    pub fn new() -> Self {
        Self {
            startup_hooks: Vec::new(),
            shutdown_hooks: Vec::new(),
        }
    }

    /// Register a startup hook
    pub fn add_startup_hook(&mut self, name: String, handler: PyObject, is_async: bool) {
        self.startup_hooks.push(LifecycleHookEntry {
            name,
            handler,
            is_async,
        });
    }

    /// Register a shutdown hook
    pub fn add_shutdown_hook(&mut self, name: String, handler: PyObject, is_async: bool) {
        self.shutdown_hooks.push(LifecycleHookEntry {
            name,
            handler,
            is_async,
        });
    }

    /// Get startup hooks
    pub fn startup_hooks(&self) -> &[LifecycleHookEntry] {
        &self.startup_hooks
    }

    /// Get shutdown hooks
    pub fn shutdown_hooks(&self) -> &[LifecycleHookEntry] {
        &self.shutdown_hooks
    }

    /// Number of startup hooks
    pub fn startup_count(&self) -> usize {
        self.startup_hooks.len()
    }

    /// Number of shutdown hooks
    pub fn shutdown_count(&self) -> usize {
        self.shutdown_hooks.len()
    }

    /// Run all startup hooks
    ///
    /// Hooks run in registration order. If any hook fails, remaining hooks
    /// are not executed.
    pub fn run_startup_hooks(&self, py: Python<'_>) -> PyResult<()> {
        for hook in &self.startup_hooks {
            tracing::info!("Running startup hook: {}", hook.name);

            let result = if hook.is_async {
                // Run async hook using asyncio
                run_async_hook(py, &hook.handler)
            } else {
                // Run sync hook directly
                hook.handler.call0(py).map(|_| ())
            };

            if let Err(e) = result {
                tracing::error!("Startup hook '{}' failed: {:?}", hook.name, e);
                return Err(PyArchimedesError::new_err(format!(
                    "Startup hook '{}' failed: {}",
                    hook.name, e
                )));
            }

            tracing::info!("Startup hook '{}' completed successfully", hook.name);
        }

        Ok(())
    }

    /// Run all shutdown hooks
    ///
    /// Hooks run in reverse registration order (LIFO). All hooks are executed
    /// even if some fail; errors are collected and returned.
    pub fn run_shutdown_hooks(&self, py: Python<'_>) -> PyResult<()> {
        let mut errors = Vec::new();

        // Run in reverse order (LIFO)
        for hook in self.shutdown_hooks.iter().rev() {
            tracing::info!("Running shutdown hook: {}", hook.name);

            let result = if hook.is_async {
                run_async_hook(py, &hook.handler)
            } else {
                hook.handler.call0(py).map(|_| ())
            };

            if let Err(e) = result {
                tracing::error!("Shutdown hook '{}' failed: {:?}", hook.name, e);
                errors.push(format!("{}: {}", hook.name, e));
            } else {
                tracing::info!("Shutdown hook '{}' completed successfully", hook.name);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(PyArchimedesError::new_err(format!(
                "Shutdown hooks failed: {}",
                errors.join("; ")
            )))
        }
    }
}

impl Default for PyLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

/// Run an async Python hook using asyncio.run()
fn run_async_hook(py: Python<'_>, handler: &PyObject) -> PyResult<()> {
    let asyncio = py.import("asyncio")?;

    // Call the async function to get the coroutine
    let coro = handler.call0(py)?;

    // Run the coroutine with asyncio.run()
    asyncio.call_method1("run", (coro,))?;

    Ok(())
}

/// Check if a Python callable is async (coroutine function)
pub fn is_async_callable(py: Python<'_>, obj: &PyObject) -> bool {
    let inspect = match py.import("inspect") {
        Ok(m) => m,
        Err(_) => return false,
    };

    match inspect.call_method1("iscoroutinefunction", (obj,)) {
        Ok(result) => result.extract::<bool>().unwrap_or(false),
        Err(_) => false,
    }
}

/// Decorator for startup hooks
#[pyclass(name = "StartupDecorator")]
pub struct StartupDecorator {
    /// Hook name (optional)
    name: Option<String>,
    /// Reference to the lifecycle manager
    lifecycle: Arc<RwLock<PyLifecycle>>,
}

impl StartupDecorator {
    /// Create a new startup decorator
    pub fn new(name: Option<String>, lifecycle: Arc<RwLock<PyLifecycle>>) -> Self {
        Self { name, lifecycle }
    }
}

#[pymethods]
impl StartupDecorator {
    fn __call__(&self, py: Python<'_>, handler: PyObject) -> PyResult<PyObject> {
        // Determine hook name
        let name = self.name.clone().unwrap_or_else(|| {
            handler
                .getattr(py, "__name__")
                .and_then(|n| n.extract::<String>(py))
                .unwrap_or_else(|_| "anonymous_startup".to_string())
        });

        // Check if async
        let is_async = is_async_callable(py, &handler);

        // Register the hook
        // We need to use try_write() since we're in sync context
        // In a real app, you'd handle this differently
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => {
                let lifecycle = Arc::clone(&self.lifecycle);
                let handler_clone = handler.clone_ref(py);
                handle.block_on(async move {
                    let mut lc = lifecycle.write().await;
                    lc.add_startup_hook(name, handler_clone, is_async);
                });
            }
            Err(_) => {
                // No tokio runtime, use blocking approach
                let rt = tokio::runtime::Runtime::new().map_err(|e| {
                    PyArchimedesError::new_err(format!("Failed to create runtime: {e}"))
                })?;
                let lifecycle = Arc::clone(&self.lifecycle);
                let handler_clone = handler.clone_ref(py);
                rt.block_on(async move {
                    let mut lc = lifecycle.write().await;
                    lc.add_startup_hook(name, handler_clone, is_async);
                });
            }
        }

        Ok(handler)
    }
}

/// Decorator for shutdown hooks
#[pyclass(name = "ShutdownDecorator")]
pub struct ShutdownDecorator {
    /// Hook name (optional)
    name: Option<String>,
    /// Reference to the lifecycle manager
    lifecycle: Arc<RwLock<PyLifecycle>>,
}

impl ShutdownDecorator {
    /// Create a new shutdown decorator
    pub fn new(name: Option<String>, lifecycle: Arc<RwLock<PyLifecycle>>) -> Self {
        Self { name, lifecycle }
    }
}

#[pymethods]
impl ShutdownDecorator {
    fn __call__(&self, py: Python<'_>, handler: PyObject) -> PyResult<PyObject> {
        // Determine hook name
        let name = self.name.clone().unwrap_or_else(|| {
            handler
                .getattr(py, "__name__")
                .and_then(|n| n.extract::<String>(py))
                .unwrap_or_else(|_| "anonymous_shutdown".to_string())
        });

        // Check if async
        let is_async = is_async_callable(py, &handler);

        // Register the hook
        let rt = tokio::runtime::Handle::try_current();
        match rt {
            Ok(handle) => {
                let lifecycle = Arc::clone(&self.lifecycle);
                let handler_clone = handler.clone_ref(py);
                handle.block_on(async move {
                    let mut lc = lifecycle.write().await;
                    lc.add_shutdown_hook(name, handler_clone, is_async);
                });
            }
            Err(_) => {
                let rt = tokio::runtime::Runtime::new().map_err(|e| {
                    PyArchimedesError::new_err(format!("Failed to create runtime: {e}"))
                })?;
                let lifecycle = Arc::clone(&self.lifecycle);
                let handler_clone = handler.clone_ref(py);
                rt.block_on(async move {
                    let mut lc = lifecycle.write().await;
                    lc.add_shutdown_hook(name, handler_clone, is_async);
                });
            }
        }

        Ok(handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_new() {
        let lifecycle = PyLifecycle::new();
        assert_eq!(lifecycle.startup_count(), 0);
        assert_eq!(lifecycle.shutdown_count(), 0);
    }

    #[test]
    fn test_lifecycle_add_startup_hook() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut lifecycle = PyLifecycle::new();
            let handler = py.None().into_pyobject(py).unwrap().unbind();

            lifecycle.add_startup_hook("test_hook".to_string(), handler, false);

            assert_eq!(lifecycle.startup_count(), 1);
            assert_eq!(lifecycle.startup_hooks()[0].name, "test_hook");
            assert!(!lifecycle.startup_hooks()[0].is_async);
        });
    }

    #[test]
    fn test_lifecycle_add_shutdown_hook() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut lifecycle = PyLifecycle::new();
            let handler = py.None().into_pyobject(py).unwrap().unbind();

            lifecycle.add_shutdown_hook("cleanup".to_string(), handler, true);

            assert_eq!(lifecycle.shutdown_count(), 1);
            assert_eq!(lifecycle.shutdown_hooks()[0].name, "cleanup");
            assert!(lifecycle.shutdown_hooks()[0].is_async);
        });
    }

    #[test]
    fn test_lifecycle_hook_order() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut lifecycle = PyLifecycle::new();

            lifecycle.add_startup_hook(
                "first".to_string(),
                py.None().into_pyobject(py).unwrap().unbind(),
                false,
            );
            lifecycle.add_startup_hook(
                "second".to_string(),
                py.None().into_pyobject(py).unwrap().unbind(),
                false,
            );
            lifecycle.add_startup_hook(
                "third".to_string(),
                py.None().into_pyobject(py).unwrap().unbind(),
                false,
            );

            assert_eq!(lifecycle.startup_hooks()[0].name, "first");
            assert_eq!(lifecycle.startup_hooks()[1].name, "second");
            assert_eq!(lifecycle.startup_hooks()[2].name, "third");
        });
    }
}
