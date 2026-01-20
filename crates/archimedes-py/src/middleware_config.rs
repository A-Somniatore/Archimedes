//! Configurable middleware bindings for Python
//!
//! This module exposes configurable middleware from archimedes-middleware to Python:
//! - CORS middleware with origin/method/header configuration
//! - Rate limiting middleware with per-IP/key limits
//! - Compression middleware (gzip, brotli, deflate)
//! - Static file serving
//!
//! ## Example
//!
//! ```python
//! from archimedes import App, CorsConfig, RateLimitConfig
//!
//! # Configure CORS
//! cors = CorsConfig()
//!     .allow_origin("https://example.com")
//!     .allow_methods(["GET", "POST"])
//!     .allow_credentials(True)
//!
//! # Configure rate limiting
//! rate_limit = RateLimitConfig()
//!     .requests_per_second(100)
//!     .burst_size(20)
//!
//! app = App(cors=cors, rate_limit=rate_limit)
//! ```

use pyo3::prelude::*;
use std::collections::HashSet;

// ============================================================================
// CORS Configuration
// ============================================================================

/// CORS (Cross-Origin Resource Sharing) configuration.
///
/// Configure which origins, methods, and headers are allowed for cross-origin
/// requests. This is essential for browser-based API clients.
///
/// Example:
///     cors = CorsConfig()
///         .allow_origin("https://app.example.com")
///         .allow_origin("https://admin.example.com")
///         .allow_methods(["GET", "POST", "PUT", "DELETE"])
///         .allow_headers(["Content-Type", "Authorization"])
///         .allow_credentials(True)
///         .max_age(3600)
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyCorsConfig {
    allow_any_origin: bool,
    allowed_origins: HashSet<String>,
    allowed_methods: HashSet<String>,
    allowed_headers: HashSet<String>,
    expose_headers: HashSet<String>,
    allow_credentials: bool,
    max_age_seconds: Option<u64>,
}

impl Default for PyCorsConfig {
    fn default() -> Self {
        Self {
            allow_any_origin: false,
            allowed_origins: HashSet::new(),
            allowed_methods: HashSet::from([
                "GET".to_string(),
                "HEAD".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
            ]),
            allowed_headers: HashSet::from([
                "content-type".to_string(),
                "authorization".to_string(),
                "x-request-id".to_string(),
            ]),
            expose_headers: HashSet::new(),
            allow_credentials: false,
            max_age_seconds: Some(86400), // 24 hours
        }
    }
}

#[pymethods]
impl PyCorsConfig {
    /// Create a new CORS configuration with defaults.
    ///
    /// Default allowed methods: GET, HEAD, POST, PUT, DELETE, PATCH
    /// Default allowed headers: content-type, authorization, x-request-id
    /// Default max age: 24 hours
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Allow any origin (wildcard "*").
    ///
    /// Warning: Should not be used with allow_credentials(True).
    /// Browsers reject responses with both wildcard origin and credentials.
    ///
    /// Returns:
    ///     Self for method chaining
    fn allow_any_origin(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.allow_any_origin = true;
        slf.allowed_origins.clear();
        slf
    }

    /// Add an allowed origin.
    ///
    /// Args:
    ///     origin: The origin URL (e.g., "https://example.com")
    ///
    /// Returns:
    ///     Self for method chaining
    fn allow_origin(mut slf: PyRefMut<'_, Self>, origin: String) -> PyRefMut<'_, Self> {
        if !slf.allow_any_origin {
            slf.allowed_origins.insert(origin);
        }
        slf
    }

    /// Set multiple allowed origins at once.
    ///
    /// Args:
    ///     origins: List of origin URLs
    ///
    /// Returns:
    ///     Self for method chaining
    fn allow_origins(mut slf: PyRefMut<'_, Self>, origins: Vec<String>) -> PyRefMut<'_, Self> {
        slf.allow_any_origin = false;
        slf.allowed_origins = origins.into_iter().collect();
        slf
    }

    /// Set allowed HTTP methods.
    ///
    /// Args:
    ///     methods: List of method names (e.g., ["GET", "POST"])
    ///
    /// Returns:
    ///     Self for method chaining
    fn allow_methods(mut slf: PyRefMut<'_, Self>, methods: Vec<String>) -> PyRefMut<'_, Self> {
        slf.allowed_methods = methods.into_iter().map(|m| m.to_uppercase()).collect();
        slf
    }

    /// Set allowed request headers.
    ///
    /// Args:
    ///     headers: List of header names (case-insensitive)
    ///
    /// Returns:
    ///     Self for method chaining
    fn allow_headers(mut slf: PyRefMut<'_, Self>, headers: Vec<String>) -> PyRefMut<'_, Self> {
        slf.allowed_headers = headers.into_iter().map(|h| h.to_lowercase()).collect();
        slf
    }

    /// Add a single allowed header.
    ///
    /// Args:
    ///     header: Header name (case-insensitive)
    ///
    /// Returns:
    ///     Self for method chaining
    fn allow_header(mut slf: PyRefMut<'_, Self>, header: String) -> PyRefMut<'_, Self> {
        slf.allowed_headers.insert(header.to_lowercase());
        slf
    }

    /// Set headers exposed to JavaScript.
    ///
    /// These headers can be read by JavaScript in cross-origin responses.
    ///
    /// Args:
    ///     headers: List of header names to expose
    ///
    /// Returns:
    ///     Self for method chaining
    fn expose_headers(mut slf: PyRefMut<'_, Self>, headers: Vec<String>) -> PyRefMut<'_, Self> {
        slf.expose_headers = headers.into_iter().map(|h| h.to_lowercase()).collect();
        slf
    }

    /// Allow credentials (cookies, authorization headers).
    ///
    /// Args:
    ///     allow: Whether to allow credentials
    ///
    /// Returns:
    ///     Self for method chaining
    fn allow_credentials(mut slf: PyRefMut<'_, Self>, allow: bool) -> PyRefMut<'_, Self> {
        slf.allow_credentials = allow;
        slf
    }

    /// Set preflight cache max age in seconds.
    ///
    /// Args:
    ///     seconds: Max age for preflight response cache
    ///
    /// Returns:
    ///     Self for method chaining
    fn max_age(mut slf: PyRefMut<'_, Self>, seconds: u64) -> PyRefMut<'_, Self> {
        slf.max_age_seconds = Some(seconds);
        slf
    }

    /// Check if an origin is allowed.
    fn is_origin_allowed(&self, origin: &str) -> bool {
        if self.allow_any_origin {
            return true;
        }
        self.allowed_origins.contains(origin)
    }

    /// Check if a method is allowed.
    fn is_method_allowed(&self, method: &str) -> bool {
        self.allowed_methods.contains(&method.to_uppercase())
    }

    /// Check if a header is allowed.
    fn is_header_allowed(&self, header: &str) -> bool {
        self.allowed_headers.contains(&header.to_lowercase())
    }

    /// Get allowed origins as a list.
    fn get_allowed_origins(&self) -> Vec<String> {
        if self.allow_any_origin {
            vec!["*".to_string()]
        } else {
            self.allowed_origins.iter().cloned().collect()
        }
    }

    /// Get allowed methods as a list.
    fn get_allowed_methods(&self) -> Vec<String> {
        self.allowed_methods.iter().cloned().collect()
    }

    /// Get allowed headers as a list.
    fn get_allowed_headers(&self) -> Vec<String> {
        self.allowed_headers.iter().cloned().collect()
    }

    fn __repr__(&self) -> String {
        if self.allow_any_origin {
            format!(
                "CorsConfig(allow_any_origin=True, methods={:?}, credentials={})",
                self.allowed_methods, self.allow_credentials
            )
        } else {
            format!(
                "CorsConfig(origins={:?}, methods={:?}, credentials={})",
                self.allowed_origins, self.allowed_methods, self.allow_credentials
            )
        }
    }
}

// ============================================================================
// Rate Limit Configuration
// ============================================================================

/// Rate limiting configuration.
///
/// Configure request rate limits to protect against abuse.
/// Supports per-IP and per-key rate limiting with token bucket algorithm.
///
/// Example:
///     rate_limit = RateLimitConfig()
///         .requests_per_second(100)
///         .burst_size(20)
///         .key_extractor("ip")  # or "header:X-API-Key"
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyRateLimitConfig {
    requests_per_second: f64,
    burst_size: u32,
    key_extractor: String,
    exempt_paths: HashSet<String>,
    enabled: bool,
}

impl Default for PyRateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: 100.0,
            burst_size: 10,
            key_extractor: "ip".to_string(),
            exempt_paths: HashSet::from(["/health".to_string(), "/ready".to_string()]),
            enabled: true,
        }
    }
}

#[pymethods]
impl PyRateLimitConfig {
    /// Create a new rate limit configuration with defaults.
    ///
    /// Defaults:
    /// - 100 requests per second
    /// - Burst size of 10
    /// - Key by IP address
    /// - /health and /ready exempt
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Set requests per second limit.
    ///
    /// Args:
    ///     rps: Requests per second (can be fractional, e.g., 0.5 for 1 per 2 seconds)
    ///
    /// Returns:
    ///     Self for method chaining
    fn requests_per_second(mut slf: PyRefMut<'_, Self>, rps: f64) -> PyRefMut<'_, Self> {
        slf.requests_per_second = rps;
        slf
    }

    /// Set burst size (token bucket capacity).
    ///
    /// Allows short bursts above the rate limit.
    ///
    /// Args:
    ///     size: Maximum burst size
    ///
    /// Returns:
    ///     Self for method chaining
    fn burst_size(mut slf: PyRefMut<'_, Self>, size: u32) -> PyRefMut<'_, Self> {
        slf.burst_size = size;
        slf
    }

    /// Set key extractor for rate limiting.
    ///
    /// Args:
    ///     extractor: Key extractor type:
    ///         - "ip": Rate limit by client IP
    ///         - "header:X-API-Key": Rate limit by header value
    ///         - "user": Rate limit by authenticated user ID
    ///
    /// Returns:
    ///     Self for method chaining
    fn key_extractor(mut slf: PyRefMut<'_, Self>, extractor: String) -> PyRefMut<'_, Self> {
        slf.key_extractor = extractor;
        slf
    }

    /// Add a path exempt from rate limiting.
    ///
    /// Args:
    ///     path: Path to exempt (e.g., "/health")
    ///
    /// Returns:
    ///     Self for method chaining
    fn exempt_path(mut slf: PyRefMut<'_, Self>, path: String) -> PyRefMut<'_, Self> {
        slf.exempt_paths.insert(path);
        slf
    }

    /// Set multiple exempt paths at once.
    ///
    /// Args:
    ///     paths: List of paths to exempt
    ///
    /// Returns:
    ///     Self for method chaining
    fn exempt_paths(mut slf: PyRefMut<'_, Self>, paths: Vec<String>) -> PyRefMut<'_, Self> {
        slf.exempt_paths = paths.into_iter().collect();
        slf
    }

    /// Enable or disable rate limiting.
    ///
    /// Args:
    ///     enabled: Whether rate limiting is enabled
    ///
    /// Returns:
    ///     Self for method chaining
    fn enabled(mut slf: PyRefMut<'_, Self>, enabled: bool) -> PyRefMut<'_, Self> {
        slf.enabled = enabled;
        slf
    }

    /// Check if a path is exempt from rate limiting.
    fn is_path_exempt(&self, path: &str) -> bool {
        self.exempt_paths.contains(path)
    }

    /// Get the configured requests per second.
    fn get_requests_per_second(&self) -> f64 {
        self.requests_per_second
    }

    /// Get the configured burst size.
    fn get_burst_size(&self) -> u32 {
        self.burst_size
    }

    /// Check if rate limiting is enabled.
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn __repr__(&self) -> String {
        format!(
            "RateLimitConfig(rps={}, burst={}, key={}, enabled={})",
            self.requests_per_second, self.burst_size, self.key_extractor, self.enabled
        )
    }
}

// ============================================================================
// Compression Configuration
// ============================================================================

/// Compression algorithm enum.
#[pyclass]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyCompressionAlgorithm {
    /// Gzip compression (most compatible)
    Gzip,
    /// Brotli compression (best ratio)
    Brotli,
    /// Deflate compression
    Deflate,
    /// Zstandard compression (fastest)
    Zstd,
}

/// Compression middleware configuration.
///
/// Configure automatic response compression based on Accept-Encoding header.
///
/// Example:
///     compression = CompressionConfig()
///         .enable_gzip(True)
///         .enable_brotli(True)
///         .min_size(1024)  # Only compress responses > 1KB
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyCompressionConfig {
    enable_gzip: bool,
    enable_brotli: bool,
    enable_deflate: bool,
    enable_zstd: bool,
    min_size_bytes: usize,
    compression_level: u32,
    content_types: HashSet<String>,
    enabled: bool,
}

impl Default for PyCompressionConfig {
    fn default() -> Self {
        Self {
            enable_gzip: true,
            enable_brotli: true,
            enable_deflate: false,
            enable_zstd: false,
            min_size_bytes: 860, // Typical MTU - headers
            compression_level: 4, // Balanced speed/ratio
            content_types: HashSet::from([
                "text/html".to_string(),
                "text/css".to_string(),
                "text/javascript".to_string(),
                "application/json".to_string(),
                "application/xml".to_string(),
                "text/plain".to_string(),
                "text/xml".to_string(),
                "application/javascript".to_string(),
            ]),
            enabled: true,
        }
    }
}

#[pymethods]
impl PyCompressionConfig {
    /// Create a new compression configuration with defaults.
    ///
    /// Defaults:
    /// - Gzip and Brotli enabled
    /// - Minimum size: 860 bytes
    /// - Compression level: 4 (balanced)
    /// - Common text/JSON content types
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Enable or disable gzip compression.
    fn enable_gzip(mut slf: PyRefMut<'_, Self>, enable: bool) -> PyRefMut<'_, Self> {
        slf.enable_gzip = enable;
        slf
    }

    /// Enable or disable brotli compression.
    fn enable_brotli(mut slf: PyRefMut<'_, Self>, enable: bool) -> PyRefMut<'_, Self> {
        slf.enable_brotli = enable;
        slf
    }

    /// Enable or disable deflate compression.
    fn enable_deflate(mut slf: PyRefMut<'_, Self>, enable: bool) -> PyRefMut<'_, Self> {
        slf.enable_deflate = enable;
        slf
    }

    /// Enable or disable zstandard compression.
    fn enable_zstd(mut slf: PyRefMut<'_, Self>, enable: bool) -> PyRefMut<'_, Self> {
        slf.enable_zstd = enable;
        slf
    }

    /// Set minimum response size for compression.
    ///
    /// Responses smaller than this won't be compressed.
    ///
    /// Args:
    ///     bytes: Minimum size in bytes (default: 860)
    fn min_size(mut slf: PyRefMut<'_, Self>, bytes: usize) -> PyRefMut<'_, Self> {
        slf.min_size_bytes = bytes;
        slf
    }

    /// Set compression level (1-9, higher = better ratio but slower).
    ///
    /// Args:
    ///     level: Compression level (1-9, default: 4)
    fn level(mut slf: PyRefMut<'_, Self>, level: u32) -> PyRefMut<'_, Self> {
        slf.compression_level = level.clamp(1, 9);
        slf
    }

    /// Set content types to compress.
    ///
    /// Args:
    ///     types: List of MIME types to compress
    fn content_types(mut slf: PyRefMut<'_, Self>, types: Vec<String>) -> PyRefMut<'_, Self> {
        slf.content_types = types.into_iter().collect();
        slf
    }

    /// Add a content type to compress.
    fn add_content_type(mut slf: PyRefMut<'_, Self>, content_type: String) -> PyRefMut<'_, Self> {
        slf.content_types.insert(content_type);
        slf
    }

    /// Enable or disable compression.
    fn enabled(mut slf: PyRefMut<'_, Self>, enabled: bool) -> PyRefMut<'_, Self> {
        slf.enabled = enabled;
        slf
    }

    /// Check if a content type should be compressed.
    fn should_compress_type(&self, content_type: &str) -> bool {
        // Check if content type matches any configured type
        let ct_lower = content_type.to_lowercase();
        self.content_types.iter().any(|t| ct_lower.starts_with(t))
    }

    /// Check if compression is enabled.
    fn is_enabled(&self) -> bool {
        self.enabled && (self.enable_gzip || self.enable_brotli || self.enable_deflate || self.enable_zstd)
    }

    /// Get the minimum size for compression.
    fn get_min_size(&self) -> usize {
        self.min_size_bytes
    }

    /// Get the compression level.
    fn get_level(&self) -> u32 {
        self.compression_level
    }

    fn __repr__(&self) -> String {
        let mut algos = Vec::new();
        if self.enable_gzip { algos.push("gzip"); }
        if self.enable_brotli { algos.push("brotli"); }
        if self.enable_deflate { algos.push("deflate"); }
        if self.enable_zstd { algos.push("zstd"); }
        format!(
            "CompressionConfig(algorithms={:?}, min_size={}, level={}, enabled={})",
            algos, self.min_size_bytes, self.compression_level, self.enabled
        )
    }
}

// ============================================================================
// Static Files Configuration
// ============================================================================

/// Static file serving configuration.
///
/// Configure serving static files from a directory.
///
/// Example:
///     static_files = StaticFilesConfig()
///         .directory("./static")
///         .prefix("/static")
///         .index("index.html")
///         .cache_max_age(3600)
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyStaticFilesConfig {
    directory: String,
    prefix: String,
    index_file: Option<String>,
    cache_max_age: Option<u64>,
    precompressed: bool,
    fallback: Option<String>,
    enabled: bool,
}

impl Default for PyStaticFilesConfig {
    fn default() -> Self {
        Self {
            directory: "./static".to_string(),
            prefix: "/static".to_string(),
            index_file: Some("index.html".to_string()),
            cache_max_age: Some(86400), // 24 hours
            precompressed: true,
            fallback: None,
            enabled: true,
        }
    }
}

#[pymethods]
impl PyStaticFilesConfig {
    /// Create a new static files configuration with defaults.
    #[new]
    fn new() -> Self {
        Self::default()
    }

    /// Set the directory to serve files from.
    ///
    /// Args:
    ///     path: Directory path (relative or absolute)
    fn directory(mut slf: PyRefMut<'_, Self>, path: String) -> PyRefMut<'_, Self> {
        slf.directory = path;
        slf
    }

    /// Set the URL prefix for static files.
    ///
    /// Args:
    ///     prefix: URL prefix (e.g., "/static", "/assets")
    fn prefix(mut slf: PyRefMut<'_, Self>, prefix: String) -> PyRefMut<'_, Self> {
        slf.prefix = if prefix.starts_with('/') {
            prefix
        } else {
            format!("/{}", prefix)
        };
        slf
    }

    /// Set the index file for directories.
    ///
    /// Args:
    ///     filename: Index filename (e.g., "index.html")
    fn index(mut slf: PyRefMut<'_, Self>, filename: String) -> PyRefMut<'_, Self> {
        slf.index_file = Some(filename);
        slf
    }

    /// Disable index file serving.
    fn no_index(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.index_file = None;
        slf
    }

    /// Set Cache-Control max-age in seconds.
    ///
    /// Args:
    ///     seconds: Max age for cache (None to disable caching)
    fn cache_max_age(mut slf: PyRefMut<'_, Self>, seconds: Option<u64>) -> PyRefMut<'_, Self> {
        slf.cache_max_age = seconds;
        slf
    }

    /// Enable or disable precompressed file serving (.gz, .br files).
    ///
    /// When enabled, serves pre-compressed versions if they exist.
    fn precompressed(mut slf: PyRefMut<'_, Self>, enable: bool) -> PyRefMut<'_, Self> {
        slf.precompressed = enable;
        slf
    }

    /// Set a fallback file for SPA routing.
    ///
    /// When a file is not found, serve this file instead (e.g., "index.html").
    ///
    /// Args:
    ///     filename: Fallback filename
    fn fallback(mut slf: PyRefMut<'_, Self>, filename: String) -> PyRefMut<'_, Self> {
        slf.fallback = Some(filename);
        slf
    }

    /// Enable or disable static file serving.
    fn enabled(mut slf: PyRefMut<'_, Self>, enabled: bool) -> PyRefMut<'_, Self> {
        slf.enabled = enabled;
        slf
    }

    /// Get the configured directory.
    fn get_directory(&self) -> &str {
        &self.directory
    }

    /// Get the configured prefix.
    fn get_prefix(&self) -> &str {
        &self.prefix
    }

    /// Get the configured index file.
    fn get_index(&self) -> Option<&str> {
        self.index_file.as_deref()
    }

    /// Check if static file serving is enabled.
    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn __repr__(&self) -> String {
        format!(
            "StaticFilesConfig(directory='{}', prefix='{}', index={:?}, enabled={})",
            self.directory, self.prefix, self.index_file, self.enabled
        )
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // CORS Tests
    #[test]
    fn test_cors_default() {
        let cors = PyCorsConfig::default();
        assert!(!cors.allow_any_origin);
        assert!(cors.allowed_methods.contains("GET"));
        assert!(cors.allowed_methods.contains("POST"));
        assert!(!cors.allow_credentials);
    }

    #[test]
    fn test_cors_is_origin_allowed() {
        let mut cors = PyCorsConfig::default();
        cors.allowed_origins.insert("https://example.com".to_string());
        
        assert!(cors.is_origin_allowed("https://example.com"));
        assert!(!cors.is_origin_allowed("https://other.com"));
    }

    #[test]
    fn test_cors_allow_any_origin() {
        let mut cors = PyCorsConfig::default();
        cors.allow_any_origin = true;
        
        assert!(cors.is_origin_allowed("https://any.com"));
        assert!(cors.is_origin_allowed("https://other.com"));
    }

    #[test]
    fn test_cors_method_allowed() {
        let cors = PyCorsConfig::default();
        assert!(cors.is_method_allowed("GET"));
        assert!(cors.is_method_allowed("get")); // Case insensitive
        assert!(!cors.is_method_allowed("TRACE"));
    }

    #[test]
    fn test_cors_header_allowed() {
        let cors = PyCorsConfig::default();
        assert!(cors.is_header_allowed("Content-Type"));
        assert!(cors.is_header_allowed("content-type")); // Case insensitive
        assert!(!cors.is_header_allowed("X-Custom-Header"));
    }

    // Rate Limit Tests
    #[test]
    fn test_rate_limit_default() {
        let rl = PyRateLimitConfig::default();
        assert_eq!(rl.requests_per_second, 100.0);
        assert_eq!(rl.burst_size, 10);
        assert!(rl.enabled);
    }

    #[test]
    fn test_rate_limit_exempt_paths() {
        let rl = PyRateLimitConfig::default();
        assert!(rl.is_path_exempt("/health"));
        assert!(rl.is_path_exempt("/ready"));
        assert!(!rl.is_path_exempt("/api/users"));
    }

    // Compression Tests
    #[test]
    fn test_compression_default() {
        let comp = PyCompressionConfig::default();
        assert!(comp.enable_gzip);
        assert!(comp.enable_brotli);
        assert!(!comp.enable_deflate);
        assert!(comp.is_enabled());
    }

    #[test]
    fn test_compression_should_compress_type() {
        let comp = PyCompressionConfig::default();
        assert!(comp.should_compress_type("application/json"));
        assert!(comp.should_compress_type("application/json; charset=utf-8"));
        assert!(comp.should_compress_type("text/html"));
        assert!(!comp.should_compress_type("image/png"));
    }

    #[test]
    fn test_compression_disabled_when_no_algorithms() {
        let mut comp = PyCompressionConfig::default();
        comp.enable_gzip = false;
        comp.enable_brotli = false;
        comp.enable_deflate = false;
        comp.enable_zstd = false;
        assert!(!comp.is_enabled());
    }

    // Static Files Tests
    #[test]
    fn test_static_files_default() {
        let sf = PyStaticFilesConfig::default();
        assert_eq!(sf.directory, "./static");
        assert_eq!(sf.prefix, "/static");
        assert_eq!(sf.index_file, Some("index.html".to_string()));
        assert!(sf.enabled);
    }

    #[test]
    fn test_static_files_prefix_normalization() {
        let mut sf = PyStaticFilesConfig::default();
        sf.prefix = "assets".to_string();
        // Should work even without leading slash
        assert!(!sf.prefix.starts_with('/'));
    }
}
