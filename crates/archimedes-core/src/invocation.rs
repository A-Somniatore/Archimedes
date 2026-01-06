//! Handler invocation context.
//!
//! The [`InvocationContext`] provides all context needed for handler invocation,
//! including HTTP request details, middleware context, and DI container.

use crate::di::Container;
use crate::RequestContext;
use archimedes_router::Params;
use bytes::Bytes;
use http::{HeaderMap, Method, Uri};
use std::sync::Arc;

/// Complete context for invoking a handler.
///
/// This struct aggregates all the information that macro-generated handlers
/// need to extract parameters and execute business logic.
///
/// # Overview
///
/// `InvocationContext` combines:
/// - **HTTP Request Details**: Method, URI, headers, body, path parameters
/// - **Middleware Context**: Request ID, identity, trace info from [`RequestContext`]
/// - **DI Container**: Optional dependency injection container
///
/// # Example
///
/// ```rust
/// use archimedes_core::InvocationContext;
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// let mut params = Params::new();
/// params.push("id", "123");
///
/// let ctx = InvocationContext::new(
///     Method::GET,
///     Uri::from_static("/users/123"),
///     HeaderMap::new(),
///     Bytes::new(),
///     params,
/// );
///
/// assert_eq!(ctx.method(), &Method::GET);
/// assert_eq!(ctx.path_params().get("id"), Some("123"));
/// ```
#[derive(Debug, Clone)]
pub struct InvocationContext {
    /// HTTP method
    method: Method,
    /// Request URI
    uri: Uri,
    /// Request headers
    headers: HeaderMap,
    /// Request body
    body: Bytes,
    /// Extracted path parameters
    path_params: Params,
    /// Middleware context (identity, request ID, trace info)
    request_context: RequestContext,
    /// Optional DI container for dependency injection
    container: Option<Arc<Container>>,
}

impl InvocationContext {
    /// Creates a new invocation context.
    #[must_use]
    pub fn new(
        method: Method,
        uri: Uri,
        headers: HeaderMap,
        body: Bytes,
        path_params: Params,
    ) -> Self {
        Self {
            method,
            uri,
            headers,
            body,
            path_params,
            request_context: RequestContext::new(),
            container: None,
        }
    }

    /// Creates an invocation context with a request context.
    #[must_use]
    pub fn with_request_context(mut self, ctx: RequestContext) -> Self {
        self.request_context = ctx;
        self
    }

    /// Creates an invocation context with a DI container.
    #[must_use]
    pub fn with_container(mut self, container: Arc<Container>) -> Self {
        self.container = Some(container);
        self
    }

    /// Returns the HTTP method.
    #[must_use]
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Returns the request URI.
    #[must_use]
    pub fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Returns the path portion of the URI.
    #[must_use]
    pub fn path(&self) -> &str {
        self.uri.path()
    }

    /// Returns the query string if present.
    #[must_use]
    pub fn query_string(&self) -> Option<&str> {
        self.uri.query()
    }

    /// Returns the request headers.
    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Returns a specific header value as a string.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Returns the request body.
    #[must_use]
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Returns the extracted path parameters.
    #[must_use]
    pub fn path_params(&self) -> &Params {
        &self.path_params
    }

    /// Returns the request context.
    #[must_use]
    pub fn request_context(&self) -> &RequestContext {
        &self.request_context
    }

    /// Returns a reference to the DI container if available.
    #[must_use]
    pub fn container(&self) -> Option<&Container> {
        self.container.as_deref()
    }

    /// Returns a clone of the DI container Arc if available.
    #[must_use]
    pub fn container_arc(&self) -> Option<Arc<Container>> {
        self.container.clone()
    }
}

/// Builder for creating [`InvocationContext`].
///
/// This is useful for tests where you want to construct contexts
/// with specific values.
#[derive(Debug, Default)]
pub struct InvocationContextBuilder {
    method: Option<Method>,
    uri: Option<Uri>,
    headers: HeaderMap,
    body: Bytes,
    path_params: Params,
    request_context: Option<RequestContext>,
    container: Option<Arc<Container>>,
}

impl InvocationContextBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the HTTP method.
    #[must_use]
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Sets the URI.
    #[must_use]
    pub fn uri(mut self, uri: Uri) -> Self {
        self.uri = Some(uri);
        self
    }

    /// Sets the headers.
    #[must_use]
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }

    /// Adds a single header.
    #[must_use]
    pub fn header(mut self, name: &'static str, value: &str) -> Self {
        if let Ok(value) = value.parse() {
            self.headers.insert(name, value);
        }
        self
    }

    /// Sets the body.
    #[must_use]
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = body.into();
        self
    }

    /// Sets the path parameters.
    #[must_use]
    pub fn path_params(mut self, params: Params) -> Self {
        self.path_params = params;
        self
    }

    /// Adds a single path parameter.
    #[must_use]
    pub fn path_param(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.path_params.push(name, value);
        self
    }

    /// Sets the request context.
    #[must_use]
    pub fn request_context(mut self, ctx: RequestContext) -> Self {
        self.request_context = Some(ctx);
        self
    }

    /// Sets the DI container.
    #[must_use]
    pub fn container(mut self, container: Arc<Container>) -> Self {
        self.container = Some(container);
        self
    }

    /// Builds the invocation context.
    ///
    /// # Panics
    ///
    /// Panics if method or uri were not set.
    #[must_use]
    pub fn build(self) -> InvocationContext {
        InvocationContext {
            method: self.method.expect("method is required"),
            uri: self.uri.expect("uri is required"),
            headers: self.headers,
            body: self.body,
            path_params: self.path_params,
            request_context: self.request_context.unwrap_or_else(RequestContext::new),
            container: self.container,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invocation_context_new() {
        let ctx = InvocationContext::new(
            Method::GET,
            Uri::from_static("/users/123"),
            HeaderMap::new(),
            Bytes::new(),
            Params::new(),
        );

        assert_eq!(ctx.method(), &Method::GET);
        assert_eq!(ctx.path(), "/users/123");
    }

    #[test]
    fn test_invocation_context_with_params() {
        let mut params = Params::new();
        params.push("id", "123");

        let ctx = InvocationContext::new(
            Method::GET,
            Uri::from_static("/users/123"),
            HeaderMap::new(),
            Bytes::new(),
            params,
        );

        assert_eq!(ctx.path_params().get("id"), Some("123"));
    }

    #[test]
    fn test_invocation_context_with_container() {
        let mut container = Container::new();
        container.register(Arc::new(42i32));
        let container = Arc::new(container);

        let ctx = InvocationContext::new(
            Method::GET,
            Uri::from_static("/test"),
            HeaderMap::new(),
            Bytes::new(),
            Params::new(),
        )
        .with_container(container);

        assert!(ctx.container().is_some());
        assert_eq!(
            ctx.container().unwrap().resolve::<i32>(),
            Some(std::sync::Arc::new(42))
        );
    }

    #[test]
    fn test_builder_basic() {
        let ctx = InvocationContextBuilder::new()
            .method(Method::POST)
            .uri(Uri::from_static("/api/users"))
            .body(r#"{"name":"test"}"#)
            .build();

        assert_eq!(ctx.method(), &Method::POST);
        assert_eq!(ctx.path(), "/api/users");
        assert!(!ctx.body().is_empty());
    }

    #[test]
    fn test_builder_with_path_param() {
        let ctx = InvocationContextBuilder::new()
            .method(Method::GET)
            .uri(Uri::from_static("/users/456"))
            .path_param("userId", "456")
            .build();

        assert_eq!(ctx.path_params().get("userId"), Some("456"));
    }

    #[test]
    fn test_builder_with_header() {
        let ctx = InvocationContextBuilder::new()
            .method(Method::GET)
            .uri(Uri::from_static("/test"))
            .header("content-type", "application/json")
            .build();

        assert_eq!(ctx.header("content-type"), Some("application/json"));
    }

    #[test]
    fn test_request_context_access() {
        let request_ctx = RequestContext::new();
        let request_id = request_ctx.request_id();

        let ctx = InvocationContextBuilder::new()
            .method(Method::GET)
            .uri(Uri::from_static("/test"))
            .request_context(request_ctx)
            .build();

        assert_eq!(ctx.request_context().request_id(), request_id);
    }
}
