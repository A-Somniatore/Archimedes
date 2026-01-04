//! Middleware context types.
//!
//! The [`MiddlewareContext`] carries state through the middleware pipeline.
//! It is separate from [`RequestContext`] to allow middleware to modify
//! context before the final context is passed to handlers.

use archimedes_core::{CallerIdentity, RequestId};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::time::Instant;

/// Context that flows through the middleware pipeline.
///
/// This context is mutable during middleware processing, allowing each
/// middleware stage to enrich it with extracted information (identity,
/// operation ID, etc.). Once the pipeline is complete, it is converted
/// to an immutable [`RequestContext`] for the handler.
///
/// # Example
///
/// ```
/// use archimedes_middleware::context::MiddlewareContext;
/// use archimedes_core::CallerIdentity;
///
/// let mut ctx = MiddlewareContext::new();
/// ctx.set_identity(CallerIdentity::User {
///     user_id: "user-123".to_string(),
///     email: Some("alice@example.com".to_string()),
///     name: Some("Alice".to_string()),
///     roles: vec!["admin".to_string()],
/// });
///
/// assert!(matches!(ctx.identity(), CallerIdentity::User { .. }));
/// ```
#[derive(Debug)]
pub struct MiddlewareContext {
    /// Unique identifier for this request.
    request_id: RequestId,

    /// The authenticated identity of the caller.
    identity: CallerIdentity,

    /// OpenTelemetry trace ID (hex string).
    trace_id: Option<String>,

    /// OpenTelemetry span ID (hex string).
    span_id: Option<String>,

    /// The resolved operation ID from the contract.
    operation_id: Option<String>,

    /// When the request started processing.
    started_at: Instant,

    /// Type-erased extension data.
    ///
    /// Middleware can store arbitrary data here using type-safe keys.
    extensions: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl MiddlewareContext {
    /// Creates a new middleware context with a fresh request ID.
    #[must_use]
    pub fn new() -> Self {
        Self {
            request_id: RequestId::new(),
            identity: CallerIdentity::Anonymous,
            trace_id: None,
            span_id: None,
            operation_id: None,
            started_at: Instant::now(),
            extensions: HashMap::new(),
        }
    }

    /// Creates a context with a specific request ID.
    ///
    /// Useful when the request ID was provided by a client or upstream service.
    #[must_use]
    pub fn with_request_id(request_id: RequestId) -> Self {
        Self {
            request_id,
            identity: CallerIdentity::Anonymous,
            trace_id: None,
            span_id: None,
            operation_id: None,
            started_at: Instant::now(),
            extensions: HashMap::new(),
        }
    }

    /// Returns the request ID.
    #[must_use]
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Sets the request ID.
    ///
    /// This should only be called by the RequestId middleware.
    pub fn set_request_id(&mut self, request_id: RequestId) {
        self.request_id = request_id;
    }

    /// Returns the caller identity.
    #[must_use]
    pub fn identity(&self) -> &CallerIdentity {
        &self.identity
    }

    /// Sets the caller identity.
    ///
    /// This should only be called by the Identity middleware.
    pub fn set_identity(&mut self, identity: CallerIdentity) {
        self.identity = identity;
    }

    /// Returns the trace ID, if set.
    #[must_use]
    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id.as_deref()
    }

    /// Sets the trace ID.
    ///
    /// This should only be called by the Tracing middleware.
    pub fn set_trace_id(&mut self, trace_id: String) {
        self.trace_id = Some(trace_id);
    }

    /// Returns the span ID, if set.
    #[must_use]
    pub fn span_id(&self) -> Option<&str> {
        self.span_id.as_deref()
    }

    /// Sets the span ID.
    ///
    /// This should only be called by the Tracing middleware.
    pub fn set_span_id(&mut self, span_id: String) {
        self.span_id = Some(span_id);
    }

    /// Returns the operation ID, if resolved.
    #[must_use]
    pub fn operation_id(&self) -> Option<&str> {
        self.operation_id.as_deref()
    }

    /// Sets the operation ID.
    ///
    /// This is set after routing resolves the path to an operation.
    pub fn set_operation_id(&mut self, operation_id: String) {
        self.operation_id = Some(operation_id);
    }

    /// Returns when the request started processing.
    #[must_use]
    pub fn started_at(&self) -> Instant {
        self.started_at
    }

    /// Returns the elapsed time since the request started.
    #[must_use]
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }

    /// Stores a typed extension value.
    ///
    /// Extensions allow middleware to store arbitrary data that can be
    /// retrieved by later middleware or handlers.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_middleware::context::MiddlewareContext;
    ///
    /// #[derive(Clone)]
    /// struct RateLimitInfo {
    ///     remaining: u32,
    /// }
    ///
    /// let mut ctx = MiddlewareContext::new();
    /// ctx.set_extension(RateLimitInfo { remaining: 100 });
    ///
    /// let info = ctx.get_extension::<RateLimitInfo>().unwrap();
    /// assert_eq!(info.remaining, 100);
    /// ```
    pub fn set_extension<T: Send + Sync + 'static>(&mut self, value: T) {
        self.extensions.insert(TypeId::of::<T>(), Box::new(value));
    }

    /// Retrieves a typed extension value.
    ///
    /// Returns `None` if no extension of the given type was stored.
    #[must_use]
    pub fn get_extension<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.extensions
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref())
    }

    /// Removes and returns a typed extension value.
    pub fn remove_extension<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.extensions
            .remove(&TypeId::of::<T>())
            .and_then(|v| v.downcast().ok())
            .map(|b| *b)
    }

    /// Checks if an extension of the given type exists.
    #[must_use]
    pub fn has_extension<T: Send + Sync + 'static>(&self) -> bool {
        self.extensions.contains_key(&TypeId::of::<T>())
    }

    /// Converts this middleware context to a [`RequestContext`].
    ///
    /// This is called after all pre-handler middleware has run, before
    /// invoking the handler.
    #[must_use]
    pub fn to_request_context(&self) -> archimedes_core::RequestContext {
        let mut ctx = archimedes_core::RequestContext::with_request_id(self.request_id);
        ctx = ctx.with_identity(self.identity.clone());

        if let Some(trace_id) = &self.trace_id {
            ctx = ctx.with_trace_id(trace_id.clone());
        }

        if let Some(span_id) = &self.span_id {
            ctx = ctx.with_span_id(span_id.clone());
        }

        if let Some(op_id) = &self.operation_id {
            ctx = ctx.with_operation_id(op_id.clone());
        }

        ctx
    }
}

impl Default for MiddlewareContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MiddlewareContext {
    fn clone(&self) -> Self {
        // Note: Extensions are not cloned - they don't implement Clone
        Self {
            request_id: self.request_id,
            identity: self.identity.clone(),
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            operation_id: self.operation_id.clone(),
            started_at: self.started_at,
            extensions: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_context_has_anonymous_identity() {
        let ctx = MiddlewareContext::new();
        assert!(matches!(ctx.identity(), CallerIdentity::Anonymous));
    }

    #[test]
    fn test_set_identity() {
        let mut ctx = MiddlewareContext::new();
        ctx.set_identity(CallerIdentity::User {
            user_id: "u123".to_string(),
            email: Some("alice@example.com".to_string()),
            name: Some("Alice".to_string()),
            roles: vec!["admin".to_string()],
        });

        match ctx.identity() {
            CallerIdentity::User { user_id, email, name, roles } => {
                assert_eq!(user_id, "u123");
                assert_eq!(email, &Some("alice@example.com".to_string()));
                assert_eq!(name, &Some("Alice".to_string()));
                assert_eq!(roles, &vec!["admin".to_string()]);
            }
            _ => panic!("Expected User identity"),
        }
    }

    #[test]
    fn test_set_trace_context() {
        let mut ctx = MiddlewareContext::new();
        ctx.set_trace_id("abc123".to_string());
        ctx.set_span_id("def456".to_string());

        assert_eq!(ctx.trace_id(), Some("abc123"));
        assert_eq!(ctx.span_id(), Some("def456"));
    }

    #[test]
    fn test_set_operation_id() {
        let mut ctx = MiddlewareContext::new();
        assert!(ctx.operation_id().is_none());

        ctx.set_operation_id("getUser".to_string());
        assert_eq!(ctx.operation_id(), Some("getUser"));
    }

    #[test]
    fn test_extensions() {
        #[derive(Debug, Clone, PartialEq)]
        struct MyExtension {
            value: i32,
        }

        let mut ctx = MiddlewareContext::new();

        // Initially no extension
        assert!(!ctx.has_extension::<MyExtension>());
        assert!(ctx.get_extension::<MyExtension>().is_none());

        // Set extension
        ctx.set_extension(MyExtension { value: 42 });
        assert!(ctx.has_extension::<MyExtension>());
        assert_eq!(ctx.get_extension::<MyExtension>(), Some(&MyExtension { value: 42 }));

        // Remove extension
        let removed = ctx.remove_extension::<MyExtension>();
        assert_eq!(removed, Some(MyExtension { value: 42 }));
        assert!(!ctx.has_extension::<MyExtension>());
    }

    #[test]
    fn test_elapsed_time() {
        let ctx = MiddlewareContext::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(ctx.elapsed() >= std::time::Duration::from_millis(10));
    }

    #[test]
    fn test_to_request_context() {
        let mut ctx = MiddlewareContext::new();
        ctx.set_identity(CallerIdentity::User {
            user_id: "u123".to_string(),
            email: None,
            name: None,
            roles: vec![],
        });
        ctx.set_trace_id("trace-123".to_string());
        ctx.set_span_id("span-456".to_string());
        ctx.set_operation_id("createUser".to_string());

        let req_ctx = ctx.to_request_context();
        assert_eq!(req_ctx.request_id(), ctx.request_id());
        assert_eq!(req_ctx.trace_id(), Some("trace-123"));
        assert_eq!(req_ctx.span_id(), Some("span-456"));
        assert_eq!(req_ctx.operation_id(), Some("createUser"));
    }
}
