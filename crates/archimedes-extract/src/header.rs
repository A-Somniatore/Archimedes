//! Header extractors.
//!
//! This module provides extractors for HTTP headers.

use crate::{ExtractionContext, ExtractionError, ExtractionSource, FromRequest};
use http::HeaderMap;
use std::ops::Deref;

/// Extractor for a single header value by name.
///
/// `Header` extracts a specific header from the request. The header name
/// is specified as a const generic parameter or type parameter.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{Header, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert("x-request-id", "abc-123".parse().unwrap());
///
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/"),
///     headers,
///     Bytes::new(),
///     Params::new(),
/// );
///
/// // Extract using header name
/// let request_id = ctx.header("x-request-id");
/// assert_eq!(request_id, Some("abc-123"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header(pub String);

impl Header {
    /// Returns the header value as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consumes the Header and returns the inner String.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Deref for Header {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Typed header extractor.
///
/// Use this to extract a specific header with a known name and parse it
/// into a specific type.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{TypedHeader, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// // Define a typed header
/// struct ContentLength(pub u64);
///
/// impl TypedHeader for ContentLength {
///     const NAME: &'static str = "content-length";
///
///     fn parse(value: &str) -> Option<Self> {
///         value.parse().ok().map(ContentLength)
///     }
/// }
/// ```
pub trait TypedHeader: Sized {
    /// The header name (lowercase).
    const NAME: &'static str;

    /// Parses the header value into this type.
    fn parse(value: &str) -> Option<Self>;
}

/// Extract a typed header from the request.
#[derive(Debug, Clone)]
pub struct ExtractTypedHeader<T: TypedHeader>(pub T);

impl<T: TypedHeader> ExtractTypedHeader<T> {
    /// Returns a reference to the inner value.
    #[must_use]
    pub fn inner(&self) -> &T {
        &self.0
    }

    /// Consumes the extractor and returns the inner value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: TypedHeader> Deref for ExtractTypedHeader<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TypedHeader> FromRequest for ExtractTypedHeader<T> {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        let value = ctx
            .header(T::NAME)
            .ok_or_else(|| ExtractionError::missing(ExtractionSource::Header, T::NAME))?;

        let parsed = T::parse(value).ok_or_else(|| {
            ExtractionError::invalid_type(
                ExtractionSource::Header,
                T::NAME,
                "failed to parse header value",
            )
        })?;

        Ok(ExtractTypedHeader(parsed))
    }
}

/// Extractor for all request headers.
///
/// `Headers` provides access to all HTTP headers in the request.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{Headers, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert("content-type", "application/json".parse().unwrap());
/// headers.insert("accept", "application/json".parse().unwrap());
///
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/"),
///     headers,
///     Bytes::new(),
///     Params::new(),
/// );
///
/// let Headers(h) = Headers::from_request(&ctx).unwrap();
/// assert!(h.contains_key("content-type"));
/// assert!(h.contains_key("accept"));
/// ```
#[derive(Debug, Clone)]
pub struct Headers(pub HeaderMap);

impl Headers {
    /// Returns a reference to the header map.
    #[must_use]
    pub fn inner(&self) -> &HeaderMap {
        &self.0
    }

    /// Consumes the Headers and returns the inner HeaderMap.
    #[must_use]
    pub fn into_inner(self) -> HeaderMap {
        self.0
    }

    /// Gets a header value by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).and_then(|v| v.to_str().ok())
    }

    /// Checks if a header exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }
}

impl Deref for Headers {
    type Target = HeaderMap;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for Headers {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        Ok(Headers(ctx.headers().clone()))
    }
}

// Common typed headers

/// Content-Type header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentType(pub String);

impl TypedHeader for ContentType {
    const NAME: &'static str = "content-type";

    fn parse(value: &str) -> Option<Self> {
        Some(ContentType(value.to_string()))
    }
}

impl ContentType {
    /// Checks if the content type is JSON.
    #[must_use]
    pub fn is_json(&self) -> bool {
        self.0.starts_with("application/json")
    }

    /// Checks if the content type is form data.
    #[must_use]
    pub fn is_form(&self) -> bool {
        self.0.starts_with("application/x-www-form-urlencoded")
    }

    /// Checks if the content type is multipart.
    #[must_use]
    pub fn is_multipart(&self) -> bool {
        self.0.starts_with("multipart/")
    }

    /// Returns the MIME type without parameters.
    #[must_use]
    pub fn mime_type(&self) -> &str {
        self.0.split(';').next().unwrap_or(&self.0).trim()
    }
}

/// Accept header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accept(pub String);

impl TypedHeader for Accept {
    const NAME: &'static str = "accept";

    fn parse(value: &str) -> Option<Self> {
        Some(Accept(value.to_string()))
    }
}

impl Accept {
    /// Checks if JSON is acceptable.
    #[must_use]
    pub fn accepts_json(&self) -> bool {
        self.0.contains("application/json") || self.0.contains("*/*")
    }

    /// Checks if HTML is acceptable.
    #[must_use]
    pub fn accepts_html(&self) -> bool {
        self.0.contains("text/html") || self.0.contains("*/*")
    }
}

/// Authorization header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorization(pub String);

impl TypedHeader for Authorization {
    const NAME: &'static str = "authorization";

    fn parse(value: &str) -> Option<Self> {
        Some(Authorization(value.to_string()))
    }
}

impl Authorization {
    /// Returns the bearer token if present.
    #[must_use]
    pub fn bearer_token(&self) -> Option<&str> {
        self.0
            .strip_prefix("Bearer ")
            .or_else(|| self.0.strip_prefix("bearer "))
    }

    /// Returns the basic auth credentials if present.
    #[must_use]
    pub fn basic_credentials(&self) -> Option<&str> {
        self.0
            .strip_prefix("Basic ")
            .or_else(|| self.0.strip_prefix("basic "))
    }
}

/// User-Agent header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAgent(pub String);

impl TypedHeader for UserAgent {
    const NAME: &'static str = "user-agent";

    fn parse(value: &str) -> Option<Self> {
        Some(UserAgent(value.to_string()))
    }
}

/// Helper function to extract a header by name.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{header, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert("x-api-key", "secret".parse().unwrap());
///
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/"),
///     headers,
///     Bytes::new(),
///     Params::new(),
/// );
///
/// let api_key = header(&ctx, "x-api-key").unwrap();
/// assert_eq!(api_key, "secret");
/// ```
pub fn header<'a>(ctx: &'a ExtractionContext, name: &str) -> Result<&'a str, ExtractionError> {
    ctx.header(name)
        .ok_or_else(|| ExtractionError::missing(ExtractionSource::Header, name))
}

/// Helper function to extract an optional header.
#[must_use]
pub fn header_opt<'a>(ctx: &'a ExtractionContext, name: &str) -> Option<&'a str> {
    ctx.header(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use archimedes_router::Params;
    use bytes::Bytes;
    use http::{HeaderMap, Method, Uri};

    fn make_ctx(headers: HeaderMap) -> ExtractionContext {
        ExtractionContext::new(
            Method::GET,
            Uri::from_static("/"),
            headers,
            Bytes::new(),
            Params::new(),
        )
    }

    #[test]
    fn test_headers_extractor() {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", "application/json".parse().unwrap());
        headers.insert("accept", "*/*".parse().unwrap());

        let ctx = make_ctx(headers);
        let Headers(h) = Headers::from_request(&ctx).unwrap();

        assert!(h.contains_key("content-type"));
        assert!(h.contains_key("accept"));
    }

    #[test]
    fn test_headers_get() {
        let mut headers = HeaderMap::new();
        headers.insert("x-request-id", "abc-123".parse().unwrap());

        let ctx = make_ctx(headers);
        let h = Headers::from_request(&ctx).unwrap();

        assert_eq!(h.get("x-request-id"), Some("abc-123"));
        assert_eq!(h.get("missing"), None);
    }

    #[test]
    fn test_typed_header_content_type() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "content-type",
            "application/json; charset=utf-8".parse().unwrap(),
        );

        let ctx = make_ctx(headers);
        let ExtractTypedHeader(ct) = ExtractTypedHeader::<ContentType>::from_request(&ctx).unwrap();

        assert!(ct.is_json());
        assert!(!ct.is_form());
        assert_eq!(ct.mime_type(), "application/json");
    }

    #[test]
    fn test_typed_header_authorization() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer abc123".parse().unwrap());

        let ctx = make_ctx(headers);
        let ExtractTypedHeader(auth) =
            ExtractTypedHeader::<Authorization>::from_request(&ctx).unwrap();

        assert_eq!(auth.bearer_token(), Some("abc123"));
        assert_eq!(auth.basic_credentials(), None);
    }

    #[test]
    fn test_typed_header_missing() {
        let ctx = make_ctx(HeaderMap::new());
        let result = ExtractTypedHeader::<ContentType>::from_request(&ctx);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.source(), ExtractionSource::Header);
    }

    #[test]
    fn test_header_function() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "secret".parse().unwrap());

        let ctx = make_ctx(headers);
        let key = header(&ctx, "x-api-key").unwrap();

        assert_eq!(key, "secret");
    }

    #[test]
    fn test_header_function_missing() {
        let ctx = make_ctx(HeaderMap::new());
        let result = header(&ctx, "missing");

        assert!(result.is_err());
    }

    #[test]
    fn test_header_opt_function() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "secret".parse().unwrap());

        let ctx = make_ctx(headers);

        assert_eq!(header_opt(&ctx, "x-api-key"), Some("secret"));
        assert_eq!(header_opt(&ctx, "missing"), None);
    }

    #[test]
    fn test_content_type_methods() {
        let json = ContentType("application/json".to_string());
        assert!(json.is_json());
        assert!(!json.is_form());

        let form = ContentType("application/x-www-form-urlencoded".to_string());
        assert!(!form.is_json());
        assert!(form.is_form());

        let multipart = ContentType("multipart/form-data; boundary=abc".to_string());
        assert!(multipart.is_multipart());
    }

    #[test]
    fn test_accept_methods() {
        let accept = Accept("application/json, text/html".to_string());
        assert!(accept.accepts_json());
        assert!(accept.accepts_html());

        let wildcard = Accept("*/*".to_string());
        assert!(wildcard.accepts_json());
        assert!(wildcard.accepts_html());
    }

    #[test]
    fn test_authorization_methods() {
        let bearer = Authorization("Bearer token123".to_string());
        assert_eq!(bearer.bearer_token(), Some("token123"));
        assert_eq!(bearer.basic_credentials(), None);

        let basic = Authorization("Basic dXNlcjpwYXNz".to_string());
        assert_eq!(basic.bearer_token(), None);
        assert_eq!(basic.basic_credentials(), Some("dXNlcjpwYXNz"));
    }
}
