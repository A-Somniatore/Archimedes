//! Path parameter extractor.
//!
//! The [`Path`] extractor deserializes URL path parameters into a typed struct.

use crate::{ExtractionContext, ExtractionError, ExtractionSource, FromRequest};
use serde::de::DeserializeOwned;
use std::ops::Deref;

/// Extractor for URL path parameters.
///
/// `Path<T>` deserializes the path parameters into the type `T`, which must
/// implement [`serde::Deserialize`]. Path parameters are extracted from
/// segments like `/users/{user_id}/posts/{post_id}`.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{Path, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct UserPath {
///     user_id: u64,
/// }
///
/// // Simulate a request to /users/42
/// let mut params = Params::new();
/// params.push("user_id", "42");
///
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/users/42"),
///     HeaderMap::new(),
///     Bytes::new(),
///     params,
/// );
///
/// let Path(user_path) = Path::<UserPath>::from_request(&ctx).unwrap();
/// assert_eq!(user_path.user_id, 42);
/// ```
///
/// # Multiple Parameters
///
/// ```rust
/// use archimedes_extract::{Path, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct PostPath {
///     user_id: u64,
///     post_id: u64,
/// }
///
/// let mut params = Params::new();
/// params.push("user_id", "42");
/// params.push("post_id", "123");
///
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/users/42/posts/123"),
///     HeaderMap::new(),
///     Bytes::new(),
///     params,
/// );
///
/// let Path(path) = Path::<PostPath>::from_request(&ctx).unwrap();
/// assert_eq!(path.user_id, 42);
/// assert_eq!(path.post_id, 123);
/// ```
///
/// # Single Parameter Shorthand
///
/// For routes with a single parameter, you can use primitive types directly:
///
/// ```rust
/// use archimedes_extract::{Path, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// // For single parameter, use a tuple struct or primitive
/// let mut params = Params::new();
/// params.push("id", "42");
///
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/items/42"),
///     HeaderMap::new(),
///     Bytes::new(),
///     params,
/// );
///
/// // Can extract as a simple u64 if there's only one param
/// // (requires the struct to have a single field matching param name)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<T>(pub T);

impl<T> Path<T> {
    /// Consumes the Path and returns the inner value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned> FromRequest for Path<T> {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        // If there are no parameters, return an error
        if ctx.path_params().is_empty() {
            return Err(ExtractionError::missing(
                ExtractionSource::Path,
                "<path parameters>",
            ));
        }

        // Convert path params to a URL-encoded query string format
        // This allows serde_urlencoded to handle type coercion (string -> int, etc.)
        let query_string: String = ctx
            .path_params()
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Deserialize using serde_urlencoded which handles string-to-type conversion
        let value: T = serde_urlencoded::from_str(&query_string).map_err(|e| {
            ExtractionError::deserialization_failed(ExtractionSource::Path, e.to_string())
        })?;

        Ok(Path(value))
    }
}

/// Extract a single path parameter by name.
///
/// This is a convenience function for extracting a single parameter
/// without needing to define a struct.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{path_param, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// let mut params = Params::new();
/// params.push("user_id", "42");
///
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/users/42"),
///     HeaderMap::new(),
///     Bytes::new(),
///     params,
/// );
///
/// let user_id: u64 = path_param(&ctx, "user_id").unwrap();
/// assert_eq!(user_id, 42);
/// ```
///
/// # Errors
///
/// Returns an error if the parameter is missing or cannot be parsed.
pub fn path_param<T: std::str::FromStr>(
    ctx: &ExtractionContext,
    name: &str,
) -> Result<T, ExtractionError> {
    let value = ctx
        .path_params()
        .get(name)
        .ok_or_else(|| ExtractionError::missing(ExtractionSource::Path, name))?;

    value.parse().map_err(|_| {
        ExtractionError::invalid_type(
            ExtractionSource::Path,
            name,
            format!("failed to parse as {}", std::any::type_name::<T>()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use archimedes_router::Params;
    use bytes::Bytes;
    use http::{HeaderMap, Method, Uri};
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct UserPath {
        user_id: u64,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct PostPath {
        user_id: u64,
        post_id: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct OptionalPath {
        id: u64,
        #[serde(default)]
        version: Option<String>,
    }

    fn make_ctx(params: Params) -> ExtractionContext {
        ExtractionContext::new(
            Method::GET,
            Uri::from_static("/test"),
            HeaderMap::new(),
            Bytes::new(),
            params,
        )
    }

    #[test]
    fn test_single_path_param() {
        let mut params = Params::new();
        params.push("user_id", "42");

        let ctx = make_ctx(params);
        let Path(path) = Path::<UserPath>::from_request(&ctx).unwrap();

        assert_eq!(path.user_id, 42);
    }

    #[test]
    fn test_multiple_path_params() {
        let mut params = Params::new();
        params.push("user_id", "42");
        params.push("post_id", "abc-123");

        let ctx = make_ctx(params);
        let Path(path) = Path::<PostPath>::from_request(&ctx).unwrap();

        assert_eq!(path.user_id, 42);
        assert_eq!(path.post_id, "abc-123");
    }

    #[test]
    fn test_optional_path_param() {
        let mut params = Params::new();
        params.push("id", "42");

        let ctx = make_ctx(params);
        let Path(path) = Path::<OptionalPath>::from_request(&ctx).unwrap();

        assert_eq!(path.id, 42);
        assert_eq!(path.version, None);
    }

    #[test]
    fn test_optional_path_param_present() {
        let mut params = Params::new();
        params.push("id", "42");
        params.push("version", "v2");

        let ctx = make_ctx(params);
        let Path(path) = Path::<OptionalPath>::from_request(&ctx).unwrap();

        assert_eq!(path.id, 42);
        assert_eq!(path.version, Some("v2".to_string()));
    }

    #[test]
    fn test_missing_required_param() {
        let params = Params::new();
        let ctx = make_ctx(params);

        let result = Path::<UserPath>::from_request(&ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_type_conversion() {
        let mut params = Params::new();
        params.push("user_id", "not-a-number");

        let ctx = make_ctx(params);
        let result = Path::<UserPath>::from_request(&ctx);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.source(), ExtractionSource::Path);
    }

    #[test]
    fn test_deref() {
        let mut params = Params::new();
        params.push("user_id", "42");

        let ctx = make_ctx(params);
        let path: Path<UserPath> = Path::from_request(&ctx).unwrap();

        // Can access fields via Deref
        assert_eq!(path.user_id, 42);
    }

    #[test]
    fn test_into_inner() {
        let mut params = Params::new();
        params.push("user_id", "42");

        let ctx = make_ctx(params);
        let path: Path<UserPath> = Path::from_request(&ctx).unwrap();
        let inner = path.into_inner();

        assert_eq!(inner.user_id, 42);
    }

    #[test]
    fn test_path_param_function() {
        let mut params = Params::new();
        params.push("id", "42");
        params.push("name", "test");

        let ctx = make_ctx(params);

        let id: u64 = path_param(&ctx, "id").unwrap();
        assert_eq!(id, 42);

        let name: String = path_param(&ctx, "name").unwrap();
        assert_eq!(name, "test");
    }

    #[test]
    fn test_path_param_function_missing() {
        let params = Params::new();
        let ctx = make_ctx(params);

        let result: Result<u64, _> = path_param(&ctx, "id");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_param_function_invalid_type() {
        let mut params = Params::new();
        params.push("id", "not-a-number");

        let ctx = make_ctx(params);
        let result: Result<u64, _> = path_param(&ctx, "id");

        assert!(result.is_err());
    }
}
