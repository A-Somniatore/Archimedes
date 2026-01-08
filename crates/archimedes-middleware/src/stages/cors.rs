//! CORS (Cross-Origin Resource Sharing) middleware.
//!
//! This middleware handles CORS preflight requests and adds appropriate
//! CORS headers to responses. It is a **P0 migration blocker** required
//! for any browser-facing API.
//!
//! ## CORS Headers
//!
//! The middleware handles these headers:
//!
//! - `Access-Control-Allow-Origin`: Allowed origins
//! - `Access-Control-Allow-Methods`: Allowed HTTP methods
//! - `Access-Control-Allow-Headers`: Allowed request headers
//! - `Access-Control-Allow-Credentials`: Allow credentials
//! - `Access-Control-Max-Age`: Preflight cache duration
//! - `Access-Control-Expose-Headers`: Headers exposed to JavaScript
//!
//! ## Preflight Requests
//!
//! When a browser makes a cross-origin request with certain conditions
//! (non-simple methods, custom headers, etc.), it first sends an OPTIONS
//! preflight request. This middleware handles preflight requests and returns
//! appropriate CORS headers without invoking the handler.
//!
//! ## Example
//!
//! ```ignore
//! use archimedes_middleware::stages::CorsMiddleware;
//! use http::Method;
//! use std::time::Duration;
//!
//! let cors = CorsMiddleware::builder()
//!     .allow_origin("https://app.example.com")
//!     .allow_origin("https://admin.example.com")
//!     .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
//!     .allow_headers(["Content-Type", "Authorization", "X-Request-ID"])
//!     .allow_credentials(true)
//!     .max_age(Duration::from_secs(3600))
//!     .build();
//! ```

use crate::context::MiddlewareContext;
use crate::middleware::{BoxFuture, Middleware, Next};
use crate::types::{Request, Response};
use bytes::Bytes;
use http::{header, HeaderValue, Method, StatusCode};
use http_body_util::Full;
use std::collections::HashSet;
use std::time::Duration;

/// CORS header names.
pub mod headers {
    /// `Access-Control-Allow-Origin` header.
    pub const ALLOW_ORIGIN: &str = "access-control-allow-origin";
    /// `Access-Control-Allow-Methods` header.
    pub const ALLOW_METHODS: &str = "access-control-allow-methods";
    /// `Access-Control-Allow-Headers` header.
    pub const ALLOW_HEADERS: &str = "access-control-allow-headers";
    /// `Access-Control-Allow-Credentials` header.
    pub const ALLOW_CREDENTIALS: &str = "access-control-allow-credentials";
    /// `Access-Control-Max-Age` header.
    pub const MAX_AGE: &str = "access-control-max-age";
    /// `Access-Control-Expose-Headers` header.
    pub const EXPOSE_HEADERS: &str = "access-control-expose-headers";
    /// `Access-Control-Request-Method` header (preflight).
    pub const REQUEST_METHOD: &str = "access-control-request-method";
    /// `Access-Control-Request-Headers` header (preflight).
    pub const REQUEST_HEADERS: &str = "access-control-request-headers";
    /// `Origin` header.
    pub const ORIGIN: &str = "origin";
    /// `Vary` header.
    pub const VARY: &str = "vary";
}

/// CORS middleware that handles preflight requests and adds CORS headers.
///
/// This middleware must run **before** all other middleware to handle
/// preflight OPTIONS requests early and avoid unnecessary processing.
///
/// # Preflight Handling
///
/// When a preflight OPTIONS request is received, this middleware:
/// 1. Validates the origin against allowed origins
/// 2. Validates the requested method against allowed methods
/// 3. Validates requested headers against allowed headers
/// 4. Returns a 204 No Content response with CORS headers
///
/// # Regular Requests
///
/// For non-preflight requests:
/// 1. Validates the origin against allowed origins
/// 2. Adds `Access-Control-Allow-Origin` header
/// 3. Adds `Access-Control-Allow-Credentials` if configured
/// 4. Adds `Access-Control-Expose-Headers` if configured
/// 5. Continues to next middleware
#[derive(Debug, Clone)]
pub struct CorsMiddleware {
    config: CorsConfig,
}

/// Configuration for CORS middleware.
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins. Empty means no origins allowed.
    /// Use `*` for all origins (not recommended with credentials).
    allowed_origins: AllowedOrigins,
    /// Allowed HTTP methods.
    allowed_methods: HashSet<Method>,
    /// Allowed request headers.
    allowed_headers: HashSet<String>,
    /// Headers exposed to JavaScript.
    expose_headers: HashSet<String>,
    /// Whether to allow credentials (cookies, authorization headers).
    allow_credentials: bool,
    /// Max age for preflight cache (in seconds).
    max_age: Option<Duration>,
}

/// Represents the set of allowed origins.
#[derive(Debug, Clone)]
pub enum AllowedOrigins {
    /// Allow any origin (wildcard `*`).
    Any,
    /// Allow specific origins.
    List(HashSet<String>),
}

impl AllowedOrigins {
    /// Checks if an origin is allowed.
    pub fn is_allowed(&self, origin: &str) -> bool {
        match self {
            AllowedOrigins::Any => true,
            AllowedOrigins::List(origins) => origins.contains(origin),
        }
    }

    /// Returns the header value for a given origin.
    pub fn header_value(&self, origin: &str) -> Option<HeaderValue> {
        match self {
            AllowedOrigins::Any => HeaderValue::from_static("*").into(),
            AllowedOrigins::List(origins) => {
                if origins.contains(origin) {
                    HeaderValue::from_str(origin).ok()
                } else {
                    None
                }
            }
        }
    }
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: AllowedOrigins::List(HashSet::new()),
            allowed_methods: HashSet::from([
                Method::GET,
                Method::HEAD,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
            ]),
            allowed_headers: HashSet::from([
                "content-type".to_string(),
                "authorization".to_string(),
                "x-request-id".to_string(),
            ]),
            expose_headers: HashSet::new(),
            allow_credentials: false,
            max_age: Some(Duration::from_secs(86400)), // 24 hours
        }
    }
}

/// Builder for CORS configuration.
#[derive(Debug, Clone, Default)]
pub struct CorsBuilder {
    config: CorsConfig,
}

impl CorsBuilder {
    /// Creates a new CORS builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Allows any origin (wildcard `*`).
    ///
    /// **Warning**: This should not be used with `allow_credentials(true)`.
    /// Browsers will reject responses with `Access-Control-Allow-Origin: *`
    /// and `Access-Control-Allow-Credentials: true`.
    #[must_use]
    pub fn allow_any_origin(mut self) -> Self {
        self.config.allowed_origins = AllowedOrigins::Any;
        self
    }

    /// Adds an allowed origin.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cors = CorsBuilder::new()
    ///     .allow_origin("https://example.com")
    ///     .allow_origin("https://app.example.com")
    ///     .build();
    /// ```
    #[must_use]
    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        match &mut self.config.allowed_origins {
            AllowedOrigins::Any => {
                // If already allowing any, keep it
            }
            AllowedOrigins::List(origins) => {
                origins.insert(origin.into());
            }
        }
        self
    }

    /// Sets multiple allowed origins.
    #[must_use]
    pub fn allow_origins<I, S>(mut self, origins: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.allowed_origins =
            AllowedOrigins::List(origins.into_iter().map(Into::into).collect());
        self
    }

    /// Sets the allowed HTTP methods.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cors = CorsBuilder::new()
    ///     .allow_methods([Method::GET, Method::POST])
    ///     .build();
    /// ```
    #[must_use]
    pub fn allow_methods<I>(mut self, methods: I) -> Self
    where
        I: IntoIterator<Item = Method>,
    {
        self.config.allowed_methods = methods.into_iter().collect();
        self
    }

    /// Adds an allowed request header.
    #[must_use]
    pub fn allow_header(mut self, header: impl Into<String>) -> Self {
        self.config.allowed_headers.insert(header.into().to_lowercase());
        self
    }

    /// Sets the allowed request headers.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let cors = CorsBuilder::new()
    ///     .allow_headers(["Content-Type", "Authorization", "X-Custom-Header"])
    ///     .build();
    /// ```
    #[must_use]
    pub fn allow_headers<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.allowed_headers = headers.into_iter().map(|h| h.into().to_lowercase()).collect();
        self
    }

    /// Sets headers that should be exposed to JavaScript.
    ///
    /// By default, only simple response headers are exposed. Use this
    /// to expose custom headers like `X-Request-ID`.
    #[must_use]
    pub fn expose_headers<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config.expose_headers = headers.into_iter().map(|h| h.into().to_lowercase()).collect();
        self
    }

    /// Sets whether to allow credentials (cookies, authorization headers).
    ///
    /// **Warning**: Cannot be used with `allow_any_origin()`. If credentials
    /// are allowed, you must specify explicit origins.
    #[must_use]
    pub fn allow_credentials(mut self, allow: bool) -> Self {
        self.config.allow_credentials = allow;
        self
    }

    /// Sets the max age for preflight cache.
    ///
    /// This tells browsers how long they can cache preflight responses.
    /// Default is 24 hours (86400 seconds).
    #[must_use]
    pub fn max_age(mut self, duration: Duration) -> Self {
        self.config.max_age = Some(duration);
        self
    }

    /// Disables preflight caching.
    #[must_use]
    pub fn no_max_age(mut self) -> Self {
        self.config.max_age = None;
        self
    }

    /// Builds the CORS middleware.
    #[must_use]
    pub fn build(self) -> CorsMiddleware {
        CorsMiddleware {
            config: self.config,
        }
    }
}

impl CorsMiddleware {
    /// Creates a new CORS builder.
    #[must_use]
    pub fn builder() -> CorsBuilder {
        CorsBuilder::new()
    }

    /// Creates a permissive CORS middleware that allows any origin.
    ///
    /// **Warning**: This is for development only. Do not use in production.
    #[must_use]
    pub fn permissive() -> Self {
        CorsBuilder::new()
            .allow_any_origin()
            .allow_methods([
                Method::GET,
                Method::HEAD,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ])
            .allow_headers(["*"])
            .expose_headers(["*"])
            .max_age(Duration::from_secs(86400))
            .build()
    }

    /// Checks if a request is a CORS preflight request.
    fn is_preflight(&self, request: &Request) -> bool {
        request.method() == Method::OPTIONS
            && request.headers().contains_key(headers::ORIGIN)
            && request.headers().contains_key(headers::REQUEST_METHOD)
    }

    /// Gets the origin from a request.
    fn get_origin<'a>(&self, request: &'a Request) -> Option<&'a str> {
        request
            .headers()
            .get(headers::ORIGIN)
            .and_then(|v| v.to_str().ok())
    }

    /// Handles a preflight OPTIONS request.
    fn handle_preflight(&self, request: &Request) -> Response {
        let origin = match self.get_origin(request) {
            Some(o) => o,
            None => return self.forbidden_response("Missing Origin header"),
        };

        // Check if origin is allowed
        if !self.config.allowed_origins.is_allowed(origin) {
            return self.forbidden_response("Origin not allowed");
        }

        // Check requested method
        if let Some(requested_method) = request.headers().get(headers::REQUEST_METHOD) {
            if let Ok(method_str) = requested_method.to_str() {
                if let Ok(method) = method_str.parse::<Method>() {
                    if !self.config.allowed_methods.contains(&method) {
                        return self.forbidden_response("Method not allowed");
                    }
                }
            }
        }

        // Check requested headers
        if let Some(requested_headers) = request.headers().get(headers::REQUEST_HEADERS) {
            if let Ok(headers_str) = requested_headers.to_str() {
                for header in headers_str.split(',').map(|h| h.trim().to_lowercase()) {
                    // Allow wildcard header
                    if self.config.allowed_headers.contains("*") {
                        continue;
                    }
                    if !self.config.allowed_headers.contains(&header) {
                        return self.forbidden_response(&format!("Header '{}' not allowed", header));
                    }
                }
            }
        }

        // Build successful preflight response
        self.preflight_response(origin)
    }

    /// Creates a 204 No Content preflight response with CORS headers.
    fn preflight_response(&self, origin: &str) -> Response {
        let mut builder = http::Response::builder().status(StatusCode::NO_CONTENT);

        // Access-Control-Allow-Origin
        if let Some(header_value) = self.config.allowed_origins.header_value(origin) {
            builder = builder.header(headers::ALLOW_ORIGIN, header_value);
        }

        // Access-Control-Allow-Methods
        let methods: Vec<_> = self.config.allowed_methods.iter().map(|m| m.as_str()).collect();
        if !methods.is_empty() {
            builder = builder.header(headers::ALLOW_METHODS, methods.join(", "));
        }

        // Access-Control-Allow-Headers
        let headers_list: Vec<_> = self.config.allowed_headers.iter().cloned().collect();
        if !headers_list.is_empty() {
            builder = builder.header(headers::ALLOW_HEADERS, headers_list.join(", "));
        }

        // Access-Control-Allow-Credentials
        if self.config.allow_credentials {
            builder = builder.header(headers::ALLOW_CREDENTIALS, "true");
        }

        // Access-Control-Max-Age
        if let Some(max_age) = self.config.max_age {
            builder = builder.header(headers::MAX_AGE, max_age.as_secs().to_string());
        }

        // Vary header to indicate caching varies by origin
        builder = builder.header(headers::VARY, "Origin, Access-Control-Request-Method, Access-Control-Request-Headers");

        builder
            .body(Full::new(Bytes::new()))
            .expect("valid response")
    }

    /// Creates a 403 Forbidden response.
    fn forbidden_response(&self, message: &str) -> Response {
        http::Response::builder()
            .status(StatusCode::FORBIDDEN)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Full::new(Bytes::from(message.to_string())))
            .expect("valid response")
    }

    /// Adds CORS headers to a response for non-preflight requests.
    fn add_cors_headers(&self, response: &mut Response, origin: &str) {
        let headers = response.headers_mut();

        // Access-Control-Allow-Origin
        if let Some(header_value) = self.config.allowed_origins.header_value(origin) {
            headers.insert(headers::ALLOW_ORIGIN, header_value);
        }

        // Access-Control-Allow-Credentials
        if self.config.allow_credentials {
            headers.insert(
                headers::ALLOW_CREDENTIALS,
                HeaderValue::from_static("true"),
            );
        }

        // Access-Control-Expose-Headers
        let expose_list: Vec<_> = self.config.expose_headers.iter().cloned().collect();
        if !expose_list.is_empty() {
            if let Ok(value) = HeaderValue::from_str(&expose_list.join(", ")) {
                headers.insert(headers::EXPOSE_HEADERS, value);
            }
        }

        // Vary header
        headers.insert(headers::VARY, HeaderValue::from_static("Origin"));
    }
}

impl Middleware for CorsMiddleware {
    fn name(&self) -> &'static str {
        "cors"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Handle preflight requests early
            if self.is_preflight(&request) {
                return self.handle_preflight(&request);
            }

            // Get origin for non-preflight requests
            let origin = self.get_origin(&request).map(String::from);

            // Process request through remaining middleware
            let mut response = next.run(ctx, request).await;

            // Add CORS headers to response if origin is present and allowed
            if let Some(ref origin) = origin {
                if self.config.allowed_origins.is_allowed(origin) {
                    self.add_cors_headers(&mut response, origin);
                }
            }

            response
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Request as HttpRequest;

    fn create_request_with_origin(method: Method, origin: &str) -> Request {
        HttpRequest::builder()
            .method(method)
            .uri("/test")
            .header(headers::ORIGIN, origin)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_preflight_request(origin: &str, method: &str, headers: Option<&str>) -> Request {
        let mut builder = HttpRequest::builder()
            .method(Method::OPTIONS)
            .uri("/test")
            .header(headers::ORIGIN, origin)
            .header(headers::REQUEST_METHOD, method);

        if let Some(h) = headers {
            builder = builder.header(headers::REQUEST_HEADERS, h);
        }

        builder.body(Full::new(Bytes::new())).unwrap()
    }

    fn create_handler(
    ) -> impl FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> {
        |_ctx, _req| {
            Box::pin(async {
                http::Response::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from("OK")))
                    .unwrap()
            })
        }
    }

    #[test]
    fn test_builder_default() {
        let cors = CorsMiddleware::builder().build();
        assert!(!cors.config.allow_credentials);
        assert!(cors.config.max_age.is_some());
    }

    #[test]
    fn test_builder_allow_origin() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .allow_origin("https://app.example.com")
            .build();

        assert!(cors.config.allowed_origins.is_allowed("https://example.com"));
        assert!(cors.config.allowed_origins.is_allowed("https://app.example.com"));
        assert!(!cors.config.allowed_origins.is_allowed("https://evil.com"));
    }

    #[test]
    fn test_builder_allow_any_origin() {
        let cors = CorsMiddleware::builder().allow_any_origin().build();

        assert!(cors.config.allowed_origins.is_allowed("https://example.com"));
        assert!(cors.config.allowed_origins.is_allowed("https://anything.com"));
    }

    #[test]
    fn test_builder_allow_methods() {
        let cors = CorsMiddleware::builder()
            .allow_methods([Method::GET, Method::POST])
            .build();

        assert!(cors.config.allowed_methods.contains(&Method::GET));
        assert!(cors.config.allowed_methods.contains(&Method::POST));
        assert!(!cors.config.allowed_methods.contains(&Method::DELETE));
    }

    #[test]
    fn test_builder_allow_headers() {
        let cors = CorsMiddleware::builder()
            .allow_headers(["Content-Type", "X-Custom-Header"])
            .build();

        assert!(cors.config.allowed_headers.contains("content-type"));
        assert!(cors.config.allowed_headers.contains("x-custom-header"));
    }

    #[test]
    fn test_builder_allow_credentials() {
        let cors = CorsMiddleware::builder().allow_credentials(true).build();
        assert!(cors.config.allow_credentials);
    }

    #[test]
    fn test_builder_max_age() {
        let cors = CorsMiddleware::builder()
            .max_age(Duration::from_secs(3600))
            .build();
        assert_eq!(cors.config.max_age, Some(Duration::from_secs(3600)));
    }

    #[test]
    fn test_is_preflight() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .build();

        // Valid preflight
        let preflight = create_preflight_request("https://example.com", "POST", None);
        assert!(cors.is_preflight(&preflight));

        // Not a preflight - wrong method
        let get_request = create_request_with_origin(Method::GET, "https://example.com");
        assert!(!cors.is_preflight(&get_request));

        // Not a preflight - OPTIONS without required headers
        let options_no_method = HttpRequest::builder()
            .method(Method::OPTIONS)
            .uri("/test")
            .header(headers::ORIGIN, "https://example.com")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert!(!cors.is_preflight(&options_no_method));
    }

    #[tokio::test]
    async fn test_preflight_allowed_origin() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .allow_methods([Method::GET, Method::POST])
            .build();

        let request = create_preflight_request("https://example.com", "POST", None);
        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            response.headers().get(headers::ALLOW_ORIGIN).unwrap(),
            "https://example.com"
        );
    }

    #[tokio::test]
    async fn test_preflight_disallowed_origin() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .build();

        let request = create_preflight_request("https://evil.com", "POST", None);
        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_preflight_disallowed_method() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .allow_methods([Method::GET])
            .build();

        let request = create_preflight_request("https://example.com", "DELETE", None);
        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_preflight_disallowed_header() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .allow_headers(["Content-Type"])
            .build();

        let request =
            create_preflight_request("https://example.com", "POST", Some("X-Forbidden-Header"));
        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_preflight_with_credentials() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .allow_credentials(true)
            .build();

        let request = create_preflight_request("https://example.com", "POST", None);
        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            response.headers().get(headers::ALLOW_CREDENTIALS).unwrap(),
            "true"
        );
    }

    #[tokio::test]
    async fn test_preflight_max_age() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .max_age(Duration::from_secs(3600))
            .build();

        let request = create_preflight_request("https://example.com", "POST", None);
        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        assert_eq!(
            response.headers().get(headers::MAX_AGE).unwrap(),
            "3600"
        );
    }

    #[tokio::test]
    async fn test_non_preflight_adds_headers() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .allow_credentials(true)
            .expose_headers(["X-Request-ID"])
            .build();

        let request = create_request_with_origin(Method::GET, "https://example.com");
        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(headers::ALLOW_ORIGIN).unwrap(),
            "https://example.com"
        );
        assert_eq!(
            response.headers().get(headers::ALLOW_CREDENTIALS).unwrap(),
            "true"
        );
        assert!(response.headers().contains_key(headers::EXPOSE_HEADERS));
    }

    #[tokio::test]
    async fn test_non_preflight_disallowed_origin_no_headers() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .build();

        let request = create_request_with_origin(Method::GET, "https://evil.com");
        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        // Request still succeeds, but no CORS headers
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key(headers::ALLOW_ORIGIN));
    }

    #[tokio::test]
    async fn test_request_without_origin() {
        let cors = CorsMiddleware::builder()
            .allow_origin("https://example.com")
            .build();

        // Request without Origin header (same-origin or non-browser)
        let request = HttpRequest::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap();

        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        // Request succeeds, no CORS headers needed
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key(headers::ALLOW_ORIGIN));
    }

    #[tokio::test]
    async fn test_permissive_cors() {
        let cors = CorsMiddleware::permissive();

        let request = create_request_with_origin(Method::DELETE, "https://any-origin.com");
        let mut ctx = MiddlewareContext::new();
        let next = Next::handler(create_handler());

        let response = cors.process(&mut ctx, request, next).await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(headers::ALLOW_ORIGIN).unwrap(),
            "*"
        );
    }

    #[test]
    fn test_middleware_name() {
        let cors = CorsMiddleware::builder().build();
        assert_eq!(cors.name(), "cors");
    }
}
