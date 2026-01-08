//! Request context and identity types.

use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the caller's identity extracted from authentication.
///
/// ## Example
///
/// ```typescript
/// const identity = request.identity;
/// if (identity) {
///   console.log(`User: ${identity.subject}`);
///   console.log(`Roles: ${identity.roles}`);
/// }
/// ```
#[napi(object)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Identity {
    /// Subject identifier (e.g., user ID, service account)
    pub subject: Option<String>,

    /// Issuer of the identity token
    pub issuer: Option<String>,

    /// Audience the token was issued for
    pub audience: Option<String>,

    /// Token expiration timestamp (Unix epoch seconds)
    pub expires_at: Option<i64>,

    /// Token issued-at timestamp (Unix epoch seconds)
    pub issued_at: Option<i64>,

    /// Roles assigned to the identity
    pub roles: Option<Vec<String>>,

    /// Scopes/permissions granted
    pub scopes: Option<Vec<String>>,

    /// Additional claims from the token
    pub claims: Option<HashMap<String, String>>,
}

/// Identity builder for programmatic construction.
#[napi]
#[derive(Debug, Clone, Default)]
pub struct IdentityBuilder {
    identity: Identity,
}

#[napi]
impl IdentityBuilder {
    /// Create a new identity builder.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the subject.
    #[napi]
    pub fn subject(&mut self, subject: String) -> &Self {
        self.identity.subject = Some(subject);
        self
    }

    /// Set the issuer.
    #[napi]
    pub fn issuer(&mut self, issuer: String) -> &Self {
        self.identity.issuer = Some(issuer);
        self
    }

    /// Set the audience.
    #[napi]
    pub fn audience(&mut self, audience: String) -> &Self {
        self.identity.audience = Some(audience);
        self
    }

    /// Set the expiration timestamp.
    #[napi]
    pub fn expires_at(&mut self, expires_at: i64) -> &Self {
        self.identity.expires_at = Some(expires_at);
        self
    }

    /// Add a role.
    #[napi]
    pub fn role(&mut self, role: String) -> &Self {
        let roles = self.identity.roles.get_or_insert_with(Vec::new);
        roles.push(role);
        self
    }

    /// Set all roles.
    #[napi]
    pub fn roles(&mut self, roles: Vec<String>) -> &Self {
        self.identity.roles = Some(roles);
        self
    }

    /// Add a scope.
    #[napi]
    pub fn scope(&mut self, scope: String) -> &Self {
        let scopes = self.identity.scopes.get_or_insert_with(Vec::new);
        scopes.push(scope);
        self
    }

    /// Set all scopes.
    #[napi]
    pub fn scopes(&mut self, scopes: Vec<String>) -> &Self {
        self.identity.scopes = Some(scopes);
        self
    }

    /// Add a claim.
    #[napi]
    pub fn claim(&mut self, key: String, value: String) -> &Self {
        let claims = self.identity.claims.get_or_insert_with(HashMap::new);
        claims.insert(key, value);
        self
    }

    /// Build the identity.
    #[napi]
    pub fn build(&self) -> Identity {
        self.identity.clone()
    }
}

/// Request context containing all request information.
///
/// Passed to handlers with path params, query params, headers, body, and identity.
///
/// ## Example
///
/// ```typescript
/// app.operation('getUser', async (ctx: RequestContext): Promise<Response> => {
///   const userId = ctx.pathParams['userId'];
///   const verbose = ctx.queryParams['verbose'] === 'true';
///   const authHeader = ctx.headers['authorization'];
///   const identity = ctx.identity;
///   
///   // Handler logic...
/// });
/// ```
#[napi(object)]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RequestContext {
    /// Unique request ID for tracing
    pub request_id: String,

    /// HTTP method (GET, POST, etc.)
    pub method: String,

    /// Request path (e.g., "/users/123")
    pub path: String,

    /// Resolved operation ID from the contract
    pub operation_id: Option<String>,

    /// Path parameters extracted from the URL
    pub path_params: HashMap<String, String>,

    /// Query parameters from the URL
    pub query_params: HashMap<String, String>,

    /// Request headers (lowercase keys)
    pub headers: HashMap<String, String>,

    /// Request body as JSON string
    pub body: Option<String>,

    /// Parsed body as key-value pairs (for simple objects)
    pub body_json: Option<HashMap<String, serde_json::Value>>,

    /// Caller identity (if authenticated)
    pub identity: Option<Identity>,

    /// Client IP address
    pub client_ip: Option<String>,

    /// Content type of the request body
    pub content_type: Option<String>,

    /// Accept header value
    pub accept: Option<String>,

    /// Custom context data set by middleware
    pub custom: HashMap<String, String>,
}

/// Request context builder for programmatic construction.
#[napi]
#[derive(Debug, Clone, Default)]
pub struct RequestContextBuilder {
    ctx: RequestContext,
}

#[napi]
impl RequestContextBuilder {
    /// Create a new request context builder.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            ctx: RequestContext {
                request_id: uuid::Uuid::new_v4().to_string(),
                ..Default::default()
            },
        }
    }

    /// Set the request ID.
    #[napi]
    pub fn request_id(&mut self, id: String) -> &Self {
        self.ctx.request_id = id;
        self
    }

    /// Set the HTTP method.
    #[napi]
    pub fn method(&mut self, method: String) -> &Self {
        self.ctx.method = method;
        self
    }

    /// Set the request path.
    #[napi]
    pub fn path(&mut self, path: String) -> &Self {
        self.ctx.path = path;
        self
    }

    /// Set the operation ID.
    #[napi]
    pub fn operation_id(&mut self, id: String) -> &Self {
        self.ctx.operation_id = Some(id);
        self
    }

    /// Add a path parameter.
    #[napi]
    pub fn path_param(&mut self, key: String, value: String) -> &Self {
        self.ctx.path_params.insert(key, value);
        self
    }

    /// Add a query parameter.
    #[napi]
    pub fn query_param(&mut self, key: String, value: String) -> &Self {
        self.ctx.query_params.insert(key, value);
        self
    }

    /// Add a header.
    #[napi]
    pub fn header(&mut self, key: String, value: String) -> &Self {
        self.ctx.headers.insert(key.to_lowercase(), value);
        self
    }

    /// Set the request body.
    #[napi]
    pub fn body(&mut self, body: String) -> &Self {
        self.ctx.body = Some(body);
        self
    }

    /// Set the identity.
    #[napi]
    pub fn identity(&mut self, identity: Identity) -> &Self {
        self.ctx.identity = Some(identity);
        self
    }

    /// Set the client IP.
    #[napi]
    pub fn client_ip(&mut self, ip: String) -> &Self {
        self.ctx.client_ip = Some(ip);
        self
    }

    /// Add custom context data.
    #[napi]
    pub fn custom(&mut self, key: String, value: String) -> &Self {
        self.ctx.custom.insert(key, value);
        self
    }

    /// Build the request context.
    #[napi]
    pub fn build(&self) -> RequestContext {
        self.ctx.clone()
    }
}

/// Create a mock request context for testing.
#[napi]
pub fn mock_request_context() -> RequestContext {
    RequestContext {
        request_id: "test-request-id".to_string(),
        method: "GET".to_string(),
        path: "/test".to_string(),
        operation_id: Some("testOperation".to_string()),
        path_params: HashMap::new(),
        query_params: HashMap::new(),
        headers: HashMap::new(),
        body: None,
        body_json: None,
        identity: None,
        client_ip: Some("127.0.0.1".to_string()),
        content_type: None,
        accept: None,
        custom: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_builder() {
        let mut builder = IdentityBuilder::new();
        builder.subject("user123".to_string());
        builder.issuer("https://auth.example.com".to_string());
        builder.role("admin".to_string());
        builder.role("user".to_string());
        builder.scope("read".to_string());
        builder.scope("write".to_string());
        builder.claim("tenant".to_string(), "acme".to_string());
        let identity = builder.build();

        assert_eq!(identity.subject, Some("user123".to_string()));
        assert_eq!(
            identity.issuer,
            Some("https://auth.example.com".to_string())
        );
        assert_eq!(
            identity.roles,
            Some(vec!["admin".to_string(), "user".to_string()])
        );
        assert_eq!(
            identity.scopes,
            Some(vec!["read".to_string(), "write".to_string()])
        );
        assert_eq!(
            identity.claims.as_ref().and_then(|c| c.get("tenant")),
            Some(&"acme".to_string())
        );
    }

    #[test]
    fn test_request_context_builder() {
        let mut builder = RequestContextBuilder::new();
        builder.method("POST".to_string());
        builder.path("/users".to_string());
        builder.operation_id("createUser".to_string());
        builder.header("content-type".to_string(), "application/json".to_string());
        builder.body(r#"{"name":"test"}"#.to_string());
        builder.client_ip("192.168.1.1".to_string());
        builder.custom("trace_id".to_string(), "abc123".to_string());
        let ctx = builder.build();

        assert_eq!(ctx.method, "POST");
        assert_eq!(ctx.path, "/users");
        assert_eq!(ctx.operation_id, Some("createUser".to_string()));
        assert_eq!(
            ctx.headers.get("content-type"),
            Some(&"application/json".to_string())
        );
        assert_eq!(ctx.body, Some(r#"{"name":"test"}"#.to_string()));
        assert_eq!(ctx.custom.get("trace_id"), Some(&"abc123".to_string()));
    }

    #[test]
    fn test_mock_request_context() {
        let ctx = mock_request_context();
        assert_eq!(ctx.request_id, "test-request-id");
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/test");
    }

    #[test]
    fn test_request_context_path_params() {
        let mut builder = RequestContextBuilder::new();
        builder.path_param("userId".to_string(), "123".to_string());
        builder.path_param("orderId".to_string(), "456".to_string());
        let ctx = builder.build();

        assert_eq!(ctx.path_params.get("userId"), Some(&"123".to_string()));
        assert_eq!(ctx.path_params.get("orderId"), Some(&"456".to_string()));
    }

    #[test]
    fn test_identity_with_timestamps() {
        // Use a fixed timestamp for testing
        let now: i64 = 1700000000; // 2023-11-14T22:13:20Z
        let expires = now + 3600;

        let mut builder = IdentityBuilder::new();
        builder.subject("user123".to_string());
        builder.expires_at(expires);
        let identity = builder.build();

        assert_eq!(identity.expires_at, Some(expires));
    }
}
