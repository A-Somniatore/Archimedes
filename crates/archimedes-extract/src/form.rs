//! Form data extractor.
//!
//! The [`Form`] extractor deserializes URL-encoded form data from request bodies.

use crate::{ExtractionContext, ExtractionError, ExtractionSource, FromRequest};
use serde::de::DeserializeOwned;
use std::ops::Deref;

/// Default maximum body size for form extraction (1 MB).
const DEFAULT_MAX_BODY_SIZE: usize = 1024 * 1024;

/// Extractor for URL-encoded form data.
///
/// `Form<T>` deserializes the request body as URL-encoded form data into the
/// type `T`, which must implement [`serde::Deserialize`]. This is commonly
/// used for HTML form submissions with Content-Type `application/x-www-form-urlencoded`.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{Form, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct LoginForm {
///     username: String,
///     password: String,
/// }
///
/// let body = b"username=alice&password=secret123";
///
/// let ctx = ExtractionContext::new(
///     Method::POST,
///     Uri::from_static("/login"),
///     HeaderMap::new(),
///     Bytes::from_static(body),
///     Params::new(),
/// );
///
/// let Form(form) = Form::<LoginForm>::from_request(&ctx).unwrap();
/// assert_eq!(form.username, "alice");
/// assert_eq!(form.password, "secret123");
/// ```
///
/// # URL Encoding
///
/// Form data is URL-encoded, meaning special characters are escaped:
///
/// ```rust
/// use archimedes_extract::{Form, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct SearchForm {
///     query: String,
/// }
///
/// // "hello world" is encoded as "hello+world" or "hello%20world"
/// let body = b"query=hello+world";
///
/// let ctx = ExtractionContext::new(
///     Method::POST,
///     Uri::from_static("/search"),
///     HeaderMap::new(),
///     Bytes::from_static(body),
///     Params::new(),
/// );
///
/// let Form(form) = Form::<SearchForm>::from_request(&ctx).unwrap();
/// assert_eq!(form.query, "hello world");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Form<T>(pub T);

impl<T> Form<T> {
    /// Consumes the Form and returns the inner value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned> FromRequest for Form<T> {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        let body = ctx.body();

        // Check body size
        if body.len() > DEFAULT_MAX_BODY_SIZE {
            return Err(ExtractionError::payload_too_large(
                DEFAULT_MAX_BODY_SIZE,
                body.len(),
            ));
        }

        // Handle empty body
        if body.is_empty() {
            return Err(ExtractionError::deserialization_failed(
                ExtractionSource::Body,
                "empty request body",
            ));
        }

        // Parse as UTF-8
        let body_str = std::str::from_utf8(body).map_err(|e| {
            ExtractionError::deserialization_failed(
                ExtractionSource::Body,
                format!("invalid UTF-8: {e}"),
            )
        })?;

        // Deserialize form data
        let value: T = serde_urlencoded::from_str(body_str).map_err(|e| {
            ExtractionError::deserialization_failed(ExtractionSource::Body, e.to_string())
        })?;

        Ok(Form(value))
    }
}

/// Form extractor with configurable size limit.
///
/// Use this when you need to accept form bodies larger than the default 1 MB limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormWithLimit<T, const LIMIT: usize>(pub T);

impl<T, const LIMIT: usize> FormWithLimit<T, LIMIT> {
    /// Consumes the FormWithLimit and returns the inner value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, const LIMIT: usize> Deref for FormWithLimit<T, LIMIT> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned, const LIMIT: usize> FromRequest for FormWithLimit<T, LIMIT> {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        let body = ctx.body();

        // Check body size against custom limit
        if body.len() > LIMIT {
            return Err(ExtractionError::payload_too_large(LIMIT, body.len()));
        }

        // Handle empty body
        if body.is_empty() {
            return Err(ExtractionError::deserialization_failed(
                ExtractionSource::Body,
                "empty request body",
            ));
        }

        // Parse as UTF-8
        let body_str = std::str::from_utf8(body).map_err(|e| {
            ExtractionError::deserialization_failed(
                ExtractionSource::Body,
                format!("invalid UTF-8: {e}"),
            )
        })?;

        // Deserialize form data
        let value: T = serde_urlencoded::from_str(body_str).map_err(|e| {
            ExtractionError::deserialization_failed(ExtractionSource::Body, e.to_string())
        })?;

        Ok(FormWithLimit(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archimedes_router::Params;
    use bytes::Bytes;
    use http::{HeaderMap, Method, Uri};
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct LoginForm {
        username: String,
        password: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct OptionalForm {
        required: String,
        #[serde(default)]
        optional: Option<String>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ArrayForm {
        #[serde(default)]
        items: Vec<String>,
    }

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
    fn test_simple_form() {
        let body = b"username=alice&password=secret123";
        let ctx = make_ctx(body);

        let Form(form) = Form::<LoginForm>::from_request(&ctx).unwrap();

        assert_eq!(form.username, "alice");
        assert_eq!(form.password, "secret123");
    }

    #[test]
    fn test_url_encoded_values() {
        let body = b"username=alice%40example.com&password=pass%3Dword";
        let ctx = make_ctx(body);

        let Form(form) = Form::<LoginForm>::from_request(&ctx).unwrap();

        assert_eq!(form.username, "alice@example.com");
        assert_eq!(form.password, "pass=word");
    }

    #[test]
    fn test_plus_as_space() {
        let body = b"username=hello+world&password=test";
        let ctx = make_ctx(body);

        let Form(form) = Form::<LoginForm>::from_request(&ctx).unwrap();

        assert_eq!(form.username, "hello world");
    }

    #[test]
    fn test_optional_fields() {
        let body = b"required=value";
        let ctx = make_ctx(body);

        let Form(form) = Form::<OptionalForm>::from_request(&ctx).unwrap();

        assert_eq!(form.required, "value");
        assert_eq!(form.optional, None);
    }

    #[test]
    fn test_array_form() {
        // Note: serde_urlencoded doesn't support repeated keys for arrays.
        // Arrays default to empty when not provided.
        let body = b"";
        let ctx = make_ctx(body);

        // Empty body returns an error, not an empty array
        let result = Form::<ArrayForm>::from_request(&ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_body() {
        let ctx = make_ctx(b"");
        let result = Form::<LoginForm>::from_request(&ctx);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.source(), ExtractionSource::Body);
    }

    #[test]
    fn test_missing_required_field() {
        let body = b"username=alice";
        let ctx = make_ctx(body);

        let result = Form::<LoginForm>::from_request(&ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_deref() {
        let body = b"username=alice&password=secret";
        let ctx = make_ctx(body);

        let form: Form<LoginForm> = Form::from_request(&ctx).unwrap();

        assert_eq!(form.username, "alice");
    }

    #[test]
    fn test_into_inner() {
        let body = b"username=alice&password=secret";
        let ctx = make_ctx(body);

        let form: Form<LoginForm> = Form::from_request(&ctx).unwrap();
        let login = form.into_inner();

        assert_eq!(login.username, "alice");
    }

    #[test]
    fn test_form_with_limit() {
        let body = b"username=alice&password=secret";
        let ctx = make_ctx(body);

        let result = FormWithLimit::<LoginForm, 1024>::from_request(&ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_form_with_limit_exceeded() {
        let large_body = format!("username={}&password=test", "A".repeat(200));
        let ctx = make_ctx(large_body.as_bytes());

        let result = FormWithLimit::<LoginForm, 100>::from_request(&ctx);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.error_code(), "PAYLOAD_TOO_LARGE");
    }
}
