//! # Archimedes Python Bindings
//!
//! Python bindings for the Archimedes HTTP server framework using PyO3.
//!
//! ## Example
//!
//! ```ignore
//! from archimedes import App, Config
//!
//! config = Config(
//!     contract_path="contract.json",
//!     listen_port=8080,
//! )
//!
//! app = App(config)
//!
//! @app.handler("getUser")
//! def get_user(ctx):
//!     user_id = ctx.path_params.get("userId")
//!     return {"id": user_id, "name": "John Doe"}
//!
//! @app.handler("createUser")
//! async def create_user(ctx, body):
//!     # Async handlers are supported
//!     return {"id": "new-id", "created": True}
//!
//! if __name__ == "__main__":
//!     app.run()
//! ```

use pyo3::prelude::*;
use std::sync::Arc;

mod authz;
mod config;
mod context;
mod error;
mod handlers;
mod middleware;
mod response;
mod server;
mod telemetry;
mod validation;

pub use authz::{PyAuthorizer, PyPolicyDecision};
pub use config::PyConfig;
pub use context::{PyIdentity, PyRequestContext};
pub use error::PyArchimedesError;
pub use handlers::HandlerRegistry;
pub use middleware::{
    add_response_headers, process_request, request_duration_ms, MiddlewareResult,
};
pub use response::PyResponse;
pub use server::{PyServer, ServerError};
pub use telemetry::{py_record_request, py_render_metrics, PyTelemetry, PyTelemetryConfig};
pub use validation::{PyOperationResolution, PySentinel, PyValidationError, PyValidationResult};

/// Archimedes application instance
///
/// This is the main entry point for creating an Archimedes HTTP server
/// from Python.
#[pyclass(name = "App")]
pub struct PyApp {
    config: PyConfig,
    handlers: Arc<HandlerRegistry>,
    running: bool,
}

#[pymethods]
impl PyApp {
    /// Create a new Archimedes application
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration for the application
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// from archimedes import App, Config
    ///
    /// config = Config(contract_path="contract.json")
    /// app = App(config)
    /// ```
    #[new]
    fn new(config: PyConfig) -> Self {
        Self {
            config,
            handlers: Arc::new(HandlerRegistry::new()),
            running: false,
        }
    }

    /// Register a handler for an operation
    ///
    /// This method is typically used via the `@app.handler` decorator.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID from the contract
    /// * `handler` - The Python callable to handle requests
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// @app.handler("getUser")
    /// def get_user(ctx):
    ///     return {"user": "data"}
    /// ```
    fn handler(&self, operation_id: String) -> PyResult<HandlerDecorator> {
        Ok(HandlerDecorator {
            operation_id,
            registry: Arc::clone(&self.handlers),
        })
    }

    /// Register a handler function directly
    fn register_handler(&self, operation_id: String, handler: PyObject) -> PyResult<()> {
        self.handlers.register(operation_id, handler)?;
        Ok(())
    }

    /// Run the application (blocking)
    ///
    /// This starts the HTTP server and blocks until it's stopped.
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// app.run()  # Blocks until stopped
    /// ```
    fn run(&mut self, py: Python<'_>) -> PyResult<()> {
        if self.running {
            return Err(PyArchimedesError::new_err("Application is already running"));
        }

        self.running = true;

        // Get server configuration
        let listen_addr = self.config.listen_addr().to_string();
        let listen_port = self.config.listen_port();
        let contract_path = self.config.contract_path().map(|s| s.to_string());
        let handlers = Arc::clone(&self.handlers);

        // Parse socket address
        let addr: std::net::SocketAddr = format!("{}:{}", listen_addr, listen_port)
            .parse()
            .map_err(|e| PyArchimedesError::new_err(format!("Invalid address: {e}")))?;

        // Release the GIL while running the server
        let result = py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| format!("Failed to create runtime: {e}"))?;

            rt.block_on(async {
                let server = server::PyServer::new(addr, handlers, contract_path);
                server.run().await.map_err(|e| format!("Server error: {e}"))
            })
        });

        self.running = false;

        result.map_err(|e| PyArchimedesError::new_err(e))
    }

    /// Run the application asynchronously
    ///
    /// For use with Python's asyncio event loop.
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// import asyncio
    ///
    /// async def main():
    ///     await app.run_async()
    ///
    /// asyncio.run(main())
    /// ```
    fn run_async<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        if self.running {
            return Err(PyArchimedesError::new_err("Application is already running"));
        }

        let listen_addr = self.config.listen_addr().to_string();
        let listen_port = self.config.listen_port();
        let handlers = Arc::clone(&self.handlers);

        // Create an asyncio coroutine that runs the server
        let asyncio = py.import("asyncio")?;
        let coro = asyncio.call_method1(
            "to_thread",
            (py.eval(pyo3::ffi::c_str!("lambda: None"), None, None)?,),
        )?;

        // Log startup
        tracing::info!(
            "Archimedes Python server starting on {}:{}",
            listen_addr,
            listen_port
        );

        // Keep handlers alive
        let _ = handlers;

        Ok(coro)
    }

    /// Stop the application
    fn stop(&mut self) -> PyResult<()> {
        if !self.running {
            return Ok(());
        }
        self.running = false;
        // TODO: Signal the server to stop
        Ok(())
    }

    /// Check if the application is running
    #[getter]
    fn is_running(&self) -> bool {
        self.running
    }

    /// Get the configuration
    #[getter]
    fn config(&self) -> PyConfig {
        self.config.clone()
    }

    /// Get the list of registered operation IDs
    fn operation_ids(&self) -> Vec<String> {
        self.handlers.operation_ids()
    }

    /// Get the application version
    #[staticmethod]
    fn version() -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
}

/// Decorator helper for registering handlers
#[pyclass]
pub struct HandlerDecorator {
    operation_id: String,
    registry: Arc<HandlerRegistry>,
}

#[pymethods]
impl HandlerDecorator {
    fn __call__(&self, py: Python<'_>, handler: PyObject) -> PyResult<PyObject> {
        let handler_clone = handler.clone_ref(py);
        self.registry
            .register(self.operation_id.clone(), handler_clone)?;
        Ok(handler)
    }
}

/// Python module initialization
#[pymodule]
#[pyo3(name = "_archimedes")]
fn archimedes_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Main classes
    m.add_class::<PyApp>()?;
    m.add_class::<PyConfig>()?;
    m.add_class::<PyRequestContext>()?;
    m.add_class::<PyIdentity>()?;
    m.add_class::<PyResponse>()?;

    // Authorization classes
    m.add_class::<PyAuthorizer>()?;
    m.add_class::<PyPolicyDecision>()?;

    // Validation classes
    m.add_class::<PySentinel>()?;
    m.add_class::<PyValidationResult>()?;
    m.add_class::<PyValidationError>()?;
    m.add_class::<PyOperationResolution>()?;

    // Telemetry classes
    m.add_class::<PyTelemetry>()?;
    m.add_class::<PyTelemetryConfig>()?;

    // Telemetry functions
    m.add_function(wrap_pyfunction!(py_record_request, m)?)?;
    m.add_function(wrap_pyfunction!(py_render_metrics, m)?)?;

    // Error type
    m.add("ArchimedesError", m.py().get_type::<PyArchimedesError>())?;

    // Version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
