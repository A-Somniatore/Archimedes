//! Request context types.
//!
//! The [`RequestContext`] carries all per-request state through the middleware
//! pipeline and into handlers.

use crate::identity::CallerIdentity;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use uuid::Uuid;

/// A unique identifier for each request, using UUID v7.
///
/// UUID v7 is time-ordered, which makes it ideal for request tracking
/// and log correlation.
///
/// # Example
///
/// ```
/// use archimedes_core::RequestId;
///
/// let id = RequestId::new();
/// println!("Request ID: {}", id);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(Uuid);

impl RequestId {
    /// Creates a new unique request ID using UUID v7.
    ///
    /// UUID v7 incorporates a Unix timestamp, making IDs time-ordered
    /// and suitable for distributed systems.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Creates a `RequestId` from an existing UUID.
    ///
    /// This is useful when parsing request IDs from headers or other sources.
    #[must_use]
    pub const fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the underlying UUID.
    #[must_use]
    pub const fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for RequestId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<RequestId> for Uuid {
    fn from(id: RequestId) -> Self {
        id.0
    }
}

/// Per-request context that flows through the middleware pipeline.
///
/// `RequestContext` carries all the information needed to process a request:
/// - Unique request ID for tracing
/// - Caller identity (authenticated or anonymous)
/// - OpenTelemetry trace/span IDs
/// - Request timing information
/// - Operation metadata
///
/// # Example
///
/// ```
/// use archimedes_core::{RequestContext, CallerIdentity};
///
/// let ctx = RequestContext::new();
/// println!("Processing request: {}", ctx.request_id());
/// ```
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique identifier for this request.
    request_id: RequestId,

    /// The authenticated identity of the caller.
    identity: CallerIdentity,

    /// OpenTelemetry trace ID (hex string).
    trace_id: Option<String>,

    /// OpenTelemetry span ID (hex string).
    span_id: Option<String>,

    /// The operation ID from the contract (e.g., "getUser").
    operation_id: Option<String>,

    /// When the request started processing.
    #[allow(dead_code)]
    started_at: Instant,
}

impl RequestContext {
    /// Creates a new request context with a fresh request ID.
    ///
    /// The identity defaults to [`CallerIdentity::Anonymous`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            request_id: RequestId::new(),
            identity: CallerIdentity::Anonymous,
            trace_id: None,
            span_id: None,
            operation_id: None,
            started_at: Instant::now(),
        }
    }

    /// Creates a new request context with the specified request ID.
    #[must_use]
    pub fn with_request_id(request_id: RequestId) -> Self {
        Self {
            request_id,
            identity: CallerIdentity::Anonymous,
            trace_id: None,
            span_id: None,
            operation_id: None,
            started_at: Instant::now(),
        }
    }

    /// Creates a mock context for testing purposes.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_core::RequestContext;
    ///
    /// let ctx = RequestContext::mock();
    /// // Use ctx in tests...
    /// ```
    #[must_use]
    pub fn mock() -> Self {
        Self::new()
    }

    /// Returns the request ID.
    #[must_use]
    pub const fn request_id(&self) -> RequestId {
        self.request_id
    }

    /// Returns the caller identity.
    #[must_use]
    pub const fn identity(&self) -> &CallerIdentity {
        &self.identity
    }

    /// Sets the caller identity.
    pub fn set_identity(&mut self, identity: CallerIdentity) {
        self.identity = identity;
    }

    /// Returns a new context with the specified identity.
    #[must_use]
    pub fn with_identity(mut self, identity: CallerIdentity) -> Self {
        self.identity = identity;
        self
    }

    /// Returns the trace ID if set.
    #[must_use]
    pub fn trace_id(&self) -> Option<&str> {
        self.trace_id.as_deref()
    }

    /// Sets the OpenTelemetry trace ID.
    pub fn set_trace_id(&mut self, trace_id: impl Into<String>) {
        self.trace_id = Some(trace_id.into());
    }

    /// Returns a new context with the specified trace ID.
    #[must_use]
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Returns the span ID if set.
    #[must_use]
    pub fn span_id(&self) -> Option<&str> {
        self.span_id.as_deref()
    }

    /// Sets the OpenTelemetry span ID.
    pub fn set_span_id(&mut self, span_id: impl Into<String>) {
        self.span_id = Some(span_id.into());
    }

    /// Returns a new context with the specified span ID.
    #[must_use]
    pub fn with_span_id(mut self, span_id: impl Into<String>) -> Self {
        self.span_id = Some(span_id.into());
        self
    }

    /// Returns the operation ID if set.
    #[must_use]
    pub fn operation_id(&self) -> Option<&str> {
        self.operation_id.as_deref()
    }

    /// Sets the operation ID from the contract.
    pub fn set_operation_id(&mut self, operation_id: impl Into<String>) {
        self.operation_id = Some(operation_id.into());
    }

    /// Returns a new context with the specified operation ID.
    #[must_use]
    pub fn with_operation_id(mut self, operation_id: impl Into<String>) -> Self {
        self.operation_id = Some(operation_id.into());
        self
    }

    /// Returns the elapsed time since the request started.
    #[must_use]
    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_new_generates_unique_ids() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();
        assert_ne!(id1, id2, "Each RequestId should be unique");
    }

    #[test]
    fn test_request_id_display() {
        let id = RequestId::new();
        let display = id.to_string();
        // UUID v7 format: xxxxxxxx-xxxx-7xxx-xxxx-xxxxxxxxxxxx
        assert_eq!(display.len(), 36, "UUID string should be 36 characters");
        assert!(display.contains('-'), "UUID should contain hyphens");
    }

    #[test]
    fn test_request_id_from_uuid() {
        let uuid = Uuid::now_v7();
        let id = RequestId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn test_request_id_serialization() {
        let id = RequestId::new();
        let json = serde_json::to_string(&id).expect("serialization should work");
        let parsed: RequestId = serde_json::from_str(&json).expect("deserialization should work");
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new();
        assert!(matches!(ctx.identity(), CallerIdentity::Anonymous));
        assert!(ctx.trace_id().is_none());
        assert!(ctx.operation_id().is_none());
    }

    #[test]
    fn test_request_context_builder_pattern() {
        let ctx = RequestContext::new()
            .with_trace_id("abc123")
            .with_span_id("def456")
            .with_operation_id("getUser");

        assert_eq!(ctx.trace_id(), Some("abc123"));
        assert_eq!(ctx.span_id(), Some("def456"));
        assert_eq!(ctx.operation_id(), Some("getUser"));
    }

    #[test]
    fn test_request_context_elapsed() {
        let ctx = RequestContext::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = ctx.elapsed();
        assert!(elapsed >= std::time::Duration::from_millis(10));
    }
}
