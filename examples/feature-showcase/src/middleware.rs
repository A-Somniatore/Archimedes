//! Middleware configuration - demonstrates all middleware features.
//!
//! ## Features Demonstrated
//! - CORS middleware
//! - Rate limiting middleware
//! - Compression middleware
//! - Custom middleware patterns

use archimedes_middleware::{
    compression::CompressionMiddleware,
    cors::{CorsConfig, CorsMiddleware},
    rate_limit::{RateLimitConfig, RateLimitMiddleware},
};
use std::time::Duration;

/// Create CORS middleware with configuration.
///
/// # Example
/// ```
/// let cors = create_cors_middleware();
/// server.middleware(cors);
/// ```
pub fn create_cors_middleware() -> CorsMiddleware {
    CorsMiddleware::new(CorsConfig {
        // Allow specific origins (or "*" for any)
        allowed_origins: vec![
            "http://localhost:3000".to_string(),
            "https://example.com".to_string(),
        ],
        // Allow common HTTP methods
        allowed_methods: vec![
            "GET".to_string(),
            "POST".to_string(),
            "PUT".to_string(),
            "DELETE".to_string(),
            "PATCH".to_string(),
            "OPTIONS".to_string(),
        ],
        // Allow common headers
        allowed_headers: vec![
            "Content-Type".to_string(),
            "Authorization".to_string(),
            "X-Request-ID".to_string(),
            "X-API-Key".to_string(),
        ],
        // Expose headers to client
        exposed_headers: vec![
            "X-RateLimit-Limit".to_string(),
            "X-RateLimit-Remaining".to_string(),
            "X-RateLimit-Reset".to_string(),
        ],
        // Allow credentials (cookies, auth headers)
        allow_credentials: true,
        // Cache preflight for 1 hour
        max_age: Some(Duration::from_secs(3600)),
    })
}

/// Create rate limiting middleware.
///
/// # Example
/// ```
/// let rate_limit = create_rate_limit_middleware();
/// server.middleware(rate_limit);
/// ```
pub fn create_rate_limit_middleware() -> RateLimitMiddleware {
    RateLimitMiddleware::new(RateLimitConfig {
        // 100 requests per minute
        requests_per_window: 100,
        window_duration: Duration::from_secs(60),
        // Key by IP address
        key_extractor: Box::new(|req| {
            req.headers()
                .get("x-forwarded-for")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        }),
        // Custom response when rate limited
        exceeded_response: Some(serde_json::json!({
            "error": "Rate limit exceeded",
            "retry_after": 60
        })),
    })
}

/// Create compression middleware.
///
/// # Example
/// ```
/// let compression = create_compression_middleware();
/// server.middleware(compression);
/// ```
pub fn create_compression_middleware() -> CompressionMiddleware {
    CompressionMiddleware::new()
        .gzip(true)
        .brotli(true)
        .deflate(true)
        .min_size(1024) // Only compress responses > 1KB
        .exclude_content_types(vec![
            "image/png".to_string(),
            "image/jpeg".to_string(),
            "image/gif".to_string(),
            "application/zip".to_string(),
        ])
}

/// Middleware configuration for the entire application.
#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    /// Enable CORS
    pub cors_enabled: bool,
    /// Enable rate limiting
    pub rate_limit_enabled: bool,
    /// Enable compression
    pub compression_enabled: bool,
    /// Rate limit - requests per window
    pub rate_limit_requests: u32,
    /// Rate limit - window duration in seconds
    pub rate_limit_window_secs: u64,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            cors_enabled: true,
            rate_limit_enabled: true,
            compression_enabled: true,
            rate_limit_requests: 100,
            rate_limit_window_secs: 60,
        }
    }
}

/// Custom request logging middleware example.
///
/// This demonstrates how to create custom middleware.
pub struct RequestLoggingMiddleware {
    /// Include request body in logs
    pub log_body: bool,
    /// Log level
    pub level: tracing::Level,
}

impl RequestLoggingMiddleware {
    pub fn new() -> Self {
        Self {
            log_body: false,
            level: tracing::Level::INFO,
        }
    }

    pub fn with_body(mut self) -> Self {
        self.log_body = true;
        self
    }

    pub fn with_level(mut self, level: tracing::Level) -> Self {
        self.level = level;
        self
    }
}

impl Default for RequestLoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom request ID middleware example.
///
/// This adds a unique request ID to each request for tracing.
pub struct RequestIdMiddleware;

impl RequestIdMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RequestIdMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom timeout middleware example.
///
/// This adds request timeout handling.
pub struct TimeoutMiddleware {
    pub timeout: Duration,
}

impl TimeoutMiddleware {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub fn from_secs(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_middleware_creation() {
        let cors = create_cors_middleware();
        // Would verify config is set correctly
        assert!(true); // Placeholder
    }

    #[test]
    fn test_rate_limit_middleware_creation() {
        let rate_limit = create_rate_limit_middleware();
        // Would verify config is set correctly
        assert!(true); // Placeholder
    }

    #[test]
    fn test_compression_middleware_creation() {
        let compression = create_compression_middleware();
        // Would verify config is set correctly
        assert!(true); // Placeholder
    }

    #[test]
    fn test_middleware_config_defaults() {
        let config = MiddlewareConfig::default();
        assert!(config.cors_enabled);
        assert!(config.rate_limit_enabled);
        assert!(config.compression_enabled);
        assert_eq!(config.rate_limit_requests, 100);
    }

    #[test]
    fn test_request_logging_middleware() {
        let middleware = RequestLoggingMiddleware::new()
            .with_body()
            .with_level(tracing::Level::DEBUG);
        
        assert!(middleware.log_body);
        assert_eq!(middleware.level, tracing::Level::DEBUG);
    }

    #[test]
    fn test_timeout_middleware() {
        let middleware = TimeoutMiddleware::from_secs(30);
        assert_eq!(middleware.timeout, Duration::from_secs(30));
    }
}
