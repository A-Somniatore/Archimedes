//! Static file serving for Archimedes.
//!
//! This module provides a static file server that can serve files from a
//! directory, with support for:
//!
//! - Index file fallback (`index.html`)
//! - Cache headers (`Cache-Control`, `ETag`, `Last-Modified`)
//! - Range requests for large files
//! - Security measures against directory traversal
//! - MIME type detection
//!
//! # Example
//!
//! ```rust
//! use archimedes_server::static_files::{StaticFiles, StaticFilesBuilder};
//!
//! let static_files = StaticFiles::new("./public")
//!     .index("index.html")
//!     .cache_control("max-age=3600");
//! ```
//!
//! # Security
//!
//! The static file server includes several security measures:
//!
//! - Path traversal prevention (rejects `..` in paths)
//! - Symlink resolution validation (optional)
//! - Hidden file filtering (files starting with `.`)

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bytes::Bytes;
use http::{header, HeaderMap, Method, Response, StatusCode};
use http_body_util::Full;
use thiserror::Error;

/// Type alias for HTTP response body.
pub type ResponseBody = Full<Bytes>;

/// Type alias for the HTTP response.
pub type HttpResponse = Response<ResponseBody>;

/// Errors that can occur when serving static files.
#[derive(Debug, Error)]
pub enum StaticFileError {
    /// The requested file was not found.
    #[error("File not found: {0}")]
    NotFound(String),

    /// The path is forbidden (e.g., directory traversal attempt).
    #[error("Forbidden path: {0}")]
    Forbidden(String),

    /// Method not allowed (e.g., POST to static file).
    #[error("Method not allowed")]
    MethodNotAllowed,

    /// I/O error while reading file.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Invalid range header.
    #[error("Invalid range: {0}")]
    InvalidRange(String),
}

impl StaticFileError {
    /// Returns the HTTP status code for this error.
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
            Self::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidRange(_) => StatusCode::RANGE_NOT_SATISFIABLE,
        }
    }
}

/// Static file server configuration and handler.
///
/// # Example
///
/// ```rust
/// use archimedes_server::static_files::StaticFiles;
///
/// let files = StaticFiles::new("./public")
///     .index("index.html")
///     .cache_control("max-age=86400")
///     .precompressed_gzip(true);
/// ```
#[derive(Debug, Clone)]
pub struct StaticFiles {
    /// Root directory for static files
    root: PathBuf,

    /// Index file name (e.g., "index.html")
    index_file: Option<String>,

    /// Default Cache-Control header value
    cache_control: Option<String>,

    /// Whether to include `ETag` headers
    etag_enabled: bool,

    /// Whether to include Last-Modified headers
    last_modified_enabled: bool,

    /// Whether to look for precompressed .gz files
    precompressed_gzip: bool,

    /// Whether to look for precompressed .br files
    precompressed_brotli: bool,

    /// Whether to serve hidden files (starting with .)
    serve_hidden: bool,

    /// Whether to follow symlinks
    follow_symlinks: bool,

    /// Custom MIME type mappings
    mime_types: HashMap<String, String>,
}

impl StaticFiles {
    /// Creates a new static file server for the given root directory.
    ///
    /// # Arguments
    ///
    /// * `root` - Path to the directory containing static files
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public");
    /// ```
    #[must_use]
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            index_file: None,
            cache_control: None,
            etag_enabled: true,
            last_modified_enabled: true,
            precompressed_gzip: false,
            precompressed_brotli: false,
            serve_hidden: false,
            follow_symlinks: true,
            mime_types: HashMap::new(),
        }
    }

    /// Sets the index file to serve for directory requests.
    ///
    /// # Arguments
    ///
    /// * `index` - Index file name (e.g., "index.html")
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public")
    ///     .index("index.html");
    /// ```
    #[must_use]
    pub fn index<S: Into<String>>(mut self, index: S) -> Self {
        self.index_file = Some(index.into());
        self
    }

    /// Sets the Cache-Control header value for responses.
    ///
    /// # Arguments
    ///
    /// * `value` - Cache-Control header value (e.g., "max-age=3600")
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public")
    ///     .cache_control("max-age=86400, public");
    /// ```
    #[must_use]
    pub fn cache_control<S: Into<String>>(mut self, value: S) -> Self {
        self.cache_control = Some(value.into());
        self
    }

    /// Enables or disables `ETag` headers.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to include `ETag` headers
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public")
    ///     .etag(true);
    /// ```
    #[must_use]
    pub fn etag(mut self, enabled: bool) -> Self {
        self.etag_enabled = enabled;
        self
    }

    /// Enables or disables Last-Modified headers.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to include Last-Modified headers
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public")
    ///     .last_modified(true);
    /// ```
    #[must_use]
    pub fn last_modified(mut self, enabled: bool) -> Self {
        self.last_modified_enabled = enabled;
        self
    }

    /// Enables or disables serving precompressed `.gz` files.
    ///
    /// When enabled, the server will look for a `.gz` version of the file
    /// if the client accepts gzip encoding.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to serve precompressed gzip files
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public")
    ///     .precompressed_gzip(true);
    /// ```
    #[must_use]
    pub fn precompressed_gzip(mut self, enabled: bool) -> Self {
        self.precompressed_gzip = enabled;
        self
    }

    /// Enables or disables serving precompressed `.br` files.
    ///
    /// When enabled, the server will look for a `.br` version of the file
    /// if the client accepts brotli encoding.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to serve precompressed brotli files
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public")
    ///     .precompressed_brotli(true);
    /// ```
    #[must_use]
    pub fn precompressed_brotli(mut self, enabled: bool) -> Self {
        self.precompressed_brotli = enabled;
        self
    }

    /// Enables or disables serving hidden files (starting with `.`).
    ///
    /// By default, hidden files are not served.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to serve hidden files
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public")
    ///     .serve_hidden(true);
    /// ```
    #[must_use]
    pub fn serve_hidden(mut self, enabled: bool) -> Self {
        self.serve_hidden = enabled;
        self
    }

    /// Enables or disables following symlinks.
    ///
    /// By default, symlinks are followed.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to follow symlinks
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public")
    ///     .follow_symlinks(false);
    /// ```
    #[must_use]
    pub fn follow_symlinks(mut self, enabled: bool) -> Self {
        self.follow_symlinks = enabled;
        self
    }

    /// Adds a custom MIME type mapping.
    ///
    /// # Arguments
    ///
    /// * `extension` - File extension (without leading dot)
    /// * `mime_type` - MIME type string
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::static_files::StaticFiles;
    ///
    /// let files = StaticFiles::new("./public")
    ///     .mime_type("wasm", "application/wasm")
    ///     .mime_type("map", "application/json");
    /// ```
    #[must_use]
    pub fn mime_type<S1: Into<String>, S2: Into<String>>(
        mut self,
        extension: S1,
        mime_type: S2,
    ) -> Self {
        self.mime_types.insert(extension.into(), mime_type.into());
        self
    }

    /// Returns the root directory path.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the configured index file.
    #[must_use]
    pub fn index_file(&self) -> Option<&str> {
        self.index_file.as_deref()
    }

    /// Handles an HTTP request for a static file.
    ///
    /// # Arguments
    ///
    /// * `request_path` - The URL path from the request (relative to mount point)
    /// * `headers` - The request headers (for Accept-Encoding, If-None-Match, etc.)
    /// * `method` - The HTTP method
    ///
    /// # Returns
    ///
    /// Returns an HTTP response containing the file contents, or an error response.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The method is not GET or HEAD
    /// - The path contains directory traversal attempts
    /// - The file is not found
    /// - An I/O error occurs
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use archimedes_server::static_files::StaticFiles;
    /// use http::{Method, HeaderMap};
    ///
    /// let files = StaticFiles::new("./public");
    /// let response = files.handle("/styles.css", &HeaderMap::new(), &Method::GET)?;
    /// ```
    pub fn handle(
        &self,
        request_path: &str,
        headers: &HeaderMap,
        method: &Method,
    ) -> Result<HttpResponse, StaticFileError> {
        // Only allow GET and HEAD methods
        if method != Method::GET && method != Method::HEAD {
            return Err(StaticFileError::MethodNotAllowed);
        }

        // Sanitize and resolve the path
        let file_path = self.resolve_path(request_path)?;

        // Check if it's a directory
        if file_path.is_dir() {
            // Try to serve index file
            if let Some(ref index) = self.index_file {
                let index_path = file_path.join(index);
                if index_path.is_file() {
                    return self.serve_file(&index_path, headers, method);
                }
            }
            return Err(StaticFileError::NotFound(request_path.to_string()));
        }

        // Serve the file
        self.serve_file(&file_path, headers, method)
    }

    /// Resolves a request path to an absolute file path.
    ///
    /// This method performs security checks to prevent directory traversal.
    fn resolve_path(&self, request_path: &str) -> Result<PathBuf, StaticFileError> {
        // Remove leading slash and normalize
        let path = request_path.trim_start_matches('/');

        // Check for directory traversal attempts
        for component in Path::new(path).components() {
            match component {
                std::path::Component::ParentDir => {
                    return Err(StaticFileError::Forbidden(
                        "Directory traversal not allowed".to_string(),
                    ));
                }
                std::path::Component::Normal(name) => {
                    // Check for hidden files if not allowed
                    if !self.serve_hidden {
                        if let Some(name_str) = name.to_str() {
                            if name_str.starts_with('.') {
                                return Err(StaticFileError::Forbidden(
                                    "Hidden files not allowed".to_string(),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Build the full path
        let full_path = self.root.join(path);

        // Canonicalize to resolve symlinks and get absolute path
        let canonical = if self.follow_symlinks {
            full_path.canonicalize().map_err(|_| {
                StaticFileError::NotFound(request_path.to_string())
            })?
        } else {
            // If not following symlinks, check if it's a symlink
            if full_path.is_symlink() {
                return Err(StaticFileError::Forbidden(
                    "Symlinks not allowed".to_string(),
                ));
            }
            full_path
        };

        // Verify the resolved path is within the root directory
        let canonical_root = self.root.canonicalize().map_err(|e| {
            StaticFileError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Root directory not found: {}", e),
            ))
        })?;

        if !canonical.starts_with(&canonical_root) {
            return Err(StaticFileError::Forbidden(
                "Path escapes root directory".to_string(),
            ));
        }

        Ok(canonical)
    }

    /// Serves a file from the filesystem.
    fn serve_file(
        &self,
        path: &Path,
        headers: &HeaderMap,
        method: &Method,
    ) -> Result<HttpResponse, StaticFileError> {
        // Get file metadata
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();
        let modified = metadata.modified().ok();

        // Check for precompressed versions
        let (actual_path, content_encoding) = self.find_precompressed(path, headers);
        let actual_metadata = if actual_path == path {
            metadata
        } else {
            std::fs::metadata(&actual_path)?
        };

        // Generate ETag
        let etag = if self.etag_enabled {
            self.generate_etag(&actual_metadata, &actual_path)
        } else {
            None
        };

        // Check If-None-Match for 304 Not Modified
        if let Some(ref etag) = etag {
            if let Some(if_none_match) = headers.get(header::IF_NONE_MATCH) {
                if let Ok(value) = if_none_match.to_str() {
                    if value == etag || value == "*" {
                        return Ok(self.not_modified_response(etag));
                    }
                }
            }
        }

        // Check If-Modified-Since for 304 Not Modified
        if self.last_modified_enabled {
            if let Some(ref last_mod) = modified {
                if let Some(if_modified_since) = headers.get(header::IF_MODIFIED_SINCE) {
                    if let Ok(value) = if_modified_since.to_str() {
                        if let Ok(since) = httpdate::parse_http_date(value) {
                            // Compare timestamps (truncating to seconds)
                            if let Ok(duration) = last_mod.duration_since(SystemTime::UNIX_EPOCH) {
                                if let Ok(since_duration) = since.duration_since(SystemTime::UNIX_EPOCH) {
                                    if duration.as_secs() <= since_duration.as_secs() {
                                        return Ok(self.not_modified_response(&etag.unwrap_or_default()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Parse Range header
        let range = self.parse_range_header(headers, file_size)?;

        // Determine MIME type
        let mime_type = self.detect_mime_type(path);

        // Build response
        if method == Method::HEAD {
            // HEAD request - return headers without body
            self.build_response(
                StatusCode::OK,
                Bytes::new(),
                file_size,
                &mime_type,
                etag.as_ref(),
                modified.as_ref(),
                content_encoding.as_ref(),
                None,
            )
        } else if let Some((start, end)) = range {
            // Range request
            let content = self.read_file_range(&actual_path, start, end)?;
            self.build_response(
                StatusCode::PARTIAL_CONTENT,
                content,
                file_size,
                &mime_type,
                etag.as_ref(),
                modified.as_ref(),
                content_encoding.as_ref(),
                Some((start, end, file_size)),
            )
        } else {
            // Full file
            let content = std::fs::read(&actual_path)?;
            self.build_response(
                StatusCode::OK,
                Bytes::from(content),
                file_size,
                &mime_type,
                etag.as_ref(),
                modified.as_ref(),
                content_encoding.as_ref(),
                None,
            )
        }
    }

    /// Finds a precompressed version of the file if available.
    fn find_precompressed(
        &self,
        path: &Path,
        headers: &HeaderMap,
    ) -> (PathBuf, Option<String>) {
        let accept_encoding = headers
            .get(header::ACCEPT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Try brotli first (better compression)
        if self.precompressed_brotli && accept_encoding.contains("br") {
            let br_path = path.with_extension(format!(
                "{}.br",
                path.extension().map(|e| e.to_str().unwrap_or("")).unwrap_or("")
            ));
            if br_path.is_file() {
                return (br_path, Some("br".to_string()));
            }
        }

        // Try gzip
        if self.precompressed_gzip && accept_encoding.contains("gzip") {
            let gz_path = path.with_extension(format!(
                "{}.gz",
                path.extension().map(|e| e.to_str().unwrap_or("")).unwrap_or("")
            ));
            if gz_path.is_file() {
                return (gz_path, Some("gzip".to_string()));
            }
        }

        (path.to_path_buf(), None)
    }

    /// Generates an `ETag` for a file.
    fn generate_etag(&self, metadata: &std::fs::Metadata, path: &Path) -> Option<String> {
        let modified = metadata.modified().ok()?;
        let duration = modified.duration_since(SystemTime::UNIX_EPOCH).ok()?;
        let size = metadata.len();

        // Include path in etag to handle different files with same size/mtime
        let path_hash = {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            path.hash(&mut hasher);
            hasher.finish()
        };

        Some(format!(
            "\"{}{}{}\"",
            duration.as_secs(),
            size,
            path_hash % 10000
        ))
    }

    /// Parses the Range header.
    fn parse_range_header(
        &self,
        headers: &HeaderMap,
        file_size: u64,
    ) -> Result<Option<(u64, u64)>, StaticFileError> {
        let range_header = match headers.get(header::RANGE) {
            Some(h) => h,
            None => return Ok(None),
        };

        let range_str = range_header
            .to_str()
            .map_err(|_| StaticFileError::InvalidRange("Invalid range header encoding".to_string()))?;

        // Parse "bytes=start-end" format
        if !range_str.starts_with("bytes=") {
            return Err(StaticFileError::InvalidRange(
                "Only byte ranges supported".to_string(),
            ));
        }

        let range_spec = &range_str[6..];

        // Handle single range only for now
        let parts: Vec<&str> = range_spec.split('-').collect();
        if parts.len() != 2 {
            return Err(StaticFileError::InvalidRange(
                "Invalid range format".to_string(),
            ));
        }

        let (start, end) = if parts[0].is_empty() {
            // Suffix range: "-500" means last 500 bytes
            let suffix_len: u64 = parts[1]
                .parse()
                .map_err(|_| StaticFileError::InvalidRange("Invalid suffix length".to_string()))?;
            let start = file_size.saturating_sub(suffix_len);
            let end = file_size - 1;
            (start, end)
        } else {
            let start: u64 = parts[0]
                .parse()
                .map_err(|_| StaticFileError::InvalidRange("Invalid start".to_string()))?;
            
            let end = if parts[1].is_empty() {
                // Open-ended range: "500-" means from 500 to end
                file_size - 1
            } else {
                parts[1]
                    .parse()
                    .map_err(|_| StaticFileError::InvalidRange("Invalid end".to_string()))?
            };
            (start, end)
        };

        // Validate range
        if start > end || start >= file_size {
            return Err(StaticFileError::InvalidRange(format!(
                "Range {}-{} not satisfiable for file size {}",
                start, end, file_size
            )));
        }

        // Clamp end to file size
        let end = end.min(file_size - 1);

        Ok(Some((start, end)))
    }

    /// Reads a range of bytes from a file.
    fn read_file_range(&self, path: &Path, start: u64, end: u64) -> Result<Bytes, StaticFileError> {
        use std::io::{Read, Seek, SeekFrom};

        let mut file = std::fs::File::open(path)?;
        file.seek(SeekFrom::Start(start))?;

        // Safe cast: range requests for files larger than usize::MAX are unrealistic
        #[allow(clippy::cast_possible_truncation)]
        let length = (end - start + 1) as usize;
        let mut buffer = vec![0u8; length];
        file.read_exact(&mut buffer)?;

        Ok(Bytes::from(buffer))
    }

    /// Detects the MIME type for a file.
    fn detect_mime_type(&self, path: &Path) -> String {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Check custom mappings first
        if let Some(mime) = self.mime_types.get(&extension) {
            return mime.clone();
        }

        // Default mappings
        match extension.as_str() {
            // Text
            "html" | "htm" => "text/html; charset=utf-8",
            "css" => "text/css; charset=utf-8",
            "js" | "mjs" => "text/javascript; charset=utf-8",
            "json" | "map" => "application/json",
            "xml" => "application/xml",
            "txt" => "text/plain; charset=utf-8",
            "csv" => "text/csv; charset=utf-8",
            "md" => "text/markdown; charset=utf-8",

            // Images
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "webp" => "image/webp",
            "ico" => "image/x-icon",
            "avif" => "image/avif",

            // Fonts
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "otf" => "font/otf",
            "eot" => "application/vnd.ms-fontobject",

            // Documents
            "pdf" => "application/pdf",
            "zip" => "application/zip",
            "gz" | "gzip" => "application/gzip",
            "br" => "application/brotli",
            "tar" => "application/x-tar",

            // Media
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "mp4" => "video/mp4",
            "webm" => "video/webm",
            "avi" => "video/x-msvideo",

            // Web
            "wasm" => "application/wasm",
            "manifest" | "webmanifest" => "application/manifest+json",

            // Default
            _ => "application/octet-stream",
        }
        .to_string()
    }

    /// Builds the HTTP response.
    #[allow(clippy::too_many_arguments)]
    fn build_response(
        &self,
        status: StatusCode,
        body: Bytes,
        _total_size: u64,
        mime_type: &str,
        etag: Option<&String>,
        modified: Option<&SystemTime>,
        content_encoding: Option<&String>,
        range: Option<(u64, u64, u64)>,
    ) -> Result<HttpResponse, StaticFileError> {
        let mut builder = Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, mime_type)
            .header(header::ACCEPT_RANGES, "bytes");

        // Add content length (actual body length, not total file size)
        builder = builder.header(header::CONTENT_LENGTH, body.len().to_string());

        // Add Cache-Control
        if let Some(ref cache_control) = self.cache_control {
            builder = builder.header(header::CACHE_CONTROL, cache_control.as_str());
        }

        // Add ETag
        if let Some(etag) = etag {
            builder = builder.header(header::ETAG, etag.as_str());
        }

        // Add Last-Modified
        if self.last_modified_enabled {
            if let Some(modified) = modified {
                let formatted = httpdate::fmt_http_date(*modified);
                builder = builder.header(header::LAST_MODIFIED, formatted);
            }
        }

        // Add Content-Encoding for precompressed files
        if let Some(encoding) = content_encoding {
            builder = builder.header(header::CONTENT_ENCODING, encoding.as_str());
        }

        // Add Content-Range for partial content
        if let Some((start, end, total)) = range {
            builder = builder.header(
                header::CONTENT_RANGE,
                format!("bytes {}-{}/{}", start, end, total),
            );
        }

        builder
            .body(Full::new(body))
            .map_err(|e| StaticFileError::IoError(std::io::Error::other(e.to_string())))
    }

    /// Builds a 304 Not Modified response.
    fn not_modified_response(&self, etag: &str) -> HttpResponse {
        let mut builder = Response::builder()
            .status(StatusCode::NOT_MODIFIED);

        if !etag.is_empty() {
            builder = builder.header(header::ETAG, etag);
        }

        if let Some(ref cache_control) = self.cache_control {
            builder = builder.header(header::CACHE_CONTROL, cache_control.as_str());
        }

        builder
            .body(Full::new(Bytes::new()))
            .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
    }
}

/// Builder for creating a [`StaticFiles`] instance.
///
/// # Example
///
/// ```rust
/// use archimedes_server::static_files::StaticFilesBuilder;
///
/// let files = StaticFilesBuilder::new()
///     .root("./public")
///     .index("index.html")
///     .cache_control("max-age=3600")
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct StaticFilesBuilder {
    root: Option<PathBuf>,
    index_file: Option<String>,
    cache_control: Option<String>,
    etag_enabled: bool,
    last_modified_enabled: bool,
    precompressed_gzip: bool,
    precompressed_brotli: bool,
    serve_hidden: bool,
    follow_symlinks: bool,
    mime_types: HashMap<String, String>,
}

impl StaticFilesBuilder {
    /// Creates a new builder with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: None,
            index_file: None,
            cache_control: None,
            etag_enabled: true,
            last_modified_enabled: true,
            precompressed_gzip: false,
            precompressed_brotli: false,
            serve_hidden: false,
            follow_symlinks: true,
            mime_types: HashMap::new(),
        }
    }

    /// Sets the root directory for static files.
    #[must_use]
    pub fn root<P: AsRef<Path>>(mut self, root: P) -> Self {
        self.root = Some(root.as_ref().to_path_buf());
        self
    }

    /// Sets the index file name.
    #[must_use]
    pub fn index<S: Into<String>>(mut self, index: S) -> Self {
        self.index_file = Some(index.into());
        self
    }

    /// Sets the Cache-Control header value.
    #[must_use]
    pub fn cache_control<S: Into<String>>(mut self, value: S) -> Self {
        self.cache_control = Some(value.into());
        self
    }

    /// Enables or disables `ETag` headers.
    #[must_use]
    pub fn etag(mut self, enabled: bool) -> Self {
        self.etag_enabled = enabled;
        self
    }

    /// Enables or disables Last-Modified headers.
    #[must_use]
    pub fn last_modified(mut self, enabled: bool) -> Self {
        self.last_modified_enabled = enabled;
        self
    }

    /// Enables or disables serving precompressed `.gz` files.
    #[must_use]
    pub fn precompressed_gzip(mut self, enabled: bool) -> Self {
        self.precompressed_gzip = enabled;
        self
    }

    /// Enables or disables serving precompressed `.br` files.
    #[must_use]
    pub fn precompressed_brotli(mut self, enabled: bool) -> Self {
        self.precompressed_brotli = enabled;
        self
    }

    /// Enables or disables serving hidden files.
    #[must_use]
    pub fn serve_hidden(mut self, enabled: bool) -> Self {
        self.serve_hidden = enabled;
        self
    }

    /// Enables or disables following symlinks.
    #[must_use]
    pub fn follow_symlinks(mut self, enabled: bool) -> Self {
        self.follow_symlinks = enabled;
        self
    }

    /// Adds a custom MIME type mapping.
    #[must_use]
    pub fn mime_type<S1: Into<String>, S2: Into<String>>(
        mut self,
        extension: S1,
        mime_type: S2,
    ) -> Self {
        self.mime_types.insert(extension.into(), mime_type.into());
        self
    }

    /// Builds the [`StaticFiles`] instance.
    ///
    /// # Panics
    ///
    /// Panics if `root` has not been set.
    #[must_use]
    pub fn build(self) -> StaticFiles {
        let root = self.root.expect("root directory must be set");
        let mut files = StaticFiles::new(root);

        if let Some(index) = self.index_file {
            files.index_file = Some(index);
        }
        if let Some(cache_control) = self.cache_control {
            files.cache_control = Some(cache_control);
        }
        files.etag_enabled = self.etag_enabled;
        files.last_modified_enabled = self.last_modified_enabled;
        files.precompressed_gzip = self.precompressed_gzip;
        files.precompressed_brotli = self.precompressed_brotli;
        files.serve_hidden = self.serve_hidden;
        files.follow_symlinks = self.follow_symlinks;
        files.mime_types = self.mime_types;

        files
    }

    /// Tries to build the [`StaticFiles`] instance.
    ///
    /// # Errors
    ///
    /// Returns `None` if `root` has not been set.
    #[must_use]
    pub fn try_build(self) -> Option<StaticFiles> {
        let root = self.root?;
        let mut files = StaticFiles::new(root);

        if let Some(index) = self.index_file {
            files.index_file = Some(index);
        }
        if let Some(cache_control) = self.cache_control {
            files.cache_control = Some(cache_control);
        }
        files.etag_enabled = self.etag_enabled;
        files.last_modified_enabled = self.last_modified_enabled;
        files.precompressed_gzip = self.precompressed_gzip;
        files.precompressed_brotli = self.precompressed_brotli;
        files.serve_hidden = self.serve_hidden;
        files.follow_symlinks = self.follow_symlinks;
        files.mime_types = self.mime_types;

        Some(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();

        // Create test files
        fs::write(dir.path().join("index.html"), "<html>Hello</html>").unwrap();
        fs::write(dir.path().join("style.css"), "body { color: red }").unwrap();
        fs::write(dir.path().join("script.js"), "console.log('hi')").unwrap();
        fs::write(dir.path().join("data.json"), r#"{"key": "value"}"#).unwrap();
        fs::write(dir.path().join("image.png"), &[0x89, 0x50, 0x4e, 0x47]).unwrap();
        fs::write(dir.path().join(".hidden"), "secret").unwrap();

        // Create subdirectory
        let subdir = dir.path().join("sub");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("page.html"), "<html>Sub</html>").unwrap();
        fs::write(subdir.join("index.html"), "<html>Index</html>").unwrap();

        dir
    }

    #[test]
    fn test_new_static_files() {
        let files = StaticFiles::new("./public");
        assert_eq!(files.root(), Path::new("./public"));
        assert!(files.index_file().is_none());
        assert!(files.etag_enabled);
        assert!(files.last_modified_enabled);
    }

    #[test]
    fn test_builder_pattern() {
        let files = StaticFiles::new("./public")
            .index("index.html")
            .cache_control("max-age=3600")
            .etag(false)
            .last_modified(false)
            .precompressed_gzip(true)
            .precompressed_brotli(true)
            .serve_hidden(true)
            .follow_symlinks(false)
            .mime_type("wasm", "application/wasm");

        assert_eq!(files.index_file(), Some("index.html"));
        assert_eq!(files.cache_control.as_deref(), Some("max-age=3600"));
        assert!(!files.etag_enabled);
        assert!(!files.last_modified_enabled);
        assert!(files.precompressed_gzip);
        assert!(files.precompressed_brotli);
        assert!(files.serve_hidden);
        assert!(!files.follow_symlinks);
        assert_eq!(files.mime_types.get("wasm"), Some(&"application/wasm".to_string()));
    }

    #[test]
    fn test_static_files_builder() {
        let files = StaticFilesBuilder::new()
            .root("./public")
            .index("index.html")
            .cache_control("max-age=3600")
            .build();

        assert_eq!(files.root(), Path::new("./public"));
        assert_eq!(files.index_file(), Some("index.html"));
    }

    #[test]
    fn test_serve_html_file() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let response = files.handle("/index.html", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[test]
    fn test_serve_css_file() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let response = files.handle("/style.css", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/css; charset=utf-8"
        );
    }

    #[test]
    fn test_serve_javascript_file() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let response = files.handle("/script.js", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/javascript; charset=utf-8"
        );
    }

    #[test]
    fn test_serve_json_file() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let response = files.handle("/data.json", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_serve_image_file() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let response = files.handle("/image.png", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/png"
        );
    }

    #[test]
    fn test_serve_subdirectory_file() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let response = files.handle("/sub/page.html", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_serve_directory_with_index() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path()).index("index.html");

        let response = files.handle("/sub/", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[test]
    fn test_directory_traversal_blocked() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let result = files.handle("/../etc/passwd", &HeaderMap::new(), &Method::GET);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StaticFileError::Forbidden(_)));
    }

    #[test]
    fn test_hidden_files_blocked_by_default() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let result = files.handle("/.hidden", &HeaderMap::new(), &Method::GET);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StaticFileError::Forbidden(_)));
    }

    #[test]
    fn test_hidden_files_allowed_when_enabled() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path()).serve_hidden(true);

        let response = files.handle("/.hidden", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_file_not_found() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let result = files.handle("/nonexistent.html", &HeaderMap::new(), &Method::GET);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StaticFileError::NotFound(_)));
    }

    #[test]
    fn test_method_not_allowed() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let result = files.handle("/index.html", &HeaderMap::new(), &Method::POST);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StaticFileError::MethodNotAllowed));
    }

    #[test]
    fn test_head_request() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let response = files.handle("/index.html", &HeaderMap::new(), &Method::HEAD).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        // HEAD should have Content-Length but empty body
        assert!(response.headers().contains_key(header::CONTENT_LENGTH));
    }

    #[test]
    fn test_etag_header() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path()).etag(true);

        let response = files.handle("/index.html", &HeaderMap::new(), &Method::GET).unwrap();

        assert!(response.headers().contains_key(header::ETAG));
    }

    #[test]
    fn test_etag_disabled() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path()).etag(false);

        let response = files.handle("/index.html", &HeaderMap::new(), &Method::GET).unwrap();

        assert!(!response.headers().contains_key(header::ETAG));
    }

    #[test]
    fn test_last_modified_header() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path()).last_modified(true);

        let response = files.handle("/index.html", &HeaderMap::new(), &Method::GET).unwrap();

        assert!(response.headers().contains_key(header::LAST_MODIFIED));
    }

    #[test]
    fn test_cache_control_header() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path()).cache_control("max-age=86400, public");

        let response = files.handle("/index.html", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(
            response.headers().get(header::CACHE_CONTROL).unwrap(),
            "max-age=86400, public"
        );
    }

    #[test]
    fn test_accept_ranges_header() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let response = files.handle("/index.html", &HeaderMap::new(), &Method::GET).unwrap();

        assert_eq!(
            response.headers().get(header::ACCEPT_RANGES).unwrap(),
            "bytes"
        );
    }

    #[test]
    fn test_if_none_match_returns_304() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path()).etag(true);

        // First request to get the ETag
        let response1 = files.handle("/index.html", &HeaderMap::new(), &Method::GET).unwrap();
        let etag = response1.headers().get(header::ETAG).unwrap().to_str().unwrap().to_string();

        // Second request with If-None-Match
        let mut headers = HeaderMap::new();
        headers.insert(header::IF_NONE_MATCH, HeaderValue::from_str(&etag).unwrap());

        let response2 = files.handle("/index.html", &headers, &Method::GET).unwrap();

        assert_eq!(response2.status(), StatusCode::NOT_MODIFIED);
    }

    #[test]
    fn test_range_request_partial_content() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, HeaderValue::from_static("bytes=0-4"));

        let response = files.handle("/index.html", &headers, &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert!(response.headers().contains_key(header::CONTENT_RANGE));
    }

    #[test]
    fn test_range_request_suffix() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, HeaderValue::from_static("bytes=-5"));

        let response = files.handle("/index.html", &headers, &Method::GET).unwrap();

        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    }

    #[test]
    fn test_invalid_range() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, HeaderValue::from_static("bytes=100-200"));

        let result = files.handle("/index.html", &headers, &Method::GET);

        // File is smaller than the range
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StaticFileError::InvalidRange(_)));
    }

    #[test]
    fn test_mime_type_detection() {
        let dir = create_test_dir();
        let files = StaticFiles::new(dir.path());

        // Test various extensions
        assert_eq!(files.detect_mime_type(Path::new("file.html")), "text/html; charset=utf-8");
        assert_eq!(files.detect_mime_type(Path::new("file.css")), "text/css; charset=utf-8");
        assert_eq!(files.detect_mime_type(Path::new("file.js")), "text/javascript; charset=utf-8");
        assert_eq!(files.detect_mime_type(Path::new("file.json")), "application/json");
        assert_eq!(files.detect_mime_type(Path::new("file.png")), "image/png");
        assert_eq!(files.detect_mime_type(Path::new("file.woff2")), "font/woff2");
        assert_eq!(files.detect_mime_type(Path::new("file.wasm")), "application/wasm");
        assert_eq!(files.detect_mime_type(Path::new("file.unknown")), "application/octet-stream");
    }

    #[test]
    fn test_custom_mime_type() {
        let files = StaticFiles::new("./public")
            .mime_type("custom", "application/custom");

        assert_eq!(files.detect_mime_type(Path::new("file.custom")), "application/custom");
    }

    #[test]
    fn test_error_status_codes() {
        assert_eq!(StaticFileError::NotFound("".to_string()).status_code(), StatusCode::NOT_FOUND);
        assert_eq!(StaticFileError::Forbidden("".to_string()).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(StaticFileError::MethodNotAllowed.status_code(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(StaticFileError::InvalidRange("".to_string()).status_code(), StatusCode::RANGE_NOT_SATISFIABLE);
    }

    #[test]
    fn test_builder_try_build_without_root() {
        let result = StaticFilesBuilder::new()
            .index("index.html")
            .try_build();

        assert!(result.is_none());
    }

    #[test]
    fn test_builder_try_build_with_root() {
        let result = StaticFilesBuilder::new()
            .root("./public")
            .try_build();

        assert!(result.is_some());
    }
}
