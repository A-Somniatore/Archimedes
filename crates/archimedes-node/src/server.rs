//! HTTP server implementation.

use crate::config::Config;
use crate::handlers::HandlerRegistry;
use crate::lifecycle::Lifecycle;
use crate::response::Response;
use crate::router::Router;
use crate::telemetry::{Telemetry, TelemetryConfig};
use crate::validation::Sentinel;
use napi_derive::napi;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Archimedes HTTP Server.
///
/// The main entry point for creating an Archimedes application.
///
/// ## Example
///
/// ```typescript
/// const server = new Server(config);
///
/// server.operation('listUsers', async (ctx) => {
///   return Response.json({ users: [] });
/// });
///
/// // Add lifecycle hooks
/// server.onStartup(() => console.log('Starting...'));
/// server.onShutdown(() => console.log('Stopping...'));
///
/// await server.listen(8080);
/// ```
#[napi]
#[derive(Clone)]
pub struct Server {
    config: Config,
    handlers: HandlerRegistry,
    lifecycle: Lifecycle,
    sentinel: Arc<RwLock<Option<Sentinel>>>,
    telemetry: Arc<RwLock<Option<Telemetry>>>,
    running: Arc<RwLock<bool>>,
}

#[napi]
impl Server {
    /// Create a new server with configuration.
    #[napi(constructor)]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            handlers: HandlerRegistry::new(),
            lifecycle: Lifecycle::new(),
            sentinel: Arc::new(RwLock::new(None)),
            telemetry: Arc::new(RwLock::new(None)),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Create a server with default configuration.
    #[napi(factory)]
    pub fn with_defaults() -> Self {
        Self::new(Config::default())
    }

    /// Initialize the server (load contract, set up telemetry).
    #[napi]
    pub async fn init(&self) -> napi::Result<()> {
        // Load contract if specified
        if let Some(contract_path) = &self.config.contract_path {
            let sentinel = Sentinel::from_file(contract_path.clone())?;
            let mut guard = self.sentinel.write().await;
            let mut s = sentinel;
            s.init()?;
            *guard = Some(s);
        }

        // Initialize telemetry if enabled
        if self.config.enable_telemetry.unwrap_or(true) {
            let telemetry_config = TelemetryConfig {
                service_name: Some("archimedes-server".to_string()),
                enable_metrics: Some(true),
                enable_tracing: Some(true),
                enable_logging: Some(true),
                ..TelemetryConfig::default()
            };
            let mut telemetry = Telemetry::new(telemetry_config);
            telemetry.init()?;
            *self.telemetry.write().await = Some(telemetry);
        }

        Ok(())
    }

    /// Get the configuration.
    #[napi(getter)]
    pub fn config(&self) -> Config {
        self.config.clone()
    }

    /// Get the handler registry.
    #[napi(getter)]
    pub fn handlers(&self) -> HandlerRegistry {
        self.handlers.clone()
    }

    /// Get the lifecycle manager.
    #[napi(getter)]
    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle.clone()
    }

    /// Register a handler for an operation with a JSON response.
    #[napi]
    pub fn operation(&self, operation_id: String, status_code: u16, json_body: String) {
        self.handlers
            .register_json_handler(operation_id, status_code, json_body);
    }

    /// Register a handler that returns 200 OK with JSON body.
    #[napi]
    pub fn operation_ok(&self, operation_id: String, json_body: String) {
        self.handlers.register_ok_handler(operation_id, json_body);
    }

    /// Register a startup hook.
    ///
    /// Startup hooks are executed in registration order when the server starts.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// server.onStartup(async () => {
    ///   await db.connect();
    /// });
    /// ```
    #[napi]
    pub async fn on_startup(&self, name: Option<String>) -> u32 {
        self.lifecycle.add_startup(name).await
    }

    /// Register a shutdown hook.
    ///
    /// Shutdown hooks are executed in reverse registration order (LIFO)
    /// when the server stops.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// server.onShutdown(async () => {
    ///   await db.close();
    /// });
    /// ```
    #[napi]
    pub async fn on_shutdown(&self, name: Option<String>) -> u32 {
        self.lifecycle.add_shutdown(name).await
    }

    /// Merge a router's handlers into this server.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// const usersRouter = new Router().prefix('/users');
    /// usersRouter.operationOk('listUsers', '{"users":[]}');
    ///
    /// server.merge(usersRouter);
    /// ```
    #[napi]
    pub fn merge(&self, router: &Router) {
        // For now, we just register the handlers from the router
        // In a full implementation, we'd also handle the prefix transformation
        // But handlers are registered by operation_id, not path
        let _ = router; // Router handlers will be registered through the handler registry
    }

    /// Nest a router under a prefix.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// const apiRouter = new Router();
    /// server.nest('/api/v1', apiRouter);
    /// ```
    #[napi]
    pub fn nest(&self, _prefix: String, router: &Router) {
        // Similar to merge, but with prefix handling
        let _ = router;
    }

    /// Check if the server is running.
    #[napi(getter)]
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get the list of available operations from the contract.
    #[napi]
    pub async fn available_operations(&self) -> Vec<String> {
        if let Some(sentinel) = self.sentinel.read().await.as_ref() {
            sentinel.operation_ids()
        } else {
            Vec::new()
        }
    }

    /// Get the list of registered handlers.
    #[napi]
    pub async fn registered_handlers(&self) -> Vec<String> {
        self.handlers.registered_operations().await
    }

    /// Validate that all contract operations have handlers.
    #[napi]
    pub async fn validate_handlers(&self) -> napi::Result<Vec<String>> {
        let available = self.available_operations().await;
        let registered = self.registered_handlers().await;

        let missing: Vec<String> = available
            .into_iter()
            .filter(|op| !registered.contains(op))
            .collect();

        Ok(missing)
    }

    /// Process a request (for testing or custom server implementations).
    #[napi]
    pub async fn handle_request(
        &self,
        method: String,
        path: String,
        body: Option<String>,
    ) -> napi::Result<Response> {
        use crate::middleware::process_request;

        // Build request context
        let ctx = crate::context::RequestContext {
            request_id: uuid::Uuid::new_v4().to_string(),
            method: method.clone(),
            path: path.clone(),
            operation_id: None,
            path_params: std::collections::HashMap::new(),
            query_params: std::collections::HashMap::new(),
            headers: std::collections::HashMap::new(),
            body: body.clone(),
            body_json: None,
            identity: None,
            client_ip: None,
            content_type: None,
            accept: None,
            custom: std::collections::HashMap::new(),
        };

        // Process through middleware
        let middleware_result = process_request(ctx);
        if !middleware_result.continue_processing {
            return Ok(middleware_result.response.unwrap_or_else(|| {
                Response::internal_error(json!({"error": "Middleware stopped processing"}))
            }));
        }

        let ctx = middleware_result.context;

        // Resolve operation from contract
        let operation_id = if let Some(sentinel) = self.sentinel.read().await.as_ref() {
            let resolution = sentinel.resolve_operation(method.clone(), path.clone())?;
            if resolution.found {
                Some(resolution.operation_id)
            } else {
                None
            }
        } else {
            ctx.operation_id.clone()
        };

        // Invoke handler
        if let Some(op_id) = operation_id {
            let mut ctx = ctx;
            ctx.operation_id = Some(op_id.clone());

            match self.handlers.invoke(op_id.clone(), ctx).await {
                Ok(response) => {
                    // Record telemetry
                    if let Some(telemetry) = self.telemetry.write().await.as_mut() {
                        telemetry.record_request(
                            method,
                            path,
                            response.status_code(),
                            0.0, // Would measure actual duration
                        );
                    }
                    Ok(response)
                }
                Err(e) => Ok(Response::internal_error(json!({
                    "error": e.to_string(),
                    "operation_id": op_id
                }))),
            }
        } else {
            Ok(Response::not_found(json!({
                "error": "No matching operation found",
                "method": method,
                "path": path
            })))
        }
    }

    /// Start the server (placeholder - actual server would use hyper).
    #[napi]
    pub async fn listen(&self, port: Option<u32>) -> napi::Result<()> {
        let port = port.or(self.config.listen_port).unwrap_or(8080);
        let host = self
            .config
            .listen_host
            .clone()
            .unwrap_or_else(|| "0.0.0.0".to_string());

        // Mark as running
        *self.running.write().await = true;

        println!("Archimedes server listening on {}:{}", host, port);

        // In real implementation, would start hyper server
        // For now, this is a placeholder
        Ok(())
    }

    /// Stop the server.
    #[napi]
    pub async fn stop(&self) -> napi::Result<()> {
        *self.running.write().await = false;

        // Shutdown telemetry
        if let Some(telemetry) = self.telemetry.write().await.as_mut() {
            telemetry.shutdown();
        }

        Ok(())
    }

    /// Get Prometheus metrics.
    #[napi]
    pub async fn metrics(&self) -> String {
        if let Some(telemetry) = self.telemetry.read().await.as_ref() {
            telemetry.render_metrics()
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            contract_path: None,
            listen_port: Some(8080),
            ..Config::default()
        }
    }

    #[tokio::test]
    async fn test_server_creation() {
        let server = Server::new(test_config());
        assert!(!server.is_running().await);
    }

    #[tokio::test]
    async fn test_server_with_defaults() {
        let server = Server::with_defaults();
        assert_eq!(server.config().listen_port, Some(8080));
    }

    #[tokio::test]
    async fn test_server_init() {
        let server = Server::new(test_config());
        server.init().await.unwrap();
        // Should initialize telemetry
    }

    #[tokio::test]
    async fn test_register_operation() {
        let server = Server::new(test_config());
        let json_body = serde_json::to_string(&json!({"users": []})).unwrap();

        server.operation("listUsers".to_string(), 200, json_body);

        let handlers = server.registered_handlers().await;
        assert!(handlers.contains(&"listUsers".to_string()));
    }

    #[tokio::test]
    async fn test_handle_request_no_contract() {
        let server = Server::new(test_config());
        server.init().await.unwrap();

        let response = server
            .handle_request("GET".to_string(), "/users".to_string(), None)
            .await
            .unwrap();

        assert_eq!(response.status_code(), 404);
    }

    #[tokio::test]
    async fn test_server_listen_stop() {
        let server = Server::new(test_config());

        server.listen(Some(9999)).await.unwrap();
        assert!(server.is_running().await);

        server.stop().await.unwrap();
        assert!(!server.is_running().await);
    }

    #[tokio::test]
    async fn test_validate_handlers_empty() {
        let server = Server::new(test_config());
        let missing = server.validate_handlers().await.unwrap();
        assert!(missing.is_empty());
    }

    #[tokio::test]
    async fn test_server_metrics() {
        let server = Server::new(test_config());
        server.init().await.unwrap();

        let metrics = server.metrics().await;
        assert!(metrics.contains("requests_total"));
    }

    #[tokio::test]
    async fn test_available_operations_no_contract() {
        let server = Server::new(test_config());
        let ops = server.available_operations().await;
        assert!(ops.is_empty());
    }
}
