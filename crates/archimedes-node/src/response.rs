//! Response types for handlers.

use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP Response returned by handlers.
///
/// ## Example
///
/// ```typescript
/// // Return JSON response
/// return Response.json({ users: [] });
///
/// // Return with status code
/// return Response.status(201).json({ id: newUser.id });
///
/// // Return error response
/// return Response.notFound({ error: 'User not found' });
/// ```
#[napi]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// HTTP status code
    status_code: u16,

    /// Response headers
    headers: HashMap<String, String>,

    /// Response body (JSON string)
    body: Option<String>,

    /// Content type
    content_type: String,
}

#[napi]
impl Response {
    /// Create a new response with status code 200.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            status_code: 200,
            headers: HashMap::new(),
            body: None,
            content_type: "application/json".to_string(),
        }
    }

    /// Create a response with a specific status code.
    #[napi(factory)]
    pub fn status(code: u16) -> Self {
        Self {
            status_code: code,
            headers: HashMap::new(),
            body: None,
            content_type: "application/json".to_string(),
        }
    }

    /// Create a 200 OK response with JSON body.
    #[napi(factory)]
    pub fn ok(body: serde_json::Value) -> Self {
        Self {
            status_code: 200,
            headers: HashMap::new(),
            body: Some(serde_json::to_string(&body).unwrap_or_default()),
            content_type: "application/json".to_string(),
        }
    }

    /// Create a 201 Created response with JSON body.
    #[napi(factory)]
    pub fn created(body: serde_json::Value) -> Self {
        Self {
            status_code: 201,
            headers: HashMap::new(),
            body: Some(serde_json::to_string(&body).unwrap_or_default()),
            content_type: "application/json".to_string(),
        }
    }

    /// Create a 204 No Content response.
    #[napi(factory)]
    pub fn no_content() -> Self {
        Self {
            status_code: 204,
            headers: HashMap::new(),
            body: None,
            content_type: "application/json".to_string(),
        }
    }

    /// Create a 400 Bad Request response with JSON body.
    #[napi(factory)]
    pub fn bad_request(body: serde_json::Value) -> Self {
        Self {
            status_code: 400,
            headers: HashMap::new(),
            body: Some(serde_json::to_string(&body).unwrap_or_default()),
            content_type: "application/json".to_string(),
        }
    }

    /// Create a 401 Unauthorized response.
    #[napi(factory)]
    pub fn unauthorized(body: serde_json::Value) -> Self {
        Self {
            status_code: 401,
            headers: HashMap::new(),
            body: Some(serde_json::to_string(&body).unwrap_or_default()),
            content_type: "application/json".to_string(),
        }
    }

    /// Create a 403 Forbidden response.
    #[napi(factory)]
    pub fn forbidden(body: serde_json::Value) -> Self {
        Self {
            status_code: 403,
            headers: HashMap::new(),
            body: Some(serde_json::to_string(&body).unwrap_or_default()),
            content_type: "application/json".to_string(),
        }
    }

    /// Create a 404 Not Found response.
    #[napi(factory)]
    pub fn not_found(body: serde_json::Value) -> Self {
        Self {
            status_code: 404,
            headers: HashMap::new(),
            body: Some(serde_json::to_string(&body).unwrap_or_default()),
            content_type: "application/json".to_string(),
        }
    }

    /// Create a 500 Internal Server Error response.
    #[napi(factory)]
    pub fn internal_error(body: serde_json::Value) -> Self {
        Self {
            status_code: 500,
            headers: HashMap::new(),
            body: Some(serde_json::to_string(&body).unwrap_or_default()),
            content_type: "application/json".to_string(),
        }
    }

    /// Create a redirect response (302 Found).
    #[napi(factory)]
    pub fn redirect(location: String) -> Self {
        let mut headers = HashMap::new();
        headers.insert("location".to_string(), location);
        Self {
            status_code: 302,
            headers,
            body: None,
            content_type: "text/plain".to_string(),
        }
    }

    /// Create a permanent redirect response (301 Moved Permanently).
    #[napi(factory)]
    pub fn permanent_redirect(location: String) -> Self {
        let mut headers = HashMap::new();
        headers.insert("location".to_string(), location);
        Self {
            status_code: 301,
            headers,
            body: None,
            content_type: "text/plain".to_string(),
        }
    }

    /// Create a See Other redirect (303) - typically used after POST.
    #[napi(factory)]
    pub fn see_other(location: String) -> Self {
        let mut headers = HashMap::new();
        headers.insert("location".to_string(), location);
        Self {
            status_code: 303,
            headers,
            body: None,
            content_type: "text/plain".to_string(),
        }
    }

    /// Create a temporary redirect (307) - preserves HTTP method.
    #[napi(factory)]
    pub fn temporary_redirect(location: String) -> Self {
        let mut headers = HashMap::new();
        headers.insert("location".to_string(), location);
        Self {
            status_code: 307,
            headers,
            body: None,
            content_type: "text/plain".to_string(),
        }
    }

    /// Create a JSON response with the given body.
    #[napi]
    pub fn json(&self, body: serde_json::Value) -> Response {
        Response {
            status_code: self.status_code,
            headers: self.headers.clone(),
            body: Some(serde_json::to_string(&body).unwrap_or_default()),
            content_type: "application/json".to_string(),
        }
    }

    /// Set a response header.
    #[napi]
    pub fn set_header(&mut self, key: String, value: String) -> &Self {
        self.headers.insert(key.to_lowercase(), value);
        self
    }

    /// Set multiple headers.
    #[napi]
    pub fn set_headers(&mut self, headers: HashMap<String, String>) -> &Self {
        for (k, v) in headers {
            self.headers.insert(k.to_lowercase(), v);
        }
        self
    }

    /// Set the content type.
    #[napi]
    pub fn set_content_type(&mut self, content_type: String) -> &Self {
        self.content_type = content_type;
        self
    }

    /// Set the response body as a string.
    #[napi]
    pub fn set_body(&mut self, body: String) -> &Self {
        self.body = Some(body);
        self
    }

    /// Get the status code.
    #[napi(getter)]
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Get the headers.
    #[napi(getter)]
    pub fn headers(&self) -> HashMap<String, String> {
        self.headers.clone()
    }

    /// Get the body.
    #[napi(getter)]
    pub fn body(&self) -> Option<String> {
        self.body.clone()
    }

    /// Get the content type.
    #[napi(getter)]
    pub fn content_type(&self) -> String {
        self.content_type.clone()
    }

    /// Check if this is a success response (2xx).
    #[napi]
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Check if this is a client error (4xx).
    #[napi]
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status_code)
    }

    /// Check if this is a server error (5xx).
    #[napi]
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status_code)
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_response_new() {
        let resp = Response::new();
        assert_eq!(resp.status_code(), 200);
        assert_eq!(resp.content_type(), "application/json");
        assert!(resp.body().is_none());
    }

    #[test]
    fn test_response_status() {
        let resp = Response::status(201);
        assert_eq!(resp.status_code(), 201);
    }

    #[test]
    fn test_response_ok() {
        let resp = Response::ok(json!({"success": true}));
        assert_eq!(resp.status_code(), 200);
        assert!(resp.body().unwrap().contains("success"));
    }

    #[test]
    fn test_response_created() {
        let resp = Response::created(json!({"id": "123"}));
        assert_eq!(resp.status_code(), 201);
        assert!(resp.body().unwrap().contains("123"));
    }

    #[test]
    fn test_response_no_content() {
        let resp = Response::no_content();
        assert_eq!(resp.status_code(), 204);
        assert!(resp.body().is_none());
    }

    #[test]
    fn test_response_bad_request() {
        let resp = Response::bad_request(json!({"error": "Invalid input"}));
        assert_eq!(resp.status_code(), 400);
        assert!(resp.is_client_error());
    }

    #[test]
    fn test_response_not_found() {
        let resp = Response::not_found(json!({"error": "Not found"}));
        assert_eq!(resp.status_code(), 404);
        assert!(resp.is_client_error());
    }

    #[test]
    fn test_response_internal_error() {
        let resp = Response::internal_error(json!({"error": "Server error"}));
        assert_eq!(resp.status_code(), 500);
        assert!(resp.is_server_error());
    }

    #[test]
    fn test_response_headers() {
        let mut resp = Response::new();
        resp.set_header("x-custom".to_string(), "value".to_string());
        assert_eq!(resp.headers().get("x-custom"), Some(&"value".to_string()));
    }

    #[test]
    fn test_response_json_method() {
        let resp = Response::status(200).json(json!({"data": "test"}));
        assert_eq!(resp.status_code(), 200);
        assert!(resp.body().unwrap().contains("test"));
    }

    #[test]
    fn test_response_status_checks() {
        assert!(Response::ok(json!({})).is_success());
        assert!(Response::created(json!({})).is_success());
        assert!(Response::no_content().is_success());
        assert!(Response::bad_request(json!({})).is_client_error());
        assert!(Response::not_found(json!({})).is_client_error());
        assert!(Response::internal_error(json!({})).is_server_error());
    }
}
