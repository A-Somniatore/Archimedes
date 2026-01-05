//! Query string extractor.
//!
//! The [`Query`] extractor deserializes URL query parameters into a typed struct.

use crate::{ExtractionContext, ExtractionError, ExtractionSource, FromRequest};
use serde::de::DeserializeOwned;
use std::ops::Deref;

/// Extractor for URL query string parameters.
///
/// `Query<T>` deserializes the query string into the type `T`, which must
/// implement [`serde::Deserialize`]. Query parameters are extracted from
/// the URL after the `?` character.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{Query, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct ListParams {
///     #[serde(default)]
///     limit: Option<u32>,
///     #[serde(default)]
///     offset: Option<u32>,
///     #[serde(default)]
///     search: Option<String>,
/// }
///
/// // Simulate a request to /users?limit=10&offset=20
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/users?limit=10&offset=20"),
///     HeaderMap::new(),
///     Bytes::new(),
///     Params::new(),
/// );
///
/// let Query(params) = Query::<ListParams>::from_request(&ctx).unwrap();
/// assert_eq!(params.limit, Some(10));
/// assert_eq!(params.offset, Some(20));
/// assert_eq!(params.search, None);
/// ```
///
/// # Required vs Optional Parameters
///
/// Use `Option<T>` for optional parameters and bare types for required ones:
///
/// ```rust
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct SearchParams {
///     query: String,          // Required - error if missing
///     #[serde(default)]
///     page: Option<u32>,      // Optional - None if missing
///     #[serde(default = "default_limit")]
///     limit: u32,             // Optional with default value
/// }
///
/// fn default_limit() -> u32 { 20 }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query<T>(pub T);

impl<T> Query<T> {
    /// Consumes the Query and returns the inner value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned> FromRequest for Query<T> {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        let query_string = ctx.query_string().unwrap_or("");

        let value: T = serde_urlencoded::from_str(query_string).map_err(|e| {
            ExtractionError::deserialization_failed(ExtractionSource::Query, e.to_string())
        })?;

        Ok(Query(value))
    }
}

/// Raw query string access.
///
/// Use this when you need access to the raw query string without deserialization.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{RawQuery, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
///
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/search?q=rust+lang&limit=10"),
///     HeaderMap::new(),
///     Bytes::new(),
///     Params::new(),
/// );
///
/// let RawQuery(query) = RawQuery::from_request(&ctx).unwrap();
/// assert_eq!(query, Some("q=rust+lang&limit=10".to_string()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawQuery(pub Option<String>);

impl FromRequest for RawQuery {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        Ok(RawQuery(ctx.query_string().map(String::from)))
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
    struct ListParams {
        #[serde(default)]
        limit: Option<u32>,
        #[serde(default)]
        offset: Option<u32>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct RequiredParams {
        name: String,
        age: u32,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct ArrayParams {
        #[serde(default)]
        ids: Vec<u64>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct DefaultParams {
        #[serde(default = "default_page")]
        page: u32,
    }

    fn default_page() -> u32 {
        1
    }

    fn make_ctx(uri: &'static str) -> ExtractionContext {
        ExtractionContext::new(
            Method::GET,
            Uri::from_static(uri),
            HeaderMap::new(),
            Bytes::new(),
            Params::new(),
        )
    }

    #[test]
    fn test_optional_params() {
        let ctx = make_ctx("/users?limit=10&offset=20");
        let Query(params) = Query::<ListParams>::from_request(&ctx).unwrap();

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, Some(20));
    }

    #[test]
    fn test_partial_params() {
        let ctx = make_ctx("/users?limit=10");
        let Query(params) = Query::<ListParams>::from_request(&ctx).unwrap();

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.offset, None);
    }

    #[test]
    fn test_no_params() {
        let ctx = make_ctx("/users");
        let Query(params) = Query::<ListParams>::from_request(&ctx).unwrap();

        assert_eq!(params.limit, None);
        assert_eq!(params.offset, None);
    }

    #[test]
    fn test_required_params() {
        let ctx = make_ctx("/users?name=Alice&age=30");
        let Query(params) = Query::<RequiredParams>::from_request(&ctx).unwrap();

        assert_eq!(params.name, "Alice");
        assert_eq!(params.age, 30);
    }

    #[test]
    fn test_missing_required_param() {
        let ctx = make_ctx("/users?name=Alice");
        let result = Query::<RequiredParams>::from_request(&ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_array_params() {
        // Note: serde_urlencoded doesn't support repeated keys for arrays.
        // Arrays default to empty when not provided.
        let ctx = make_ctx("/items");
        let Query(params) = Query::<ArrayParams>::from_request(&ctx).unwrap();

        assert_eq!(params.ids, Vec::<u64>::new());
    }

    #[test]
    fn test_default_params() {
        let ctx = make_ctx("/items");
        let Query(params) = Query::<DefaultParams>::from_request(&ctx).unwrap();

        assert_eq!(params.page, 1);
    }

    #[test]
    fn test_default_params_override() {
        let ctx = make_ctx("/items?page=5");
        let Query(params) = Query::<DefaultParams>::from_request(&ctx).unwrap();

        assert_eq!(params.page, 5);
    }

    #[test]
    fn test_url_encoded_values() {
        let ctx = make_ctx("/search?name=Hello%20World");
        let result = Query::<RequiredParams>::from_request(&ctx);

        // This should fail because age is missing, but name should decode
        assert!(result.is_err());
    }

    #[test]
    fn test_special_characters() {
        #[derive(Debug, Deserialize)]
        struct SearchParams {
            q: String,
        }

        let ctx = make_ctx("/search?q=rust%2Blang");
        let Query(params) = Query::<SearchParams>::from_request(&ctx).unwrap();

        assert_eq!(params.q, "rust+lang");
    }

    #[test]
    fn test_deref() {
        let ctx = make_ctx("/users?limit=10");
        let query: Query<ListParams> = Query::from_request(&ctx).unwrap();

        // Can access fields via Deref
        assert_eq!(query.limit, Some(10));
    }

    #[test]
    fn test_into_inner() {
        let ctx = make_ctx("/users?limit=10");
        let query: Query<ListParams> = Query::from_request(&ctx).unwrap();
        let params = query.into_inner();

        assert_eq!(params.limit, Some(10));
    }

    #[test]
    fn test_raw_query_with_params() {
        let ctx = make_ctx("/search?q=test&limit=10");
        let RawQuery(query) = RawQuery::from_request(&ctx).unwrap();

        assert_eq!(query, Some("q=test&limit=10".to_string()));
    }

    #[test]
    fn test_raw_query_without_params() {
        let ctx = make_ctx("/search");
        let RawQuery(query) = RawQuery::from_request(&ctx).unwrap();

        assert_eq!(query, None);
    }

    #[test]
    fn test_invalid_type_in_query() {
        let ctx = make_ctx("/users?limit=not-a-number");
        let result = Query::<ListParams>::from_request(&ctx);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.source(), ExtractionSource::Query);
    }
}
