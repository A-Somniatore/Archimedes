//! Rate limiting middleware.
//!
//! This middleware enforces request rate limits to protect services from
//! abuse and ensure fair resource allocation. It is a **P1 migration blocker**
//! required for production APIs.
//!
//! ## Rate Limiting Strategies
//!
//! The middleware supports multiple rate limiting strategies:
//!
//! - **Per-IP**: Limit requests from a single IP address
//! - **Per-User**: Limit requests from an authenticated user
//! - **Per-API-Key**: Limit requests by API key
//! - **Global**: Limit total requests across all clients
//!
//! ## Algorithm
//!
//! Uses a sliding window algorithm for accurate rate limiting:
//!
//! - Tracks request counts in time windows
//! - Smoothly transitions between windows
//! - More accurate than fixed window approach
//!
//! ## Example
//!
//! ```ignore
//! use archimedes_middleware::stages::RateLimitMiddleware;
//! use std::time::Duration;
//!
//! let rate_limit = RateLimitMiddleware::builder()
//!     .limit(100)
//!     .window(Duration::from_secs(60))
//!     .key_extractor(KeyExtractor::Ip)
//!     .build();
//! ```

use crate::context::MiddlewareContext;
use crate::middleware::{BoxFuture, Middleware, Next};
use crate::types::{Request, Response};
use archimedes_core::CallerIdentity;
use bytes::Bytes;
use http::{header, HeaderValue, StatusCode};
use http_body_util::Full;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Rate limit header names.
pub mod headers {
    /// Maximum requests allowed in the window.
    pub const LIMIT: &str = "x-ratelimit-limit";
    /// Remaining requests in current window.
    pub const REMAINING: &str = "x-ratelimit-remaining";
    /// Unix timestamp when the window resets.
    pub const RESET: &str = "x-ratelimit-reset";
    /// Seconds until the window resets.
    pub const RESET_AFTER: &str = "x-ratelimit-reset-after";
    /// Seconds to wait before retrying (on 429).
    pub const RETRY_AFTER: &str = "retry-after";
}

/// Rate limiting middleware.
///
/// This middleware tracks request rates and rejects requests that exceed
/// the configured limit with a `429 Too Many Requests` response.
///
/// # Response Headers
///
/// The middleware adds these headers to all responses:
///
/// - `X-RateLimit-Limit`: Maximum requests allowed
/// - `X-RateLimit-Remaining`: Remaining requests in window
/// - `X-RateLimit-Reset`: Unix timestamp when window resets
///
/// On rate limit exceeded (429), it also adds:
///
/// - `Retry-After`: Seconds until requests are allowed again
#[derive(Debug)]
pub struct RateLimitMiddleware {
    config: RateLimitConfig,
    store: Arc<Mutex<RateLimitStore>>,
}

/// Configuration for rate limiting middleware.
#[derive(Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed per window.
    limit: u64,
    /// Time window for rate limiting.
    window: Duration,
    /// How to extract the rate limit key from requests.
    key_extractor: KeyExtractor,
    /// Whether to skip rate limiting for certain requests.
    skip_predicate: Option<Arc<dyn Fn(&Request) -> bool + Send + Sync>>,
    /// Message to return when rate limited.
    error_message: String,
}

impl Clone for RateLimitMiddleware {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            store: Arc::clone(&self.store),
        }
    }
}

/// How to extract the rate limit key from a request.
#[derive(Clone, Default)]
pub enum KeyExtractor {
    /// Use client IP address as the key.
    #[default]
    Ip,
    /// Use a specific header value as the key.
    Header(String),
    /// Use the authenticated user ID as the key.
    UserId,
    /// Use a custom function to extract the key.
    Custom(Arc<dyn Fn(&Request) -> Option<String> + Send + Sync>),
    /// Global rate limit (single key for all requests).
    Global,
}

impl std::fmt::Debug for KeyExtractor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ip => write!(f, "KeyExtractor::Ip"),
            Self::Header(h) => f.debug_tuple("KeyExtractor::Header").field(h).finish(),
            Self::UserId => write!(f, "KeyExtractor::UserId"),
            Self::Custom(_) => write!(f, "KeyExtractor::Custom(<fn>)"),
            Self::Global => write!(f, "KeyExtractor::Global"),
        }
    }
}

impl std::fmt::Debug for RateLimitConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimitConfig")
            .field("limit", &self.limit)
            .field("window", &self.window)
            .field("key_extractor", &self.key_extractor)
            .field("skip_predicate", &self.skip_predicate.is_some())
            .field("error_message", &self.error_message)
            .finish()
    }
}

/// Internal store for rate limit tracking.
#[derive(Debug, Default)]
struct RateLimitStore {
    /// Map from key to window data.
    windows: HashMap<String, WindowData>,
}

/// Data for a single rate limit window.
#[derive(Debug, Clone)]
struct WindowData {
    /// Number of requests in current window.
    count: u64,
    /// When the window started.
    window_start: Instant,
    /// Number of requests in previous window (for sliding calculation).
    prev_count: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            limit: 100,
            window: Duration::from_secs(60),
            key_extractor: KeyExtractor::default(),
            skip_predicate: None,
            error_message: "Too many requests. Please try again later.".to_string(),
        }
    }
}

/// Builder for rate limit configuration.
#[derive(Debug, Clone, Default)]
pub struct RateLimitBuilder {
    config: RateLimitConfig,
}

impl RateLimitBuilder {
    /// Creates a new rate limit builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of requests allowed per window.
    ///
    /// Default: 100 requests.
    #[must_use]
    pub fn limit(mut self, limit: u64) -> Self {
        self.config.limit = limit;
        self
    }

    /// Sets the time window for rate limiting.
    ///
    /// Default: 60 seconds.
    #[must_use]
    pub fn window(mut self, window: Duration) -> Self {
        self.config.window = window;
        self
    }

    /// Sets the time window in seconds.
    #[must_use]
    pub fn window_secs(self, seconds: u64) -> Self {
        self.window(Duration::from_secs(seconds))
    }

    /// Uses IP address as the rate limit key.
    #[must_use]
    pub fn per_ip(mut self) -> Self {
        self.config.key_extractor = KeyExtractor::Ip;
        self
    }

    /// Uses a header value as the rate limit key.
    ///
    /// This is useful for API key-based rate limiting.
    #[must_use]
    pub fn per_header(mut self, header_name: impl Into<String>) -> Self {
        self.config.key_extractor = KeyExtractor::Header(header_name.into());
        self
    }

    /// Uses the authenticated user ID as the rate limit key.
    ///
    /// Requires identity middleware to be configured.
    #[must_use]
    pub fn per_user(mut self) -> Self {
        self.config.key_extractor = KeyExtractor::UserId;
        self
    }

    /// Uses a global rate limit (single limit for all requests).
    #[must_use]
    pub fn global(mut self) -> Self {
        self.config.key_extractor = KeyExtractor::Global;
        self
    }

    /// Uses a custom key extractor function.
    #[must_use]
    pub fn key_extractor<F>(mut self, f: F) -> Self
    where
        F: Fn(&Request) -> Option<String> + Send + Sync + 'static,
    {
        self.config.key_extractor = KeyExtractor::Custom(Arc::new(f));
        self
    }

    /// Sets a predicate to skip rate limiting for certain requests.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Skip rate limiting for health checks
    /// builder.skip(|req| req.uri().path() == "/health")
    /// ```
    #[must_use]
    pub fn skip<F>(mut self, f: F) -> Self
    where
        F: Fn(&Request) -> bool + Send + Sync + 'static,
    {
        self.config.skip_predicate = Some(Arc::new(f));
        self
    }

    /// Sets the error message returned when rate limited.
    #[must_use]
    pub fn error_message(mut self, message: impl Into<String>) -> Self {
        self.config.error_message = message.into();
        self
    }

    /// Builds the rate limit middleware.
    #[must_use]
    pub fn build(self) -> RateLimitMiddleware {
        RateLimitMiddleware {
            config: self.config,
            store: Arc::new(Mutex::new(RateLimitStore::default())),
        }
    }
}

impl RateLimitMiddleware {
    /// Creates a new rate limit builder.
    #[must_use]
    pub fn builder() -> RateLimitBuilder {
        RateLimitBuilder::new()
    }

    /// Creates a rate limit middleware with default settings (100 req/min per IP).
    #[must_use]
    pub fn default_limits() -> Self {
        RateLimitBuilder::new().build()
    }

    /// Creates a strict rate limit (10 req/min per IP).
    #[must_use]
    pub fn strict() -> Self {
        RateLimitBuilder::new()
            .limit(10)
            .window_secs(60)
            .per_ip()
            .build()
    }

    /// Creates a lenient rate limit (1000 req/min per IP).
    #[must_use]
    pub fn lenient() -> Self {
        RateLimitBuilder::new()
            .limit(1000)
            .window_secs(60)
            .per_ip()
            .build()
    }

    /// Returns the rate limit configuration.
    #[must_use]
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Extracts the rate limit key from a request.
    fn extract_key(&self, request: &Request, ctx: &MiddlewareContext) -> Option<String> {
        match &self.config.key_extractor {
            KeyExtractor::Ip => {
                // Try X-Forwarded-For, X-Real-IP, then fall back to connection IP
                if let Some(xff) = request.headers().get("x-forwarded-for") {
                    if let Ok(value) = xff.to_str() {
                        // X-Forwarded-For can contain multiple IPs, take the first
                        return Some(value.split(',').next()?.trim().to_string());
                    }
                }
                if let Some(real_ip) = request.headers().get("x-real-ip") {
                    if let Ok(value) = real_ip.to_str() {
                        return Some(value.to_string());
                    }
                }
                // Fall back to a default key
                Some("unknown-ip".to_string())
            }
            KeyExtractor::Header(header_name) => request
                .headers()
                .get(header_name)
                .and_then(|v| v.to_str().ok())
                .map(String::from),
            KeyExtractor::UserId => {
                // Get user ID from context (set by identity middleware)
                match ctx.identity() {
                    CallerIdentity::User(user) => Some(user.user_id.clone()),
                    CallerIdentity::ApiKey(api_key) => Some(api_key.key_id.clone()),
                    CallerIdentity::Spiffe(spiffe) => Some(spiffe.spiffe_id.clone()),
                    CallerIdentity::Anonymous => None,
                }
            }
            KeyExtractor::Custom(f) => f(request),
            KeyExtractor::Global => Some("global".to_string()),
        }
    }

    /// Checks and updates the rate limit for a key.
    #[allow(clippy::significant_drop_tightening)]
    async fn check_rate_limit(&self, key: &str) -> RateLimitResult {
        let mut store = self.store.lock().await;
        let now = Instant::now();
        let window = self.config.window;
        let limit = self.config.limit;

        let window_data = store.windows.entry(key.to_string()).or_insert_with(|| {
            WindowData {
                count: 0,
                window_start: now,
                prev_count: 0,
            }
        });

        // Check if we need to advance to a new window
        let elapsed = now.duration_since(window_data.window_start);
        if elapsed >= window {
            // Move to new window
            let windows_passed = elapsed.as_secs() / window.as_secs();
            if windows_passed >= 2 {
                // More than 2 windows passed, reset completely
                window_data.prev_count = 0;
            } else {
                // Move current to previous
                window_data.prev_count = window_data.count;
            }
            window_data.count = 0;
            window_data.window_start = now;
        }

        // Calculate sliding window count
        // Weight the previous window's count by how much of the current window has elapsed
        let window_progress = now
            .duration_since(window_data.window_start)
            .as_secs_f64()
            / window.as_secs_f64();
        let prev_weight = 1.0 - window_progress;

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let weighted_count =
            window_data.count + (window_data.prev_count as f64 * prev_weight) as u64;

        let elapsed_in_window = now.duration_since(window_data.window_start);
        let reset_in = window.saturating_sub(elapsed_in_window);

        if weighted_count >= limit {
            // Rate limited
            RateLimitResult::Limited {
                limit,
                remaining: 0,
                reset_in,
            }
        } else {
            // Allowed, increment counter
            window_data.count += 1;
            let remaining = limit.saturating_sub(weighted_count + 1);
            RateLimitResult::Allowed {
                limit,
                remaining,
                reset_in,
            }
        }
    }

    /// Builds a 429 Too Many Requests response.
    fn build_rate_limit_response(&self, limit: u64, reset_in: Duration) -> Response {
        let retry_after = reset_in.as_secs().max(1);
        let reset_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + retry_after;

        let body = serde_json::json!({
            "error": {
                "code": "RATE_LIMITED",
                "message": self.config.error_message,
            }
        });

        http::Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(header::CONTENT_TYPE, "application/json")
            .header(headers::LIMIT, limit.to_string())
            .header(headers::REMAINING, "0")
            .header(headers::RESET, reset_timestamp.to_string())
            .header(headers::RESET_AFTER, retry_after.to_string())
            .header(headers::RETRY_AFTER, retry_after.to_string())
            .body(Full::new(Bytes::from(body.to_string())))
            .expect("failed to build rate limit response")
    }

    /// Adds rate limit headers to a response.
    fn add_rate_limit_headers(
        mut response: Response,
        limit: u64,
        remaining: u64,
        reset_in: Duration,
    ) -> Response {
        let reset_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + reset_in.as_secs();

        let headers = response.headers_mut();
        headers.insert(headers::LIMIT, HeaderValue::from(limit));
        headers.insert(headers::REMAINING, HeaderValue::from(remaining));
        headers.insert(
            headers::RESET,
            HeaderValue::from_str(&reset_timestamp.to_string()).unwrap_or_else(|_| {
                HeaderValue::from_static("0")
            }),
        );

        response
    }
}

/// Result of a rate limit check.
#[derive(Debug, Clone)]
enum RateLimitResult {
    /// Request is allowed.
    Allowed {
        limit: u64,
        remaining: u64,
        reset_in: Duration,
    },
    /// Request is rate limited.
    Limited {
        limit: u64,
        #[allow(dead_code)]
        remaining: u64,
        reset_in: Duration,
    },
}

impl Middleware for RateLimitMiddleware {
    fn name(&self) -> &'static str {
        "rate-limit"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Check if we should skip rate limiting
            if let Some(ref predicate) = self.config.skip_predicate {
                if predicate(&request) {
                    return next.run(ctx, request).await;
                }
            }

            // Extract the rate limit key
            let key = match self.extract_key(&request, ctx) {
                Some(k) => k,
                None => {
                    // If we can't extract a key, skip rate limiting
                    return next.run(ctx, request).await;
                }
            };

            // Check rate limit
            match self.check_rate_limit(&key).await {
                RateLimitResult::Allowed {
                    limit,
                    remaining,
                    reset_in,
                } => {
                    let response = next.run(ctx, request).await;
                    Self::add_rate_limit_headers(response, limit, remaining, reset_in)
                }
                RateLimitResult::Limited {
                    limit, reset_in, ..
                } => self.build_rate_limit_response(limit, reset_in),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::MiddlewareContext;
    use bytes::Bytes;
    use http::{Method, Request as HttpRequest};
    use http_body_util::Full;

    fn create_test_request() -> Request {
        HttpRequest::builder()
            .method(Method::GET)
            .uri("/api/test")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_test_request_with_ip(ip: &str) -> Request {
        HttpRequest::builder()
            .method(Method::GET)
            .uri("/api/test")
            .header("x-forwarded-for", ip)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_test_request_with_header(name: &str, value: &str) -> Request {
        HttpRequest::builder()
            .method(Method::GET)
            .uri("/api/test")
            .header(name, value)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    #[test]
    fn test_builder_default() {
        let middleware = RateLimitMiddleware::builder().build();
        assert_eq!(middleware.config.limit, 100);
        assert_eq!(middleware.config.window, Duration::from_secs(60));
    }

    #[test]
    fn test_builder_custom_limit() {
        let middleware = RateLimitMiddleware::builder()
            .limit(50)
            .window_secs(30)
            .build();
        assert_eq!(middleware.config.limit, 50);
        assert_eq!(middleware.config.window, Duration::from_secs(30));
    }

    #[test]
    fn test_builder_per_ip() {
        let middleware = RateLimitMiddleware::builder().per_ip().build();
        assert!(matches!(middleware.config.key_extractor, KeyExtractor::Ip));
    }

    #[test]
    fn test_builder_per_header() {
        let middleware = RateLimitMiddleware::builder()
            .per_header("x-api-key")
            .build();
        assert!(matches!(
            middleware.config.key_extractor,
            KeyExtractor::Header(ref h) if h == "x-api-key"
        ));
    }

    #[test]
    fn test_builder_per_user() {
        let middleware = RateLimitMiddleware::builder().per_user().build();
        assert!(matches!(
            middleware.config.key_extractor,
            KeyExtractor::UserId
        ));
    }

    #[test]
    fn test_builder_global() {
        let middleware = RateLimitMiddleware::builder().global().build();
        assert!(matches!(
            middleware.config.key_extractor,
            KeyExtractor::Global
        ));
    }

    #[test]
    fn test_builder_error_message() {
        let middleware = RateLimitMiddleware::builder()
            .error_message("Custom error")
            .build();
        assert_eq!(middleware.config.error_message, "Custom error");
    }

    #[test]
    fn test_default_limits() {
        let middleware = RateLimitMiddleware::default_limits();
        assert_eq!(middleware.config.limit, 100);
    }

    #[test]
    fn test_strict_limits() {
        let middleware = RateLimitMiddleware::strict();
        assert_eq!(middleware.config.limit, 10);
    }

    #[test]
    fn test_lenient_limits() {
        let middleware = RateLimitMiddleware::lenient();
        assert_eq!(middleware.config.limit, 1000);
    }

    #[test]
    fn test_extract_key_ip_xff() {
        let middleware = RateLimitMiddleware::builder().per_ip().build();
        let request = create_test_request_with_ip("192.168.1.1");
        let ctx = MiddlewareContext::new();

        let key = middleware.extract_key(&request, &ctx);
        assert_eq!(key, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_extract_key_ip_xff_multiple() {
        let middleware = RateLimitMiddleware::builder().per_ip().build();
        let request = create_test_request_with_header(
            "x-forwarded-for",
            "192.168.1.1, 10.0.0.1, 172.16.0.1",
        );
        let ctx = MiddlewareContext::new();

        let key = middleware.extract_key(&request, &ctx);
        assert_eq!(key, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_extract_key_header() {
        let middleware = RateLimitMiddleware::builder()
            .per_header("x-api-key")
            .build();
        let request = create_test_request_with_header("x-api-key", "my-api-key");
        let ctx = MiddlewareContext::new();

        let key = middleware.extract_key(&request, &ctx);
        assert_eq!(key, Some("my-api-key".to_string()));
    }

    #[test]
    fn test_extract_key_header_missing() {
        let middleware = RateLimitMiddleware::builder()
            .per_header("x-api-key")
            .build();
        let request = create_test_request();
        let ctx = MiddlewareContext::new();

        let key = middleware.extract_key(&request, &ctx);
        assert!(key.is_none());
    }

    #[test]
    fn test_extract_key_global() {
        let middleware = RateLimitMiddleware::builder().global().build();
        let request = create_test_request();
        let ctx = MiddlewareContext::new();

        let key = middleware.extract_key(&request, &ctx);
        assert_eq!(key, Some("global".to_string()));
    }

    #[test]
    fn test_extract_key_custom() {
        let middleware = RateLimitMiddleware::builder()
            .key_extractor(|_| Some("custom-key".to_string()))
            .build();
        let request = create_test_request();
        let ctx = MiddlewareContext::new();

        let key = middleware.extract_key(&request, &ctx);
        assert_eq!(key, Some("custom-key".to_string()));
    }

    #[tokio::test]
    async fn test_rate_limit_allowed() {
        let middleware = RateLimitMiddleware::builder()
            .limit(10)
            .window_secs(60)
            .global()
            .build();

        let result = middleware.check_rate_limit("test-key").await;
        assert!(matches!(result, RateLimitResult::Allowed { .. }));
    }

    #[tokio::test]
    async fn test_rate_limit_exceeded() {
        let middleware = RateLimitMiddleware::builder()
            .limit(3)
            .window_secs(60)
            .global()
            .build();

        // Make 3 requests (should be allowed)
        for _ in 0..3 {
            let result = middleware.check_rate_limit("test-key").await;
            assert!(matches!(result, RateLimitResult::Allowed { .. }));
        }

        // 4th request should be limited
        let result = middleware.check_rate_limit("test-key").await;
        assert!(matches!(result, RateLimitResult::Limited { .. }));
    }

    #[tokio::test]
    async fn test_rate_limit_remaining_decreases() {
        let middleware = RateLimitMiddleware::builder()
            .limit(5)
            .window_secs(60)
            .global()
            .build();

        let result = middleware.check_rate_limit("test-key").await;
        if let RateLimitResult::Allowed { remaining, .. } = result {
            assert_eq!(remaining, 4);
        } else {
            panic!("Expected Allowed");
        }

        let result = middleware.check_rate_limit("test-key").await;
        if let RateLimitResult::Allowed { remaining, .. } = result {
            assert_eq!(remaining, 3);
        } else {
            panic!("Expected Allowed");
        }
    }

    #[tokio::test]
    async fn test_different_keys_independent() {
        let middleware = RateLimitMiddleware::builder()
            .limit(2)
            .window_secs(60)
            .build();

        // Use up key1's limit
        middleware.check_rate_limit("key1").await;
        middleware.check_rate_limit("key1").await;
        let result = middleware.check_rate_limit("key1").await;
        assert!(matches!(result, RateLimitResult::Limited { .. }));

        // key2 should still have capacity
        let result = middleware.check_rate_limit("key2").await;
        assert!(matches!(result, RateLimitResult::Allowed { .. }));
    }

    #[test]
    fn test_rate_limit_response() {
        let middleware = RateLimitMiddleware::builder()
            .limit(100)
            .error_message("Rate limited!")
            .build();

        let response = middleware.build_rate_limit_response(100, Duration::from_secs(30));

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(response.headers().contains_key(headers::LIMIT));
        assert!(response.headers().contains_key(headers::REMAINING));
        assert!(response.headers().contains_key(headers::RESET));
        assert!(response.headers().contains_key(headers::RETRY_AFTER));
    }

    #[test]
    fn test_add_rate_limit_headers() {
        let response = http::Response::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::new()))
            .unwrap();

        let response =
            RateLimitMiddleware::add_rate_limit_headers(response, 100, 50, Duration::from_secs(30));

        assert_eq!(
            response.headers().get(headers::LIMIT).unwrap(),
            "100"
        );
        assert_eq!(
            response.headers().get(headers::REMAINING).unwrap(),
            "50"
        );
        assert!(response.headers().contains_key(headers::RESET));
    }

    #[test]
    fn test_middleware_name() {
        let middleware = RateLimitMiddleware::default_limits();
        assert_eq!(middleware.name(), "rate-limit");
    }

    #[test]
    fn test_middleware_clone() {
        let middleware = RateLimitMiddleware::builder()
            .limit(50)
            .build();
        let cloned = middleware.clone();
        assert_eq!(cloned.config.limit, 50);
    }

    #[test]
    fn test_key_extractor_default() {
        let extractor = KeyExtractor::default();
        assert!(matches!(extractor, KeyExtractor::Ip));
    }

    #[test]
    fn test_builder_skip_predicate() {
        let middleware = RateLimitMiddleware::builder()
            .skip(|req| req.uri().path() == "/health")
            .build();
        assert!(middleware.config.skip_predicate.is_some());
    }

    #[test]
    fn test_config_debug() {
        let config = RateLimitConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("limit"));
        assert!(debug.contains("window"));
    }
}
