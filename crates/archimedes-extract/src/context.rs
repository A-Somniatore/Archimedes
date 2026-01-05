//! Extraction context providing access to request data.
//!
//! The [`ExtractionContext`] is the primary interface for extractors to access
//! different parts of an HTTP request.

use archimedes_router::Params;
use bytes::Bytes;
use http::{HeaderMap, Method, Uri};

/// Context providing access to all parts of an HTTP request.
///
/// Extractors use this context to access path parameters, query strings,
/// headers, and the request body. The context is designed to allow
/// extractors to be composable and independent.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::ExtractionContext;
/// use archimedes_router::Params;
/// use http::{HeaderMap, Method, Uri};
/// use bytes::Bytes;
///
/// let mut params = Params::new();
/// params.push("id", "123");
///
/// let ctx = ExtractionContext::new(
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
pub struct ExtractionContext {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
    path_params: Params,
}

impl ExtractionContext {
    /// Creates a new extraction context.
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
        }
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

    /// Returns the request body as bytes.
    #[must_use]
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Consumes the context and returns the body.
    #[must_use]
    pub fn into_body(self) -> Bytes {
        self.body
    }

    /// Returns the extracted path parameters.
    #[must_use]
    pub fn path_params(&self) -> &Params {
        &self.path_params
    }

    /// Returns a mutable reference to path parameters.
    pub fn path_params_mut(&mut self) -> &mut Params {
        &mut self.path_params
    }

    /// Returns a specific header value as a string.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).and_then(|v| v.to_str().ok())
    }

    /// Returns the Content-Type header value.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.header("content-type")
    }

    /// Returns the Content-Length header value.
    #[must_use]
    pub fn content_length(&self) -> Option<u64> {
        self.header("content-length")
            .and_then(|v| v.parse().ok())
    }

    /// Checks if the request body is empty.
    #[must_use]
    pub fn is_body_empty(&self) -> bool {
        self.body.is_empty()
    }
}

/// Builder for constructing an `ExtractionContext`.
#[derive(Debug, Default)]
pub struct ExtractionContextBuilder {
    method: Option<Method>,
    uri: Option<Uri>,
    headers: HeaderMap,
    body: Bytes,
    path_params: Params,
}

impl ExtractionContextBuilder {
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

    /// Builds the extraction context.
    ///
    /// # Panics
    ///
    /// Panics if method or uri were not set.
    #[must_use]
    pub fn build(self) -> ExtractionContext {
        ExtractionContext {
            method: self.method.expect("method is required"),
            uri: self.uri.expect("uri is required"),
            headers: self.headers,
            body: self.body,
            path_params: self.path_params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extraction_context_creation() {
        let mut params = Params::new();
        params.push("userId", "42");

        let ctx = ExtractionContext::new(
            Method::GET,
            Uri::from_static("/users/42?active=true"),
            HeaderMap::new(),
            Bytes::from_static(b""),
            params,
        );

        assert_eq!(ctx.method(), &Method::GET);
        assert_eq!(ctx.path(), "/users/42");
        assert_eq!(ctx.query_string(), Some("active=true"));
        assert_eq!(ctx.path_params().get("userId"), Some("42"));
    }

    #[test]
    fn test_extraction_context_builder() {
        let ctx = ExtractionContextBuilder::new()
            .method(Method::POST)
            .uri(Uri::from_static("/api/users"))
            .header("content-type", "application/json")
            .body(r#"{"name": "Alice"}"#)
            .path_param("version", "v1")
            .build();

        assert_eq!(ctx.method(), &Method::POST);
        assert_eq!(ctx.path(), "/api/users");
        assert_eq!(ctx.content_type(), Some("application/json"));
        assert!(!ctx.is_body_empty());
        assert_eq!(ctx.path_params().get("version"), Some("v1"));
    }

    #[test]
    fn test_header_access() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("content-length", "100".parse().unwrap());
        headers.insert("x-request-id", "abc-123".parse().unwrap());

        let ctx = ExtractionContext::new(
            Method::GET,
            Uri::from_static("/"),
            headers,
            Bytes::new(),
            Params::new(),
        );

        assert_eq!(ctx.content_type(), Some("application/json"));
        assert_eq!(ctx.content_length(), Some(100));
        assert_eq!(ctx.header("x-request-id"), Some("abc-123"));
        assert_eq!(ctx.header("missing"), None);
    }

    #[test]
    fn test_body_operations() {
        let body = Bytes::from_static(b"hello world");
        let ctx = ExtractionContext::new(
            Method::POST,
            Uri::from_static("/"),
            HeaderMap::new(),
            body.clone(),
            Params::new(),
        );

        assert!(!ctx.is_body_empty());
        assert_eq!(ctx.body(), &body);

        let owned_body = ctx.into_body();
        assert_eq!(owned_body, body);
    }

    #[test]
    fn test_empty_body() {
        let ctx = ExtractionContext::new(
            Method::GET,
            Uri::from_static("/"),
            HeaderMap::new(),
            Bytes::new(),
            Params::new(),
        );

        assert!(ctx.is_body_empty());
    }
}
