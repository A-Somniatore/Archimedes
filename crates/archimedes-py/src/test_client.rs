//! Python bindings for Archimedes TestClient.
//!
//! This module provides Python classes for testing Archimedes applications
//! without starting a real HTTP server.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::collections::HashMap;

/// A test client for making in-memory HTTP requests.
///
/// The test client allows you to test your Archimedes application without
/// starting a real HTTP server or binding to a port.
///
/// Example:
///     ```python
///     from archimedes import ArchimedesApp, TestClient
///
///     app = ArchimedesApp()
///
///     @app.operation("getUser")
///     async def get_user(request):
///         return {"id": "123", "name": "Alice"}
///
///     # Create test client
///     client = TestClient(app)
///
///     # Make a request
///     response = client.get("/users/123")
///     response.assert_status(200)
///     response.assert_json({"id": "123", "name": "Alice"})
///     ```
#[pyclass(name = "TestClient")]
#[derive(Clone)]
pub struct PyTestClient {
    default_headers: HashMap<String, String>,
    base_url: String,
}

#[pymethods]
impl PyTestClient {
    /// Creates a new test client.
    ///
    /// Args:
    ///     app: The Archimedes application to test (optional, for future use)
    ///     base_url: Base URL for requests (default: "http://test")
    #[new]
    #[pyo3(signature = (app=None, base_url=None))]
    fn new(app: Option<&Bound<'_, PyAny>>, base_url: Option<String>) -> Self {
        let _ = app; // Reserved for future integration
        Self {
            default_headers: HashMap::new(),
            base_url: base_url.unwrap_or_else(|| "http://test".to_string()),
        }
    }

    /// Adds a default header to all requests.
    ///
    /// Args:
    ///     name: Header name
    ///     value: Header value
    ///
    /// Returns:
    ///     Self for method chaining
    fn with_header(mut slf: PyRefMut<'_, Self>, name: String, value: String) -> PyRefMut<'_, Self> {
        slf.default_headers.insert(name, value);
        slf
    }

    /// Sets a bearer token for all requests.
    ///
    /// Args:
    ///     token: The bearer token
    ///
    /// Returns:
    ///     Self for method chaining
    fn with_bearer_token(mut slf: PyRefMut<'_, Self>, token: String) -> PyRefMut<'_, Self> {
        slf.default_headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        slf
    }

    /// Makes a GET request.
    ///
    /// Args:
    ///     path: Request path
    ///     headers: Optional additional headers
    ///
    /// Returns:
    ///     TestResponse
    #[pyo3(signature = (path, headers=None))]
    fn get(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<PyTestResponse> {
        self.request("GET", path, headers, None, None)
    }

    /// Makes a POST request.
    ///
    /// Args:
    ///     path: Request path
    ///     headers: Optional additional headers
    ///     json: Optional JSON body
    ///     body: Optional raw body
    ///
    /// Returns:
    ///     TestResponse
    #[pyo3(signature = (path, headers=None, json=None, body=None))]
    fn post(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
        json: Option<&Bound<'_, PyAny>>,
        body: Option<Vec<u8>>,
    ) -> PyResult<PyTestResponse> {
        self.request("POST", path, headers, json, body)
    }

    /// Makes a PUT request.
    ///
    /// Args:
    ///     path: Request path
    ///     headers: Optional additional headers
    ///     json: Optional JSON body
    ///     body: Optional raw body
    ///
    /// Returns:
    ///     TestResponse
    #[pyo3(signature = (path, headers=None, json=None, body=None))]
    fn put(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
        json: Option<&Bound<'_, PyAny>>,
        body: Option<Vec<u8>>,
    ) -> PyResult<PyTestResponse> {
        self.request("PUT", path, headers, json, body)
    }

    /// Makes a PATCH request.
    ///
    /// Args:
    ///     path: Request path
    ///     headers: Optional additional headers
    ///     json: Optional JSON body
    ///     body: Optional raw body
    ///
    /// Returns:
    ///     TestResponse
    #[pyo3(signature = (path, headers=None, json=None, body=None))]
    fn patch(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
        json: Option<&Bound<'_, PyAny>>,
        body: Option<Vec<u8>>,
    ) -> PyResult<PyTestResponse> {
        self.request("PATCH", path, headers, json, body)
    }

    /// Makes a DELETE request.
    ///
    /// Args:
    ///     path: Request path
    ///     headers: Optional additional headers
    ///
    /// Returns:
    ///     TestResponse
    #[pyo3(signature = (path, headers=None))]
    fn delete(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<PyTestResponse> {
        self.request("DELETE", path, headers, None, None)
    }

    /// Makes an OPTIONS request.
    ///
    /// Args:
    ///     path: Request path
    ///     headers: Optional additional headers
    ///
    /// Returns:
    ///     TestResponse
    #[pyo3(signature = (path, headers=None))]
    fn options(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<PyTestResponse> {
        self.request("OPTIONS", path, headers, None, None)
    }

    /// Makes a HEAD request.
    ///
    /// Args:
    ///     path: Request path
    ///     headers: Optional additional headers
    ///
    /// Returns:
    ///     TestResponse
    #[pyo3(signature = (path, headers=None))]
    fn head(
        &self,
        path: String,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<PyTestResponse> {
        self.request("HEAD", path, headers, None, None)
    }

    /// Makes a request with a custom method.
    ///
    /// Args:
    ///     method: HTTP method
    ///     path: Request path
    ///     headers: Optional additional headers
    ///     json: Optional JSON body
    ///     body: Optional raw body
    ///
    /// Returns:
    ///     TestResponse
    #[pyo3(signature = (method, path, headers=None, json=None, body=None))]
    fn request(
        &self,
        method: &str,
        path: String,
        headers: Option<HashMap<String, String>>,
        json: Option<&Bound<'_, PyAny>>,
        body: Option<Vec<u8>>,
    ) -> PyResult<PyTestResponse> {
        // Build the full URL
        let url = if path.starts_with("http://") || path.starts_with("https://") {
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
        let body_bytes = if let Some(json_val) = json {
            // Convert Python object to JSON string
            let json_str = Python::with_gil(|py| {
                let json_module = py.import("json")?;
                let dumps = json_module.getattr("dumps")?;
                let result = dumps.call1((json_val,))?;
                result.extract::<String>()
            })?;
            all_headers.insert("Content-Type".to_string(), "application/json".to_string());
            Some(json_str.into_bytes())
        } else {
            body
        };

        // For now, create a mock response (in full implementation, this would call the actual handler)
        // This is a placeholder that will be replaced with actual handler invocation
        Ok(PyTestResponse::mock(200, all_headers, body_bytes))
    }
}

/// A test response with helper methods for assertions.
///
/// Provides methods to inspect the response and assert expected values.
///
/// Example:
///     ```python
///     response = client.get("/users/123")
///
///     # Check status
///     assert response.status_code == 200
///
///     # Use assertion helpers
///     response.assert_status(200)
///     response.assert_json({"id": "123"})
///     response.assert_header("Content-Type", "application/json")
///     ```
#[pyclass(name = "TestResponse")]
#[derive(Clone)]
pub struct PyTestResponse {
    status_code: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl PyTestResponse {
    /// Creates a mock response for testing.
    fn mock(
        status_code: u16,
        headers: HashMap<String, String>,
        body: Option<Vec<u8>>,
    ) -> Self {
        Self {
            status_code,
            headers,
            body: body.unwrap_or_default(),
        }
    }
}

#[pymethods]
impl PyTestResponse {
    /// Creates a new test response.
    ///
    /// Args:
    ///     status_code: HTTP status code
    ///     headers: Response headers
    ///     body: Response body bytes
    #[new]
    #[pyo3(signature = (status_code, headers=None, body=None))]
    fn new(
        status_code: u16,
        headers: Option<HashMap<String, String>>,
        body: Option<Vec<u8>>,
    ) -> Self {
        Self {
            status_code,
            headers: headers.unwrap_or_default(),
            body: body.unwrap_or_default(),
        }
    }

    /// Returns the HTTP status code.
    #[getter]
    fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Returns the response headers as a dict.
    #[getter]
    fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    /// Returns the raw response body as bytes.
    #[getter]
    fn content<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.body)
    }

    /// Returns the response body as text (UTF-8).
    #[getter]
    fn text(&self) -> PyResult<String> {
        String::from_utf8(self.body.clone())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid UTF-8: {}", e)))
    }

    /// Returns the response body as JSON.
    fn json<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let text = self.text()?;
        let json_module = py.import("json")?;
        let loads = json_module.getattr("loads")?;
        loads.call1((text,))
    }

    /// Returns true if the status is successful (2xx).
    fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Returns true if the status is a client error (4xx).
    fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    /// Returns true if the status is a server error (5xx).
    fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }

    /// Gets a header value by name (case-insensitive).
    ///
    /// Args:
    ///     name: Header name
    ///
    /// Returns:
    ///     Header value or None if not found
    fn get_header(&self, name: &str) -> Option<String> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.clone())
    }

    /// Returns the Content-Type header value.
    fn content_type(&self) -> Option<String> {
        self.get_header("Content-Type")
    }

    /// Returns the Content-Length header value.
    fn content_length(&self) -> Option<u64> {
        self.get_header("Content-Length")
            .and_then(|v| v.parse().ok())
    }

    // Assertion methods

    /// Asserts that the status code equals the expected value.
    ///
    /// Args:
    ///     expected: Expected status code
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     AssertionError: If status code doesn't match
    fn assert_status(slf: PyRef<'_, Self>, expected: u16) -> PyResult<PyRef<'_, Self>> {
        if slf.status_code != expected {
            return Err(pyo3::exceptions::PyAssertionError::new_err(format!(
                "Expected status {}, got {}",
                expected, slf.status_code
            )));
        }
        Ok(slf)
    }

    /// Asserts that the response is successful (2xx).
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     AssertionError: If status is not 2xx
    fn assert_success(slf: PyRef<'_, Self>) -> PyResult<PyRef<'_, Self>> {
        if !slf.is_success() {
            return Err(pyo3::exceptions::PyAssertionError::new_err(format!(
                "Expected success status (2xx), got {}",
                slf.status_code
            )));
        }
        Ok(slf)
    }

    /// Asserts that a header exists with the expected value.
    ///
    /// Args:
    ///     name: Header name
    ///     expected: Expected header value
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     AssertionError: If header doesn't exist or doesn't match
    fn assert_header(
        slf: PyRef<'_, Self>,
        name: &str,
        expected: &str,
    ) -> PyResult<PyRef<'_, Self>> {
        let actual = slf.get_header(name).ok_or_else(|| {
            pyo3::exceptions::PyAssertionError::new_err(format!("Header '{}' not found", name))
        })?;
        if actual != expected {
            return Err(pyo3::exceptions::PyAssertionError::new_err(format!(
                "Header '{}': expected '{}', got '{}'",
                name, expected, actual
            )));
        }
        Ok(slf)
    }

    /// Asserts that the Content-Type header starts with the expected value.
    ///
    /// Args:
    ///     expected: Expected Content-Type prefix
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     AssertionError: If Content-Type doesn't match
    fn assert_content_type(slf: PyRef<'_, Self>, expected: &str) -> PyResult<PyRef<'_, Self>> {
        let actual = slf.content_type().ok_or_else(|| {
            pyo3::exceptions::PyAssertionError::new_err("Content-Type header not found")
        })?;
        if !actual.starts_with(expected) {
            return Err(pyo3::exceptions::PyAssertionError::new_err(format!(
                "Content-Type: expected '{}', got '{}'",
                expected, actual
            )));
        }
        Ok(slf)
    }

    /// Asserts that the body contains the expected substring.
    ///
    /// Args:
    ///     expected: Expected substring
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     AssertionError: If body doesn't contain substring
    fn assert_body_contains(slf: PyRef<'_, Self>, expected: &str) -> PyResult<PyRef<'_, Self>> {
        let body = slf.text()?;
        if !body.contains(expected) {
            return Err(pyo3::exceptions::PyAssertionError::new_err(format!(
                "Body should contain '{}', got: {}",
                expected, body
            )));
        }
        Ok(slf)
    }

    /// Asserts that the body equals the expected string.
    ///
    /// Args:
    ///     expected: Expected body
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     AssertionError: If body doesn't match
    fn assert_body_eq(slf: PyRef<'_, Self>, expected: &str) -> PyResult<PyRef<'_, Self>> {
        let body = slf.text()?;
        if body != expected {
            return Err(pyo3::exceptions::PyAssertionError::new_err(format!(
                "Body mismatch: expected '{}', got '{}'",
                expected, body
            )));
        }
        Ok(slf)
    }

    /// Asserts that the JSON body matches the expected value.
    ///
    /// Args:
    ///     expected: Expected JSON (dict or list)
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     AssertionError: If JSON doesn't match
    fn assert_json<'py>(
        slf: PyRef<'py, Self>,
        py: Python<'py>,
        expected: &Bound<'py, PyAny>,
    ) -> PyResult<PyRef<'py, Self>> {
        let actual = slf.json(py)?;

        // Compare using Python's == operator
        let eq = actual.eq(expected)?;
        if !eq {
            let json_module = py.import("json")?;
            let dumps = json_module.getattr("dumps")?;
            let actual_str: String = dumps.call1((&actual,))?.extract()?;
            let expected_str: String = dumps.call1((expected,))?.extract()?;
            return Err(pyo3::exceptions::PyAssertionError::new_err(format!(
                "JSON mismatch:\nExpected: {}\nActual: {}",
                expected_str, actual_str
            )));
        }
        Ok(slf)
    }

    /// Asserts that a JSON field exists and equals the expected value.
    ///
    /// Args:
    ///     path: JSON path (dot-separated, e.g., "user.name")
    ///     expected: Expected value
    ///
    /// Returns:
    ///     Self for method chaining
    ///
    /// Raises:
    ///     AssertionError: If field doesn't exist or doesn't match
    fn assert_json_field<'py>(
        slf: PyRef<'py, Self>,
        py: Python<'py>,
        path: &str,
        expected: &Bound<'py, PyAny>,
    ) -> PyResult<PyRef<'py, Self>> {
        let json = slf.json(py)?;

        // Navigate the path
        let mut current = json;
        for key in path.split('.') {
            // Try as dict key first
            if let Ok(dict) = current.downcast::<PyDict>() {
                current = dict.get_item(key)?.ok_or_else(|| {
                    pyo3::exceptions::PyAssertionError::new_err(format!(
                        "JSON path '{}' not found at key '{}'",
                        path, key
                    ))
                })?;
            } else if let Ok(idx) = key.parse::<usize>() {
                // Try as list index
                current = current.get_item(idx).map_err(|_| {
                    pyo3::exceptions::PyAssertionError::new_err(format!(
                        "JSON path '{}' not found at index {}",
                        path, idx
                    ))
                })?;
            } else {
                return Err(pyo3::exceptions::PyAssertionError::new_err(format!(
                    "Cannot navigate JSON path '{}' at '{}'",
                    path, key
                )));
            }
        }

        // Compare
        let eq = current.eq(expected)?;
        if !eq {
            let json_module = py.import("json")?;
            let dumps = json_module.getattr("dumps")?;
            let actual_str: String = dumps
                .call1((&current,))
                .unwrap_or_else(|_| current.str().unwrap())
                .extract()
                .unwrap_or_else(|_| format!("{:?}", current));
            let expected_str: String = dumps
                .call1((expected,))
                .unwrap_or_else(|_| expected.str().unwrap())
                .extract()
                .unwrap_or_else(|_| format!("{:?}", expected));
            return Err(pyo3::exceptions::PyAssertionError::new_err(format!(
                "JSON field '{}': expected {}, got {}",
                path, expected_str, actual_str
            )));
        }
        Ok(slf)
    }

    fn __repr__(&self) -> String {
        format!(
            "TestResponse(status_code={}, headers={:?}, body_len={})",
            self.status_code,
            self.headers.keys().collect::<Vec<_>>(),
            self.body.len()
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_response_status() {
        let response = PyTestResponse::new(200, None, None);
        assert_eq!(response.status_code, 200);
        assert!(response.is_success());
        assert!(!response.is_client_error());
        assert!(!response.is_server_error());
    }

    #[test]
    fn test_test_response_client_error() {
        let response = PyTestResponse::new(404, None, None);
        assert_eq!(response.status_code, 404);
        assert!(!response.is_success());
        assert!(response.is_client_error());
        assert!(!response.is_server_error());
    }

    #[test]
    fn test_test_response_server_error() {
        let response = PyTestResponse::new(500, None, None);
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

        let response = PyTestResponse::new(200, Some(headers), None);
        assert_eq!(
            response.get_header("Content-Type"),
            Some("application/json".to_string())
        );
        assert_eq!(
            response.get_header("content-type"),
            Some("application/json".to_string())
        );
        assert_eq!(
            response.get_header("X-Request-Id"),
            Some("abc123".to_string())
        );
        assert_eq!(response.get_header("Not-Found"), None);
    }

    #[test]
    fn test_test_response_body() {
        let body = b"Hello, World!".to_vec();
        let response = PyTestResponse::new(200, None, Some(body));
        assert_eq!(response.text().unwrap(), "Hello, World!");
    }

    #[test]
    fn test_test_client_default_headers() {
        let client = PyTestClient::new(None, None);
        assert!(client.default_headers.is_empty());
        assert_eq!(client.base_url, "http://test");
    }

    #[test]
    fn test_test_client_custom_base_url() {
        let client = PyTestClient::new(None, Some("http://localhost:8080".to_string()));
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_test_response_content_type() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json; charset=utf-8".to_string());
        let response = PyTestResponse::new(200, Some(headers), None);
        assert_eq!(
            response.content_type(),
            Some("application/json; charset=utf-8".to_string())
        );
    }

    #[test]
    fn test_test_response_content_length() {
        let mut headers = HashMap::new();
        headers.insert("Content-Length".to_string(), "1234".to_string());
        let response = PyTestResponse::new(200, Some(headers), None);
        assert_eq!(response.content_length(), Some(1234));
    }
}
