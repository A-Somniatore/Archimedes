//! Core middleware trait and types.
//!
//! This module defines the [`Middleware`] trait that all middleware stages implement.
//! Middleware processes requests before they reach handlers and responses after
//! handlers complete.
//!
//! # Design Philosophy
//!
//! Archimedes uses a fixed-order middleware pipeline. Unlike general-purpose
//! frameworks, middleware cannot be reordered, disabled, or inserted between
//! core stages. This ensures consistent behavior across all services.
//!
//! # Example
//!
//! ```ignore
//! use archimedes_middleware::{Middleware, Next, Request, Response, BoxFuture};
//! use archimedes_middleware::context::MiddlewareContext;
//!
//! struct LoggingMiddleware;
//!
//! impl Middleware for LoggingMiddleware {
//!     fn name(&self) -> &'static str {
//!         "logging"
//!     }
//!
//!     fn process<'a>(
//!         &'a self,
//!         ctx: &'a mut MiddlewareContext,
//!         request: Request,
//!         next: Next<'a>,
//!     ) -> BoxFuture<'a, Response> {
//!         Box::pin(async move {
//!             println!("Request: {:?}", ctx.request_id());
//!             let response = next.run(ctx, request).await;
//!             println!("Response: {:?}", response.status());
//!             response
//!         })
//!     }
//! }
//! ```

use crate::context::MiddlewareContext;
use crate::types::{Request, Response};
use std::future::Future;
use std::pin::Pin;

/// A boxed future that returns a response.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The core middleware trait.
///
/// All middleware stages implement this trait. Middleware receives a mutable
/// context, the incoming request, and a [`Next`] callback to invoke the
/// next middleware in the chain.
///
/// # Invariants
///
/// - Middleware MUST call `next.run()` exactly once (unless short-circuiting)
/// - Middleware SHOULD NOT suppress errors from downstream middleware
/// - Middleware MUST NOT modify the pipeline order
///
/// # Example
///
/// ```ignore
/// impl Middleware for MyMiddleware {
///     fn name(&self) -> &'static str { "my-middleware" }
///
///     async fn process(
///         &self,
///         ctx: &mut MiddlewareContext,
///         request: Request,
///         next: Next<'_>,
///     ) -> Response {
///         // Pre-processing
///         let response = next.run(ctx, request).await;
///         // Post-processing
///         response
///     }
/// }
/// ```
pub trait Middleware: Send + Sync + 'static {
    /// Returns the unique name of this middleware stage.
    ///
    /// This name is used for logging, metrics, and debugging.
    fn name(&self) -> &'static str;

    /// Process the request through this middleware.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The mutable middleware context
    /// * `request` - The incoming HTTP request
    /// * `next` - Callback to invoke the next middleware
    ///
    /// # Returns
    ///
    /// The HTTP response (either from downstream or generated here)
    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response>;
}

/// Callback to invoke the next middleware in the chain.
///
/// This type is passed to middleware and must be called (exactly once)
/// to continue processing. If not called, the middleware short-circuits
/// the pipeline and returns its own response.
pub struct Next<'a> {
    /// The remaining middleware chain
    inner: NextInner<'a>,
}

/// Internal representation of the next middleware chain.
enum NextInner<'a> {
    /// More middleware to process
    Chain {
        middleware: &'a dyn Middleware,
        next: Box<Next<'a>>,
    },
    /// End of chain - invoke the handler
    Handler(Box<dyn FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> + Send + 'a>),
}

impl<'a> Next<'a> {
    /// Creates a new `Next` that will invoke the given middleware.
    pub(crate) fn new(middleware: &'a dyn Middleware, next: Next<'a>) -> Self {
        Self {
            inner: NextInner::Chain {
                middleware,
                next: Box::new(next),
            },
        }
    }

    /// Creates a terminal `Next` that invokes the handler.
    pub(crate) fn handler<F>(f: F) -> Self
    where
        F: FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> + Send + 'a,
    {
        Self {
            inner: NextInner::Handler(Box::new(f)),
        }
    }

    /// Invokes the next middleware or handler in the chain.
    ///
    /// This consumes `self` to ensure it can only be called once.
    pub async fn run(self, ctx: &mut MiddlewareContext, request: Request) -> Response {
        match self.inner {
            NextInner::Chain { middleware, next } => {
                middleware.process(ctx, request, *next).await
            }
            NextInner::Handler(handler) => handler(ctx, request).await,
        }
    }
}

/// A middleware that can be created from an async function.
///
/// This allows defining simple middleware without implementing the trait directly.
///
/// # Example
///
/// ```ignore
/// let middleware = FnMiddleware::new("timing", |ctx, req, next| async move {
///     let start = Instant::now();
///     let response = next.run(ctx, req).await;
///     println!("Request took {:?}", start.elapsed());
///     response
/// });
/// ```
pub struct FnMiddleware<F> {
    name: &'static str,
    func: F,
}

impl<F> FnMiddleware<F> {
    /// Creates a new function-based middleware.
    pub const fn new(name: &'static str, func: F) -> Self {
        Self { name, func }
    }
}

impl<F, Fut> Middleware for FnMiddleware<F>
where
    F: Fn(&mut MiddlewareContext, Request, Next<'_>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn name(&self) -> &'static str {
        self.name
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // We need to be careful here - the closure borrows self
            // so we need to invoke it in a way that respects lifetimes
            (self.func)(ctx, request, next).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Request as HttpRequest, Response as HttpResponse, StatusCode};
    use http_body_util::Full;
    use bytes::Bytes;

    struct TestMiddleware {
        name: &'static str,
    }

    impl Middleware for TestMiddleware {
        fn name(&self) -> &'static str {
            self.name
        }

        fn process<'a>(
            &'a self,
            ctx: &'a mut MiddlewareContext,
            request: Request,
            next: Next<'a>,
        ) -> BoxFuture<'a, Response> {
            Box::pin(async move {
                // Record that this middleware was called
                ctx.set_extension(format!("visited:{}", self.name));
                next.run(ctx, request).await
            })
        }
    }

    #[tokio::test]
    async fn test_middleware_name() {
        let mw = TestMiddleware { name: "test" };
        assert_eq!(mw.name(), "test");
    }

    #[tokio::test]
    async fn test_next_handler() {
        let mut ctx = MiddlewareContext::new();
        let request: Request = HttpRequest::builder()
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap();

        let next = Next::handler(|_ctx, _req| {
            Box::pin(async {
                HttpResponse::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from("OK")))
                    .unwrap()
            })
        });

        let response = next.run(&mut ctx, request).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_chain() {
        let mw1 = TestMiddleware { name: "first" };
        let mw2 = TestMiddleware { name: "second" };

        let mut ctx = MiddlewareContext::new();
        let request: Request = HttpRequest::builder()
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap();

        // Build chain: mw1 -> mw2 -> handler
        let handler = Next::handler(|_ctx, _req| {
            Box::pin(async {
                HttpResponse::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from("OK")))
                    .unwrap()
            })
        });

        let next2 = Next::new(&mw2, handler);
        let next1 = Next::new(&mw1, next2);

        let response = next1.run(&mut ctx, request).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
