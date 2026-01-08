//! JSON body extractor.
//!
//! The [`Json`] extractor deserializes JSON request bodies into typed structs.

use crate::{ExtractionContext, ExtractionError, ExtractionSource, FromRequest};
use serde::de::DeserializeOwned;
use std::ops::Deref;

/// Default maximum body size for JSON extraction (1 MB).
const DEFAULT_MAX_BODY_SIZE: usize = 1024 * 1024;

/// Extractor for JSON request bodies.
///
/// `Json<T>` deserializes the request body as JSON into the type `T`, which
/// must implement [`serde::Deserialize`]. The Content-Type header should be
/// `application/json` (though this is validated by middleware, not the extractor).
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{Json, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct CreateUser {
///     name: String,
///     email: String,
/// }
///
/// let body = br#"{"name": "Alice", "email": "alice@example.com"}"#;
///
/// let ctx = ExtractionContext::new(
///     Method::POST,
///     Uri::from_static("/users"),
///     HeaderMap::new(),
///     Bytes::from_static(body),
///     Params::new(),
/// );
///
/// let Json(user) = Json::<CreateUser>::from_request(&ctx).unwrap();
/// assert_eq!(user.name, "Alice");
/// assert_eq!(user.email, "alice@example.com");
/// ```
///
/// # Validation
///
/// The `Json` extractor performs basic JSON deserialization. For contract-based
/// validation against schemas, use the validation middleware which runs before
/// the handler and validates the body against the operation's request schema.
///
/// # Empty Bodies
///
/// For endpoints that may receive an empty body, use `Option<Json<T>>`:
///
/// ```rust
/// use archimedes_extract::{Json, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct UpdateUser {
///     name: Option<String>,
/// }
///
/// // Empty body
/// let ctx = ExtractionContext::new(
///     Method::PATCH,
///     Uri::from_static("/users/1"),
///     HeaderMap::new(),
///     Bytes::new(),
///     Params::new(),
/// );
///
/// // Option<Json<T>> returns None for empty bodies
/// let result = Option::<Json<UpdateUser>>::from_request(&ctx).unwrap();
/// assert!(result.is_none());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    /// Consumes the Json and returns the inner value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned> FromRequest for Json<T> {
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

        // Deserialize JSON
        let value: T = serde_json::from_slice(body).map_err(|e| {
            ExtractionError::deserialization_failed(ExtractionSource::Body, e.to_string())
        })?;

        Ok(Json(value))
    }
}

/// JSON extractor with configurable size limit.
///
/// Use this when you need to accept bodies larger than the default 1 MB limit.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{JsonWithLimit, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap};
/// use bytes::Bytes;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct LargeData {
///     items: Vec<String>,
/// }
///
/// // Create a 10 MB limit extractor
/// type LargeJson<T> = JsonWithLimit<T, 10485760>;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonWithLimit<T, const LIMIT: usize>(pub T);

impl<T, const LIMIT: usize> JsonWithLimit<T, LIMIT> {
    /// Consumes the `JsonWithLimit` and returns the inner value.
    #[must_use]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T, const LIMIT: usize> Deref for JsonWithLimit<T, LIMIT> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned, const LIMIT: usize> FromRequest for JsonWithLimit<T, LIMIT> {
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

        // Deserialize JSON
        let value: T = serde_json::from_slice(body).map_err(|e| {
            ExtractionError::deserialization_failed(ExtractionSource::Body, e.to_string())
        })?;

        Ok(JsonWithLimit(value))
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
    struct CreateUser {
        name: String,
        email: String,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct NestedData {
        user: CreateUser,
        tags: Vec<String>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct OptionalFields {
        required: String,
        #[serde(default)]
        optional: Option<String>,
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
    fn test_simple_json() {
        let body = br#"{"name": "Alice", "email": "alice@example.com"}"#;
        let ctx = make_ctx(body);

        let Json(user) = Json::<CreateUser>::from_request(&ctx).unwrap();

        assert_eq!(user.name, "Alice");
        assert_eq!(user.email, "alice@example.com");
    }

    #[test]
    fn test_nested_json() {
        let body = br#"{"user": {"name": "Bob", "email": "bob@example.com"}, "tags": ["admin", "active"]}"#;
        let ctx = make_ctx(body);

        let Json(data) = Json::<NestedData>::from_request(&ctx).unwrap();

        assert_eq!(data.user.name, "Bob");
        assert_eq!(data.tags, vec!["admin", "active"]);
    }

    #[test]
    fn test_optional_fields() {
        let body = br#"{"required": "value"}"#;
        let ctx = make_ctx(body);

        let Json(data) = Json::<OptionalFields>::from_request(&ctx).unwrap();

        assert_eq!(data.required, "value");
        assert_eq!(data.optional, None);
    }

    #[test]
    fn test_empty_body() {
        let ctx = make_ctx(b"");
        let result = Json::<CreateUser>::from_request(&ctx);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.source(), ExtractionSource::Body);
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_invalid_json() {
        let body = br#"{"name": "Alice", invalid json"#;
        let ctx = make_ctx(body);

        let result = Json::<CreateUser>::from_request(&ctx);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.source(), ExtractionSource::Body);
    }

    #[test]
    fn test_missing_required_field() {
        let body = br#"{"name": "Alice"}"#;
        let ctx = make_ctx(body);

        let result = Json::<CreateUser>::from_request(&ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_type() {
        let body = br#"{"name": 123, "email": "alice@example.com"}"#;
        let ctx = make_ctx(body);

        let result = Json::<CreateUser>::from_request(&ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_deref() {
        let body = br#"{"name": "Alice", "email": "alice@example.com"}"#;
        let ctx = make_ctx(body);

        let json: Json<CreateUser> = Json::from_request(&ctx).unwrap();

        // Can access fields via Deref
        assert_eq!(json.name, "Alice");
    }

    #[test]
    fn test_into_inner() {
        let body = br#"{"name": "Alice", "email": "alice@example.com"}"#;
        let ctx = make_ctx(body);

        let json: Json<CreateUser> = Json::from_request(&ctx).unwrap();
        let user = json.into_inner();

        assert_eq!(user.name, "Alice");
    }

    #[test]
    fn test_json_with_limit() {
        let body = br#"{"name": "Alice", "email": "alice@example.com"}"#;
        let ctx = make_ctx(body);

        // 1KB limit should work for small bodies
        let result = JsonWithLimit::<CreateUser, 1024>::from_request(&ctx);
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_with_limit_exceeded() {
        // Create body larger than 100 bytes
        let large_body = format!(
            r#"{{"name": "{}", "email": "alice@example.com"}}"#,
            "A".repeat(200)
        );
        let ctx = make_ctx(large_body.as_bytes());

        // 100 byte limit should fail
        let result = JsonWithLimit::<CreateUser, 100>::from_request(&ctx);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.error_code(), "PAYLOAD_TOO_LARGE");
    }

    #[test]
    fn test_option_json_with_empty_body() {
        let ctx = make_ctx(b"");

        // Option<Json<T>> returns None for empty bodies (via FromRequest impl for Option)
        let result = Option::<Json<CreateUser>>::from_request(&ctx).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_option_json_with_body() {
        let body = br#"{"name": "Alice", "email": "alice@example.com"}"#;
        let ctx = make_ctx(body);

        let result = Option::<Json<CreateUser>>::from_request(&ctx).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "Alice");
    }
}
