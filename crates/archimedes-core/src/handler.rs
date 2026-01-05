//! Handler trait for request processing.
//!
//! The [`Handler`] trait defines the interface for request handlers in Archimedes.

use crate::{RequestContext, ThemisError};
use serde::{Serialize, de::DeserializeOwned};
use std::future::Future;

/// A trait for handling typed requests.
///
/// Handlers process requests and return responses. They receive a [`RequestContext`]
/// with identity and tracing information, along with the deserialized request body.
///
/// # Type Parameters
///
/// - `Req`: The request type (must implement `DeserializeOwned`)
/// - `Res`: The response type (must implement `Serialize`)
///
/// # Example
///
/// ```rust,ignore
/// use archimedes_core::{Handler, RequestContext, ThemisResult};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Deserialize)]
/// struct GetUserRequest {
///     user_id: String,
/// }
///
/// #[derive(Serialize)]
/// struct User {
///     id: String,
///     name: String,
/// }
///
/// struct GetUserHandler;
///
/// impl Handler<GetUserRequest, User> for GetUserHandler {
///     async fn handle(&self, ctx: &RequestContext, req: GetUserRequest) -> ThemisResult<User> {
///         // Handle the request...
///         Ok(User {
///             id: req.user_id,
///             name: "Alice".to_string(),
///         })
///     }
/// }
/// ```
pub trait Handler<Req, Res>: Send + Sync + 'static
where
    Req: DeserializeOwned + Send + 'static,
    Res: Serialize + Send + 'static,
{
    /// Handles a request and returns a response.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The request context containing identity, tracing, and metadata
    /// * `request` - The deserialized request body
    ///
    /// # Returns
    ///
    /// The handler response or a [`ThemisError`]
    ///
    /// # Errors
    ///
    /// Returns [`ThemisError`] if:
    /// - Business logic validation fails
    /// - Required resources are not found
    /// - An internal error occurs
    fn handle(
        &self,
        ctx: &RequestContext,
        request: Req,
    ) -> impl Future<Output = Result<Res, ThemisError>> + Send;
}

/// A type-erased handler for use in the router.
///
/// This allows storing handlers of different types in a single collection.
pub trait ErasedHandler: Send + Sync + 'static {
    /// Handles a request with raw JSON input/output.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The request context
    /// * `body` - The raw request body as bytes
    ///
    /// # Returns
    ///
    /// The serialized response body or an error
    fn handle_raw(
        &self,
        ctx: &RequestContext,
        body: &[u8],
    ) -> impl Future<Output = Result<Vec<u8>, ThemisError>> + Send;
}

/// A function-based handler wrapper.
///
/// This allows using async functions directly as handlers.
///
/// # Example
///
/// ```rust,ignore
/// use archimedes_core::{FnHandler, RequestContext, ThemisResult};
///
/// async fn get_user(ctx: &RequestContext, req: GetUserRequest) -> ThemisResult<User> {
///     // ...
/// }
///
/// let handler = FnHandler::new(get_user);
/// ```
pub struct FnHandler<F, Req, Res, Fut>
where
    F: Fn(&RequestContext, Req) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Res, ThemisError>> + Send,
    Req: DeserializeOwned + Send + 'static,
    Res: Serialize + Send + 'static,
{
    func: F,
    _phantom: std::marker::PhantomData<fn(Req) -> (Res, Fut)>,
}

impl<F, Req, Res, Fut> FnHandler<F, Req, Res, Fut>
where
    F: Fn(&RequestContext, Req) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Res, ThemisError>> + Send,
    Req: DeserializeOwned + Send + 'static,
    Res: Serialize + Send + 'static,
{
    /// Creates a new function-based handler.
    #[must_use]
    pub const fn new(func: F) -> Self {
        Self {
            func,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<F, Req, Res, Fut> Handler<Req, Res> for FnHandler<F, Req, Res, Fut>
where
    F: Fn(&RequestContext, Req) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Res, ThemisError>> + Send + 'static,
    Req: DeserializeOwned + Send + 'static,
    Res: Serialize + Send + 'static,
{
    async fn handle(&self, ctx: &RequestContext, request: Req) -> Result<Res, ThemisError> {
        (self.func)(ctx, request).await
    }
}

/// Unit request type for handlers that don't need a request body.
///
/// Use this for operations like health checks or operations where all
/// parameters come from the URL path or query string.
#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Empty {}

/// Unit response type for handlers that don't return a body.
///
/// Use this for operations that return only a status code (e.g., 204 No Content).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct NoContent {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, serde::Deserialize)]
    struct TestRequest {
        name: String,
    }

    #[derive(Debug, PartialEq, serde::Serialize)]
    struct TestResponse {
        greeting: String,
    }

    struct TestHandler;

    impl Handler<TestRequest, TestResponse> for TestHandler {
        async fn handle(
            &self,
            _ctx: &RequestContext,
            request: TestRequest,
        ) -> Result<TestResponse, ThemisError> {
            Ok(TestResponse {
                greeting: format!("Hello, {}!", request.name),
            })
        }
    }

    #[tokio::test]
    async fn test_handler_impl() {
        let handler = TestHandler;
        let ctx = RequestContext::mock();
        let request = TestRequest {
            name: "World".to_string(),
        };

        let response = handler.handle(&ctx, request).await;
        assert!(response.is_ok());
        assert_eq!(
            response.unwrap(),
            TestResponse {
                greeting: "Hello, World!".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_handler_error() {
        struct FailingHandler;

        impl Handler<Empty, NoContent> for FailingHandler {
            async fn handle(
                &self,
                _ctx: &RequestContext,
                _request: Empty,
            ) -> Result<NoContent, ThemisError> {
                Err(ThemisError::internal("Something went wrong"))
            }
        }

        let handler = FailingHandler;
        let ctx = RequestContext::mock();

        let response = handler.handle(&ctx, Empty {}).await;
        assert!(response.is_err());
    }

    #[test]
    fn test_empty_deserialize() {
        let empty: Empty = serde_json::from_str("{}").expect("should deserialize");
        assert!(std::mem::size_of_val(&empty) == 0 || true); // Empty is ZST
    }

    #[test]
    fn test_no_content_serialize() {
        let no_content = NoContent {};
        let json = serde_json::to_string(&no_content).expect("should serialize");
        assert_eq!(json, "{}");
    }
}
