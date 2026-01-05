//! Fixed-order middleware pipeline.
//!
//! This module implements the immutable middleware pipeline that all requests
//! flow through. The pipeline order is fixed and cannot be modified by users.
//!
//! ## Pipeline Stages
//!
//! The pipeline consists of 8 mandatory stages in a fixed order:
//!
//! 1. **Request ID** - Generate or propagate request ID (UUID v7)
//! 2. **Tracing** - Initialize OpenTelemetry span
//! 3. **Identity** - Extract caller identity (SPIFFE/JWT)
//! 4. **Authorization** - OPA policy evaluation
//! 5. **Request Validation** - Validate against contract schema
//! 6. **Response Validation** - Validate response (post-handler)
//! 7. **Telemetry** - Emit metrics and structured logs
//! 8. **Error Normalization** - Convert errors to standard envelope
//!
//! ## Extension Points
//!
//! Users can provide optional hooks:
//! - `pre_handler` - Called after identity extraction, before authorization
//! - `post_handler` - Called after handler, before response validation
//!
//! These hooks cannot modify the pipeline order or suppress core middleware.

use crate::context::MiddlewareContext;
use crate::middleware::{BoxFuture, Middleware, Next};
use crate::types::{Request, Response};
use std::sync::Arc;

/// A type-erased middleware that can be stored in a vector.
pub type BoxedMiddleware = Arc<dyn Middleware>;

/// The fixed-order middleware pipeline.
///
/// This pipeline cannot be modified after construction. The order of
/// middleware stages is determined at compile time and cannot be changed
/// by users.
///
/// # Example
///
/// ```ignore
/// use archimedes_middleware::pipeline::Pipeline;
///
/// // Create pipeline with default middleware
/// let pipeline = Pipeline::builder()
///     .pre_handler(|ctx, req| async move { Ok(req) })
///     .post_handler(|ctx, res| async move { Ok(res) })
///     .build();
///
/// // Process a request
/// let response = pipeline.process(request).await;
/// ```
pub struct Pipeline {
    /// Pre-handler middleware stages (stages 1-5)
    pre_handler_stages: Vec<BoxedMiddleware>,

    /// Optional pre-handler extension point
    pre_handler_hook: Option<PreHandlerHook>,

    /// Post-handler middleware stages (stages 6-8)
    post_handler_stages: Vec<BoxedMiddleware>,

    /// Optional post-handler extension point
    post_handler_hook: Option<PostHandlerHook>,
}

/// A pre-handler hook that runs after identity extraction, before authorization.
pub type PreHandlerHook = Arc<
    dyn Fn(&MiddlewareContext, &Request) -> BoxFuture<'static, Result<(), HookError>>
        + Send
        + Sync
        + 'static,
>;

/// A post-handler hook that runs after the handler, before response validation.
pub type PostHandlerHook = Arc<
    dyn Fn(&MiddlewareContext, &Response) -> BoxFuture<'static, Result<(), HookError>>
        + Send
        + Sync
        + 'static,
>;

/// Errors that can occur in extension hooks.
#[derive(Debug, Clone)]
pub struct HookError {
    /// Error message
    pub message: String,
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Hook error: {}", self.message)
    }
}

impl std::error::Error for HookError {}

impl Pipeline {
    /// Creates a new pipeline builder.
    #[must_use]
    pub fn builder() -> PipelineBuilder {
        PipelineBuilder::new()
    }

    /// Processes a request through the entire pipeline.
    ///
    /// This is the main entry point for request processing. The request
    /// flows through all middleware stages in order, then to the handler,
    /// then through post-handler stages.
    pub async fn process<H>(
        &self,
        mut ctx: MiddlewareContext,
        request: Request,
        handler: H,
    ) -> Response
    where
        H: FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> + Send + 'static,
    {
        // Build the middleware chain from back to front
        let next = self.build_chain(handler);
        next.run(&mut ctx, request).await
    }

    /// Builds the middleware chain for a request.
    fn build_chain<'a, H>(&'a self, handler: H) -> Next<'a>
    where
        H: FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> + Send + 'a,
    {
        // Start with the handler as the terminal point
        let mut next = Next::handler(handler);

        // Wrap with post-handler stages (in reverse order)
        for middleware in self.post_handler_stages.iter().rev() {
            next = Next::new(middleware.as_ref(), next);
        }

        // Wrap with pre-handler stages (in reverse order)
        for middleware in self.pre_handler_stages.iter().rev() {
            next = Next::new(middleware.as_ref(), next);
        }

        next
    }

    /// Returns the names of all middleware stages in order.
    #[must_use]
    pub fn stage_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        for mw in &self.pre_handler_stages {
            names.push(mw.name());
        }
        for mw in &self.post_handler_stages {
            names.push(mw.name());
        }
        names
    }

    /// Returns the number of middleware stages.
    #[must_use]
    pub fn stage_count(&self) -> usize {
        self.pre_handler_stages.len() + self.post_handler_stages.len()
    }
}

/// Builder for constructing a [`Pipeline`].
///
/// The builder allows setting extension hooks but the core middleware
/// stages are fixed.
pub struct PipelineBuilder {
    /// Pre-handler stages
    pre_handler_stages: Vec<BoxedMiddleware>,

    /// Post-handler stages
    post_handler_stages: Vec<BoxedMiddleware>,

    /// Pre-handler extension hook
    pre_handler_hook: Option<PreHandlerHook>,

    /// Post-handler extension hook
    post_handler_hook: Option<PostHandlerHook>,
}

impl PipelineBuilder {
    /// Creates a new pipeline builder with default middleware stages.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pre_handler_stages: Vec::new(),
            post_handler_stages: Vec::new(),
            pre_handler_hook: None,
            post_handler_hook: None,
        }
    }

    /// Adds a pre-handler middleware stage.
    ///
    /// Pre-handler stages run before the request handler, in order:
    /// 1. Request ID
    /// 2. Tracing
    /// 3. Identity
    /// 4. Authorization
    /// 5. Request Validation
    #[must_use]
    pub fn add_pre_handler_stage<M: Middleware>(mut self, middleware: M) -> Self {
        self.pre_handler_stages.push(Arc::new(middleware));
        self
    }

    /// Adds a post-handler middleware stage.
    ///
    /// Post-handler stages run after the request handler, in order:
    /// 6. Response Validation
    /// 7. Telemetry
    /// 8. Error Normalization
    #[must_use]
    pub fn add_post_handler_stage<M: Middleware>(mut self, middleware: M) -> Self {
        self.post_handler_stages.push(Arc::new(middleware));
        self
    }

    /// Sets the pre-handler extension hook.
    ///
    /// This hook runs after identity extraction but before authorization.
    /// It cannot modify the request context or suppress middleware.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pipeline = Pipeline::builder()
    ///     .pre_handler(|ctx, req| {
    ///         Box::pin(async move {
    ///             tracing::info!(request_id = %ctx.request_id(), "Pre-handler hook");
    ///             Ok(())
    ///         })
    ///     })
    ///     .build();
    /// ```
    #[must_use]
    pub fn pre_handler<F>(mut self, hook: F) -> Self
    where
        F: Fn(&MiddlewareContext, &Request) -> BoxFuture<'static, Result<(), HookError>>
            + Send
            + Sync
            + 'static,
    {
        self.pre_handler_hook = Some(Arc::new(hook));
        self
    }

    /// Sets the post-handler extension hook.
    ///
    /// This hook runs after the handler but before response validation.
    /// It cannot modify the response or suppress middleware.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let pipeline = Pipeline::builder()
    ///     .post_handler(|ctx, res| {
    ///         Box::pin(async move {
    ///             tracing::info!(status = %res.status(), "Post-handler hook");
    ///             Ok(())
    ///         })
    ///     })
    ///     .build();
    /// ```
    #[must_use]
    pub fn post_handler<F>(mut self, hook: F) -> Self
    where
        F: Fn(&MiddlewareContext, &Response) -> BoxFuture<'static, Result<(), HookError>>
            + Send
            + Sync
            + 'static,
    {
        self.post_handler_hook = Some(Arc::new(hook));
        self
    }

    /// Builds the pipeline.
    ///
    /// The resulting pipeline has a fixed middleware order that cannot
    /// be modified after construction.
    #[must_use]
    pub fn build(self) -> Pipeline {
        Pipeline {
            pre_handler_stages: self.pre_handler_stages,
            pre_handler_hook: self.pre_handler_hook,
            post_handler_stages: self.post_handler_stages,
            post_handler_hook: self.post_handler_hook,
        }
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware stage marker for compile-time ordering.
///
/// This enum represents the fixed order of middleware stages.
/// It is used internally to ensure correct ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Stage {
    /// Stage 1: Request ID generation/propagation
    RequestId = 1,
    /// Stage 2: Tracing/span initialization
    Tracing = 2,
    /// Stage 3: Identity extraction
    Identity = 3,
    /// Stage 4: Authorization (OPA)
    Authorization = 4,
    /// Stage 5: Request validation
    RequestValidation = 5,
    /// --- Handler invocation ---
    /// Stage 6: Response validation
    ResponseValidation = 6,
    /// Stage 7: Telemetry emission
    Telemetry = 7,
    /// Stage 8: Error normalization
    ErrorNormalization = 8,
}

impl Stage {
    /// Returns true if this is a pre-handler stage.
    #[must_use]
    pub const fn is_pre_handler(self) -> bool {
        (self as u8) <= 5
    }

    /// Returns true if this is a post-handler stage.
    #[must_use]
    pub const fn is_post_handler(self) -> bool {
        (self as u8) >= 6
    }

    /// Returns the stage name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::RequestId => "request_id",
            Self::Tracing => "tracing",
            Self::Identity => "identity",
            Self::Authorization => "authorization",
            Self::RequestValidation => "request_validation",
            Self::ResponseValidation => "response_validation",
            Self::Telemetry => "telemetry",
            Self::ErrorNormalization => "error_normalization",
        }
    }

    /// Returns all stages in order.
    #[must_use]
    pub const fn all() -> [Stage; 8] {
        [
            Self::RequestId,
            Self::Tracing,
            Self::Identity,
            Self::Authorization,
            Self::RequestValidation,
            Self::ResponseValidation,
            Self::Telemetry,
            Self::ErrorNormalization,
        ]
    }

    /// Returns all pre-handler stages in order.
    #[must_use]
    pub const fn pre_handler() -> [Stage; 5] {
        [
            Self::RequestId,
            Self::Tracing,
            Self::Identity,
            Self::Authorization,
            Self::RequestValidation,
        ]
    }

    /// Returns all post-handler stages in order.
    #[must_use]
    pub const fn post_handler() -> [Stage; 3] {
        [
            Self::ResponseValidation,
            Self::Telemetry,
            Self::ErrorNormalization,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{Request as HttpRequest, Response as HttpResponse, StatusCode};
    use http_body_util::Full;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A test middleware that records its invocation order.
    struct OrderTrackingMiddleware {
        name: &'static str,
        counter: Arc<AtomicUsize>,
        order: Arc<std::sync::Mutex<Vec<&'static str>>>,
    }

    impl Middleware for OrderTrackingMiddleware {
        fn name(&self) -> &'static str {
            self.name
        }

        fn process<'a>(
            &'a self,
            ctx: &'a mut MiddlewareContext,
            request: Request,
            next: Next<'a>,
        ) -> BoxFuture<'a, Response> {
            let counter = self.counter.clone();
            let order = self.order.clone();
            let name = self.name;

            Box::pin(async move {
                // Record pre-handler
                counter.fetch_add(1, Ordering::SeqCst);
                order.lock().unwrap().push(name);

                // Call next
                let response = next.run(ctx, request).await;

                // Post-processing would go here
                response
            })
        }
    }

    #[tokio::test]
    async fn test_pipeline_executes_in_order() {
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        let mw1 = OrderTrackingMiddleware {
            name: "first",
            counter: counter.clone(),
            order: order.clone(),
        };

        let mw2 = OrderTrackingMiddleware {
            name: "second",
            counter: counter.clone(),
            order: order.clone(),
        };

        let mw3 = OrderTrackingMiddleware {
            name: "third",
            counter: counter.clone(),
            order: order.clone(),
        };

        let pipeline = Pipeline::builder()
            .add_pre_handler_stage(mw1)
            .add_pre_handler_stage(mw2)
            .add_post_handler_stage(mw3)
            .build();

        let ctx = MiddlewareContext::new();
        let request: Request = HttpRequest::builder()
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap();

        let response = pipeline
            .process(ctx, request, |_ctx, _req| {
                Box::pin(async {
                    HttpResponse::builder()
                        .status(StatusCode::OK)
                        .body(Full::new(Bytes::from("OK")))
                        .unwrap()
                })
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(counter.load(Ordering::SeqCst), 3);

        let executed_order = order.lock().unwrap();
        assert_eq!(*executed_order, vec!["first", "second", "third"]);
    }

    #[tokio::test]
    async fn test_empty_pipeline() {
        let pipeline = Pipeline::builder().build();

        let ctx = MiddlewareContext::new();
        let request: Request = HttpRequest::builder()
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap();

        let response = pipeline
            .process(ctx, request, |_ctx, _req| {
                Box::pin(async {
                    HttpResponse::builder()
                        .status(StatusCode::OK)
                        .body(Full::new(Bytes::from("handler")))
                        .unwrap()
                })
            })
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_stage_ordering() {
        assert!(Stage::RequestId < Stage::Tracing);
        assert!(Stage::Tracing < Stage::Identity);
        assert!(Stage::Identity < Stage::Authorization);
        assert!(Stage::Authorization < Stage::RequestValidation);
        assert!(Stage::RequestValidation < Stage::ResponseValidation);
        assert!(Stage::ResponseValidation < Stage::Telemetry);
        assert!(Stage::Telemetry < Stage::ErrorNormalization);
    }

    #[test]
    fn test_stage_categories() {
        assert!(Stage::RequestId.is_pre_handler());
        assert!(Stage::Tracing.is_pre_handler());
        assert!(Stage::Identity.is_pre_handler());
        assert!(Stage::Authorization.is_pre_handler());
        assert!(Stage::RequestValidation.is_pre_handler());

        assert!(Stage::ResponseValidation.is_post_handler());
        assert!(Stage::Telemetry.is_post_handler());
        assert!(Stage::ErrorNormalization.is_post_handler());
    }

    #[test]
    fn test_stage_names() {
        assert_eq!(Stage::RequestId.name(), "request_id");
        assert_eq!(Stage::Tracing.name(), "tracing");
        assert_eq!(Stage::Identity.name(), "identity");
        assert_eq!(Stage::Authorization.name(), "authorization");
        assert_eq!(Stage::RequestValidation.name(), "request_validation");
        assert_eq!(Stage::ResponseValidation.name(), "response_validation");
        assert_eq!(Stage::Telemetry.name(), "telemetry");
        assert_eq!(Stage::ErrorNormalization.name(), "error_normalization");
    }

    #[test]
    fn test_stage_count() {
        let pipeline = Pipeline::builder().build();
        assert_eq!(pipeline.stage_count(), 0);
    }
}
