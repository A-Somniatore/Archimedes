//! HTTP server implementation.
//!
//! This module provides the main HTTP server for Archimedes,
//! built on Hyper and Tokio for async I/O.
//!
//! # Architecture
//!
//! The server consists of:
//!
//! - TCP listener bound to configured address
//! - Connection handler for each incoming connection
//! - Request routing via the [`Router`](crate::Router)
//! - Graceful shutdown support
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_server::{Server, ServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ServerConfig::builder()
//!         .http_addr("0.0.0.0:8080")
//!         .build();
//!
//!     let server = Server::new(config);
//!     server.run().await?;
//!     Ok(())
//! }
//! ```

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use archimedes_core::RequestContext;

use crate::config::ServerConfig;
use crate::handler::{HandlerRegistry, InvokeError};
use crate::health::{HealthCheck, ReadinessCheck};
use crate::router::{RouteMatch, Router};
use crate::shutdown::{ConnectionTracker, ShutdownSignal};

/// Type alias for HTTP response body.
pub type ResponseBody = Full<Bytes>;

/// Type alias for the HTTP response.
pub type HttpResponse = Response<ResponseBody>;

/// The Archimedes HTTP server.
///
/// Handles incoming HTTP requests and routes them to handlers.
///
/// # Example
///
/// ```rust,ignore
/// use archimedes_server::{Server, ServerConfig};
///
/// let config = ServerConfig::builder()
///     .http_addr("127.0.0.1:8080")
///     .build();
///
/// let server = Server::new(config);
/// ```
/// The Archimedes HTTP server.
///
/// Handles incoming HTTP requests and routes them to handlers.
///
/// # Example
///
/// ```rust,ignore
/// use archimedes_server::{Server, ServerConfig, HandlerRegistry};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Deserialize)]
/// struct GetUserRequest { user_id: String }
///
/// #[derive(Serialize)]
/// struct User { id: String, name: String }
///
/// async fn get_user(ctx: archimedes_core::RequestContext, req: GetUserRequest) 
///     -> Result<User, archimedes_server::HandlerError> {
///     Ok(User { id: req.user_id, name: "John".into() })
/// }
///
/// let mut registry = HandlerRegistry::new();
/// registry.register("getUser", get_user);
///
/// let server = Server::builder()
///     .http_addr("127.0.0.1:8080")
///     .handlers(registry)
///     .build();
/// ```
pub struct Server {
    /// Server configuration
    config: ServerConfig,

    /// Request router
    router: Router,

    /// Handler registry
    handlers: HandlerRegistry,

    /// Health check handler
    health: HealthCheck,

    /// Readiness check handler
    readiness: ReadinessCheck,

    /// Request timeout
    request_timeout: Duration,
}

impl Server {
    /// Creates a new server with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::{Server, ServerConfig};
    ///
    /// let config = ServerConfig::builder()
    ///     .http_addr("127.0.0.1:3000")
    ///     .build();
    ///
    /// let server = Server::new(config);
    /// ```
    #[must_use]
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            router: Router::new(),
            handlers: HandlerRegistry::new(),
            health: HealthCheck::new("archimedes", env!("CARGO_PKG_VERSION")),
            readiness: ReadinessCheck::new(),
            request_timeout: Duration::from_secs(30),
        }
    }

    /// Creates a new server builder.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::Server;
    ///
    /// let server = Server::builder()
    ///     .http_addr("0.0.0.0:8080")
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder() -> ServerBuilder {
        ServerBuilder::default()
    }

    /// Returns a reference to the router.
    #[must_use]
    pub fn router(&self) -> &Router {
        &self.router
    }

    /// Returns a mutable reference to the router.
    pub fn router_mut(&mut self) -> &mut Router {
        &mut self.router
    }

    /// Returns a reference to the health check handler.
    #[must_use]
    pub fn health(&self) -> &HealthCheck {
        &self.health
    }

    /// Returns a reference to the readiness check handler.
    #[must_use]
    pub fn readiness(&self) -> &ReadinessCheck {
        &self.readiness
    }

    /// Returns a reference to the server configuration.
    #[must_use]
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Returns a reference to the handler registry.
    #[must_use]
    pub fn handlers(&self) -> &HandlerRegistry {
        &self.handlers
    }

    /// Returns a mutable reference to the handler registry.
    pub fn handlers_mut(&mut self) -> &mut HandlerRegistry {
        &mut self.handlers
    }

    /// Returns the request timeout.
    #[must_use]
    pub fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    /// Runs the server until a shutdown signal is received.
    ///
    /// This method binds to the configured address and begins
    /// accepting connections. It handles graceful shutdown
    /// when a SIGTERM or SIGINT signal is received.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The server cannot bind to the configured address
    /// - An I/O error occurs
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use archimedes_server::{Server, ServerConfig};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let server = Server::builder()
    ///         .http_addr("0.0.0.0:8080")
    ///         .build();
    ///
    ///     server.run().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn run(self) -> Result<(), ServerError> {
        let shutdown = ShutdownSignal::with_os_signals();
        self.run_with_shutdown(shutdown).await
    }

    /// Runs the server with a custom shutdown signal.
    ///
    /// This is useful for testing or when you want to control
    /// shutdown programmatically.
    ///
    /// # Arguments
    ///
    /// * `shutdown` - The shutdown signal to listen for
    ///
    /// # Errors
    ///
    /// Returns an error if the server cannot bind or an I/O error occurs.
    pub async fn run_with_shutdown(self, shutdown: ShutdownSignal) -> Result<(), ServerError> {
        let addr = self.config.socket_addr().map_err(|e| {
            ServerError::BindError(format!("Invalid address '{}': {}", self.config.http_addr(), e))
        })?;

        let listener = TcpListener::bind(addr).await.map_err(|e| {
            ServerError::BindError(format!("Failed to bind to {}: {}", addr, e))
        })?;

        tracing::info!("Server listening on {}", addr);

        let server = Arc::new(self);
        let tracker = ConnectionTracker::new();

        // Accept connections until shutdown
        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok((stream, remote_addr)) => {
                            let server = Arc::clone(&server);
                            let token = tracker.acquire();
                            let shutdown_clone = shutdown.clone();

                            tokio::spawn(async move {
                                if let Err(e) = server.handle_connection(stream, remote_addr, shutdown_clone).await {
                                    tracing::error!("Connection error from {}: {}", remote_addr, e);
                                }
                                drop(token);
                            });
                        }
                        Err(e) => {
                            tracing::error!("Failed to accept connection: {}", e);
                        }
                    }
                }

                _ = shutdown.recv() => {
                    tracing::info!("Shutdown signal received, stopping server");
                    break;
                }
            }
        }

        // Mark as not ready during shutdown
        server.readiness.set_ready(false);

        // Wait for in-flight connections with timeout
        let shutdown_timeout = server.config.shutdown_timeout();
        tracing::info!(
            "Waiting up to {:?} for {} connections to close",
            shutdown_timeout,
            tracker.active_connections()
        );

        tokio::select! {
            _ = tracker.wait_for_shutdown() => {
                tracing::info!("All connections closed");
            }
            _ = tokio::time::sleep(shutdown_timeout) => {
                tracing::warn!(
                    "Shutdown timeout reached, {} connections still active",
                    tracker.active_connections()
                );
            }
        }

        tracing::info!("Server stopped");
        Ok(())
    }

    /// Handles a single connection.
    async fn handle_connection(
        self: &Arc<Self>,
        stream: tokio::net::TcpStream,
        remote_addr: SocketAddr,
        shutdown: ShutdownSignal,
    ) -> Result<(), hyper::Error> {
        let io = TokioIo::new(stream);
        let server = Arc::clone(self);

        let service = service_fn(move |req: Request<Incoming>| {
            let server = Arc::clone(&server);
            async move { server.handle_request(req).await }
        });

        let conn = http1::Builder::new().serve_connection(io, service);

        tokio::select! {
            result = conn => {
                result
            }
            _ = shutdown.recv() => {
                tracing::debug!("Connection from {} closed due to shutdown", remote_addr);
                Ok(())
            }
        }
    }

    /// Handles a single HTTP request.
    async fn handle_request(
        self: &Arc<Self>,
        req: Request<Incoming>,
    ) -> Result<HttpResponse, Infallible> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();

        tracing::debug!("{} {}", method, path);

        // Handle built-in health endpoints first (no body needed)
        match (method.as_ref(), path.as_str()) {
            ("GET", "/health") => return Ok(self.handle_health()),
            ("GET", "/ready") => return Ok(self.handle_ready()),
            _ => {}
        }

        // Collect request body with timeout
        let body_result = tokio::time::timeout(
            self.request_timeout,
            Self::collect_body(req),
        )
        .await;

        let body = match body_result {
            Ok(Ok(body)) => body,
            Ok(Err(e)) => {
                tracing::error!("Failed to collect request body: {}", e);
                return Ok(self.handle_error(
                    StatusCode::BAD_REQUEST,
                    "BODY_READ_ERROR",
                    &format!("Failed to read request body: {}", e),
                ));
            }
            Err(_) => {
                tracing::warn!("Request body collection timed out");
                return Ok(self.handle_error(
                    StatusCode::REQUEST_TIMEOUT,
                    "REQUEST_TIMEOUT",
                    "Request body collection timed out",
                ));
            }
        };

        // Route and invoke handler with timeout
        let response = tokio::time::timeout(
            self.request_timeout,
            self.route_request(&method, &path, body),
        )
        .await;

        match response {
            Ok(resp) => Ok(resp),
            Err(_) => {
                tracing::warn!("Handler execution timed out for {} {}", method, path);
                Ok(self.handle_error(
                    StatusCode::GATEWAY_TIMEOUT,
                    "HANDLER_TIMEOUT",
                    "Handler execution timed out",
                ))
            }
        }
    }

    /// Collects the request body into bytes.
    async fn collect_body(req: Request<Incoming>) -> Result<Bytes, hyper::Error> {
        let body = req.into_body();
        let collected = body.collect().await?;
        Ok(collected.to_bytes())
    }

    /// Handles the /health endpoint.
    fn handle_health(&self) -> HttpResponse {
        let status = self.health.status();
        let body = serde_json::to_string(&status).unwrap_or_else(|_| {
            r#"{"status":"healthy"}"#.to_string()
        });

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap_or_else(|_| {
                Response::new(Full::new(Bytes::from(r#"{"status":"healthy"}"#)))
            })
    }

    /// Handles the /ready endpoint.
    fn handle_ready(&self) -> HttpResponse {
        let status = self.readiness.status();
        let status_code = if status.is_ready() {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        };

        let body = serde_json::to_string(&status).unwrap_or_else(|_| {
            format!(r#"{{"ready":{}}}"#, status.is_ready())
        });

        Response::builder()
            .status(status_code)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap_or_else(|_| {
                Response::new(Full::new(Bytes::from(r#"{"ready":false}"#)))
            })
    }

    /// Routes a request to the appropriate handler.
    async fn route_request(&self, method: &Method, path: &str, body: Bytes) -> HttpResponse {
        match self.router.match_route(method, path) {
            Some(route_match) => self.handle_matched_route(route_match, body).await,
            None => self.handle_not_found(path),
        }
    }

    /// Handles a matched route by invoking the registered handler.
    async fn handle_matched_route(&self, route_match: RouteMatch, body: Bytes) -> HttpResponse {
        let operation_id = route_match.operation_id();

        // Check if handler is registered
        if !self.handlers.contains(operation_id) {
            tracing::warn!("No handler registered for operation: {}", operation_id);
            return self.handle_error(
                StatusCode::NOT_IMPLEMENTED,
                "HANDLER_NOT_IMPLEMENTED",
                &format!("No handler registered for operation: {}", operation_id),
            );
        }

        // Create request context with operation ID
        let ctx = RequestContext::new()
            .with_operation_id(operation_id);

        // Store path params in a way handlers can access them
        // For now, we'll pass them via the body if needed, or handlers can extract from path
        // TODO: Consider adding path_params to RequestContext in Phase A3

        // Invoke the handler
        match self.handlers.invoke(operation_id, ctx, body).await {
            Ok(response_body) => {
                Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "application/json")
                    .body(Full::new(response_body))
                    .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
            }
            Err(InvokeError::HandlerNotFound(id)) => {
                tracing::error!("Handler not found during invocation: {}", id);
                self.handle_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "HANDLER_NOT_FOUND",
                    &format!("Handler not found: {}", id),
                )
            }
            Err(InvokeError::HandlerError(e)) => {
                tracing::error!("Handler error for {}: {}", operation_id, e);
                self.handle_handler_error(operation_id, e)
            }
        }
    }

    /// Handles handler errors and converts them to HTTP responses.
    fn handle_handler_error(&self, operation_id: &str, error: crate::handler::HandlerError) -> HttpResponse {
        use crate::handler::HandlerError;

        let (status, code, message) = match &error {
            HandlerError::DeserializationError(msg) => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR".to_string(),
                format!("Invalid request body: {}", msg),
            ),
            HandlerError::SerializationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "SERIALIZATION_ERROR".to_string(),
                format!("Failed to serialize response: {}", msg),
            ),
            HandlerError::ThemisError(e) => {
                // Use to_envelope to get proper error structure
                let envelope = e.to_envelope(None);
                (
                    e.status_code(),
                    envelope.error.code,
                    envelope.error.message,
                )
            }
            HandlerError::Custom(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR".to_string(),
                format!("Internal error: {}", e),
            ),
        };

        let body = serde_json::json!({
            "error": {
                "code": code,
                "message": message,
                "operation_id": operation_id
            }
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
    }

    /// Creates a standard error response.
    fn handle_error(&self, status: StatusCode, code: &str, message: &str) -> HttpResponse {
        let body = serde_json::json!({
            "error": {
                "code": code,
                "message": message
            }
        });

        Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
    }

    /// Handles a not found response.
    fn handle_not_found(&self, path: &str) -> HttpResponse {
        let body = serde_json::json!({
            "error": "Not Found",
            "path": path
        });

        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("Content-Type", "application/json")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
    }
}

/// Builder for configuring and creating a [`Server`].
///
/// # Example
///
/// ```rust
/// use archimedes_server::{Server, ServerBuilder, HandlerRegistry};
/// use std::time::Duration;
///
/// let server = ServerBuilder::new()
///     .http_addr("0.0.0.0:9090")
///     .shutdown_timeout(Duration::from_secs(60))
///     .request_timeout(Duration::from_secs(30))
///     .build();
/// ```
#[derive(Default)]
pub struct ServerBuilder {
    config_builder: crate::config::ServerConfigBuilder,
    handlers: Option<HandlerRegistry>,
    health_service: Option<String>,
    health_version: Option<String>,
    request_timeout: Option<Duration>,
}

impl ServerBuilder {
    /// Creates a new server builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the handler registry.
    ///
    /// # Arguments
    ///
    /// * `handlers` - The handler registry with registered operation handlers
    #[must_use]
    pub fn handlers(mut self, handlers: HandlerRegistry) -> Self {
        self.handlers = Some(handlers);
        self
    }

    /// Sets the HTTP bind address.
    ///
    /// # Arguments
    ///
    /// * `addr` - Address to bind to (e.g., "0.0.0.0:8080")
    #[must_use]
    pub fn http_addr(mut self, addr: impl Into<String>) -> Self {
        self.config_builder = self.config_builder.http_addr(addr);
        self
    }

    /// Sets the graceful shutdown timeout.
    #[must_use]
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.config_builder = self.config_builder.shutdown_timeout(timeout);
        self
    }

    /// Sets the TCP keep-alive timeout.
    #[must_use]
    pub fn keep_alive_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.config_builder = self.config_builder.keep_alive_timeout(timeout);
        self
    }

    /// Sets the maximum concurrent connections.
    #[must_use]
    pub fn max_connections(mut self, max: Option<usize>) -> Self {
        self.config_builder = self.config_builder.max_connections(max);
        self
    }

    /// Enables or disables HTTP/2.
    #[must_use]
    pub fn http2_enabled(mut self, enabled: bool) -> Self {
        self.config_builder = self.config_builder.http2_enabled(enabled);
        self
    }

    /// Sets the service name for health checks.
    #[must_use]
    pub fn service_name(mut self, name: impl Into<String>) -> Self {
        self.health_service = Some(name.into());
        self
    }

    /// Sets the service version for health checks.
    #[must_use]
    pub fn service_version(mut self, version: impl Into<String>) -> Self {
        self.health_version = Some(version.into());
        self
    }

    /// Sets the request timeout.
    ///
    /// This timeout applies to both body collection and handler execution.
    /// Default is 30 seconds.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The request timeout duration
    #[must_use]
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    /// Builds the server with the configured settings.
    #[must_use]
    pub fn build(self) -> Server {
        let config = self.config_builder.build();
        let service = self.health_service.unwrap_or_else(|| "archimedes".to_string());
        let version = self
            .health_version
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

        Server {
            config,
            router: Router::new(),
            handlers: self.handlers.unwrap_or_default(),
            health: HealthCheck::new(service, version),
            readiness: ReadinessCheck::new(),
            request_timeout: self.request_timeout.unwrap_or(Duration::from_secs(30)),
        }
    }
}

/// Server error types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    /// Failed to bind to the configured address.
    BindError(String),

    /// I/O error during server operation.
    IoError(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BindError(msg) => write!(f, "Bind error: {}", msg),
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ServerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_new() {
        let config = ServerConfig::builder()
            .http_addr("127.0.0.1:8080")
            .build();

        let server = Server::new(config);
        assert_eq!(server.config().http_addr(), "127.0.0.1:8080");
    }

    #[test]
    fn test_server_builder() {
        let server = Server::builder()
            .http_addr("0.0.0.0:9090")
            .shutdown_timeout(Duration::from_secs(60))
            .build();

        assert_eq!(server.config().http_addr(), "0.0.0.0:9090");
        assert_eq!(server.config().shutdown_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_server_builder_service_name() {
        let server = Server::builder()
            .service_name("my-service")
            .service_version("2.0.0")
            .build();

        assert_eq!(server.health().service(), "my-service");
        assert_eq!(server.health().version(), "2.0.0");
    }

    #[test]
    fn test_server_router_access() {
        let mut server = Server::builder().build();

        server.router_mut().add_route(Method::GET, "/test", "testOp");
        assert!(server.router().has_operation("testOp"));
    }

    #[test]
    fn test_server_health_endpoint() {
        let server = Arc::new(Server::builder().build());
        let response = server.handle_health();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_server_ready_endpoint() {
        let server = Arc::new(Server::builder().build());
        let response = server.handle_ready();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_server_ready_not_ready() {
        let server = Arc::new(Server::builder().build());
        server.readiness().set_ready(false);

        let response = server.handle_ready();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_server_route_not_found() {
        let server = Arc::new(Server::builder().build());
        let response = server.handle_not_found("/nonexistent");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_server_route_matched_no_handler() {
        let mut server = Server::builder().build();
        server.router_mut().add_route(Method::GET, "/users/{id}", "getUser");

        let server = Arc::new(server);
        let response = server.route_request(&Method::GET, "/users/123", Bytes::new()).await;

        // Without a handler registered, should return NOT_IMPLEMENTED
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }

    #[test]
    fn test_server_error_display() {
        let bind_err = ServerError::BindError("Address in use".to_string());
        assert!(bind_err.to_string().contains("Bind error"));

        let io_err = ServerError::IoError("Connection reset".to_string());
        assert!(io_err.to_string().contains("I/O error"));
    }

    #[tokio::test]
    async fn test_server_run_invalid_address() {
        let server = Server::builder()
            .http_addr("not-a-valid-address")
            .build();

        let result = server.run_with_shutdown(ShutdownSignal::new()).await;
        assert!(result.is_err());

        if let Err(ServerError::BindError(msg)) = result {
            assert!(msg.contains("Invalid address"));
        } else {
            panic!("Expected BindError");
        }
    }

    #[tokio::test]
    async fn test_server_run_and_shutdown() {
        let server = Server::builder()
            .http_addr("127.0.0.1:0") // Use port 0 for random available port
            .shutdown_timeout(Duration::from_millis(100))
            .build();

        let shutdown = ShutdownSignal::new();
        let shutdown_trigger = shutdown.clone();

        // Trigger shutdown immediately
        shutdown_trigger.trigger();

        // Server should exit quickly
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            server.run_with_shutdown(shutdown),
        )
        .await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }

    // Integration tests for handler invocation

    #[derive(serde::Deserialize)]
    struct EchoRequest {
        message: String,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct EchoResponse {
        echo: String,
    }

    async fn echo_handler(
        _ctx: archimedes_core::RequestContext,
        req: EchoRequest,
    ) -> Result<EchoResponse, crate::handler::HandlerError> {
        Ok(EchoResponse {
            echo: format!("Echo: {}", req.message),
        })
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct HealthResponse {
        status: String,
    }

    async fn health_handler(
        _ctx: archimedes_core::RequestContext,
    ) -> Result<HealthResponse, crate::handler::HandlerError> {
        Ok(HealthResponse {
            status: "ok".to_string(),
        })
    }

    #[tokio::test]
    async fn test_handler_invocation() {
        use crate::handler::HandlerRegistry;

        let mut registry = HandlerRegistry::new();
        registry.register("echo", echo_handler);

        let mut server = Server::builder()
            .handlers(registry)
            .build();
        server.router_mut().add_route(Method::POST, "/echo", "echo");

        let server = Arc::new(server);
        let body = Bytes::from(r#"{"message":"Hello"}"#);
        let response = server.route_request(&Method::POST, "/echo", body).await;

        assert_eq!(response.status(), StatusCode::OK);

        // Extract body and verify
        let body_bytes = response.into_body();
        let collected = http_body_util::BodyExt::collect(body_bytes).await.unwrap();
        let resp: EchoResponse = serde_json::from_slice(&collected.to_bytes()).unwrap();
        assert_eq!(resp.echo, "Echo: Hello");
    }

    #[tokio::test]
    async fn test_handler_no_body_invocation() {
        use crate::handler::HandlerRegistry;

        let mut registry = HandlerRegistry::new();
        registry.register_no_body("healthCheck", health_handler);

        let mut server = Server::builder()
            .handlers(registry)
            .build();
        server.router_mut().add_route(Method::GET, "/status", "healthCheck");

        let server = Arc::new(server);
        let response = server.route_request(&Method::GET, "/status", Bytes::new()).await;

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = response.into_body();
        let collected = http_body_util::BodyExt::collect(body_bytes).await.unwrap();
        let resp: HealthResponse = serde_json::from_slice(&collected.to_bytes()).unwrap();
        assert_eq!(resp.status, "ok");
    }

    #[tokio::test]
    async fn test_handler_deserialization_error() {
        use crate::handler::HandlerRegistry;

        let mut registry = HandlerRegistry::new();
        registry.register("echo", echo_handler);

        let mut server = Server::builder()
            .handlers(registry)
            .build();
        server.router_mut().add_route(Method::POST, "/echo", "echo");

        let server = Arc::new(server);
        // Invalid JSON
        let body = Bytes::from(r#"not valid json"#);
        let response = server.route_request(&Method::POST, "/echo", body).await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_handler_not_registered() {
        use crate::handler::HandlerRegistry;

        let registry = HandlerRegistry::new();

        let mut server = Server::builder()
            .handlers(registry)
            .build();
        server.router_mut().add_route(Method::GET, "/missing", "missingOp");

        let server = Arc::new(server);
        let response = server.route_request(&Method::GET, "/missing", Bytes::new()).await;

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
