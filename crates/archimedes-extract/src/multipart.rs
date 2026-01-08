//! Multipart form data extractor for file uploads.
//!
//! The [`Multipart`] extractor handles `multipart/form-data` requests,
//! commonly used for file uploads.
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_extract::{Multipart, UploadedFile};
//!
//! async fn upload_handler(mut multipart: Multipart) -> Result<Response, ThemisError> {
//!     while let Some(field) = multipart.next_field().await? {
//!         let filename = field.file_name().unwrap_or("unnamed");
//!         let content_type = field.content_type();
//!         let data = field.bytes().await?;
//!         
//!         // Process the file...
//!         println!("Received file: {} ({} bytes)", filename, data.len());
//!     }
//!     Ok(Response::no_content())
//! }
//! ```

use bytes::Bytes;
use http::{header, HeaderMap};
use std::io;

use crate::{ExtractionError, ExtractionSource};

/// Default maximum total body size for multipart (50 MB).
pub const DEFAULT_MAX_BODY_SIZE: usize = 50 * 1024 * 1024;

/// Default maximum size per field (10 MB).
pub const DEFAULT_MAX_FIELD_SIZE: usize = 10 * 1024 * 1024;

/// Configuration for multipart parsing.
#[derive(Debug, Clone)]
pub struct MultipartConfig {
    /// Maximum total body size in bytes.
    pub max_body_size: usize,
    /// Maximum size per field in bytes.
    pub max_field_size: usize,
    /// Maximum number of fields allowed.
    pub max_fields: usize,
}

impl Default for MultipartConfig {
    fn default() -> Self {
        Self {
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            max_field_size: DEFAULT_MAX_FIELD_SIZE,
            max_fields: 100,
        }
    }
}

impl MultipartConfig {
    /// Create a new configuration with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum body size.
    #[must_use]
    pub fn max_body_size(mut self, size: usize) -> Self {
        self.max_body_size = size;
        self
    }

    /// Set the maximum field size.
    #[must_use]
    pub fn max_field_size(mut self, size: usize) -> Self {
        self.max_field_size = size;
        self
    }

    /// Set the maximum number of fields.
    #[must_use]
    pub fn max_fields(mut self, count: usize) -> Self {
        self.max_fields = count;
        self
    }
}

/// Extractor for multipart form data.
///
/// Handles `multipart/form-data` content type, commonly used for file uploads.
/// Fields are extracted one at a time using async iteration.
///
/// # Example
///
/// ```rust,ignore
/// async fn upload(mut multipart: Multipart) -> Result<Response, ThemisError> {
///     while let Some(field) = multipart.next_field().await? {
///         if let Some(filename) = field.file_name() {
///             let data = field.bytes().await?;
///             storage.save(filename, &data).await?;
///         }
///     }
///     Ok(Response::no_content())
/// }
/// ```
pub struct Multipart {
    inner: multer::Multipart<'static>,
    config: MultipartConfig,
    field_count: usize,
}

impl Multipart {
    /// Create a new Multipart extractor from request components.
    ///
    /// # Errors
    ///
    /// Returns an error if the Content-Type header is missing or invalid.
    pub fn from_request(
        headers: &HeaderMap,
        body: Bytes,
        config: MultipartConfig,
    ) -> Result<Self, ExtractionError> {
        // Extract boundary from Content-Type header
        let content_type = headers
            .get(header::CONTENT_TYPE)
            .ok_or_else(|| {
                ExtractionError::missing_content_type("multipart/form-data")
            })?
            .to_str()
            .map_err(|_| {
                ExtractionError::invalid_content_type(
                    "invalid UTF-8 in Content-Type header",
                )
            })?;

        let boundary = multer::parse_boundary(content_type).map_err(|_| {
            ExtractionError::invalid_content_type(
                "missing or invalid boundary in multipart Content-Type",
            )
        })?;

        // Check body size
        if body.len() > config.max_body_size {
            return Err(ExtractionError::payload_too_large(
                config.max_body_size,
                body.len(),
            ));
        }

        // Create a stream from the body
        let stream = futures_util::stream::once(async move {
            Ok::<_, io::Error>(body)
        });

        let inner = multer::Multipart::new(stream, boundary);

        Ok(Self {
            inner,
            config,
            field_count: 0,
        })
    }

    /// Create with default configuration.
    pub fn from_request_default(
        headers: &HeaderMap,
        body: Bytes,
    ) -> Result<Self, ExtractionError> {
        Self::from_request(headers, body, MultipartConfig::default())
    }

    /// Get the next field from the multipart stream.
    ///
    /// Returns `None` when all fields have been processed.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The maximum number of fields is exceeded
    /// - The multipart data is malformed
    pub async fn next_field(&mut self) -> Result<Option<Field>, ExtractionError> {
        // Check field count limit
        if self.field_count >= self.config.max_fields {
            return Err(ExtractionError::validation_failed(
                ExtractionSource::Body,
                "multipart",
                format!("too many fields (max {})", self.config.max_fields),
            ));
        }

        match self.inner.next_field().await {
            Ok(Some(field)) => {
                self.field_count += 1;
                Ok(Some(Field::new(field, self.config.max_field_size)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ExtractionError::deserialization_failed(
                ExtractionSource::Body,
                format!("multipart parse error: {e}"),
            )),
        }
    }

    /// Collect all files from the multipart stream.
    ///
    /// This is a convenience method that reads all file fields into memory.
    /// For large files or many files, prefer using `next_field()` directly
    /// with streaming.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails or a field exceeds size limits.
    pub async fn collect_files(&mut self) -> Result<Vec<UploadedFile>, ExtractionError> {
        let mut files = Vec::new();

        while let Some(field) = self.next_field().await? {
            if field.file_name().is_some() {
                let file = field.into_file().await?;
                files.push(file);
            }
        }

        Ok(files)
    }
}

impl std::fmt::Debug for Multipart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Multipart")
            .field("config", &self.config)
            .field("field_count", &self.field_count)
            .finish_non_exhaustive()
    }
}

/// A single field from a multipart form.
///
/// Fields can be either regular form values or files.
pub struct Field {
    inner: multer::Field<'static>,
    max_size: usize,
}

impl Field {
    fn new(inner: multer::Field<'static>, max_size: usize) -> Self {
        Self { inner, max_size }
    }

    /// Get the field name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.inner.name()
    }

    /// Get the file name (for file uploads).
    ///
    /// Returns `Some` if this field is a file upload, `None` otherwise.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.inner.file_name()
    }

    /// Get the Content-Type of this field.
    ///
    /// For text fields, this is typically `text/plain`.
    /// For file uploads, this matches the file's MIME type.
    #[must_use]
    pub fn content_type(&self) -> Option<&mime::Mime> {
        self.inner.content_type()
    }

    /// Read the entire field as bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The field size exceeds the configured limit
    /// - Reading the field fails
    pub async fn bytes(self) -> Result<Bytes, ExtractionError> {
        let bytes = self.inner.bytes().await.map_err(|e| {
            ExtractionError::deserialization_failed(
                ExtractionSource::Body,
                format!("failed to read field: {e}"),
            )
        })?;

        if bytes.len() > self.max_size {
            return Err(ExtractionError::payload_too_large(self.max_size, bytes.len()));
        }

        Ok(bytes)
    }

    /// Read the field as a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The field size exceeds the configured limit
    /// - The content is not valid UTF-8
    pub async fn text(self) -> Result<String, ExtractionError> {
        let bytes = self.bytes().await?;
        String::from_utf8(bytes.to_vec()).map_err(|e| {
            ExtractionError::deserialization_failed(
                ExtractionSource::Body,
                format!("field is not valid UTF-8: {e}"),
            )
        })
    }

    /// Convert this field into an [`UploadedFile`].
    ///
    /// # Errors
    ///
    /// Returns an error if reading the field fails.
    pub async fn into_file(self) -> Result<UploadedFile, ExtractionError> {
        let name = self.name().map(String::from);
        let file_name = self.file_name().map(String::from);
        let content_type = self.content_type().map(std::string::ToString::to_string);
        let data = self.bytes().await?;

        Ok(UploadedFile {
            name,
            file_name,
            content_type,
            data,
        })
    }
}

impl std::fmt::Debug for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Field")
            .field("name", &self.inner.name())
            .field("file_name", &self.inner.file_name())
            .field("content_type", &self.inner.content_type())
            .field("max_size", &self.max_size)
            .finish()
    }
}

/// A file that has been uploaded via multipart form.
///
/// Contains the file metadata and content.
#[derive(Debug, Clone)]
pub struct UploadedFile {
    /// The form field name.
    pub name: Option<String>,
    /// The original file name from the client.
    pub file_name: Option<String>,
    /// The MIME type of the file.
    pub content_type: Option<String>,
    /// The file content as bytes.
    pub data: Bytes,
}

impl UploadedFile {
    /// Create a new uploaded file.
    #[must_use]
    pub fn new(
        name: Option<String>,
        file_name: Option<String>,
        content_type: Option<String>,
        data: Bytes,
    ) -> Self {
        Self {
            name,
            file_name,
            content_type,
            data,
        }
    }

    /// Get the form field name.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the original file name.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    /// Get the MIME type.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Get the file data as bytes.
    #[must_use]
    pub fn data(&self) -> &Bytes {
        &self.data
    }

    /// Get the file size in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the file is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the file extension from the filename.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.file_name.as_ref().and_then(|name| {
            name.rsplit_once('.').map(|(_, ext)| ext)
        })
    }

    /// Validate the file against allowed MIME types.
    ///
    /// # Errors
    ///
    /// Returns an error if the file's content type is not in the allowed list.
    pub fn validate_content_type(&self, allowed: &[&str]) -> Result<(), ExtractionError> {
        match &self.content_type {
            Some(ct) if allowed.iter().any(|a| ct.starts_with(a)) => Ok(()),
            Some(ct) => Err(ExtractionError::validation_failed(
                ExtractionSource::Body,
                "content_type",
                format!(
                    "invalid content type '{}', expected one of: {:?}",
                    ct, allowed
                ),
            )),
            None => Err(ExtractionError::validation_failed(
                ExtractionSource::Body,
                "content_type",
                "missing content type",
            )),
        }
    }

    /// Validate the file size.
    ///
    /// # Errors
    ///
    /// Returns an error if the file size exceeds the maximum.
    pub fn validate_size(&self, max_bytes: usize) -> Result<(), ExtractionError> {
        if self.data.len() > max_bytes {
            Err(ExtractionError::payload_too_large(max_bytes, self.data.len()))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::header;

    fn create_multipart_body(boundary: &str, parts: &[(&str, &str, Option<&str>, &[u8])]) -> Vec<u8> {
        let mut body = Vec::new();
        
        for (name, content_type, filename, data) in parts {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            
            if let Some(fname) = filename {
                body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{fname}\"\r\n"
                    )
                    .as_bytes(),
                );
            } else {
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n").as_bytes(),
                );
            }
            
            body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
            body.extend_from_slice(data);
            body.extend_from_slice(b"\r\n");
        }
        
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        body
    }

    #[test]
    fn test_multipart_config_default() {
        let config = MultipartConfig::default();
        assert_eq!(config.max_body_size, DEFAULT_MAX_BODY_SIZE);
        assert_eq!(config.max_field_size, DEFAULT_MAX_FIELD_SIZE);
        assert_eq!(config.max_fields, 100);
    }

    #[test]
    fn test_multipart_config_builder() {
        let config = MultipartConfig::new()
            .max_body_size(100)
            .max_field_size(50)
            .max_fields(10);

        assert_eq!(config.max_body_size, 100);
        assert_eq!(config.max_field_size, 50);
        assert_eq!(config.max_fields, 10);
    }

    #[tokio::test]
    async fn test_multipart_single_file() {
        let boundary = "----WebKitFormBoundary";
        let body = create_multipart_body(
            boundary,
            &[("file", "text/plain", Some("test.txt"), b"Hello, World!")],
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}")
                .parse()
                .unwrap(),
        );

        let mut multipart = Multipart::from_request_default(&headers, Bytes::from(body)).unwrap();
        let field = multipart.next_field().await.unwrap().unwrap();

        assert_eq!(field.name(), Some("file"));
        assert_eq!(field.file_name(), Some("test.txt"));
        
        let data = field.bytes().await.unwrap();
        assert_eq!(&data[..], b"Hello, World!");
    }

    #[tokio::test]
    async fn test_multipart_multiple_fields() {
        let boundary = "----boundary";
        let body = create_multipart_body(
            boundary,
            &[
                ("name", "text/plain", None, b"Alice"),
                ("file", "image/png", Some("photo.png"), b"PNG_DATA"),
            ],
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}")
                .parse()
                .unwrap(),
        );

        let mut multipart = Multipart::from_request_default(&headers, Bytes::from(body)).unwrap();

        // First field: text
        let field1 = multipart.next_field().await.unwrap().unwrap();
        assert_eq!(field1.name(), Some("name"));
        assert!(field1.file_name().is_none());
        // Consume the field by reading bytes
        let text_data = field1.bytes().await.unwrap();
        assert_eq!(&text_data[..], b"Alice");

        // Second field: file
        let field2 = multipart.next_field().await.unwrap().unwrap();
        assert_eq!(field2.name(), Some("file"));
        assert_eq!(field2.file_name(), Some("photo.png"));
        let file_data = field2.bytes().await.unwrap();
        assert_eq!(&file_data[..], b"PNG_DATA");

        // No more fields
        assert!(multipart.next_field().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_multipart_collect_files() {
        let boundary = "----boundary";
        let body = create_multipart_body(
            boundary,
            &[
                ("text_field", "text/plain", None, b"not a file"),
                ("file1", "text/plain", Some("a.txt"), b"file a"),
                ("file2", "text/plain", Some("b.txt"), b"file b"),
            ],
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}")
                .parse()
                .unwrap(),
        );

        let mut multipart = Multipart::from_request_default(&headers, Bytes::from(body)).unwrap();
        let files = multipart.collect_files().await.unwrap();

        // Only file fields should be collected
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].file_name(), Some("a.txt"));
        assert_eq!(files[1].file_name(), Some("b.txt"));
    }

    #[tokio::test]
    async fn test_multipart_missing_content_type() {
        let headers = HeaderMap::new();
        let result = Multipart::from_request_default(&headers, Bytes::new());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multipart_invalid_boundary() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            "multipart/form-data".parse().unwrap(),
        );

        let result = Multipart::from_request_default(&headers, Bytes::new());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multipart_body_too_large() {
        let boundary = "----boundary";
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}")
                .parse()
                .unwrap(),
        );

        let config = MultipartConfig::new().max_body_size(10);
        let body = Bytes::from(vec![0u8; 100]);

        let result = Multipart::from_request(&headers, body, config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multipart_too_many_fields() {
        let boundary = "----boundary";
        let body = create_multipart_body(
            boundary,
            &[
                ("f1", "text/plain", None, b"1"),
                ("f2", "text/plain", None, b"2"),
                ("f3", "text/plain", None, b"3"),
            ],
        );

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}")
                .parse()
                .unwrap(),
        );

        let config = MultipartConfig::new().max_fields(2);
        let mut multipart = Multipart::from_request(&headers, Bytes::from(body), config).unwrap();

        // First two fields should succeed
        assert!(multipart.next_field().await.unwrap().is_some());
        assert!(multipart.next_field().await.unwrap().is_some());

        // Third field should fail
        assert!(multipart.next_field().await.is_err());
    }

    #[test]
    fn test_uploaded_file_extension() {
        let file = UploadedFile::new(
            Some("file".to_string()),
            Some("document.pdf".to_string()),
            Some("application/pdf".to_string()),
            Bytes::from_static(b"data"),
        );

        assert_eq!(file.extension(), Some("pdf"));
    }

    #[test]
    fn test_uploaded_file_no_extension() {
        let file = UploadedFile::new(
            Some("file".to_string()),
            Some("README".to_string()),
            None,
            Bytes::from_static(b"data"),
        );

        assert_eq!(file.extension(), None);
    }

    #[test]
    fn test_uploaded_file_validate_content_type() {
        let file = UploadedFile::new(
            None,
            None,
            Some("image/png".to_string()),
            Bytes::new(),
        );

        assert!(file.validate_content_type(&["image/"]).is_ok());
        assert!(file.validate_content_type(&["image/png"]).is_ok());
        assert!(file.validate_content_type(&["text/"]).is_err());
    }

    #[test]
    fn test_uploaded_file_validate_size() {
        let file = UploadedFile::new(
            None,
            None,
            None,
            Bytes::from_static(b"12345"),
        );

        assert!(file.validate_size(10).is_ok());
        assert!(file.validate_size(5).is_ok());
        assert!(file.validate_size(4).is_err());
    }

    #[test]
    fn test_uploaded_file_is_empty() {
        let empty = UploadedFile::new(None, None, None, Bytes::new());
        let non_empty = UploadedFile::new(None, None, None, Bytes::from_static(b"data"));

        assert!(empty.is_empty());
        assert!(!non_empty.is_empty());
    }
}
