//! TypeScript/Node.js bindings for Archimedes TestClient.
//!
//! This module provides TypeScript classes for testing Archimedes applications
//! without starting a real HTTP server.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

/// A test client for making in-memory HTTP requests.
///
/// The test client allows you to test your Archimedes application without
/// starting a real HTTP server or binding to a port.
///
/// @example
/// ```typescript
/// import { App, TestClient } from '@archimedes/node';
///
/// const app = new App();
///
/// app.operation('getUser', async (request) => {
///   return { id: '123', name: 'Alice' };
/// });
///
/// // Create test client
/// const client = new TestClient(app);
///
/// // Make a request
/// const response = await client.get('/users/123');
/// response.assertStatus(200);
/// response.assertJson({ id: '123', name: 'Alice' });
/// ```
#[napi]
pub struct TestClient {
    default_headers: HashMap<String, String>,
    base_url: String,
}

#[napi]
impl TestClient {
    /// Creates a new test client.
    ///
    /// @param app - The Archimedes application to test (optional, for future use)
    /// @param baseUrl - Base URL for requests (default: "http://test")
    #[napi(constructor)]
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            default_headers: HashMap::new(),
            base_url: base_url.unwrap_or_else(|| "http://test".to_string()),
        }
    }

    /// Adds a default header to all requests.
    ///
    /// @param name - Header name
    /// @param value - Header value
    /// @returns Self for method chaining
    #[napi]
    pub fn with_header(&mut self, name: String, value: String) -> &Self {
        self.default_headers.insert(name, value);
        self
    }

    /// Sets a bearer token for all requests.
    ///
    /// @param token - The bearer token
    /// @returns Self for method chaining
    #[napi]
    pub fn with_bearer_token(&mut self, token: String) -> &Self {
        self.default_headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }

    /// Makes a GET request.
    ///
    /// @param path - Request path
    /// @param headers - Optional additional headers
    /// @returns TestResponse
    #[napi]
    pub fn get(&self, path: String, headers: Option<HashMap<String, String>>) -> TestResponse {
        self.request("GET".to_string(), path, headers, None, None)
    }

    /// Makes a POST request.
    ///
    /// @param path - Request path
    /// @param headers - Optional additional headers
    /// @param json - Optional JSON body
    /// @param body - Optional raw body
    /// @returns TestResponse
    #[napi]
    pub fn post(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
        json: Option<String>,
        body: Option<Buffer>,
    ) -> TestResponse {
        self.request(
            "POST".to_string(),
            path,
            headers,
            json,
            body.map(|b| b.to_vec()),
        )
    }

    /// Makes a PUT request.
    ///
    /// @param path - Request path
    /// @param headers - Optional additional headers
    /// @param json - Optional JSON body
    /// @param body - Optional raw body
    /// @returns TestResponse
    #[napi]
    pub fn put(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
        json: Option<String>,
        body: Option<Buffer>,
    ) -> TestResponse {
        self.request(
            "PUT".to_string(),
            path,
            headers,
            json,
            body.map(|b| b.to_vec()),
        )
    }

    /// Makes a PATCH request.
    ///
    /// @param path - Request path
    /// @param headers - Optional additional headers
    /// @param json - Optional JSON body
    /// @param body - Optional raw body
    /// @returns TestResponse
    #[napi]
    pub fn patch(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
        json: Option<String>,
        body: Option<Buffer>,
    ) -> TestResponse {
        self.request(
            "PATCH".to_string(),
            path,
            headers,
            json,
            body.map(|b| b.to_vec()),
        )
    }

    /// Makes a DELETE request.
    ///
    /// @param path - Request path
    /// @param headers - Optional additional headers
    /// @returns TestResponse
    #[napi]
    pub fn delete(&self, path: String, headers: Option<HashMap<String, String>>) -> TestResponse {
        self.request("DELETE".to_string(), path, headers, None, None)
    }

    /// Makes an OPTIONS request.
    ///
    /// @param path - Request path
    /// @param headers - Optional additional headers
    /// @returns TestResponse
    #[napi]
    pub fn options(&self, path: String, headers: Option<HashMap<String, String>>) -> TestResponse {
        self.request("OPTIONS".to_string(), path, headers, None, None)
    }

    /// Makes a HEAD request.
    ///
    /// @param path - Request path
    /// @param headers - Optional additional headers
    /// @returns TestResponse
    #[napi]
    pub fn head(&self, path: String, headers: Option<HashMap<String, String>>) -> TestResponse {
        self.request("HEAD".to_string(), path, headers, None, None)
    }

    /// Makes a request with a custom method.
    ///
    /// @param method - HTTP method
    /// @param path - Request path
    /// @param headers - Optional additional headers
    /// @param json - Optional JSON body (as string)
    /// @param body - Optional raw body
    /// @returns TestResponse
    #[napi]
    pub fn request(
        &self,
        method: String,
        path: String,
        headers: Option<HashMap<String, String>>,
        json: Option<String>,
        body: Option<Vec<u8>>,
    ) -> TestResponse {
        // Build the full URL
        let _url = if path.starts_with("http://") || path.starts_with("https://") {
            path
        } else {
            format!("{}{}", self.base_url, path)
        };

        // Merge headers
        let mut all_headers = self.default_headers.clone();
        if let Some(h) = headers {
            all_headers.extend(h);
        }

        // Handle JSON body
        let body_bytes = if let Some(json_str) = json {
            all_headers.insert("Content-Type".to_string(), "application/json".to_string());
            Some(json_str.into_bytes())
        } else {
            body
        };

        let _ = method; // Will be used in full implementation

        // For now, create a mock response (in full implementation, this would call the actual handler)
        TestResponse::mock(200, all_headers, body_bytes)
    }
}

/// A test response with helper methods for assertions.
///
/// Provides methods to inspect the response and assert expected values.
///
/// @example
/// ```typescript
/// const response = client.get('/users/123');
///
/// // Check status
/// console.log(response.statusCode); // 200
///
/// // Use assertion helpers
/// response.assertStatus(200);
/// response.assertJson({ id: '123' });
/// response.assertHeader('Content-Type', 'application/json');
/// ```
#[napi]
pub struct TestResponse {
    status_code: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl TestResponse {
    /// Creates a mock response for testing.
    fn mock(status_code: u16, headers: HashMap<String, String>, body: Option<Vec<u8>>) -> Self {
        Self {
            status_code,
            headers,
            body: body.unwrap_or_default(),
        }
    }
}

#[napi]
impl TestResponse {
    /// Creates a new test response.
    ///
    /// @param statusCode - HTTP status code
    /// @param headers - Response headers
    /// @param body - Response body bytes
    #[napi(constructor)]
    pub fn new(
        status_code: u16,
        headers: Option<HashMap<String, String>>,
        body: Option<Buffer>,
    ) -> Self {
        Self {
            status_code,
            headers: headers.unwrap_or_default(),
            body: body.map(|b| b.to_vec()).unwrap_or_default(),
        }
    }

    /// Returns the HTTP status code.
    #[napi(getter)]
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Returns the response headers as an object.
    #[napi(getter)]
    pub fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    /// Returns the raw response body as a Buffer.
    #[napi(getter)]
    pub fn content(&self) -> Buffer {
        Buffer::from(self.body.clone())
    }

    /// Returns the response body as text (UTF-8).
    #[napi(getter)]
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.body.clone())
            .map_err(|e| Error::from_reason(format!("Invalid UTF-8: {}", e)))
    }

    /// Returns the response body as parsed JSON.
    #[napi]
    pub fn json(&self) -> Result<String> {
        // Returns the raw JSON string for JS to parse
        self.text()
    }

    /// Returns true if the status is successful (2xx).
    #[napi]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Returns true if the status is a client error (4xx).
    #[napi]
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    /// Returns true if the status is a server error (5xx).
    #[napi]
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    /// Gets a header value by name (case-insensitive).
    ///
    /// @param name - Header name
    /// @returns Header value or undefined if not found
    #[napi]
    pub fn get_header(&self, name: String) -> Option<String> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.clone())
    }

    /// Returns the Content-Type header value.
    #[napi]
    pub fn content_type(&self) -> Option<String> {
        self.get_header("Content-Type".to_string())
    }

    /// Returns the Content-Length header value.
    #[napi]
    pub fn content_length(&self) -> Option<u32> {
        self.get_header("Content-Length".to_string())
            .and_then(|v| v.parse().ok())
    }

    // Assertion methods

    /// Asserts that the status code equals the expected value.
    ///
    /// @param expected - Expected status code
    /// @returns Self for method chaining
    /// @throws Error if status code doesn't match
    #[napi]
    pub fn assert_status(&self, expected: u16) -> Result<&Self> {
        if self.status_code != expected {
            return Err(Error::from_reason(format!(
                "Expected status {}, got {}",
                expected, self.status_code
            )));
        }
        Ok(self)
    }

    /// Asserts that the response is successful (2xx).
    ///
    /// @returns Self for method chaining
    /// @throws Error if status is not 2xx
    #[napi]
    pub fn assert_success(&self) -> Result<&Self> {
        if !self.is_success() {
            return Err(Error::from_reason(format!(
                "Expected success status (2xx), got {}",
                self.status_code
            )));
        }
        Ok(self)
    }

    /// Asserts that a header exists with the expected value.
    ///
    /// @param name - Header name
    /// @param expected - Expected header value
    /// @returns Self for method chaining
    /// @throws Error if header doesn't exist or doesn't match
    #[napi]
    pub fn assert_header(&self, name: String, expected: String) -> Result<&Self> {
        let actual = self.get_header(name.clone()).ok_or_else(|| {
            Error::from_reason(format!("Header '{}' not found", name))
        })?;
        if actual != expected {
            return Err(Error::from_reason(format!(
                "Header '{}': expected '{}', got '{}'",
                name, expected, actual
            )));
        }
        Ok(self)
    }

    /// Asserts that the Content-Type header starts with the expected value.
    ///
    /// @param expected - Expected Content-Type prefix
    /// @returns Self for method chaining
    /// @throws Error if Content-Type doesn't match
    #[napi]
    pub fn assert_content_type(&self, expected: String) -> Result<&Self> {
        let actual = self.content_type().ok_or_else(|| {
            Error::from_reason("Content-Type header not found".to_string())
        })?;
        if !actual.starts_with(&expected) {
            return Err(Error::from_reason(format!(
                "Content-Type: expected '{}', got '{}'",
                expected, actual
            )));
        }
        Ok(self)
    }

    /// Asserts that the body contains the expected substring.
    ///
    /// @param expected - Expected substring
    /// @returns Self for method chaining
    /// @throws Error if body doesn't contain substring
    #[napi]
    pub fn assert_body_contains(&self, expected: String) -> Result<&Self> {
        let body = self.text()?;
        if !body.contains(&expected) {
            return Err(Error::from_reason(format!(
                "Body should contain '{}', got: {}",
                expected, body
            )));
        }
        Ok(self)
    }

    /// Asserts that the body equals the expected string.
    ///
    /// @param expected - Expected body
    /// @returns Self for method chaining
    /// @throws Error if body doesn't match
    #[napi]
    pub fn assert_body_eq(&self, expected: String) -> Result<&Self> {
        let body = self.text()?;
        if body != expected {
            return Err(Error::from_reason(format!(
                "Body mismatch: expected '{}', got '{}'",
                expected, body
            )));
        }
        Ok(self)
    }

    /// Asserts that the JSON body matches the expected value.
    ///
    /// @param expected - Expected JSON as string
    /// @returns Self for method chaining
    /// @throws Error if JSON doesn't match
    #[napi]
    pub fn assert_json_eq(&self, expected: String) -> Result<&Self> {
        let actual = self.text()?;

        // Parse both as JSON to compare semantically
        let actual_json: serde_json::Value =
            serde_json::from_str(&actual).map_err(|e| Error::from_reason(e.to_string()))?;
        let expected_json: serde_json::Value =
            serde_json::from_str(&expected).map_err(|e| Error::from_reason(e.to_string()))?;

        if actual_json != expected_json {
            return Err(Error::from_reason(format!(
                "JSON mismatch:\nExpected: {}\nActual: {}",
                expected, actual
            )));
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_response_status() {
        let response = TestResponse::new(200, None, None);
        assert_eq!(response.status_code, 200);
        assert!(response.is_success());
        assert!(!response.is_client_error());
        assert!(!response.is_server_error());
    }

    #[test]
    fn test_test_response_client_error() {
        let response = TestResponse::new(404, None, None);
        assert_eq!(response.status_code, 404);
        assert!(!response.is_success());
        assert!(response.is_client_error());
        assert!(!response.is_server_error());
    }

    #[test]
    fn test_test_response_server_error() {
        let response = TestResponse::new(500, None, None);
        assert_eq!(response.status_code, 500);
        assert!(!response.is_success());
        assert!(!response.is_client_error());
        assert!(response.is_server_error());
    }

    #[test]
    fn test_test_response_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("X-Request-Id".to_string(), "abc123".to_string());

        let response = TestResponse::new(200, Some(headers), None);
        assert_eq!(
            response.get_header("Content-Type".to_string()),
            Some("application/json".to_string())
        );
        assert_eq!(
            response.get_header("content-type".to_string()),
            Some("application/json".to_string())
        );
        assert_eq!(
            response.get_header("X-Request-Id".to_string()),
            Some("abc123".to_string())
        );
        assert_eq!(response.get_header("Not-Found".to_string()), None);
    }

    #[test]
    fn test_test_response_body() {
        let body = Buffer::from(b"Hello, World!".to_vec());
        let response = TestResponse::new(200, None, Some(body));
        assert_eq!(response.text().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_test_client_default() {
        let client = TestClient::new(None);
        assert!(client.default_headers.is_empty());
        assert_eq!(client.base_url, "http://test");
    }

    #[test]
    fn test_test_client_custom_base_url() {
        let client = TestClient::new(Some("http://localhost:8080".to_string()));
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_test_response_assert_status_success() {
        let response = TestResponse::new(200, None, None);
        assert!(response.assert_status(200).is_ok());
    }

    #[test]
    fn test_test_response_assert_status_failure() {
        let response = TestResponse::new(404, None, None);
        assert!(response.assert_status(200).is_err());
    }

    #[test]
    fn test_test_response_assert_header_success() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        let response = TestResponse::new(200, Some(headers), None);
        assert!(response
            .assert_header("Content-Type".to_string(), "application/json".to_string())
            .is_ok());
    }

    #[test]
    fn test_test_response_json_eq() {
        let body = Buffer::from(r#"{"id":"123","name":"Alice"}"#.as_bytes().to_vec());
        let response = TestResponse::new(200, None, Some(body));
        assert!(response
            .assert_json_eq(r#"{"id":"123","name":"Alice"}"#.to_string())
            .is_ok());
        // Order shouldn't matter
        assert!(response
            .assert_json_eq(r#"{"name":"Alice","id":"123"}"#.to_string())
            .is_ok());
    }
}
