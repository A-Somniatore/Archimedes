//! Test client for in-memory HTTP testing.

use crate::error::TestError;
use crate::request::{TestRequest, TestRequestBuilder};
use crate::response::TestResponse;
use archimedes_middleware::context::MiddlewareContext;
use archimedes_middleware::types::Response;
use bytes::Bytes;
use http::{Method, StatusCode};
use http_body_util::Full;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Handler function type for test client.
pub type TestHandler = Arc<
    dyn Fn(MiddlewareContext, TestRequest) -> Pin<Box<dyn Future<Output = Response> + Send>>
        + Send
        + Sync,
>;

/// A test client for making in-memory HTTP requests.
///
/// The test client allows you to test your Archimedes application without
/// starting a real HTTP server or binding to a port.
///
/// # Example
///
/// ```ignore
/// use archimedes_test::TestClient;
///
/// // Create a test client with a handler
/// let client = TestClient::new(|ctx, req| async move {
///     // Your handler logic
///     Response::builder()
///         .status(200)
///         .body(Full::new(Bytes::from("OK")))
///         .unwrap()
/// });
///
/// let response = client.get("/users").send().await;
/// assert_eq!(response.status_code(), 200);
/// ```
#[must_use]
pub struct TestClient {
    /// The handler function to process requests.
    handler: TestHandler,
    /// Default headers to add to all requests.
    default_headers: Vec<(String, String)>,
}

impl TestClient {
    /// Creates a new test client with a handler function.
    ///
    /// The handler receives the middleware context and request, and should
    /// return an HTTP response.
    pub fn new<F, Fut>(handler: F) -> Self
    where
        F: Fn(MiddlewareContext, TestRequest) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Response> + Send + 'static,
    {
        Self {
            handler: Arc::new(move |ctx, req| Box::pin(handler(ctx, req))),
            default_headers: Vec::new(),
        }
    }

    /// Creates a test client with a simple echo handler for testing.
    ///
    /// The echo handler returns the request body and method in the response.
    pub fn echo() -> Self {
        Self::new(|_ctx, req| async move {
            let body = format!(
                "{{\"method\":\"{}\",\"path\":\"{}\"}}",
                req.method,
                req.uri.path()
            );
            http::Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Full::new(Bytes::from(body)))
                .expect("valid response")
        })
    }

    /// Creates a test client that always returns a fixed response.
    pub fn fixed_response(status: StatusCode, body: impl Into<String>) -> Self {
        let body = body.into();
        Self::new(move |_ctx, _req| {
            let body = body.clone();
            async move {
                http::Response::builder()
                    .status(status)
                    .body(Full::new(Bytes::from(body)))
                    .expect("valid response")
            }
        })
    }

    /// Adds a default header that will be included in all requests.
    pub fn with_default_header(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.default_headers.push((name.into(), value.into()));
        self
    }

    /// Creates a GET request builder.
    pub fn get(&self, uri: impl AsRef<str>) -> TestClientRequest<'_> {
        TestClientRequest::new(self, TestRequest::get(uri))
    }

    /// Creates a POST request builder.
    pub fn post(&self, uri: impl AsRef<str>) -> TestClientRequest<'_> {
        TestClientRequest::new(self, TestRequest::post(uri))
    }

    /// Creates a PUT request builder.
    pub fn put(&self, uri: impl AsRef<str>) -> TestClientRequest<'_> {
        TestClientRequest::new(self, TestRequest::put(uri))
    }

    /// Creates a PATCH request builder.
    pub fn patch(&self, uri: impl AsRef<str>) -> TestClientRequest<'_> {
        TestClientRequest::new(self, TestRequest::patch(uri))
    }

    /// Creates a DELETE request builder.
    pub fn delete(&self, uri: impl AsRef<str>) -> TestClientRequest<'_> {
        TestClientRequest::new(self, TestRequest::delete(uri))
    }

    /// Creates an OPTIONS request builder.
    pub fn options(&self, uri: impl AsRef<str>) -> TestClientRequest<'_> {
        TestClientRequest::new(self, TestRequest::options(uri))
    }

    /// Creates a HEAD request builder.
    pub fn head(&self, uri: impl AsRef<str>) -> TestClientRequest<'_> {
        TestClientRequest::new(self, TestRequest::head(uri))
    }

    /// Creates a request builder with a custom method.
    pub fn request(&self, method: Method, uri: impl AsRef<str>) -> TestClientRequest<'_> {
        TestClientRequest::new(self, TestRequestBuilder::new(method, uri))
    }

    /// Sends a test request and returns the response.
    async fn send_internal(&self, request: TestRequest) -> Result<TestResponse, TestError> {
        let handler = Arc::clone(&self.handler);
        let ctx = MiddlewareContext::new();
        let response = (handler)(ctx, request).await;
        TestResponse::from_http(response).await
    }
}

/// A request builder bound to a test client.
pub struct TestClientRequest<'a> {
    client: &'a TestClient,
    builder: TestRequestBuilder,
}

impl<'a> TestClientRequest<'a> {
    fn new(client: &'a TestClient, builder: TestRequestBuilder) -> Self {
        // Apply default headers
        let mut builder = builder;
        for (name, value) in &client.default_headers {
            builder = builder.header(name, value);
        }
        Self { client, builder }
    }

    /// Sets a header on the request.
    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.builder = self.builder.header(name, value);
        self
    }

    /// Sets the Content-Type header.
    pub fn content_type(mut self, content_type: impl AsRef<str>) -> Self {
        self.builder = self.builder.content_type(content_type);
        self
    }

    /// Sets the Authorization header with a Bearer token.
    pub fn bearer_token(mut self, token: impl AsRef<str>) -> Self {
        self.builder = self.builder.bearer_token(token);
        self
    }

    /// Sets the raw request body.
    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.builder = self.builder.body(body);
        self
    }

    /// Sets the request body as JSON.
    pub fn json<T: serde::Serialize>(mut self, value: &T) -> Self {
        self.builder = self.builder.json(value);
        self
    }

    /// Sends the request and returns the response.
    pub async fn send(self) -> TestResponse {
        let request = self.builder.build().expect("valid request");
        self.client
            .send_internal(request)
            .await
            .expect("request should succeed")
    }

    /// Sends the request and returns a Result.
    pub async fn try_send(self) -> Result<TestResponse, TestError> {
        let request = self.builder.build()?;
        self.client.send_internal(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_echo_client() {
        let client = TestClient::echo();
        let response = client.get("/test/path").send().await;

        assert_eq!(response.status_code(), 200);
        let json: serde_json::Value = response.json().unwrap();
        assert_eq!(json["method"], "GET");
        assert_eq!(json["path"], "/test/path");
    }

    #[tokio::test]
    async fn test_fixed_response() {
        let client = TestClient::fixed_response(StatusCode::CREATED, "created");
        let response = client.post("/items").send().await;

        assert_eq!(response.status_code(), 201);
        assert_eq!(response.text().unwrap(), "created");
    }

    #[tokio::test]
    async fn test_custom_handler() {
        let client = TestClient::new(|_ctx, req| async move {
            let body = if req.method == Method::GET {
                "GET response"
            } else {
                "Other response"
            };
            http::Response::builder()
                .status(StatusCode::OK)
                .body(Full::new(Bytes::from(body)))
                .unwrap()
        });

        let get_response = client.get("/test").send().await;
        assert_eq!(get_response.text().unwrap(), "GET response");

        let post_response = client.post("/test").send().await;
        assert_eq!(post_response.text().unwrap(), "Other response");
    }

    #[tokio::test]
    async fn test_headers() {
        let client = TestClient::new(|_ctx, req| async move {
            let auth = req
                .headers
                .get("Authorization")
                .map(|v| v.to_str().unwrap_or("none"))
                .unwrap_or("none");
            http::Response::builder()
                .status(StatusCode::OK)
                .body(Full::new(Bytes::from(auth.to_string())))
                .unwrap()
        });

        let response = client
            .get("/test")
            .bearer_token("my_token")
            .send()
            .await;

        assert_eq!(response.text().unwrap(), "Bearer my_token");
    }

    #[tokio::test]
    async fn test_json_body() {
        let client = TestClient::new(|_ctx, req| async move {
            // Echo back content-type
            let content_type = req
                .headers
                .get("Content-Type")
                .map(|v| v.to_str().unwrap_or("none"))
                .unwrap_or("none");
            http::Response::builder()
                .status(StatusCode::OK)
                .body(Full::new(Bytes::from(content_type.to_string())))
                .unwrap()
        });

        let response = client
            .post("/users")
            .json(&json!({"name": "Alice"}))
            .send()
            .await;

        assert_eq!(response.text().unwrap(), "application/json");
    }

    #[tokio::test]
    async fn test_default_headers() {
        let client = TestClient::new(|_ctx, req| async move {
            let custom = req
                .headers
                .get("X-Custom")
                .map(|v| v.to_str().unwrap_or("none"))
                .unwrap_or("none");
            http::Response::builder()
                .status(StatusCode::OK)
                .body(Full::new(Bytes::from(custom.to_string())))
                .unwrap()
        })
        .with_default_header("X-Custom", "default-value");

        let response = client.get("/test").send().await;
        assert_eq!(response.text().unwrap(), "default-value");
    }

    #[tokio::test]
    async fn test_all_methods() {
        let client = TestClient::echo();

        let get = client.get("/test").send().await;
        assert!(get.json_value().unwrap()["method"] == "GET");

        let post = client.post("/test").send().await;
        assert!(post.json_value().unwrap()["method"] == "POST");

        let put = client.put("/test").send().await;
        assert!(put.json_value().unwrap()["method"] == "PUT");

        let patch = client.patch("/test").send().await;
        assert!(patch.json_value().unwrap()["method"] == "PATCH");

        let delete = client.delete("/test").send().await;
        assert!(delete.json_value().unwrap()["method"] == "DELETE");

        let options = client.options("/test").send().await;
        assert!(options.json_value().unwrap()["method"] == "OPTIONS");

        let head = client.head("/test").send().await;
        assert!(head.json_value().unwrap()["method"] == "HEAD");
    }
}
