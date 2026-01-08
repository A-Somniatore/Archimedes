//! Handler registration and dispatch.
//!
//! This module provides the infrastructure for registering and invoking
//! typed handlers for each operation defined in the contract.
//!
//! # Architecture
//!
//! Handlers in Archimedes are:
//!
//! - **Typed**: Request and response types are checked at compile time
//! - **Async**: All handlers are async functions
//! - **Contract-bound**: Each handler is registered against an `operationId`
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_server::{HandlerRegistry, HandlerFn};
//! use archimedes_core::RequestContext;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Deserialize)]
//! struct GetUserRequest {
//!     user_id: String,
//! }
//!
//! #[derive(Serialize)]
//! struct User {
//!     id: String,
//!     name: String,
//! }
//!
//! async fn get_user(ctx: &RequestContext, req: GetUserRequest) -> Result<User, HandlerError> {
//!     Ok(User {
//!         id: req.user_id,
//!         name: "John Doe".to_string(),
//!     })
//! }
//!
//! let mut registry = HandlerRegistry::new();
//! registry.register("getUser", get_user);
//! ```

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

use archimedes_core::{RequestContext, ThemisError};

/// Type alias for boxed handler result.
pub type BoxedHandlerResult = Pin<Box<dyn Future<Output = Result<Bytes, HandlerError>> + Send>>;

/// A type-erased handler function.
pub type ErasedHandler = Arc<dyn Fn(RequestContext, Bytes) -> BoxedHandlerResult + Send + Sync>;

/// Handler error type.
///
/// Wraps errors that can occur during handler execution.
#[derive(Debug)]
pub enum HandlerError {
    /// Request deserialization failed.
    DeserializationError(String),

    /// Response serialization failed.
    SerializationError(String),

    /// Handler returned a Themis error.
    ThemisError(ThemisError),

    /// Handler returned a custom error.
    Custom(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DeserializationError(msg) => write!(f, "Deserialization error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::ThemisError(e) => write!(f, "Themis error: {}", e),
            Self::Custom(e) => write!(f, "Handler error: {}", e),
        }
    }
}

impl std::error::Error for HandlerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ThemisError(e) => Some(e),
            Self::Custom(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<ThemisError> for HandlerError {
    fn from(err: ThemisError) -> Self {
        Self::ThemisError(err)
    }
}

impl From<serde_json::Error> for HandlerError {
    fn from(err: serde_json::Error) -> Self {
        Self::DeserializationError(err.to_string())
    }
}

/// Registry for operation handlers.
///
/// Maps operation IDs to their handler functions, handling type
/// erasure and serialization/deserialization automatically.
///
/// # Example
///
/// ```rust
/// use archimedes_server::handler::HandlerRegistry;
///
/// let registry = HandlerRegistry::new();
/// assert_eq!(registry.len(), 0);
/// ```
#[derive(Default)]
pub struct HandlerRegistry {
    handlers: HashMap<String, ErasedHandler>,
}

impl HandlerRegistry {
    /// Creates a new empty handler registry.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::handler::HandlerRegistry;
    ///
    /// let registry = HandlerRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Registers a handler for an operation.
    ///
    /// The handler function must:
    /// - Accept a `&RequestContext` and a request type `Req`
    /// - Return a `Future` resolving to `Result<Res, HandlerError>`
    /// - Have `Req: DeserializeOwned` and `Res: Serialize`
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID from the contract
    /// * `handler` - The async handler function
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use archimedes_server::handler::{HandlerRegistry, HandlerError};
    /// use archimedes_core::RequestContext;
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Deserialize)]
    /// struct Req { name: String }
    ///
    /// #[derive(Serialize)]
    /// struct Res { greeting: String }
    ///
    /// async fn greet(_ctx: &RequestContext, req: Req) -> Result<Res, HandlerError> {
    ///     Ok(Res { greeting: format!("Hello, {}!", req.name) })
    /// }
    ///
    /// let mut registry = HandlerRegistry::new();
    /// registry.register("greet", greet);
    /// ```
    pub fn register<Req, Res, F, Fut>(&mut self, operation_id: impl Into<String>, handler: F)
    where
        Req: DeserializeOwned + Send + 'static,
        Res: Serialize + Send + 'static,
        F: Fn(RequestContext, Req) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Res, HandlerError>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let erased: ErasedHandler = Arc::new(move |ctx: RequestContext, body: Bytes| {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                // Deserialize request
                let request: Req = serde_json::from_slice(&body)
                    .map_err(|e| HandlerError::DeserializationError(e.to_string()))?;

                // Invoke handler
                let response = handler(ctx, request).await?;

                // Serialize response
                let bytes = serde_json::to_vec(&response)
                    .map_err(|e| HandlerError::SerializationError(e.to_string()))?;

                Ok(Bytes::from(bytes))
            })
        });

        self.handlers.insert(operation_id.into(), erased);
    }

    /// Registers a handler that takes no request body.
    ///
    /// Useful for operations like health checks or simple GETs.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID from the contract
    /// * `handler` - The async handler function
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use archimedes_server::handler::{HandlerRegistry, HandlerError};
    /// use archimedes_core::RequestContext;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Status { ok: bool }
    ///
    /// async fn health(_ctx: &RequestContext) -> Result<Status, HandlerError> {
    ///     Ok(Status { ok: true })
    /// }
    ///
    /// let mut registry = HandlerRegistry::new();
    /// registry.register_no_body("health", health);
    /// ```
    pub fn register_no_body<Res, F, Fut>(&mut self, operation_id: impl Into<String>, handler: F)
    where
        Res: Serialize + Send + 'static,
        F: Fn(RequestContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Res, HandlerError>> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let erased: ErasedHandler = Arc::new(move |ctx: RequestContext, _body: Bytes| {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                // Invoke handler (no request body)
                let response = handler(ctx).await?;

                // Serialize response
                let bytes = serde_json::to_vec(&response)
                    .map_err(|e| HandlerError::SerializationError(e.to_string()))?;

                Ok(Bytes::from(bytes))
            })
        });

        self.handlers.insert(operation_id.into(), erased);
    }

    /// Looks up a handler by operation ID.
    ///
    /// Returns `None` if no handler is registered for the operation.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID to look up
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::handler::HandlerRegistry;
    ///
    /// let registry = HandlerRegistry::new();
    /// assert!(registry.get("nonexistent").is_none());
    /// ```
    #[must_use]
    pub fn get(&self, operation_id: &str) -> Option<&ErasedHandler> {
        self.handlers.get(operation_id)
    }

    /// Checks if a handler is registered for an operation.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID to check
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::handler::HandlerRegistry;
    ///
    /// let registry = HandlerRegistry::new();
    /// assert!(!registry.contains("test"));
    /// ```
    #[must_use]
    pub fn contains(&self, operation_id: &str) -> bool {
        self.handlers.contains_key(operation_id)
    }

    /// Returns the number of registered handlers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::handler::HandlerRegistry;
    ///
    /// let registry = HandlerRegistry::new();
    /// assert_eq!(registry.len(), 0);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// Returns `true` if no handlers are registered.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::handler::HandlerRegistry;
    ///
    /// let registry = HandlerRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }

    /// Returns an iterator over registered operation IDs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::handler::HandlerRegistry;
    ///
    /// let registry = HandlerRegistry::new();
    /// assert_eq!(registry.operation_ids().count(), 0);
    /// ```
    pub fn operation_ids(&self) -> impl Iterator<Item = &str> {
        self.handlers.keys().map(String::as_str)
    }

    /// Invokes a handler for the given operation.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID to invoke
    /// * `ctx` - The request context
    /// * `body` - The request body bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the handler is not found or execution fails.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use archimedes_server::handler::HandlerRegistry;
    /// use archimedes_core::RequestContext;
    /// use bytes::Bytes;
    ///
    /// let registry = HandlerRegistry::new();
    /// let ctx = RequestContext::new();
    /// let body = Bytes::from(r#"{"name":"test"}"#);
    ///
    /// let result = registry.invoke("test", ctx, body).await;
    /// ```
    pub async fn invoke(
        &self,
        operation_id: &str,
        ctx: RequestContext,
        body: Bytes,
    ) -> Result<Bytes, InvokeError> {
        let handler = self
            .handlers
            .get(operation_id)
            .ok_or_else(|| InvokeError::HandlerNotFound(operation_id.to_string()))?;

        handler(ctx, body).await.map_err(InvokeError::HandlerError)
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry")
            .field("handlers", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Error returned when invoking a handler fails.
#[derive(Debug)]
pub enum InvokeError {
    /// No handler registered for the operation.
    HandlerNotFound(String),

    /// Handler execution failed.
    HandlerError(HandlerError),
}

impl std::fmt::Display for InvokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HandlerNotFound(id) => write!(f, "No handler registered for operation: {}", id),
            Self::HandlerError(e) => write!(f, "Handler error: {}", e),
        }
    }
}

impl std::error::Error for InvokeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::HandlerError(e) => Some(e),
            _ => None,
        }
    }
}

/// Trait for types that can be used as handler request parameters.
///
/// This is automatically implemented for types that implement
/// `DeserializeOwned + Send + 'static`.
pub trait HandlerRequest: DeserializeOwned + Send + 'static {}
impl<T: DeserializeOwned + Send + 'static> HandlerRequest for T {}

/// Trait for types that can be returned from handlers.
///
/// This is automatically implemented for types that implement
/// `Serialize + Send + 'static`.
pub trait HandlerResponse: Serialize + Send + 'static {}
impl<T: Serialize + Send + 'static> HandlerResponse for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use archimedes_core::RequestContext;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize)]
    struct TestRequest {
        name: String,
    }

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct TestResponse {
        greeting: String,
    }

    async fn test_handler(
        _ctx: RequestContext,
        req: TestRequest,
    ) -> Result<TestResponse, HandlerError> {
        Ok(TestResponse {
            greeting: format!("Hello, {}!", req.name),
        })
    }

    async fn test_no_body_handler(_ctx: RequestContext) -> Result<TestResponse, HandlerError> {
        Ok(TestResponse {
            greeting: "Hello, World!".to_string(),
        })
    }

    #[test]
    fn test_registry_new() {
        let registry = HandlerRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let mut registry = HandlerRegistry::new();
        registry.register("test", test_handler);

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test"));
        assert!(!registry.contains("other"));
    }

    #[test]
    fn test_registry_register_no_body() {
        let mut registry = HandlerRegistry::new();
        registry.register_no_body("health", test_no_body_handler);

        assert!(registry.contains("health"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = HandlerRegistry::new();
        registry.register("test", test_handler);

        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_operation_ids() {
        let mut registry = HandlerRegistry::new();
        registry.register("op1", test_handler);
        registry.register("op2", test_handler);

        let ids: Vec<_> = registry.operation_ids().collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"op1"));
        assert!(ids.contains(&"op2"));
    }

    #[tokio::test]
    async fn test_registry_invoke() {
        let mut registry = HandlerRegistry::new();
        registry.register("test", test_handler);

        let ctx = RequestContext::new();
        let body = Bytes::from(r#"{"name":"Alice"}"#);

        let result = registry.invoke("test", ctx, body).await;
        assert!(result.is_ok());

        let response_bytes = result.unwrap();
        let response: TestResponse = serde_json::from_slice(&response_bytes).unwrap();
        assert_eq!(response.greeting, "Hello, Alice!");
    }

    #[tokio::test]
    async fn test_registry_invoke_no_body() {
        let mut registry = HandlerRegistry::new();
        registry.register_no_body("health", test_no_body_handler);

        let ctx = RequestContext::new();
        let body = Bytes::new();

        let result = registry.invoke("health", ctx, body).await;
        assert!(result.is_ok());

        let response_bytes = result.unwrap();
        let response: TestResponse = serde_json::from_slice(&response_bytes).unwrap();
        assert_eq!(response.greeting, "Hello, World!");
    }

    #[tokio::test]
    async fn test_registry_invoke_not_found() {
        let registry = HandlerRegistry::new();
        let ctx = RequestContext::new();
        let body = Bytes::new();

        let result = registry.invoke("nonexistent", ctx, body).await;
        assert!(result.is_err());

        match result {
            Err(InvokeError::HandlerNotFound(id)) => assert_eq!(id, "nonexistent"),
            _ => panic!("Expected HandlerNotFound error"),
        }
    }

    #[tokio::test]
    async fn test_registry_invoke_deserialization_error() {
        let mut registry = HandlerRegistry::new();
        registry.register("test", test_handler);

        let ctx = RequestContext::new();
        let body = Bytes::from("not valid json");

        let result = registry.invoke("test", ctx, body).await;
        assert!(result.is_err());

        match result {
            Err(InvokeError::HandlerError(HandlerError::DeserializationError(_))) => {}
            _ => panic!("Expected DeserializationError"),
        }
    }

    #[test]
    fn test_handler_error_display() {
        let de_err = HandlerError::DeserializationError("bad json".to_string());
        assert!(de_err.to_string().contains("Deserialization"));

        let se_err = HandlerError::SerializationError("bad data".to_string());
        assert!(se_err.to_string().contains("Serialization"));
    }

    #[test]
    fn test_invoke_error_display() {
        let not_found = InvokeError::HandlerNotFound("test".to_string());
        assert!(not_found.to_string().contains("No handler"));

        let handler_err =
            InvokeError::HandlerError(HandlerError::DeserializationError("x".to_string()));
        assert!(handler_err.to_string().contains("Handler error"));
    }

    #[test]
    fn test_registry_debug() {
        let mut registry = HandlerRegistry::new();
        registry.register("test", test_handler);

        let debug = format!("{:?}", registry);
        assert!(debug.contains("HandlerRegistry"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_registry_default() {
        let registry = HandlerRegistry::default();
        assert!(registry.is_empty());
    }

    async fn failing_handler(
        _ctx: RequestContext,
        _req: TestRequest,
    ) -> Result<TestResponse, HandlerError> {
        Err(HandlerError::Custom(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test error",
        ))))
    }

    #[tokio::test]
    async fn test_registry_invoke_handler_error() {
        let mut registry = HandlerRegistry::new();
        registry.register("failing", failing_handler);

        let ctx = RequestContext::new();
        let body = Bytes::from(r#"{"name":"test"}"#);

        let result = registry.invoke("failing", ctx, body).await;
        assert!(result.is_err());

        match result {
            Err(InvokeError::HandlerError(HandlerError::Custom(_))) => {}
            _ => panic!("Expected Custom error"),
        }
    }
}
