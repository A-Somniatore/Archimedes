//! Compression middleware.
//!
//! This middleware compresses HTTP response bodies using gzip or brotli
//! encoding based on the client's `Accept-Encoding` header.
//!
//! ## Features
//!
//! - **Content negotiation**: Respects `Accept-Encoding` with quality values
//! - **Algorithm selection**: Supports gzip and brotli compression
//! - **Minimum size threshold**: Skip compression for small responses
//! - **Content-Type filtering**: Only compress suitable content types
//! - **Configurable level**: Control compression ratio vs speed tradeoff
//!
//! ## Example
//!
//! ```ignore
//! use archimedes_middleware::stages::CompressionMiddleware;
//!
//! let compression = CompressionMiddleware::builder()
//!     .algorithms([Algorithm::Gzip, Algorithm::Brotli])
//!     .min_size(1024)  // Don't compress < 1KB
//!     .level(CompressionLevel::Default)
//!     .build();
//! ```

use crate::context::MiddlewareContext;
use crate::middleware::{BoxFuture, Middleware, Next};
use crate::types::{Request, Response};
use bytes::Bytes;
use flate2::write::GzEncoder;
use flate2::Compression as GzCompression;
use http::{header, HeaderValue};
use http_body_util::Full;
use std::collections::HashSet;
use std::io::Write;
use std::sync::Arc;

/// Compression algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Algorithm {
    /// Gzip compression (RFC 1952).
    Gzip,
    /// Brotli compression (RFC 7932).
    Brotli,
    /// Deflate compression (RFC 1951).
    Deflate,
    /// Identity (no compression).
    Identity,
}

impl Algorithm {
    /// Returns the HTTP content-encoding value for this algorithm.
    #[must_use]
    pub fn encoding_name(&self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Brotli => "br",
            Self::Deflate => "deflate",
            Self::Identity => "identity",
        }
    }

    /// Parses an algorithm from its HTTP encoding name.
    #[must_use]
    pub fn from_encoding(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "gzip" => Some(Self::Gzip),
            "br" | "brotli" => Some(Self::Brotli),
            "deflate" => Some(Self::Deflate),
            "identity" => Some(Self::Identity),
            _ => None,
        }
    }
}

/// Compression level setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionLevel {
    /// Fastest compression (lowest ratio).
    Fast,
    /// Default balance of speed and ratio.
    #[default]
    Default,
    /// Best compression ratio (slowest).
    Best,
    /// Custom level (0-9 for gzip, 0-11 for brotli).
    Custom(u32),
}

impl CompressionLevel {
    /// Convert to gzip compression level.
    fn to_gzip_level(self) -> GzCompression {
        match self {
            Self::Fast => GzCompression::fast(),
            Self::Default => GzCompression::default(),
            Self::Best => GzCompression::best(),
            Self::Custom(level) => GzCompression::new(level.min(9)),
        }
    }

    /// Convert to brotli compression level.
    fn to_brotli_level(self) -> u32 {
        match self {
            Self::Fast => 1,
            Self::Default => 6,
            Self::Best => 11,
            Self::Custom(level) => level.min(11),
        }
    }
}

/// Compression middleware configuration.
#[derive(Clone)]
pub struct CompressionConfig {
    /// Enabled compression algorithms in preference order.
    algorithms: Vec<Algorithm>,
    /// Minimum response size to compress (in bytes).
    min_size: usize,
    /// Compression level.
    level: CompressionLevel,
    /// Content types to compress (if empty, uses default list).
    content_types: Option<HashSet<String>>,
    /// Content types to never compress.
    excluded_types: HashSet<String>,
    /// Custom predicate to skip compression.
    skip_predicate: Option<Arc<dyn Fn(&Request, &Response) -> bool + Send + Sync>>,
}

impl std::fmt::Debug for CompressionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompressionConfig")
            .field("algorithms", &self.algorithms)
            .field("min_size", &self.min_size)
            .field("level", &self.level)
            .field("content_types", &self.content_types)
            .field("excluded_types", &self.excluded_types)
            .field("skip_predicate", &self.skip_predicate.is_some())
            .finish()
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithms: vec![Algorithm::Gzip, Algorithm::Brotli],
            min_size: 1024, // 1KB minimum
            level: CompressionLevel::Default,
            content_types: None, // Use default list
            excluded_types: Self::default_excluded_types(),
            skip_predicate: None,
        }
    }
}

impl CompressionConfig {
    /// Default content types that should be compressed.
    fn default_compressible_types() -> HashSet<String> {
        [
            // Text types
            "text/plain",
            "text/html",
            "text/css",
            "text/javascript",
            "text/xml",
            "text/csv",
            // Application types
            "application/json",
            "application/javascript",
            "application/xml",
            "application/xhtml+xml",
            "application/rss+xml",
            "application/atom+xml",
            "application/x-www-form-urlencoded",
            "application/ld+json",
            "application/manifest+json",
            "application/graphql",
            // Images that compress well
            "image/svg+xml",
            "image/x-icon",
            // Fonts
            "font/ttf",
            "font/otf",
            "application/font-woff",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
    }

    /// Default content types that should never be compressed.
    fn default_excluded_types() -> HashSet<String> {
        [
            // Already compressed
            "application/gzip",
            "application/x-gzip",
            "application/zip",
            "application/x-rar-compressed",
            "application/x-7z-compressed",
            "application/x-bzip2",
            // Compressed images
            "image/jpeg",
            "image/png",
            "image/gif",
            "image/webp",
            "image/avif",
            // Compressed audio/video
            "audio/mpeg",
            "audio/ogg",
            "video/mp4",
            "video/webm",
            // PDF (already compressed)
            "application/pdf",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
    }
}

/// Compression middleware.
///
/// Compresses HTTP response bodies based on the client's `Accept-Encoding`
/// header and the response's content type.
///
/// # Headers
///
/// - Reads: `Accept-Encoding` from request
/// - Writes: `Content-Encoding` to response
/// - Modifies: `Content-Length` (recalculated after compression)
/// - Adds: `Vary: Accept-Encoding` to response
#[derive(Debug, Clone, Default)]
pub struct CompressionMiddleware {
    config: CompressionConfig,
}

impl CompressionMiddleware {
    /// Creates a new compression middleware with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a builder for custom configuration.
    #[must_use]
    pub fn builder() -> CompressionBuilder {
        CompressionBuilder::new()
    }

    /// Creates compression middleware with specific algorithms.
    #[must_use]
    pub fn with_algorithms(algorithms: impl IntoIterator<Item = Algorithm>) -> Self {
        Self {
            config: CompressionConfig {
                algorithms: algorithms.into_iter().collect(),
                ..Default::default()
            },
        }
    }

    /// Parses the Accept-Encoding header and returns algorithms with quality values.
    fn parse_accept_encoding(header_value: &str) -> Vec<(Algorithm, f32)> {
        let mut encodings = Vec::new();

        for part in header_value.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            let mut quality = 1.0f32;
            let mut encoding = part;

            // Check for quality value (e.g., "gzip;q=0.5")
            if let Some(semicolon_pos) = part.find(';') {
                encoding = &part[..semicolon_pos];
                let params = &part[semicolon_pos + 1..];

                for param in params.split(';') {
                    let param = param.trim();
                    if let Some(q_value) = param.strip_prefix("q=") {
                        if let Ok(q) = q_value.trim().parse::<f32>() {
                            quality = q.clamp(0.0, 1.0);
                        }
                    }
                }
            }

            if let Some(algorithm) = Algorithm::from_encoding(encoding.trim()) {
                encodings.push((algorithm, quality));
            }
        }

        // Sort by quality descending
        encodings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        encodings
    }

    /// Selects the best compression algorithm based on client preferences
    /// and server configuration.
    fn select_algorithm(&self, accept_encoding: &str) -> Option<Algorithm> {
        let client_prefs = Self::parse_accept_encoding(accept_encoding);

        // Find the first client-preferred algorithm that we support
        for (algorithm, quality) in client_prefs {
            // Skip if quality is 0 (explicitly disabled)
            if quality <= 0.0 {
                continue;
            }

            // Identity means no compression
            if algorithm == Algorithm::Identity {
                return None;
            }

            // Check if we support this algorithm
            if self.config.algorithms.contains(&algorithm) {
                return Some(algorithm);
            }
        }

        None
    }

    /// Checks if the content type should be compressed.
    fn should_compress_content_type(&self, content_type: &str) -> bool {
        // Extract base content type (without charset etc.)
        let base_type = content_type
            .split(';')
            .next()
            .unwrap_or(content_type)
            .trim()
            .to_lowercase();

        // Check excluded types first
        if self.config.excluded_types.contains(&base_type) {
            return false;
        }

        // If custom content types are specified, use those
        if let Some(ref types) = self.config.content_types {
            return types.contains(&base_type);
        }

        // Otherwise use default compressible types
        CompressionConfig::default_compressible_types().contains(&base_type)
    }

    /// Compresses data with the specified algorithm.
    fn compress(&self, data: &[u8], algorithm: Algorithm) -> Result<Vec<u8>, CompressionError> {
        match algorithm {
            Algorithm::Gzip => self.compress_gzip(data),
            Algorithm::Brotli => self.compress_brotli(data),
            Algorithm::Deflate => self.compress_deflate(data),
            Algorithm::Identity => Ok(data.to_vec()),
        }
    }

    /// Compresses data with gzip.
    fn compress_gzip(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        let mut encoder = GzEncoder::new(Vec::new(), self.config.level.to_gzip_level());
        encoder
            .write_all(data)
            .map_err(|e| CompressionError::IoError(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| CompressionError::IoError(e.to_string()))
    }

    /// Compresses data with brotli.
    fn compress_brotli(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        let mut output = Vec::new();
        #[allow(clippy::cast_possible_wrap)]
        let quality = self.config.level.to_brotli_level() as i32;
        let params = brotli::enc::BrotliEncoderParams {
            quality,
            ..Default::default()
        };

        brotli::BrotliCompress(&mut std::io::Cursor::new(data), &mut output, &params)
            .map_err(|e| CompressionError::IoError(e.to_string()))?;

        Ok(output)
    }

    /// Compresses data with deflate.
    fn compress_deflate(&self, data: &[u8]) -> Result<Vec<u8>, CompressionError> {
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), self.config.level.to_gzip_level());
        encoder
            .write_all(data)
            .map_err(|e| CompressionError::IoError(e.to_string()))?;
        encoder
            .finish()
            .map_err(|e| CompressionError::IoError(e.to_string()))
    }
}

/// Error type for compression operations.
#[derive(Debug, Clone)]
pub enum CompressionError {
    /// I/O error during compression.
    IoError(String),
    /// Algorithm not supported.
    UnsupportedAlgorithm(String),
}

impl std::fmt::Display for CompressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(msg) => write!(f, "compression I/O error: {msg}"),
            Self::UnsupportedAlgorithm(alg) => write!(f, "unsupported algorithm: {alg}"),
        }
    }
}

impl std::error::Error for CompressionError {}

impl Middleware for CompressionMiddleware {
    fn name(&self) -> &'static str {
        "compression"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Get Accept-Encoding header
            let accept_encoding = request
                .headers()
                .get(header::ACCEPT_ENCODING)
                .and_then(|v| v.to_str().ok())
                .map(String::from);

            // Select compression algorithm
            let algorithm = accept_encoding
                .as_deref()
                .and_then(|ae| self.select_algorithm(ae));

            // Call next middleware
            let mut response = next.run(ctx, request).await;

            // Check if we should compress
            let should_compress = algorithm.is_some() && {
                // Check if response already has Content-Encoding
                let already_encoded = response.headers().contains_key(header::CONTENT_ENCODING);

                // Check content type
                let content_type = response
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("");

                let compressible_type = self.should_compress_content_type(content_type);

                // Check skip predicate
                let skip = self
                    .config
                    .skip_predicate
                    .as_ref()
                    .map(|p| p(&Request::default(), &response))
                    .unwrap_or(false);

                !already_encoded && compressible_type && !skip
            };

            // Add Vary header regardless of whether we compress
            let vary = HeaderValue::from_static("Accept-Encoding");
            response.headers_mut().append(header::VARY, vary);

            if !should_compress {
                return response;
            }

            let algorithm = algorithm.unwrap();

            // Get response body
            let (parts, body) = response.into_parts();

            // Collect body bytes
            let body_bytes = match http_body_util::BodyExt::collect(body).await {
                Ok(collected) => collected.to_bytes(),
                Err(_) => return Response::from_parts(parts, Full::new(Bytes::new())),
            };

            // Check minimum size
            if body_bytes.len() < self.config.min_size {
                return Response::from_parts(parts, Full::new(body_bytes));
            }

            // Compress the body
            let compressed = match self.compress(&body_bytes, algorithm) {
                Ok(data) => data,
                Err(_) => {
                    // On compression error, return uncompressed
                    return Response::from_parts(parts, Full::new(body_bytes));
                }
            };

            // Only use compressed if it's actually smaller
            if compressed.len() >= body_bytes.len() {
                return Response::from_parts(parts, Full::new(body_bytes));
            }

            // Build compressed response
            let mut response = Response::from_parts(parts, Full::new(Bytes::from(compressed)));

            // Update headers
            let encoding = HeaderValue::from_static(algorithm.encoding_name());
            response
                .headers_mut()
                .insert(header::CONTENT_ENCODING, encoding);

            // Remove Content-Length as it's now different
            response.headers_mut().remove(header::CONTENT_LENGTH);

            response
        })
    }
}

/// Builder for compression middleware configuration.
#[derive(Debug, Clone, Default)]
pub struct CompressionBuilder {
    config: CompressionConfig,
}

impl CompressionBuilder {
    /// Creates a new compression builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the enabled compression algorithms in preference order.
    ///
    /// Default: `[Gzip, Brotli]`
    #[must_use]
    pub fn algorithms(mut self, algorithms: impl IntoIterator<Item = Algorithm>) -> Self {
        self.config.algorithms = algorithms.into_iter().collect();
        self
    }

    /// Sets the minimum response size to compress (in bytes).
    ///
    /// Responses smaller than this will not be compressed.
    ///
    /// Default: 1024 (1KB)
    #[must_use]
    pub fn min_size(mut self, min_size: usize) -> Self {
        self.config.min_size = min_size;
        self
    }

    /// Sets the compression level.
    ///
    /// Default: `CompressionLevel::Default`
    #[must_use]
    pub fn level(mut self, level: CompressionLevel) -> Self {
        self.config.level = level;
        self
    }

    /// Sets specific content types to compress.
    ///
    /// If not set, uses a default list of compressible types.
    #[must_use]
    pub fn content_types(mut self, types: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.config.content_types = Some(types.into_iter().map(Into::into).collect());
        self
    }

    /// Adds a content type to the exclusion list.
    ///
    /// These types will never be compressed.
    #[must_use]
    pub fn exclude_type(mut self, content_type: impl Into<String>) -> Self {
        self.config.excluded_types.insert(content_type.into());
        self
    }

    /// Sets a predicate to skip compression for certain request/response pairs.
    #[must_use]
    pub fn skip<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&Request, &Response) -> bool + Send + Sync + 'static,
    {
        self.config.skip_predicate = Some(Arc::new(predicate));
        self
    }

    /// Builds the compression middleware.
    #[must_use]
    pub fn build(self) -> CompressionMiddleware {
        CompressionMiddleware {
            config: self.config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============== Algorithm Tests ==============

    #[test]
    fn test_algorithm_encoding_name() {
        assert_eq!(Algorithm::Gzip.encoding_name(), "gzip");
        assert_eq!(Algorithm::Brotli.encoding_name(), "br");
        assert_eq!(Algorithm::Deflate.encoding_name(), "deflate");
        assert_eq!(Algorithm::Identity.encoding_name(), "identity");
    }

    #[test]
    fn test_algorithm_from_encoding() {
        assert_eq!(Algorithm::from_encoding("gzip"), Some(Algorithm::Gzip));
        assert_eq!(Algorithm::from_encoding("br"), Some(Algorithm::Brotli));
        assert_eq!(Algorithm::from_encoding("brotli"), Some(Algorithm::Brotli));
        assert_eq!(Algorithm::from_encoding("deflate"), Some(Algorithm::Deflate));
        assert_eq!(
            Algorithm::from_encoding("identity"),
            Some(Algorithm::Identity)
        );
        assert_eq!(Algorithm::from_encoding("GZIP"), Some(Algorithm::Gzip));
        assert_eq!(Algorithm::from_encoding("unknown"), None);
    }

    // ============== Compression Level Tests ==============

    #[test]
    fn test_compression_level_default() {
        let level = CompressionLevel::default();
        assert_eq!(level, CompressionLevel::Default);
    }

    #[test]
    fn test_compression_level_to_gzip() {
        // Just verify they don't panic
        let _ = CompressionLevel::Fast.to_gzip_level();
        let _ = CompressionLevel::Default.to_gzip_level();
        let _ = CompressionLevel::Best.to_gzip_level();
        let _ = CompressionLevel::Custom(5).to_gzip_level();
        let _ = CompressionLevel::Custom(100).to_gzip_level(); // Should clamp to 9
    }

    #[test]
    fn test_compression_level_to_brotli() {
        assert_eq!(CompressionLevel::Fast.to_brotli_level(), 1);
        assert_eq!(CompressionLevel::Default.to_brotli_level(), 6);
        assert_eq!(CompressionLevel::Best.to_brotli_level(), 11);
        assert_eq!(CompressionLevel::Custom(5).to_brotli_level(), 5);
        assert_eq!(CompressionLevel::Custom(100).to_brotli_level(), 11); // Clamped
    }

    // ============== Accept-Encoding Parsing Tests ==============

    #[test]
    fn test_parse_accept_encoding_simple() {
        let encodings = CompressionMiddleware::parse_accept_encoding("gzip");
        assert_eq!(encodings.len(), 1);
        assert_eq!(encodings[0], (Algorithm::Gzip, 1.0));
    }

    #[test]
    fn test_parse_accept_encoding_multiple() {
        let encodings = CompressionMiddleware::parse_accept_encoding("gzip, br, deflate");
        assert_eq!(encodings.len(), 3);
        // All have quality 1.0, so order is preserved
        assert!(encodings.iter().any(|(a, _)| *a == Algorithm::Gzip));
        assert!(encodings.iter().any(|(a, _)| *a == Algorithm::Brotli));
        assert!(encodings.iter().any(|(a, _)| *a == Algorithm::Deflate));
    }

    #[test]
    fn test_parse_accept_encoding_with_quality() {
        let encodings = CompressionMiddleware::parse_accept_encoding("gzip;q=0.5, br;q=1.0");
        assert_eq!(encodings.len(), 2);
        // Should be sorted by quality descending
        assert_eq!(encodings[0], (Algorithm::Brotli, 1.0));
        assert_eq!(encodings[1], (Algorithm::Gzip, 0.5));
    }

    #[test]
    fn test_parse_accept_encoding_quality_zero() {
        let encodings = CompressionMiddleware::parse_accept_encoding("gzip;q=0");
        assert_eq!(encodings.len(), 1);
        assert_eq!(encodings[0], (Algorithm::Gzip, 0.0));
    }

    #[test]
    fn test_parse_accept_encoding_wildcard_ignored() {
        let encodings = CompressionMiddleware::parse_accept_encoding("*, gzip");
        // * is not a known algorithm, so only gzip is returned
        assert_eq!(encodings.len(), 1);
        assert_eq!(encodings[0].0, Algorithm::Gzip);
    }

    #[test]
    fn test_parse_accept_encoding_complex() {
        let encodings =
            CompressionMiddleware::parse_accept_encoding("br;q=1.0, gzip;q=0.8, *;q=0.1");
        assert_eq!(encodings.len(), 2); // * is ignored
        assert_eq!(encodings[0], (Algorithm::Brotli, 1.0));
        assert_eq!(encodings[1], (Algorithm::Gzip, 0.8));
    }

    // ============== Algorithm Selection Tests ==============

    #[test]
    fn test_select_algorithm_gzip_only() {
        let middleware = CompressionMiddleware::with_algorithms([Algorithm::Gzip]);
        assert_eq!(
            middleware.select_algorithm("gzip"),
            Some(Algorithm::Gzip)
        );
        assert_eq!(middleware.select_algorithm("br"), None);
    }

    #[test]
    fn test_select_algorithm_prefers_client_order() {
        let middleware =
            CompressionMiddleware::with_algorithms([Algorithm::Gzip, Algorithm::Brotli]);
        // Client prefers brotli
        assert_eq!(
            middleware.select_algorithm("br, gzip"),
            Some(Algorithm::Brotli)
        );
        // Client prefers gzip
        assert_eq!(
            middleware.select_algorithm("gzip, br"),
            Some(Algorithm::Gzip)
        );
    }

    #[test]
    fn test_select_algorithm_uses_quality() {
        let middleware =
            CompressionMiddleware::with_algorithms([Algorithm::Gzip, Algorithm::Brotli]);
        // Client prefers brotli with higher quality
        assert_eq!(
            middleware.select_algorithm("gzip;q=0.5, br;q=1.0"),
            Some(Algorithm::Brotli)
        );
    }

    #[test]
    fn test_select_algorithm_identity() {
        let middleware = CompressionMiddleware::with_algorithms([Algorithm::Gzip]);
        // Client only accepts identity (no compression)
        assert_eq!(middleware.select_algorithm("identity"), None);
    }

    #[test]
    fn test_select_algorithm_quality_zero_skipped() {
        let middleware = CompressionMiddleware::with_algorithms([Algorithm::Gzip]);
        // Client disabled gzip
        assert_eq!(middleware.select_algorithm("gzip;q=0"), None);
    }

    // ============== Content Type Tests ==============

    #[test]
    fn test_should_compress_json() {
        let middleware = CompressionMiddleware::new();
        assert!(middleware.should_compress_content_type("application/json"));
        assert!(middleware.should_compress_content_type("application/json; charset=utf-8"));
    }

    #[test]
    fn test_should_compress_text() {
        let middleware = CompressionMiddleware::new();
        assert!(middleware.should_compress_content_type("text/html"));
        assert!(middleware.should_compress_content_type("text/plain"));
        assert!(middleware.should_compress_content_type("text/css"));
        assert!(middleware.should_compress_content_type("text/javascript"));
    }

    #[test]
    fn test_should_not_compress_images() {
        let middleware = CompressionMiddleware::new();
        assert!(!middleware.should_compress_content_type("image/jpeg"));
        assert!(!middleware.should_compress_content_type("image/png"));
        assert!(!middleware.should_compress_content_type("image/gif"));
    }

    #[test]
    fn test_should_compress_svg() {
        let middleware = CompressionMiddleware::new();
        // SVG is text-based and should compress
        assert!(middleware.should_compress_content_type("image/svg+xml"));
    }

    #[test]
    fn test_should_not_compress_already_compressed() {
        let middleware = CompressionMiddleware::new();
        assert!(!middleware.should_compress_content_type("application/gzip"));
        assert!(!middleware.should_compress_content_type("application/zip"));
        assert!(!middleware.should_compress_content_type("video/mp4"));
    }

    #[test]
    fn test_custom_content_types() {
        let middleware = CompressionMiddleware::builder()
            .content_types(["application/custom"])
            .build();

        assert!(middleware.should_compress_content_type("application/custom"));
        assert!(!middleware.should_compress_content_type("application/json"));
    }

    #[test]
    fn test_exclude_type() {
        let middleware = CompressionMiddleware::builder()
            .exclude_type("application/json")
            .build();

        assert!(!middleware.should_compress_content_type("application/json"));
        assert!(middleware.should_compress_content_type("text/html"));
    }

    // ============== Compression Tests ==============

    #[test]
    fn test_compress_gzip() {
        let middleware = CompressionMiddleware::new();
        let data = b"Hello, World! This is test data for compression.";

        let compressed = middleware.compress(data, Algorithm::Gzip).unwrap();
        assert!(!compressed.is_empty());

        // Verify it's valid gzip by decompressing
        let mut decoder = flate2::read::GzDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_brotli() {
        let middleware = CompressionMiddleware::new();
        let data = b"Hello, World! This is test data for compression.";

        let compressed = middleware.compress(data, Algorithm::Brotli).unwrap();
        assert!(!compressed.is_empty());

        // Verify it's valid brotli by decompressing
        let mut decompressed = Vec::new();
        brotli::BrotliDecompress(&mut std::io::Cursor::new(&compressed), &mut decompressed)
            .unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_deflate() {
        let middleware = CompressionMiddleware::new();
        let data = b"Hello, World! This is test data for compression.";

        let compressed = middleware.compress(data, Algorithm::Deflate).unwrap();
        assert!(!compressed.is_empty());

        // Verify it's valid deflate by decompressing
        let mut decoder = flate2::read::DeflateDecoder::new(&compressed[..]);
        let mut decompressed = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decompressed).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_identity() {
        let middleware = CompressionMiddleware::new();
        let data = b"Hello, World!";

        let result = middleware.compress(data, Algorithm::Identity).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn test_compress_large_data() {
        let middleware = CompressionMiddleware::new();
        // Create large repetitive data that compresses well
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();

        let compressed = middleware.compress(&data, Algorithm::Gzip).unwrap();
        // Should be significantly smaller
        assert!(compressed.len() < data.len());
    }

    // ============== Builder Tests ==============

    #[test]
    fn test_builder_default() {
        let middleware = CompressionMiddleware::builder().build();
        assert_eq!(middleware.config.algorithms.len(), 2);
        assert_eq!(middleware.config.min_size, 1024);
    }

    #[test]
    fn test_builder_algorithms() {
        let middleware = CompressionMiddleware::builder()
            .algorithms([Algorithm::Brotli])
            .build();

        assert_eq!(middleware.config.algorithms, vec![Algorithm::Brotli]);
    }

    #[test]
    fn test_builder_min_size() {
        let middleware = CompressionMiddleware::builder().min_size(2048).build();

        assert_eq!(middleware.config.min_size, 2048);
    }

    #[test]
    fn test_builder_level() {
        let middleware = CompressionMiddleware::builder()
            .level(CompressionLevel::Best)
            .build();

        assert_eq!(middleware.config.level, CompressionLevel::Best);
    }

    #[test]
    fn test_builder_skip_predicate() {
        let middleware = CompressionMiddleware::builder()
            .skip(|_req, _res| true)
            .build();

        assert!(middleware.config.skip_predicate.is_some());
    }

    // ============== Config Tests ==============

    #[test]
    fn test_config_default() {
        let config = CompressionConfig::default();
        assert_eq!(config.algorithms.len(), 2);
        assert_eq!(config.min_size, 1024);
        assert_eq!(config.level, CompressionLevel::Default);
    }

    #[test]
    fn test_default_compressible_types() {
        let types = CompressionConfig::default_compressible_types();
        assert!(types.contains("application/json"));
        assert!(types.contains("text/html"));
        assert!(!types.contains("image/jpeg"));
    }

    #[test]
    fn test_default_excluded_types() {
        let types = CompressionConfig::default_excluded_types();
        assert!(types.contains("image/jpeg"));
        assert!(types.contains("application/zip"));
        assert!(!types.contains("text/plain"));
    }

    // ============== Debug/Display Tests ==============

    #[test]
    fn test_algorithm_debug() {
        assert_eq!(format!("{:?}", Algorithm::Gzip), "Gzip");
        assert_eq!(format!("{:?}", Algorithm::Brotli), "Brotli");
    }

    #[test]
    fn test_compression_error_display() {
        let err = CompressionError::IoError("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = CompressionError::UnsupportedAlgorithm("xyz".to_string());
        assert!(err.to_string().contains("xyz"));
    }

    #[test]
    fn test_config_debug() {
        let config = CompressionConfig::default();
        let debug = format!("{config:?}");
        assert!(debug.contains("algorithms"));
        assert!(debug.contains("min_size"));
    }
}
