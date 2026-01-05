//! Raw body extractor.
//!
//! The [`RawBody`] extractor provides access to the raw request body bytes.

use crate::{ExtractionContext, ExtractionError, FromRequest};
use bytes::Bytes;
use std::ops::Deref;

/// Extractor for raw request body bytes.
///
/// `RawBody` provides access to the raw bytes of the request body without
/// any parsing or deserialization. Useful for streaming, binary data, or
/// custom parsing.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{RawBody, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// let body_data = b"raw binary data";
///
/// let ctx = ExtractionContext::new(
///     Method::POST,
///     Uri::from_static("/upload"),
///     HeaderMap::new(),
///     Bytes::from_static(body_data),
///     Params::new(),
/// );
///
/// let RawBody(body) = RawBody::from_request(&ctx).unwrap();
/// assert_eq!(&*body, b"raw binary data");
/// ```
///
/// # Use Cases
///
/// - File uploads (when not using multipart)
/// - Binary protocols
/// - Custom content types
/// - Signature verification (need exact bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawBody(pub Bytes);

impl RawBody {
    /// Returns the body as a byte slice.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consumes the RawBody and returns the inner Bytes.
    #[must_use]
    pub fn into_inner(self) -> Bytes {
        self.0
    }

    /// Returns the length of the body in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the body is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Attempts to convert the body to a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns an error if the body is not valid UTF-8.
    pub fn to_string(&self) -> Result<String, std::str::Utf8Error> {
        std::str::from_utf8(&self.0).map(String::from)
    }
}

impl Deref for RawBody {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for RawBody {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        Ok(RawBody(ctx.body().clone()))
    }
}

impl From<RawBody> for Bytes {
    fn from(body: RawBody) -> Self {
        body.0
    }
}

impl From<RawBody> for Vec<u8> {
    fn from(body: RawBody) -> Self {
        body.0.to_vec()
    }
}

/// Extractor for raw body as a string.
///
/// `BodyString` extracts the body and converts it to a UTF-8 string.
/// Returns an error if the body is not valid UTF-8.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{BodyString, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// let body_data = "Hello, World!";
///
/// let ctx = ExtractionContext::new(
///     Method::POST,
///     Uri::from_static("/message"),
///     HeaderMap::new(),
///     Bytes::from_static(body_data.as_bytes()),
///     Params::new(),
/// );
///
/// let BodyString(text) = BodyString::from_request(&ctx).unwrap();
/// assert_eq!(text, "Hello, World!");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyString(pub String);

impl BodyString {
    /// Returns the body as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the BodyString and returns the inner String.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for BodyString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for BodyString {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        let body = ctx.body();
        let string = std::str::from_utf8(body)
            .map_err(|e| {
                ExtractionError::deserialization_failed(
                    crate::ExtractionSource::Body,
                    format!("invalid UTF-8: {e}"),
                )
            })?
            .to_string();

        Ok(BodyString(string))
    }
}

impl From<BodyString> for String {
    fn from(body: BodyString) -> Self {
        body.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archimedes_router::Params;
    use http::{HeaderMap, Method, Uri};

    fn make_ctx(body: &[u8]) -> ExtractionContext {
        ExtractionContext::new(
            Method::POST,
            Uri::from_static("/"),
            HeaderMap::new(),
            Bytes::from(body.to_vec()),
            Params::new(),
        )
    }

    #[test]
    fn test_raw_body() {
        let ctx = make_ctx(b"raw data");
        let RawBody(body) = RawBody::from_request(&ctx).unwrap();

        assert_eq!(&*body, b"raw data");
    }

    #[test]
    fn test_raw_body_empty() {
        let ctx = make_ctx(b"");
        let body = RawBody::from_request(&ctx).unwrap();

        assert!(body.is_empty());
        assert_eq!(body.len(), 0);
    }

    #[test]
    fn test_raw_body_binary() {
        let binary = vec![0x00, 0x01, 0x02, 0xFF, 0xFE];
        let ctx = make_ctx(&binary);

        let RawBody(body) = RawBody::from_request(&ctx).unwrap();

        assert_eq!(body.as_ref(), binary.as_slice());
    }

    #[test]
    fn test_raw_body_to_string() {
        let ctx = make_ctx(b"hello");
        let body = RawBody::from_request(&ctx).unwrap();

        assert_eq!(body.to_string().unwrap(), "hello");
    }

    #[test]
    fn test_raw_body_to_string_invalid_utf8() {
        let ctx = make_ctx(&[0xFF, 0xFE]);
        let body = RawBody::from_request(&ctx).unwrap();

        assert!(body.to_string().is_err());
    }

    #[test]
    fn test_raw_body_into_bytes() {
        let ctx = make_ctx(b"data");
        let body = RawBody::from_request(&ctx).unwrap();

        let bytes: Bytes = body.into();
        assert_eq!(&*bytes, b"data");
    }

    #[test]
    fn test_raw_body_into_vec() {
        let ctx = make_ctx(b"data");
        let body = RawBody::from_request(&ctx).unwrap();

        let vec: Vec<u8> = body.into();
        assert_eq!(vec, b"data");
    }

    #[test]
    fn test_body_string() {
        let ctx = make_ctx(b"Hello, World!");
        let BodyString(text) = BodyString::from_request(&ctx).unwrap();

        assert_eq!(text, "Hello, World!");
    }

    #[test]
    fn test_body_string_empty() {
        let ctx = make_ctx(b"");
        let body = BodyString::from_request(&ctx).unwrap();

        assert_eq!(body.as_str(), "");
    }

    #[test]
    fn test_body_string_invalid_utf8() {
        let ctx = make_ctx(&[0xFF, 0xFE]);
        let result = BodyString::from_request(&ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_body_string_unicode() {
        let ctx = make_ctx("Hello 世界 🌍".as_bytes());
        let BodyString(text) = BodyString::from_request(&ctx).unwrap();

        assert_eq!(text, "Hello 世界 🌍");
    }

    #[test]
    fn test_body_string_into_string() {
        let ctx = make_ctx(b"text");
        let body = BodyString::from_request(&ctx).unwrap();

        let s: String = body.into();
        assert_eq!(s, "text");
    }
}
