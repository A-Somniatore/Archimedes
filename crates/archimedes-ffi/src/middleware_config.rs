//! Middleware configuration types for C FFI.
//!
//! This module exposes configurable middleware to C/C++:
//! - CORS middleware configuration
//! - Rate limiting configuration
//! - Compression configuration
//! - Static file serving configuration
//!
//! ## Memory Management
//!
//! All config objects must be freed with their respective `_free` functions:
//! - `archimedes_cors_config_free()`
//! - `archimedes_rate_limit_config_free()`
//! - `archimedes_compression_config_free()`
//! - `archimedes_static_files_config_free()`
//!
//! ## Example (C++)
//!
//! ```cpp
//! #include <archimedes/middleware_config.h>
//!
//! // CORS configuration
//! auto* cors = archimedes_cors_config_new();
//! archimedes_cors_config_allow_origin(cors, "https://example.com");
//! archimedes_cors_config_allow_method(cors, "GET");
//! archimedes_cors_config_allow_credentials(cors, true);
//! // ... use cors config ...
//! archimedes_cors_config_free(cors);
//! ```

use std::collections::HashSet;
use std::ffi::{c_char, CStr};
use std::ptr;

// ============================================================================
// CORS Configuration
// ============================================================================

/// Opaque CORS configuration handle.
pub struct ArchimedesCorsConfig {
    allowed_origins: HashSet<String>,
    allow_any_origin: bool,
    allowed_methods: HashSet<String>,
    allowed_headers: HashSet<String>,
    exposed_headers: HashSet<String>,
    allow_credentials: bool,
    max_age_seconds: u32,
}

impl Default for ArchimedesCorsConfig {
    fn default() -> Self {
        let mut config = Self {
            allowed_origins: HashSet::new(),
            allow_any_origin: false,
            allowed_methods: HashSet::new(),
            allowed_headers: HashSet::new(),
            exposed_headers: HashSet::new(),
            allow_credentials: false,
            max_age_seconds: 3600,
        };
        // Default methods
        for method in &["GET", "HEAD", "POST", "PUT", "DELETE", "PATCH"] {
            config.allowed_methods.insert((*method).to_string());
        }
        // Default headers
        for header in &["content-type", "authorization", "x-request-id"] {
            config.allowed_headers.insert((*header).to_string());
        }
        config
    }
}

/// Create a new CORS configuration with sensible defaults.
#[unsafe(no_mangle)]
pub extern "C" fn archimedes_cors_config_new() -> *mut ArchimedesCorsConfig {
    Box::into_raw(Box::new(ArchimedesCorsConfig::default()))
}

/// Free a CORS configuration.
///
/// # Safety
/// The pointer must be valid and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_free(config: *mut ArchimedesCorsConfig) {
    if !config.is_null() {
        drop(Box::from_raw(config));
    }
}

/// Allow requests from any origin.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_allow_any_origin(config: *mut ArchimedesCorsConfig) {
    if let Some(config) = config.as_mut() {
        config.allow_any_origin = true;
    }
}

/// Add an allowed origin.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_allow_origin(
    config: *mut ArchimedesCorsConfig,
    origin: *const c_char,
) {
    if let (Some(config), Some(origin)) = (config.as_mut(), ptr_to_str(origin)) {
        config.allowed_origins.insert(origin.to_string());
    }
}

/// Add an allowed HTTP method.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_allow_method(
    config: *mut ArchimedesCorsConfig,
    method: *const c_char,
) {
    if let (Some(config), Some(method)) = (config.as_mut(), ptr_to_str(method)) {
        config.allowed_methods.insert(method.to_uppercase());
    }
}

/// Add an allowed request header.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_allow_header(
    config: *mut ArchimedesCorsConfig,
    header: *const c_char,
) {
    if let (Some(config), Some(header)) = (config.as_mut(), ptr_to_str(header)) {
        config.allowed_headers.insert(header.to_lowercase());
    }
}

/// Add a header to expose to the browser.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_expose_header(
    config: *mut ArchimedesCorsConfig,
    header: *const c_char,
) {
    if let (Some(config), Some(header)) = (config.as_mut(), ptr_to_str(header)) {
        config.exposed_headers.insert(header.to_string());
    }
}

/// Set whether credentials are allowed.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_allow_credentials(
    config: *mut ArchimedesCorsConfig,
    allow: bool,
) {
    if let Some(config) = config.as_mut() {
        config.allow_credentials = allow;
    }
}

/// Set the max age for preflight cache (in seconds).
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_max_age(
    config: *mut ArchimedesCorsConfig,
    seconds: u32,
) {
    if let Some(config) = config.as_mut() {
        config.max_age_seconds = seconds;
    }
}

/// Check if an origin is allowed.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_is_origin_allowed(
    config: *const ArchimedesCorsConfig,
    origin: *const c_char,
) -> bool {
    if let (Some(config), Some(origin)) = (config.as_ref(), ptr_to_str(origin)) {
        config.allow_any_origin || config.allowed_origins.contains(origin)
    } else {
        false
    }
}

/// Check if a method is allowed.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_is_method_allowed(
    config: *const ArchimedesCorsConfig,
    method: *const c_char,
) -> bool {
    if let (Some(config), Some(method)) = (config.as_ref(), ptr_to_str(method)) {
        config.allowed_methods.contains(&method.to_uppercase())
    } else {
        false
    }
}

/// Get the max age in seconds.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_get_max_age(
    config: *const ArchimedesCorsConfig,
) -> u32 {
    config.as_ref().map_or(0, |c| c.max_age_seconds)
}

/// Check if credentials are allowed.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_cors_config_get_allow_credentials(
    config: *const ArchimedesCorsConfig,
) -> bool {
    config.as_ref().is_some_and(|c| c.allow_credentials)
}

// ============================================================================
// Rate Limit Configuration
// ============================================================================

/// Opaque rate limit configuration handle.
pub struct ArchimedesRateLimitConfig {
    requests_per_second: f64,
    burst_size: u32,
    key_extractor: String,
    exempt_paths: HashSet<String>,
    enabled: bool,
}

impl Default for ArchimedesRateLimitConfig {
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

/// Create a new rate limit configuration with sensible defaults.
#[unsafe(no_mangle)]
pub extern "C" fn archimedes_rate_limit_config_new() -> *mut ArchimedesRateLimitConfig {
    Box::into_raw(Box::new(ArchimedesRateLimitConfig::default()))
}

/// Free a rate limit configuration.
///
/// # Safety
/// The pointer must be valid and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_free(config: *mut ArchimedesRateLimitConfig) {
    if !config.is_null() {
        drop(Box::from_raw(config));
    }
}

/// Set the requests per second limit.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_rps(
    config: *mut ArchimedesRateLimitConfig,
    rps: f64,
) {
    if let Some(config) = config.as_mut() {
        config.requests_per_second = rps;
    }
}

/// Set the burst size.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_burst(
    config: *mut ArchimedesRateLimitConfig,
    size: u32,
) {
    if let Some(config) = config.as_mut() {
        config.burst_size = size;
    }
}

/// Set the key extractor type.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_key_extractor(
    config: *mut ArchimedesRateLimitConfig,
    extractor: *const c_char,
) {
    if let (Some(config), Some(extractor)) = (config.as_mut(), ptr_to_str(extractor)) {
        config.key_extractor = extractor.to_string();
    }
}

/// Add a path to exempt from rate limiting.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_exempt_path(
    config: *mut ArchimedesRateLimitConfig,
    path: *const c_char,
) {
    if let (Some(config), Some(path)) = (config.as_mut(), ptr_to_str(path)) {
        config.exempt_paths.insert(path.to_string());
    }
}

/// Enable or disable rate limiting.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_enabled(
    config: *mut ArchimedesRateLimitConfig,
    enabled: bool,
) {
    if let Some(config) = config.as_mut() {
        config.enabled = enabled;
    }
}

/// Check if a path is exempt from rate limiting.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_is_exempt(
    config: *const ArchimedesRateLimitConfig,
    path: *const c_char,
) -> bool {
    if let (Some(config), Some(path)) = (config.as_ref(), ptr_to_str(path)) {
        config.exempt_paths.contains(path)
    } else {
        false
    }
}

/// Get the requests per second limit.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_get_rps(
    config: *const ArchimedesRateLimitConfig,
) -> f64 {
    config.as_ref().map_or(0.0, |c| c.requests_per_second)
}

/// Get the burst size.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_get_burst(
    config: *const ArchimedesRateLimitConfig,
) -> u32 {
    config.as_ref().map_or(0, |c| c.burst_size)
}

/// Check if rate limiting is enabled.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_rate_limit_config_is_enabled(
    config: *const ArchimedesRateLimitConfig,
) -> bool {
    config.as_ref().is_some_and(|c| c.enabled)
}

// ============================================================================
// Compression Configuration
// ============================================================================

/// Compression algorithm enum.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchimedesCompressionAlgorithm {
    Gzip = 0,
    Brotli = 1,
    Deflate = 2,
    Zstd = 3,
}

/// Opaque compression configuration handle.
pub struct ArchimedesCompressionConfig {
    enable_gzip: bool,
    enable_brotli: bool,
    enable_deflate: bool,
    enable_zstd: bool,
    min_size_bytes: usize,
    compression_level: u32,
    content_types: HashSet<String>,
}

impl Default for ArchimedesCompressionConfig {
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

/// Create a new compression configuration with sensible defaults.
#[unsafe(no_mangle)]
pub extern "C" fn archimedes_compression_config_new() -> *mut ArchimedesCompressionConfig {
    Box::into_raw(Box::new(ArchimedesCompressionConfig::default()))
}

/// Free a compression configuration.
///
/// # Safety
/// The pointer must be valid and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_free(
    config: *mut ArchimedesCompressionConfig,
) {
    if !config.is_null() {
        drop(Box::from_raw(config));
    }
}

/// Enable or disable gzip compression.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_gzip(
    config: *mut ArchimedesCompressionConfig,
    enable: bool,
) {
    if let Some(config) = config.as_mut() {
        config.enable_gzip = enable;
    }
}

/// Enable or disable Brotli compression.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_brotli(
    config: *mut ArchimedesCompressionConfig,
    enable: bool,
) {
    if let Some(config) = config.as_mut() {
        config.enable_brotli = enable;
    }
}

/// Enable or disable deflate compression.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_deflate(
    config: *mut ArchimedesCompressionConfig,
    enable: bool,
) {
    if let Some(config) = config.as_mut() {
        config.enable_deflate = enable;
    }
}

/// Enable or disable Zstandard compression.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_zstd(
    config: *mut ArchimedesCompressionConfig,
    enable: bool,
) {
    if let Some(config) = config.as_mut() {
        config.enable_zstd = enable;
    }
}

/// Set the minimum response size to compress.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_min_size(
    config: *mut ArchimedesCompressionConfig,
    bytes: usize,
) {
    if let Some(config) = config.as_mut() {
        config.min_size_bytes = bytes;
    }
}

/// Set the compression level (1-9).
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_level(
    config: *mut ArchimedesCompressionConfig,
    level: u32,
) {
    if let Some(config) = config.as_mut() {
        config.compression_level = level.clamp(1, 9);
    }
}

/// Add a content type to compress.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_add_content_type(
    config: *mut ArchimedesCompressionConfig,
    content_type: *const c_char,
) {
    if let (Some(config), Some(ct)) = (config.as_mut(), ptr_to_str(content_type)) {
        config.content_types.insert(ct.to_string());
    }
}

/// Check if gzip is enabled.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_is_gzip(
    config: *const ArchimedesCompressionConfig,
) -> bool {
    config.as_ref().is_some_and(|c| c.enable_gzip)
}

/// Check if Brotli is enabled.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_is_brotli(
    config: *const ArchimedesCompressionConfig,
) -> bool {
    config.as_ref().is_some_and(|c| c.enable_brotli)
}

/// Get the minimum size threshold.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_get_min_size(
    config: *const ArchimedesCompressionConfig,
) -> usize {
    config.as_ref().map_or(0, |c| c.min_size_bytes)
}

/// Get the compression level.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_get_level(
    config: *const ArchimedesCompressionConfig,
) -> u32 {
    config.as_ref().map_or(0, |c| c.compression_level)
}

/// Check if a content type should be compressed.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_compression_config_should_compress(
    config: *const ArchimedesCompressionConfig,
    content_type: *const c_char,
) -> bool {
    if let (Some(config), Some(ct)) = (config.as_ref(), ptr_to_str(content_type)) {
        config
            .content_types
            .iter()
            .any(|t| ct == t || ct.starts_with(&format!("{t};")))
    } else {
        false
    }
}

// ============================================================================
// Static Files Configuration
// ============================================================================

/// Opaque static files configuration handle.
pub struct ArchimedesStaticFilesConfig {
    directory: String,
    prefix: String,
    index_file: String,
    cache_max_age_seconds: u32,
    enable_precompressed: bool,
    fallback_file: Option<String>,
}

impl Default for ArchimedesStaticFilesConfig {
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

/// Create a new static files configuration with sensible defaults.
#[unsafe(no_mangle)]
pub extern "C" fn archimedes_static_files_config_new() -> *mut ArchimedesStaticFilesConfig {
    Box::into_raw(Box::new(ArchimedesStaticFilesConfig::default()))
}

/// Free a static files configuration.
///
/// # Safety
/// The pointer must be valid and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_free(
    config: *mut ArchimedesStaticFilesConfig,
) {
    if !config.is_null() {
        drop(Box::from_raw(config));
    }
}

/// Set the directory to serve files from.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_directory(
    config: *mut ArchimedesStaticFilesConfig,
    dir: *const c_char,
) {
    if let (Some(config), Some(dir)) = (config.as_mut(), ptr_to_str(dir)) {
        config.directory = dir.to_string();
    }
}

/// Set the URL prefix.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_prefix(
    config: *mut ArchimedesStaticFilesConfig,
    prefix: *const c_char,
) {
    if let (Some(config), Some(prefix)) = (config.as_mut(), ptr_to_str(prefix)) {
        config.prefix = if prefix.starts_with('/') {
            prefix.to_string()
        } else {
            format!("/{prefix}")
        };
    }
}

/// Set the index file name.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_index(
    config: *mut ArchimedesStaticFilesConfig,
    file: *const c_char,
) {
    if let (Some(config), Some(file)) = (config.as_mut(), ptr_to_str(file)) {
        config.index_file = file.to_string();
    }
}

/// Set the cache max age in seconds.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_cache_max_age(
    config: *mut ArchimedesStaticFilesConfig,
    seconds: u32,
) {
    if let Some(config) = config.as_mut() {
        config.cache_max_age_seconds = seconds;
    }
}

/// Enable or disable serving precompressed files.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_precompressed(
    config: *mut ArchimedesStaticFilesConfig,
    enable: bool,
) {
    if let Some(config) = config.as_mut() {
        config.enable_precompressed = enable;
    }
}

/// Set a fallback file for SPA routing.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_fallback(
    config: *mut ArchimedesStaticFilesConfig,
    file: *const c_char,
) {
    if let (Some(config), Some(file)) = (config.as_mut(), ptr_to_str(file)) {
        config.fallback_file = Some(file.to_string());
    }
}

/// Get the cache max age in seconds.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_get_cache_max_age(
    config: *const ArchimedesStaticFilesConfig,
) -> u32 {
    config.as_ref().map_or(0, |c| c.cache_max_age_seconds)
}

/// Check if precompressed files are enabled.
///
/// # Safety
/// The config pointer must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_is_precompressed(
    config: *const ArchimedesStaticFilesConfig,
) -> bool {
    config.as_ref().is_some_and(|c| c.enable_precompressed)
}

/// Resolve a request path to a file path.
/// Returns null if the path doesn't match the prefix or is invalid.
/// The returned string must be freed with `archimedes_free_string`.
///
/// # Safety
/// Both pointers must be valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn archimedes_static_files_config_resolve_path(
    config: *const ArchimedesStaticFilesConfig,
    request_path: *const c_char,
) -> *mut c_char {
    let Some(config) = config.as_ref() else {
        return ptr::null_mut();
    };
    let Some(request_path) = ptr_to_str(request_path) else {
        return ptr::null_mut();
    };

    if !request_path.starts_with(&config.prefix) {
        return ptr::null_mut();
    }

    let relative = request_path
        .strip_prefix(&config.prefix)
        .unwrap_or(request_path);
    let relative = relative.trim_start_matches('/');

    // Prevent directory traversal
    if relative.contains("..") {
        return ptr::null_mut();
    }

    let file_path = if relative.is_empty() {
        format!("{}/{}", config.directory, config.index_file)
    } else {
        format!("{}/{}", config.directory, relative)
    };

    match std::ffi::CString::new(file_path) {
        Ok(s) => s.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert a C string pointer to a Rust str.
///
/// # Safety
/// The pointer must be valid and null-terminated.
unsafe fn ptr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn c_str(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    #[test]
    fn test_cors_config_new_and_free() {
        let config = archimedes_cors_config_new();
        assert!(!config.is_null());
        unsafe {
            archimedes_cors_config_free(config);
        }
    }

    #[test]
    fn test_cors_config_defaults() {
        let config = archimedes_cors_config_new();
        unsafe {
            assert!(!archimedes_cors_config_get_allow_credentials(config));
            assert_eq!(archimedes_cors_config_get_max_age(config), 3600);

            let method = c_str("GET");
            assert!(archimedes_cors_config_is_method_allowed(
                config,
                method.as_ptr()
            ));

            archimedes_cors_config_free(config);
        }
    }

    #[test]
    fn test_cors_config_allow_origin() {
        let config = archimedes_cors_config_new();
        unsafe {
            let origin = c_str("https://example.com");
            archimedes_cors_config_allow_origin(config, origin.as_ptr());
            assert!(archimedes_cors_config_is_origin_allowed(
                config,
                origin.as_ptr()
            ));

            let other = c_str("https://other.com");
            assert!(!archimedes_cors_config_is_origin_allowed(
                config,
                other.as_ptr()
            ));

            archimedes_cors_config_free(config);
        }
    }

    #[test]
    fn test_rate_limit_config_new_and_free() {
        let config = archimedes_rate_limit_config_new();
        assert!(!config.is_null());
        unsafe {
            archimedes_rate_limit_config_free(config);
        }
    }

    #[test]
    fn test_rate_limit_config_defaults() {
        let config = archimedes_rate_limit_config_new();
        unsafe {
            assert!((archimedes_rate_limit_config_get_rps(config) - 100.0).abs() < f64::EPSILON);
            assert_eq!(archimedes_rate_limit_config_get_burst(config), 10);
            assert!(archimedes_rate_limit_config_is_enabled(config));

            let health = c_str("/health");
            assert!(archimedes_rate_limit_config_is_exempt(config, health.as_ptr()));

            archimedes_rate_limit_config_free(config);
        }
    }

    #[test]
    fn test_compression_config_new_and_free() {
        let config = archimedes_compression_config_new();
        assert!(!config.is_null());
        unsafe {
            archimedes_compression_config_free(config);
        }
    }

    #[test]
    fn test_compression_config_defaults() {
        let config = archimedes_compression_config_new();
        unsafe {
            assert!(archimedes_compression_config_is_gzip(config));
            assert!(archimedes_compression_config_is_brotli(config));
            assert_eq!(archimedes_compression_config_get_min_size(config), 860);
            assert_eq!(archimedes_compression_config_get_level(config), 4);

            let json = c_str("application/json");
            assert!(archimedes_compression_config_should_compress(
                config,
                json.as_ptr()
            ));

            let png = c_str("image/png");
            assert!(!archimedes_compression_config_should_compress(
                config,
                png.as_ptr()
            ));

            archimedes_compression_config_free(config);
        }
    }

    #[test]
    fn test_static_files_config_new_and_free() {
        let config = archimedes_static_files_config_new();
        assert!(!config.is_null());
        unsafe {
            archimedes_static_files_config_free(config);
        }
    }

    #[test]
    fn test_static_files_config_defaults() {
        let config = archimedes_static_files_config_new();
        unsafe {
            assert_eq!(
                archimedes_static_files_config_get_cache_max_age(config),
                86400
            );
            assert!(archimedes_static_files_config_is_precompressed(config));
            archimedes_static_files_config_free(config);
        }
    }

    #[test]
    fn test_static_files_config_resolve_path() {
        let config = archimedes_static_files_config_new();
        unsafe {
            let path = c_str("/static/js/app.js");
            let resolved = archimedes_static_files_config_resolve_path(config, path.as_ptr());
            assert!(!resolved.is_null());
            let resolved_str = CStr::from_ptr(resolved).to_str().unwrap();
            assert_eq!(resolved_str, "./static/js/app.js");
            // Free using CString::from_raw
            drop(std::ffi::CString::from_raw(resolved));

            // Path traversal should fail
            let traversal = c_str("/static/../secret.txt");
            let result = archimedes_static_files_config_resolve_path(config, traversal.as_ptr());
            assert!(result.is_null());

            archimedes_static_files_config_free(config);
        }
    }
}
