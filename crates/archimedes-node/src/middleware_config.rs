//! Middleware configuration types for Node.js/TypeScript bindings.
//!
//! This module exposes configurable middleware to TypeScript:
//! - CORS middleware configuration
//! - Rate limiting configuration
//! - Compression configuration
//! - Static file serving configuration
//!
//! ## Example (TypeScript)
//!
//! ```typescript
//! import { CorsConfig, RateLimitConfig, CompressionConfig, StaticFilesConfig } from '@archimedes/node';
//!
//! // CORS configuration
//! const cors = new CorsConfig()
//!   .allowOrigin('https://example.com')
//!   .allowMethods(['GET', 'POST', 'PUT', 'DELETE'])
//!   .allowCredentials(true)
//!   .maxAge(3600);
//!
//! // Rate limiting
//! const rateLimit = new RateLimitConfig()
//!   .requestsPerSecond(100)
//!   .burstSize(20)
//!   .exemptPath('/health');
//!
//! // Compression
//! const compression = new CompressionConfig()
//!   .enableGzip(true)
//!   .enableBrotli(true)
//!   .minSize(1024);
//!
//! // Static files
//! const staticFiles = new StaticFilesConfig()
//!   .directory('./public')
//!   .prefix('/static')
//!   .cacheMaxAge(86400);
//! ```

use napi_derive::napi;
use std::collections::HashSet;

// ============================================================================
// CORS Configuration
// ============================================================================

/// CORS (Cross-Origin Resource Sharing) middleware configuration.
///
/// Configure which origins, methods, and headers are allowed for cross-origin requests.
#[napi]
#[derive(Clone, Default)]
pub struct CorsConfig {
    allowed_origins: HashSet<String>,
    allow_any_origin: bool,
    allowed_methods: HashSet<String>,
    allowed_headers: HashSet<String>,
    exposed_headers: HashSet<String>,
    allow_credentials: bool,
    max_age_seconds: Option<u32>,
}

#[napi]
impl CorsConfig {
    /// Create a new CORS configuration with sensible defaults.
    ///
    /// Defaults:
    /// - Allowed methods: GET, HEAD, POST, PUT, DELETE, PATCH
    /// - Allowed headers: Content-Type, Authorization, X-Request-Id
    /// - Credentials: false
    /// - Max age: 3600 seconds (1 hour)
    #[napi(constructor)]
    pub fn new() -> Self {
        let mut config = Self::default();
        // Default methods
        for method in &["GET", "HEAD", "POST", "PUT", "DELETE", "PATCH"] {
            config.allowed_methods.insert((*method).to_string());
        }
        // Default headers
        for header in &["content-type", "authorization", "x-request-id"] {
            config.allowed_headers.insert((*header).to_string());
        }
        config.max_age_seconds = Some(3600);
        config
    }

    /// Allow requests from any origin (use with caution).
    #[napi]
    pub fn allow_any_origin(&mut self) -> &Self {
        self.allow_any_origin = true;
        self
    }

    /// Add an allowed origin.
    #[napi]
    pub fn allow_origin(&mut self, origin: String) -> &Self {
        self.allowed_origins.insert(origin);
        self
    }

    /// Add multiple allowed origins.
    #[napi]
    pub fn allow_origins(&mut self, origins: Vec<String>) -> &Self {
        for origin in origins {
            self.allowed_origins.insert(origin);
        }
        self
    }

    /// Add an allowed HTTP method.
    #[napi]
    pub fn allow_method(&mut self, method: String) -> &Self {
        self.allowed_methods.insert(method.to_uppercase());
        self
    }

    /// Set the allowed HTTP methods (replaces defaults).
    #[napi]
    pub fn allow_methods(&mut self, methods: Vec<String>) -> &Self {
        self.allowed_methods.clear();
        for method in methods {
            self.allowed_methods.insert(method.to_uppercase());
        }
        self
    }

    /// Add an allowed request header.
    #[napi]
    pub fn allow_header(&mut self, header: String) -> &Self {
        self.allowed_headers.insert(header.to_lowercase());
        self
    }

    /// Set the allowed request headers (replaces defaults).
    #[napi]
    pub fn allow_headers(&mut self, headers: Vec<String>) -> &Self {
        self.allowed_headers.clear();
        for header in headers {
            self.allowed_headers.insert(header.to_lowercase());
        }
        self
    }

    /// Add a header to expose to the browser.
    #[napi]
    pub fn expose_header(&mut self, header: String) -> &Self {
        self.exposed_headers.insert(header);
        self
    }

    /// Set the exposed headers.
    #[napi]
    pub fn expose_headers(&mut self, headers: Vec<String>) -> &Self {
        self.exposed_headers.clear();
        for header in headers {
            self.exposed_headers.insert(header);
        }
        self
    }

    /// Allow credentials (cookies, authorization headers).
    #[napi]
    pub fn allow_credentials(&mut self, allow: bool) -> &Self {
        self.allow_credentials = allow;
        self
    }

    /// Set the max age for preflight cache (in seconds).
    #[napi]
    pub fn max_age(&mut self, seconds: u32) -> &Self {
        self.max_age_seconds = Some(seconds);
        self
    }

    /// Check if an origin is allowed.
    #[napi]
    pub fn is_origin_allowed(&self, origin: String) -> bool {
        self.allow_any_origin || self.allowed_origins.contains(&origin)
    }

    /// Check if a method is allowed.
    #[napi]
    pub fn is_method_allowed(&self, method: String) -> bool {
        self.allowed_methods.contains(&method.to_uppercase())
    }

    /// Check if a header is allowed.
    #[napi]
    pub fn is_header_allowed(&self, header: String) -> bool {
        self.allowed_headers.contains(&header.to_lowercase())
    }

    /// Get allowed origins as a comma-separated string.
    #[napi]
    pub fn get_allowed_origins(&self) -> String {
        if self.allow_any_origin {
            "*".to_string()
        } else {
            self.allowed_origins
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    /// Get allowed methods as a comma-separated string.
    #[napi]
    pub fn get_allowed_methods(&self) -> String {
        self.allowed_methods
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Get allowed headers as a comma-separated string.
    #[napi]
    pub fn get_allowed_headers(&self) -> String {
        self.allowed_headers
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Get max age in seconds.
    #[napi]
    pub fn get_max_age(&self) -> Option<u32> {
        self.max_age_seconds
    }

    /// Check if credentials are allowed.
    #[napi]
    pub fn get_allow_credentials(&self) -> bool {
        self.allow_credentials
    }
}

// ============================================================================
// Rate Limiting Configuration
// ============================================================================

/// Rate limiting middleware configuration.
///
/// Configure request rate limits using token bucket algorithm.
#[napi]
#[derive(Clone)]
pub struct RateLimitConfig {
    requests_per_second: f64,
    burst_size: u32,
    key_extractor: String,
    exempt_paths: HashSet<String>,
    enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut exempt_paths = HashSet::new();
        exempt_paths.insert("/health".to_string());
        exempt_paths.insert("/ready".to_string());

        Self {
            requests_per_second: 100.0,
            burst_size: 10,
            key_extractor: "ip".to_string(),
            exempt_paths,
            enabled: true,
        }
    }
}

#[napi]
impl RateLimitConfig {
    /// Create a new rate limit configuration with sensible defaults.
    ///
    /// Defaults:
    /// - 100 requests per second
    /// - Burst size: 10
    /// - Key extractor: IP address
    /// - Exempt paths: /health, /ready
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the requests per second limit.
    #[napi]
    pub fn requests_per_second(&mut self, rps: f64) -> &Self {
        self.requests_per_second = rps;
        self
    }

    /// Set the burst size (max tokens in bucket).
    #[napi]
    pub fn burst_size(&mut self, size: u32) -> &Self {
        self.burst_size = size;
        self
    }

    /// Set the key extractor type ('ip', 'user', 'api_key', 'header:X-Custom').
    #[napi]
    pub fn key_extractor(&mut self, extractor: String) -> &Self {
        self.key_extractor = extractor;
        self
    }

    /// Add a path to exempt from rate limiting.
    #[napi]
    pub fn exempt_path(&mut self, path: String) -> &Self {
        self.exempt_paths.insert(path);
        self
    }

    /// Add multiple paths to exempt from rate limiting.
    #[napi]
    pub fn exempt_paths(&mut self, paths: Vec<String>) -> &Self {
        for path in paths {
            self.exempt_paths.insert(path);
        }
        self
    }

    /// Enable or disable rate limiting.
    #[napi]
    pub fn enabled(&mut self, enabled: bool) -> &Self {
        self.enabled = enabled;
        self
    }

    /// Check if a path is exempt from rate limiting.
    #[napi]
    pub fn is_path_exempt(&self, path: String) -> bool {
        self.exempt_paths.contains(&path)
    }

    /// Get the requests per second limit.
    #[napi]
    pub fn get_requests_per_second(&self) -> f64 {
        self.requests_per_second
    }

    /// Get the burst size.
    #[napi]
    pub fn get_burst_size(&self) -> u32 {
        self.burst_size
    }

    /// Get the key extractor type.
    #[napi]
    pub fn get_key_extractor(&self) -> String {
        self.key_extractor.clone()
    }

    /// Check if rate limiting is enabled.
    #[napi]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// ============================================================================
// Compression Configuration
// ============================================================================

/// Compression algorithm options.
#[napi(string_enum)]
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum CompressionAlgorithm {
    /// gzip compression
    Gzip,
    /// Brotli compression
    Brotli,
    /// Deflate compression
    Deflate,
    /// Zstandard compression
    Zstd,
}

/// Compression middleware configuration.
///
/// Configure response compression with multiple algorithm support.
#[napi]
#[derive(Clone)]
pub struct CompressionConfig {
    enable_gzip: bool,
    enable_brotli: bool,
    enable_deflate: bool,
    enable_zstd: bool,
    min_size_bytes: usize,
    compression_level: u32,
    content_types: HashSet<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        let mut content_types = HashSet::new();
        for ct in &[
            "text/html",
            "text/css",
            "text/plain",
            "text/xml",
            "text/javascript",
            "application/javascript",
            "application/json",
            "application/xml",
            "image/svg+xml",
        ] {
            content_types.insert((*ct).to_string());
        }

        Self {
            enable_gzip: true,
            enable_brotli: true,
            enable_deflate: false,
            enable_zstd: false,
            min_size_bytes: 860,
            compression_level: 4,
            content_types,
        }
    }
}

#[napi]
impl CompressionConfig {
    /// Create a new compression configuration with sensible defaults.
    ///
    /// Defaults:
    /// - gzip: enabled
    /// - brotli: enabled
    /// - deflate: disabled
    /// - zstd: disabled
    /// - Min size: 860 bytes
    /// - Level: 4 (balanced)
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable or disable gzip compression.
    #[napi]
    pub fn enable_gzip(&mut self, enable: bool) -> &Self {
        self.enable_gzip = enable;
        self
    }

    /// Enable or disable Brotli compression.
    #[napi]
    pub fn enable_brotli(&mut self, enable: bool) -> &Self {
        self.enable_brotli = enable;
        self
    }

    /// Enable or disable deflate compression.
    #[napi]
    pub fn enable_deflate(&mut self, enable: bool) -> &Self {
        self.enable_deflate = enable;
        self
    }

    /// Enable or disable Zstandard compression.
    #[napi]
    pub fn enable_zstd(&mut self, enable: bool) -> &Self {
        self.enable_zstd = enable;
        self
    }

    /// Set the minimum response size to compress (in bytes).
    #[napi]
    pub fn min_size(&mut self, bytes: u32) -> &Self {
        self.min_size_bytes = bytes as usize;
        self
    }

    /// Set the compression level (1-9, higher = better compression but slower).
    #[napi]
    pub fn level(&mut self, level: u32) -> &Self {
        self.compression_level = level.clamp(1, 9);
        self
    }

    /// Add a content type to compress.
    #[napi]
    pub fn add_content_type(&mut self, content_type: String) -> &Self {
        self.content_types.insert(content_type);
        self
    }

    /// Set the content types to compress (replaces defaults).
    #[napi]
    pub fn content_types(&mut self, types: Vec<String>) -> &Self {
        self.content_types.clear();
        for t in types {
            self.content_types.insert(t);
        }
        self
    }

    /// Check if gzip is enabled.
    #[napi]
    pub fn is_gzip_enabled(&self) -> bool {
        self.enable_gzip
    }

    /// Check if Brotli is enabled.
    #[napi]
    pub fn is_brotli_enabled(&self) -> bool {
        self.enable_brotli
    }

    /// Check if deflate is enabled.
    #[napi]
    pub fn is_deflate_enabled(&self) -> bool {
        self.enable_deflate
    }

    /// Check if Zstandard is enabled.
    #[napi]
    pub fn is_zstd_enabled(&self) -> bool {
        self.enable_zstd
    }

    /// Get the minimum size threshold.
    #[napi]
    pub fn get_min_size(&self) -> u32 {
        self.min_size_bytes as u32
    }

    /// Get the compression level.
    #[napi]
    pub fn get_level(&self) -> u32 {
        self.compression_level
    }

    /// Check if a content type should be compressed.
    #[napi]
    pub fn should_compress(&self, content_type: String) -> bool {
        // Check exact match or prefix match (e.g., "text/html; charset=utf-8")
        self.content_types.iter().any(|ct| {
            content_type == *ct || content_type.starts_with(&format!("{ct};"))
        })
    }

    /// Get enabled algorithms as strings.
    #[napi]
    pub fn get_enabled_algorithms(&self) -> Vec<String> {
        let mut algos = Vec::new();
        if self.enable_brotli {
            algos.push("br".to_string());
        }
        if self.enable_gzip {
            algos.push("gzip".to_string());
        }
        if self.enable_deflate {
            algos.push("deflate".to_string());
        }
        if self.enable_zstd {
            algos.push("zstd".to_string());
        }
        algos
    }
}

// ============================================================================
// Static Files Configuration
// ============================================================================

/// Static file serving middleware configuration.
///
/// Configure serving static files from a directory.
#[napi]
#[derive(Clone)]
pub struct StaticFilesConfig {
    directory: String,
    prefix: String,
    index_file: String,
    cache_max_age_seconds: u32,
    enable_precompressed: bool,
    fallback_file: Option<String>,
}

impl Default for StaticFilesConfig {
    fn default() -> Self {
        Self {
            directory: "./static".to_string(),
            prefix: "/static".to_string(),
            index_file: "index.html".to_string(),
            cache_max_age_seconds: 86400,
            enable_precompressed: true,
            fallback_file: None,
        }
    }
}

#[napi]
impl StaticFilesConfig {
    /// Create a new static files configuration with sensible defaults.
    ///
    /// Defaults:
    /// - Directory: ./static
    /// - Prefix: /static
    /// - Index: index.html
    /// - Cache max age: 86400 seconds (1 day)
    /// - Precompressed: enabled
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the directory to serve files from.
    #[napi]
    pub fn directory(&mut self, dir: String) -> &Self {
        self.directory = dir;
        self
    }

    /// Set the URL prefix for static files.
    #[napi]
    pub fn prefix(&mut self, prefix: String) -> &Self {
        self.prefix = if prefix.starts_with('/') {
            prefix
        } else {
            format!("/{prefix}")
        };
        self
    }

    /// Set the index file name.
    #[napi]
    pub fn index(&mut self, file: String) -> &Self {
        self.index_file = file;
        self
    }

    /// Set the cache max age in seconds.
    #[napi]
    pub fn cache_max_age(&mut self, seconds: u32) -> &Self {
        self.cache_max_age_seconds = seconds;
        self
    }

    /// Enable or disable serving precompressed files (.gz, .br).
    #[napi]
    pub fn precompressed(&mut self, enable: bool) -> &Self {
        self.enable_precompressed = enable;
        self
    }

    /// Set a fallback file for SPA routing.
    #[napi]
    pub fn fallback(&mut self, file: String) -> &Self {
        self.fallback_file = Some(file);
        self
    }

    /// Get the directory path.
    #[napi]
    pub fn get_directory(&self) -> String {
        self.directory.clone()
    }

    /// Get the URL prefix.
    #[napi]
    pub fn get_prefix(&self) -> String {
        self.prefix.clone()
    }

    /// Get the index file name.
    #[napi]
    pub fn get_index(&self) -> String {
        self.index_file.clone()
    }

    /// Get the cache max age in seconds.
    #[napi]
    pub fn get_cache_max_age(&self) -> u32 {
        self.cache_max_age_seconds
    }

    /// Check if precompressed files are enabled.
    #[napi]
    pub fn is_precompressed_enabled(&self) -> bool {
        self.enable_precompressed
    }

    /// Get the fallback file if set.
    #[napi]
    pub fn get_fallback(&self) -> Option<String> {
        self.fallback_file.clone()
    }

    /// Get the full path for a request path.
    #[napi]
    pub fn resolve_path(&self, request_path: String) -> Option<String> {
        if !request_path.starts_with(&self.prefix) {
            return None;
        }

        let relative = request_path
            .strip_prefix(&self.prefix)
            .unwrap_or(&request_path);
        let relative = relative.trim_start_matches('/');

        let file_path = if relative.is_empty() {
            format!("{}/{}", self.directory, self.index_file)
        } else {
            format!("{}/{}", self.directory, relative)
        };

        // Prevent directory traversal
        if file_path.contains("..") {
            return None;
        }

        Some(file_path)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_config_defaults() {
        let config = CorsConfig::new();
        assert!(!config.allow_any_origin);
        assert!(config.is_method_allowed("GET".to_string()));
        assert!(config.is_method_allowed("POST".to_string()));
        assert!(config.is_header_allowed("content-type".to_string()));
        assert_eq!(config.get_max_age(), Some(3600));
    }

    #[test]
    fn test_cors_config_allow_origin() {
        let mut config = CorsConfig::new();
        config.allow_origin("https://example.com".to_string());
        assert!(config.is_origin_allowed("https://example.com".to_string()));
        assert!(!config.is_origin_allowed("https://other.com".to_string()));
    }

    #[test]
    fn test_cors_config_allow_any_origin() {
        let mut config = CorsConfig::new();
        config.allow_any_origin();
        assert!(config.is_origin_allowed("https://any.com".to_string()));
        assert_eq!(config.get_allowed_origins(), "*");
    }

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = RateLimitConfig::new();
        assert_eq!(config.get_requests_per_second(), 100.0);
        assert_eq!(config.get_burst_size(), 10);
        assert!(config.is_path_exempt("/health".to_string()));
        assert!(config.is_enabled());
    }

    #[test]
    fn test_rate_limit_config_custom() {
        let mut config = RateLimitConfig::new();
        config.requests_per_second(50.0);
        config.burst_size(5);
        config.exempt_path("/metrics".to_string());

        assert_eq!(config.get_requests_per_second(), 50.0);
        assert_eq!(config.get_burst_size(), 5);
        assert!(config.is_path_exempt("/metrics".to_string()));
    }

    #[test]
    fn test_compression_config_defaults() {
        let config = CompressionConfig::new();
        assert!(config.is_gzip_enabled());
        assert!(config.is_brotli_enabled());
        assert!(!config.is_deflate_enabled());
        assert_eq!(config.get_min_size(), 860);
        assert_eq!(config.get_level(), 4);
    }

    #[test]
    fn test_compression_config_should_compress() {
        let config = CompressionConfig::new();
        assert!(config.should_compress("application/json".to_string()));
        assert!(config.should_compress("text/html; charset=utf-8".to_string()));
        assert!(!config.should_compress("image/png".to_string()));
    }

    #[test]
    fn test_compression_config_algorithms() {
        let config = CompressionConfig::new();
        let algos = config.get_enabled_algorithms();
        assert!(algos.contains(&"gzip".to_string()));
        assert!(algos.contains(&"br".to_string()));
    }

    #[test]
    fn test_static_files_config_defaults() {
        let config = StaticFilesConfig::new();
        assert_eq!(config.get_directory(), "./static");
        assert_eq!(config.get_prefix(), "/static");
        assert_eq!(config.get_index(), "index.html");
        assert_eq!(config.get_cache_max_age(), 86400);
        assert!(config.is_precompressed_enabled());
    }

    #[test]
    fn test_static_files_config_resolve_path() {
        let config = StaticFilesConfig::new();
        
        assert_eq!(
            config.resolve_path("/static/js/app.js".to_string()),
            Some("./static/js/app.js".to_string())
        );
        
        assert_eq!(
            config.resolve_path("/static/".to_string()),
            Some("./static/index.html".to_string())
        );
        
        // Path traversal should fail
        assert_eq!(config.resolve_path("/static/../secret.txt".to_string()), None);
        
        // Wrong prefix
        assert_eq!(config.resolve_path("/other/file.txt".to_string()), None);
    }

    #[test]
    fn test_static_files_config_custom() {
        let mut config = StaticFilesConfig::new();
        config.directory("./public".to_string());
        config.prefix("/assets".to_string());
        config.cache_max_age(7200);
        config.fallback("index.html".to_string());

        assert_eq!(config.get_directory(), "./public");
        assert_eq!(config.get_prefix(), "/assets");
        assert_eq!(config.get_cache_max_age(), 7200);
        assert_eq!(config.get_fallback(), Some("index.html".to_string()));
    }
}
