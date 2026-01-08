//! Handler registration and invocation.

use crate::context::RequestContext;
use crate::response::Response;
use napi_derive::napi;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Type alias for handler functions stored in the registry.
type HandlerFn = Arc<dyn Fn(RequestContext) -> Response + Send + Sync>;

/// Registry for operation handlers.
///
/// Maps operation IDs to handler functions.
#[napi]
#[derive(Clone)]
pub struct HandlerRegistry {
    handlers: Arc<RwLock<HashMap<String, HandlerFn>>>,
    default_responses: Arc<RwLock<HashMap<String, Response>>>,
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl HandlerRegistry {
    /// Create a new handler registry.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            default_responses: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a simple handler that returns a fixed JSON response.
    #[napi]
    pub fn register_json_handler(&self, operation_id: String, status_code: u16, json_body: String) {
        let response = Response::from_json_string(status_code, json_body);
        let resp = response.clone();
        let handler: HandlerFn = Arc::new(move |_ctx| resp.clone());

        // Use blocking lock for simplicity in napi context
        if let Ok(mut handlers) = self.handlers.try_write() {
            handlers.insert(operation_id.clone(), handler);
        }
        if let Ok(mut defaults) = self.default_responses.try_write() {
            defaults.insert(operation_id, response);
        }
    }

    /// Register a handler that returns a 200 OK with the given JSON.
    #[napi]
    pub fn register_ok_handler(&self, operation_id: String, json_body: String) {
        self.register_json_handler(operation_id, 200, json_body);
    }

    /// Check if a handler is registered for an operation.
    #[napi]
    pub async fn has_handler(&self, operation_id: String) -> bool {
        self.handlers.read().await.contains_key(&operation_id)
    }

    /// Get the list of registered operation IDs.
    #[napi]
    pub async fn registered_operations(&self) -> Vec<String> {
        self.handlers.read().await.keys().cloned().collect()
    }

    /// Get the number of registered handlers.
    #[napi]
    pub async fn handler_count(&self) -> u32 {
        self.handlers.read().await.len() as u32
    }

    /// Invoke a handler for an operation.
    #[napi]
    pub async fn invoke(
        &self,
        operation_id: String,
        ctx: RequestContext,
    ) -> napi::Result<Response> {
        let handlers = self.handlers.read().await;

        match handlers.get(&operation_id) {
            Some(handler) => Ok(handler(ctx)),
            None => Err(napi::Error::new(
                napi::Status::GenericFailure,
                format!("No handler registered for operation '{}'", operation_id),
            )),
        }
    }

    /// Get the default response for an operation (for testing).
    #[napi]
    pub async fn get_default_response(&self, operation_id: String) -> napi::Result<Response> {
        let defaults = self.default_responses.read().await;
        defaults.get(&operation_id).cloned().ok_or_else(|| {
            napi::Error::new(
                napi::Status::GenericFailure,
                format!("No default response for operation '{}'", operation_id),
            )
        })
    }

    /// Remove a handler.
    #[napi]
    pub async fn remove(&self, operation_id: String) -> bool {
        let removed_handler = self.handlers.write().await.remove(&operation_id).is_some();
        let removed_default = self
            .default_responses
            .write()
            .await
            .remove(&operation_id)
            .is_some();
        removed_handler || removed_default
    }

    /// Clear all handlers.
    #[napi]
    pub async fn clear(&self) {
        self.handlers.write().await.clear();
        self.default_responses.write().await.clear();
    }
}

/// Handler metadata for introspection.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct HandlerInfo {
    /// Operation ID
    pub operation_id: String,

    /// Whether the handler is async
    pub is_async: bool,

    /// Handler description (if provided)
    pub description: Option<String>,
}

/// Create handler info.
#[napi]
pub fn create_handler_info(
    operation_id: String,
    is_async: bool,
    description: Option<String>,
) -> HandlerInfo {
    HandlerInfo {
        operation_id,
        is_async,
        description,
    }
}

impl Response {
    /// Create response from status code and JSON string (internal helper).
    pub fn from_json_string(status_code: u16, json_body: String) -> Self {
        let mut resp = Response::status(status_code);
        resp.set_body(json_body);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handler_registry_creation() {
        let registry = HandlerRegistry::new();
        assert_eq!(registry.handler_count().await, 0);
    }

    #[tokio::test]
    async fn test_register_json_handler() {
        let registry = HandlerRegistry::new();

        registry.register_json_handler(
            "testOperation".to_string(),
            200,
            r#"{"success": true}"#.to_string(),
        );

        assert!(registry.has_handler("testOperation".to_string()).await);
        assert_eq!(registry.handler_count().await, 1);
    }

    #[tokio::test]
    async fn test_register_ok_handler() {
        let registry = HandlerRegistry::new();

        registry.register_ok_handler("testOp".to_string(), r#"{"data": "test"}"#.to_string());

        assert!(registry.has_handler("testOp".to_string()).await);

        let response = registry
            .get_default_response("testOp".to_string())
            .await
            .unwrap();
        assert_eq!(response.status_code(), 200);
    }

    #[tokio::test]
    async fn test_registered_operations() {
        let registry = HandlerRegistry::new();
        registry.register_ok_handler("op1".to_string(), "{}".to_string());
        registry.register_ok_handler("op2".to_string(), "{}".to_string());

        let ops = registry.registered_operations().await;
        assert_eq!(ops.len(), 2);
        assert!(ops.contains(&"op1".to_string()));
        assert!(ops.contains(&"op2".to_string()));
    }

    #[tokio::test]
    async fn test_invoke_handler() {
        let registry = HandlerRegistry::new();
        registry.register_ok_handler("testOp".to_string(), r#"{"data": "test"}"#.to_string());

        let ctx = RequestContext::default();
        let result = registry.invoke("testOp".to_string(), ctx).await.unwrap();

        assert_eq!(result.status_code(), 200);
        assert!(result.body().unwrap().contains("test"));
    }

    #[tokio::test]
    async fn test_invoke_missing_handler() {
        let registry = HandlerRegistry::new();
        let ctx = RequestContext::default();

        let result = registry.invoke("nonExistent".to_string(), ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_remove_handler() {
        let registry = HandlerRegistry::new();
        registry.register_ok_handler("testOp".to_string(), "{}".to_string());

        assert!(registry.has_handler("testOp".to_string()).await);

        let removed = registry.remove("testOp".to_string()).await;
        assert!(removed);
        assert!(!registry.has_handler("testOp".to_string()).await);
    }

    #[tokio::test]
    async fn test_clear_handlers() {
        let registry = HandlerRegistry::new();
        registry.register_ok_handler("op1".to_string(), "{}".to_string());
        registry.register_ok_handler("op2".to_string(), "{}".to_string());

        assert_eq!(registry.handler_count().await, 2);

        registry.clear().await;
        assert_eq!(registry.handler_count().await, 0);
    }

    #[tokio::test]
    async fn test_has_handler_false() {
        let registry = HandlerRegistry::new();
        assert!(!registry.has_handler("nonExistent".to_string()).await);
    }

    #[tokio::test]
    async fn test_get_default_response_not_found() {
        let registry = HandlerRegistry::new();
        let result = registry
            .get_default_response("nonExistent".to_string())
            .await;
        assert!(result.is_err());
    }
}
