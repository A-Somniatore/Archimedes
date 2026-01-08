//! Test request building.

use crate::error::TestError;
use bytes::Bytes;
use http::{header, HeaderMap, HeaderName, HeaderValue, Method, Uri};
use http_body_util::Full;
use serde::Serialize;

/// A test request that can be sent to a [`TestClient`](crate::TestClient).
pub struct TestRequest {
    /// HTTP method
    pub method: Method,
    /// Request URI
    pub uri: Uri,
    /// Request headers
    pub headers: HeaderMap,
    /// Request body
    pub body: Bytes,
}

impl TestRequest {
    /// Creates a new GET request.
    pub fn get(uri: impl AsRef<str>) -> TestRequestBuilder {
        TestRequestBuilder::new(Method::GET, uri)
    }

    /// Creates a new POST request.
    pub fn post(uri: impl AsRef<str>) -> TestRequestBuilder {
        TestRequestBuilder::new(Method::POST, uri)
    }

    /// Creates a new PUT request.
    pub fn put(uri: impl AsRef<str>) -> TestRequestBuilder {
        TestRequestBuilder::new(Method::PUT, uri)
    }

    /// Creates a new PATCH request.
    pub fn patch(uri: impl AsRef<str>) -> TestRequestBuilder {
        TestRequestBuilder::new(Method::PATCH, uri)
    }

    /// Creates a new DELETE request.
    pub fn delete(uri: impl AsRef<str>) -> TestRequestBuilder {
        TestRequestBuilder::new(Method::DELETE, uri)
    }

    /// Creates a new OPTIONS request.
    pub fn options(uri: impl AsRef<str>) -> TestRequestBuilder {
        TestRequestBuilder::new(Method::OPTIONS, uri)
    }

    /// Creates a new HEAD request.
    pub fn head(uri: impl AsRef<str>) -> TestRequestBuilder {
        TestRequestBuilder::new(Method::HEAD, uri)
    }

    /// Converts this request to an HTTP request.
    pub fn into_http_request(self) -> http::Request<Full<Bytes>> {
        let mut builder = http::Request::builder()
            .method(self.method)
            .uri(self.uri);

        for (name, value) in &self.headers {
            builder = builder.header(name, value);
        }

        builder
            .body(Full::new(self.body))
            .expect("valid request")
    }
}

/// Builder for constructing test requests.
#[must_use]
pub struct TestRequestBuilder {
    method: Method,
    uri: String,
    headers: HeaderMap,
    body: Option<Bytes>,
}

impl TestRequestBuilder {
    /// Creates a new request builder.
    pub fn new(method: Method, uri: impl AsRef<str>) -> Self {
        Self {
            method,
            uri: uri.as_ref().to_string(),
            headers: HeaderMap::new(),
            body: None,
        }
    }

    /// Sets a header on the request.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let request = TestRequest::get("/users")
    ///     .header("Authorization", "Bearer token")
    ///     .header("X-Request-ID", "12345")
    ///     .build();
    /// ```
    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        let name = HeaderName::try_from(name.as_ref()).expect("valid header name");
        let value = HeaderValue::try_from(value.as_ref()).expect("valid header value");
        self.headers.insert(name, value);
        self
    }

    /// Sets a typed header on the request.
    pub fn header_typed(mut self, name: HeaderName, value: HeaderValue) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Sets the Content-Type header.
    pub fn content_type(self, content_type: impl AsRef<str>) -> Self {
        self.header(header::CONTENT_TYPE.as_str(), content_type)
    }

    /// Sets the Accept header.
    pub fn accept(self, accept: impl AsRef<str>) -> Self {
        self.header(header::ACCEPT.as_str(), accept)
    }

    /// Sets the Authorization header with a Bearer token.
    pub fn bearer_token(self, token: impl AsRef<str>) -> Self {
        self.header(header::AUTHORIZATION.as_str(), format!("Bearer {}", token.as_ref()))
    }

    /// Sets the raw request body.
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Sets the request body as JSON.
    ///
    /// This also sets the `Content-Type` header to `application/json`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use serde_json::json;
    ///
    /// let request = TestRequest::post("/users")
    ///     .json(&json!({
    ///         "name": "Alice",
    ///         "email": "alice@example.com"
    ///     }))
    ///     .build();
    /// ```
    pub fn json<T: Serialize>(mut self, value: &T) -> Self {
        let bytes = serde_json::to_vec(value).expect("JSON serialization should succeed");
        self.body = Some(Bytes::from(bytes));
        self.content_type("application/json")
    }

    /// Sets the request body as form-urlencoded.
    ///
    /// This also sets the `Content-Type` header to `application/x-www-form-urlencoded`.
    pub fn form<T: Serialize>(mut self, value: &T) -> Self {
        let encoded = serde_urlencoded::to_string(value).expect("form encoding should succeed");
        self.body = Some(Bytes::from(encoded));
        self.content_type("application/x-www-form-urlencoded")
    }

    /// Builds the test request.
    pub fn build(self) -> Result<TestRequest, TestError> {
        let uri: Uri = self
            .uri
            .parse()
            .map_err(|e| TestError::RequestBuild(format!("Invalid URI: {e}")))?;

        Ok(TestRequest {
            method: self.method,
            uri,
            headers: self.headers,
            body: self.body.unwrap_or_default(),
        })
    }
}

// Add serde_urlencoded support
mod serde_urlencoded {
    use serde::Serialize;

    pub fn to_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
        // Simple implementation using serde_json for now
        // In a real implementation, we'd use a proper form encoder
        let json = serde_json::to_value(value)?;
        if let serde_json::Value::Object(map) = json {
            let pairs: Vec<String> = map
                .into_iter()
                .map(|(k, v)| {
                    let v_str = match v {
                        serde_json::Value::String(s) => s,
                        serde_json::Value::Number(n) => n.to_string(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        _ => v.to_string(),
                    };
                    format!("{}={}", urlencoding::encode(&k), urlencoding::encode(&v_str))
                })
                .collect();
            Ok(pairs.join("&"))
        } else {
            Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Expected object for form encoding",
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_request() {
        let request = TestRequest::get("/users").build().unwrap();
        assert_eq!(request.method, Method::GET);
        assert_eq!(request.uri.path(), "/users");
    }

    #[test]
    fn test_post_request() {
        let request = TestRequest::post("/users").build().unwrap();
        assert_eq!(request.method, Method::POST);
    }

    #[test]
    fn test_put_request() {
        let request = TestRequest::put("/users/123").build().unwrap();
        assert_eq!(request.method, Method::PUT);
    }

    #[test]
    fn test_delete_request() {
        let request = TestRequest::delete("/users/123").build().unwrap();
        assert_eq!(request.method, Method::DELETE);
    }

    #[test]
    fn test_header() {
        let request = TestRequest::get("/users")
            .header("Authorization", "Bearer token")
            .build()
            .unwrap();

        assert_eq!(
            request.headers.get("Authorization").unwrap(),
            "Bearer token"
        );
    }

    #[test]
    fn test_bearer_token() {
        let request = TestRequest::get("/users")
            .bearer_token("my_token")
            .build()
            .unwrap();

        assert_eq!(
            request.headers.get("Authorization").unwrap(),
            "Bearer my_token"
        );
    }

    #[test]
    fn test_json_body() {
        let request = TestRequest::post("/users")
            .json(&json!({"name": "Alice"}))
            .build()
            .unwrap();

        assert_eq!(
            request.headers.get("Content-Type").unwrap(),
            "application/json"
        );
        assert_eq!(request.body.as_ref(), b"{\"name\":\"Alice\"}");
    }

    #[test]
    fn test_raw_body() {
        let request = TestRequest::post("/data")
            .body("raw data")
            .build()
            .unwrap();

        assert_eq!(request.body.as_ref(), b"raw data");
    }

    #[test]
    fn test_content_type() {
        let request = TestRequest::post("/data")
            .content_type("text/plain")
            .build()
            .unwrap();

        assert_eq!(
            request.headers.get("Content-Type").unwrap(),
            "text/plain"
        );
    }

    #[test]
    fn test_into_http_request() {
        let request = TestRequest::get("/users")
            .header("X-Test", "value")
            .build()
            .unwrap();

        let http_request = request.into_http_request();
        assert_eq!(http_request.method(), Method::GET);
        assert_eq!(http_request.uri().path(), "/users");
        assert_eq!(http_request.headers().get("X-Test").unwrap(), "value");
    }
}
