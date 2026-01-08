//! Test response wrapper.

use crate::error::TestError;
use bytes::Bytes;
use http::{header, HeaderMap, HeaderValue, StatusCode};
use serde::de::DeserializeOwned;
use std::fmt;

/// A test response with helper methods for assertions.
pub struct TestResponse {
    /// HTTP status code
    status: StatusCode,
    /// Response headers
    headers: HeaderMap,
    /// Response body bytes
    body: Bytes,
}

impl TestResponse {
    /// Creates a new test response from an HTTP response.
    pub async fn from_http<B>(response: http::Response<B>) -> Result<Self, TestError>
    where
        B: http_body_util::BodyExt,
        B::Error: fmt::Display,
    {
        let (parts, body) = response.into_parts();
        let body_bytes = body
            .collect()
            .await
            .map_err(|e| TestError::BodyRead(e.to_string()))?
            .to_bytes();

        Ok(Self {
            status: parts.status,
            headers: parts.headers,
            body: body_bytes,
        })
    }

    /// Creates a test response from raw parts (for testing).
    pub fn new(status: StatusCode, headers: HeaderMap, body: Bytes) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }

    /// Returns the status code.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns the status code as a u16.
    #[must_use]
    pub fn status_code(&self) -> u16 {
        self.status.as_u16()
    }

    /// Returns true if the status is successful (2xx).
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Returns true if the status is a client error (4xx).
    #[must_use]
    pub fn is_client_error(&self) -> bool {
        self.status.is_client_error()
    }

    /// Returns true if the status is a server error (5xx).
    #[must_use]
    pub fn is_server_error(&self) -> bool {
        self.status.is_server_error()
    }

    /// Returns a reference to the headers.
    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Gets a header value by name.
    #[must_use]
    pub fn header(&self, name: impl AsRef<str>) -> Option<&HeaderValue> {
        self.headers.get(name.as_ref())
    }

    /// Gets a header value as a string.
    #[must_use]
    pub fn header_str(&self, name: impl AsRef<str>) -> Option<&str> {
        self.header(name).and_then(|v| v.to_str().ok())
    }

    /// Returns the Content-Type header value.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.header_str(header::CONTENT_TYPE.as_str())
    }

    /// Returns the Content-Length header value.
    #[must_use]
    pub fn content_length(&self) -> Option<u64> {
        self.header_str(header::CONTENT_LENGTH.as_str())
            .and_then(|v| v.parse().ok())
    }

    /// Returns the raw body bytes.
    #[must_use]
    pub fn body(&self) -> &Bytes {
        &self.body
    }

    /// Returns the body as a string.
    ///
    /// Returns an error if the body is not valid UTF-8.
    pub fn text(&self) -> Result<String, TestError> {
        String::from_utf8(self.body.to_vec())
            .map_err(|e| TestError::BodyRead(format!("Invalid UTF-8: {e}")))
    }

    /// Deserializes the body as JSON.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(Deserialize)]
    /// struct User {
    ///     id: String,
    ///     name: String,
    /// }
    ///
    /// let response = client.get("/users/123").send().await;
    /// let user: User = response.json().unwrap();
    /// assert_eq!(user.id, "123");
    /// ```
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, TestError> {
        serde_json::from_slice(&self.body).map_err(TestError::Json)
    }

    /// Deserializes the body as a JSON Value.
    pub fn json_value(&self) -> Result<serde_json::Value, TestError> {
        self.json()
    }

    // Assertion methods

    /// Asserts that the status code equals the expected value.
    ///
    /// # Panics
    ///
    /// Panics if the status code doesn't match.
    pub fn assert_status(&self, expected: StatusCode) -> &Self {
        assert_eq!(
            self.status, expected,
            "Expected status {}, got {}",
            expected, self.status
        );
        self
    }

    /// Asserts that the status code equals the expected u16 value.
    ///
    /// # Panics
    ///
    /// Panics if the status code doesn't match.
    pub fn assert_status_code(&self, expected: u16) -> &Self {
        assert_eq!(
            self.status.as_u16(),
            expected,
            "Expected status {}, got {}",
            expected,
            self.status.as_u16()
        );
        self
    }

    /// Asserts that the response is successful (2xx).
    ///
    /// # Panics
    ///
    /// Panics if the status is not 2xx.
    pub fn assert_success(&self) -> &Self {
        assert!(
            self.is_success(),
            "Expected success status, got {}",
            self.status
        );
        self
    }

    /// Asserts that a header exists with the expected value.
    ///
    /// # Panics
    ///
    /// Panics if the header doesn't exist or doesn't match.
    pub fn assert_header(&self, name: impl AsRef<str>, expected: impl AsRef<str>) -> &Self {
        let name = name.as_ref();
        let expected = expected.as_ref();
        let actual = self
            .header_str(name)
            .unwrap_or_else(|| panic!("Header '{}' not found", name));
        assert_eq!(
            actual, expected,
            "Header '{}': expected '{}', got '{}'",
            name, expected, actual
        );
        self
    }

    /// Asserts that the Content-Type header matches.
    ///
    /// # Panics
    ///
    /// Panics if Content-Type doesn't match.
    pub fn assert_content_type(&self, expected: impl AsRef<str>) -> &Self {
        let expected = expected.as_ref();
        let actual = self
            .content_type()
            .expect("Content-Type header not found");
        assert!(
            actual.starts_with(expected),
            "Content-Type: expected '{}', got '{}'",
            expected,
            actual
        );
        self
    }

    /// Asserts that the body contains the expected substring.
    ///
    /// # Panics
    ///
    /// Panics if the body doesn't contain the substring.
    pub fn assert_body_contains(&self, expected: impl AsRef<str>) -> &Self {
        let expected = expected.as_ref();
        let body = self.text().expect("Body should be valid UTF-8");
        assert!(
            body.contains(expected),
            "Body should contain '{}', got: {}",
            expected,
            body
        );
        self
    }

    /// Asserts that the body equals the expected string.
    ///
    /// # Panics
    ///
    /// Panics if the body doesn't match.
    pub fn assert_body_eq(&self, expected: impl AsRef<str>) -> &Self {
        let expected = expected.as_ref();
        let body = self.text().expect("Body should be valid UTF-8");
        assert_eq!(body, expected, "Body mismatch");
        self
    }

    /// Asserts that the JSON body matches the expected value.
    ///
    /// # Panics
    ///
    /// Panics if the JSON doesn't match.
    pub fn assert_json_eq(&self, expected: &serde_json::Value) -> &Self {
        let actual: serde_json::Value = self.json().expect("Body should be valid JSON");
        assert_eq!(&actual, expected, "JSON body mismatch");
        self
    }

    /// Asserts that a JSON field exists and equals the expected value.
    ///
    /// # Panics
    ///
    /// Panics if the field doesn't exist or doesn't match.
    pub fn assert_json_field(
        &self,
        path: impl AsRef<str>,
        expected: &serde_json::Value,
    ) -> &Self {
        let path = path.as_ref();
        let json: serde_json::Value = self.json().expect("Body should be valid JSON");
        let actual = json_path(&json, path).unwrap_or_else(|| {
            panic!("JSON path '{}' not found in: {:?}", path, json);
        });
        assert_eq!(
            actual, expected,
            "JSON field '{}': expected {:?}, got {:?}",
            path, expected, actual
        );
        self
    }
}

impl fmt::Debug for TestResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestResponse")
            .field("status", &self.status)
            .field("headers", &self.headers)
            .field("body_len", &self.body.len())
            .finish()
    }
}

/// Simple JSON path accessor.
fn json_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        // Handle array indexing like "items.0.name"
        if let Ok(index) = segment.parse::<usize>() {
            current = current.get(index)?;
        } else {
            current = current.get(segment)?;
        }
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_response(status: u16, body: &str) -> TestResponse {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
        TestResponse::new(
            StatusCode::from_u16(status).unwrap(),
            headers,
            Bytes::from(body.to_string()),
        )
    }

    #[test]
    fn test_status() {
        let response = create_response(200, "{}");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.status_code(), 200);
        assert!(response.is_success());
    }

    #[test]
    fn test_client_error() {
        let response = create_response(404, "{}");
        assert!(response.is_client_error());
        assert!(!response.is_success());
    }

    #[test]
    fn test_server_error() {
        let response = create_response(500, "{}");
        assert!(response.is_server_error());
        assert!(!response.is_success());
    }

    #[test]
    fn test_header() {
        let response = create_response(200, "{}");
        assert_eq!(
            response.header_str("Content-Type"),
            Some("application/json")
        );
    }

    #[test]
    fn test_body() {
        let response = create_response(200, "{\"name\":\"Alice\"}");
        assert_eq!(response.text().unwrap(), "{\"name\":\"Alice\"}");
    }

    #[test]
    fn test_json() {
        let response = create_response(200, "{\"name\":\"Alice\",\"age\":30}");
        let value: serde_json::Value = response.json().unwrap();
        assert_eq!(value["name"], "Alice");
        assert_eq!(value["age"], 30);
    }

    #[test]
    fn test_assert_status() {
        let response = create_response(200, "{}");
        response.assert_status(StatusCode::OK);
        response.assert_status_code(200);
    }

    #[test]
    fn test_assert_success() {
        let response = create_response(201, "{}");
        response.assert_success();
    }

    #[test]
    fn test_assert_header() {
        let response = create_response(200, "{}");
        response.assert_header("Content-Type", "application/json");
    }

    #[test]
    fn test_assert_content_type() {
        let response = create_response(200, "{}");
        response.assert_content_type("application/json");
    }

    #[test]
    fn test_assert_json_eq() {
        let response = create_response(200, "{\"name\":\"Alice\"}");
        response.assert_json_eq(&json!({"name": "Alice"}));
    }

    #[test]
    fn test_assert_json_field() {
        let response = create_response(200, "{\"user\":{\"name\":\"Alice\"}}");
        response.assert_json_field("user.name", &json!("Alice"));
    }

    #[test]
    fn test_json_path() {
        let value = json!({
            "user": {
                "name": "Alice",
                "tags": ["admin", "user"]
            }
        });

        assert_eq!(json_path(&value, "user.name"), Some(&json!("Alice")));
        assert_eq!(json_path(&value, "user.tags.0"), Some(&json!("admin")));
        assert_eq!(json_path(&value, "nonexistent"), None);
    }
}
