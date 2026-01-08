//! Python request context types for Archimedes

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

/// Request context available to Python handlers
///
/// This provides access to request metadata, path parameters,
/// query parameters, headers, and identity information.
///
/// Example:
///     ```python
///     @app.handler("getUser")
///     def get_user(ctx):
///         user_id = ctx.path_params["userId"]
///         auth_header = ctx.headers.get("authorization")
///         trace_id = ctx.trace_id
///         return {"id": user_id}
///     ```
#[pyclass(name = "RequestContext")]
#[derive(Clone, Debug)]
pub struct PyRequestContext {
    /// Operation ID from the contract
    #[pyo3(get)]
    pub operation_id: String,

    /// HTTP method (GET, POST, etc.)
    #[pyo3(get)]
    pub method: String,

    /// Request path
    #[pyo3(get)]
    pub path: String,

    /// Path parameters extracted from the URL
    path_params: HashMap<String, String>,

    /// Query parameters
    query_params: HashMap<String, Vec<String>>,

    /// Request headers
    headers: HashMap<String, String>,

    /// Trace ID for distributed tracing
    #[pyo3(get)]
    pub trace_id: String,

    /// Span ID for distributed tracing
    #[pyo3(get)]
    pub span_id: String,

    /// Identity information (if authenticated)
    identity: Option<PyIdentity>,
}

#[pymethods]
impl PyRequestContext {
    /// Get path parameters as a dictionary
    #[getter]
    fn path_params(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.path_params {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Get query parameters as a dictionary
    ///
    /// Values are lists since query params can have multiple values.
    #[getter]
    fn query_params(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.query_params {
            let list = PyList::new(py, v)?;
            dict.set_item(k, list)?;
        }
        Ok(dict.into())
    }

    /// Get a single query parameter value
    ///
    /// Returns the first value if multiple exist, or None.
    fn query(&self, name: &str) -> Option<String> {
        self.query_params.get(name).and_then(|v| v.first().cloned())
    }

    /// Get all values for a query parameter
    fn query_all(&self, name: &str) -> Vec<String> {
        self.query_params.get(name).cloned().unwrap_or_default()
    }

    /// Get headers as a dictionary
    #[getter]
    fn headers(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.headers {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Get a single header value
    fn header(&self, name: &str) -> Option<String> {
        // Headers are case-insensitive
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.clone())
    }

    /// Get the identity information
    #[getter]
    fn identity(&self) -> Option<PyIdentity> {
        self.identity.clone()
    }

    /// Check if the request is authenticated
    fn is_authenticated(&self) -> bool {
        self.identity.is_some()
    }

    /// Get the authenticated subject (user ID)
    fn subject(&self) -> Option<String> {
        self.identity.as_ref().map(|i| i.subject.clone())
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "RequestContext(operation_id={:?}, method={:?}, path={:?})",
            self.operation_id, self.method, self.path
        )
    }

    /// Convert to dictionary
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("operation_id", &self.operation_id)?;
        dict.set_item("method", &self.method)?;
        dict.set_item("path", &self.path)?;
        dict.set_item("path_params", self.path_params(py)?)?;
        dict.set_item("query_params", self.query_params(py)?)?;
        dict.set_item("headers", self.headers(py)?)?;
        dict.set_item("trace_id", &self.trace_id)?;
        dict.set_item("span_id", &self.span_id)?;
        if let Some(ref identity) = self.identity {
            dict.set_item("identity", identity.to_dict(py)?)?;
        }
        Ok(dict.into())
    }
}

impl PyRequestContext {
    /// Create a new request context
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation_id: String,
        method: String,
        path: String,
        path_params: HashMap<String, String>,
        query_params: HashMap<String, Vec<String>>,
        headers: HashMap<String, String>,
        trace_id: String,
        span_id: String,
        identity: Option<PyIdentity>,
    ) -> Self {
        Self {
            operation_id,
            method,
            path,
            path_params,
            query_params,
            headers,
            trace_id,
            span_id,
            identity,
        }
    }

    /// Create a test context for unit testing
    pub fn test(operation_id: &str) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            path_params: HashMap::new(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            trace_id: "test-trace-id".to_string(),
            span_id: "test-span-id".to_string(),
            identity: None,
        }
    }

    /// Check if the request is authenticated (Rust-native helper)
    pub fn is_authenticated_rs(&self) -> bool {
        self.identity.is_some()
    }

    /// Get the authenticated subject/user ID (Rust-native helper)
    pub fn subject_rs(&self) -> Option<String> {
        self.identity.as_ref().map(|i| i.subject.clone())
    }

    /// Get a reference to the identity (Rust-native helper)
    pub fn identity_ref(&self) -> Option<&PyIdentity> {
        self.identity.as_ref()
    }
}

/// Identity information for authenticated requests
#[pyclass(name = "Identity")]
#[derive(Clone, Debug)]
pub struct PyIdentity {
    /// Subject (user ID or service ID)
    #[pyo3(get)]
    pub subject: String,

    /// Issuer of the identity token
    #[pyo3(get)]
    pub issuer: Option<String>,

    /// Audience
    #[pyo3(get)]
    pub audience: Option<String>,

    /// Expiration time (Unix timestamp)
    #[pyo3(get)]
    pub expires_at: Option<i64>,

    /// Claims from the token
    claims: HashMap<String, serde_json::Value>,

    /// Roles
    roles: Vec<String>,

    /// Permissions
    permissions: Vec<String>,
}

#[pymethods]
impl PyIdentity {
    /// Get a claim value by name
    fn claim(&self, name: &str, py: Python<'_>) -> PyResult<Option<PyObject>> {
        match self.claims.get(name) {
            Some(value) => json_to_python(py, value).map(Some),
            None => Ok(None),
        }
    }

    /// Get all claims as a dictionary
    #[getter]
    fn claims(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.claims {
            dict.set_item(k, json_to_python(py, v)?)?;
        }
        Ok(dict.into())
    }

    /// Get roles
    #[getter]
    fn roles(&self) -> Vec<String> {
        self.roles.clone()
    }

    /// Check if identity has a role
    fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Get permissions
    #[getter]
    fn permissions(&self) -> Vec<String> {
        self.permissions.clone()
    }

    /// Check if identity has a permission
    fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    /// Check if the token is expired
    fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            now > exp
        } else {
            false
        }
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("Identity(subject={:?})", self.subject)
    }

    /// Convert to dictionary
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("subject", &self.subject)?;
        dict.set_item("issuer", &self.issuer)?;
        dict.set_item("audience", &self.audience)?;
        dict.set_item("expires_at", self.expires_at)?;
        dict.set_item("claims", self.claims(py)?)?;
        dict.set_item("roles", &self.roles)?;
        dict.set_item("permissions", &self.permissions)?;
        Ok(dict.into())
    }
}

impl PyIdentity {
    /// Create a new identity
    pub fn new(
        subject: String,
        issuer: Option<String>,
        audience: Option<String>,
        expires_at: Option<i64>,
        claims: HashMap<String, serde_json::Value>,
        roles: Vec<String>,
        permissions: Vec<String>,
    ) -> Self {
        Self {
            subject,
            issuer,
            audience,
            expires_at,
            claims,
            roles,
            permissions,
        }
    }

    /// Check if identity has a claim (Rust-native helper for testing)
    pub fn has_claim(&self, name: &str) -> bool {
        self.claims.contains_key(name)
    }

    /// Get the claim value (Rust-native helper for testing)
    pub fn claim_value(&self, name: &str) -> Option<&serde_json::Value> {
        self.claims.get(name)
    }

    /// Check if identity has a role (Rust-native helper for testing)
    pub fn has_role_rs(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if identity has a permission (Rust-native helper for testing)
    pub fn has_permission_rs(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    /// Check if the token is expired (Rust-native helper for testing)
    pub fn is_expired_rs(&self) -> bool {
        if let Some(exp) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            now > exp
        } else {
            false
        }
    }

    /// Get the subject (Rust-native helper for testing)
    pub fn subject_rs(&self) -> &str {
        &self.subject
    }

    /// Get roles (Rust-native helper for testing)
    pub fn roles_rs(&self) -> &[String] {
        &self.roles
    }

    /// Get permissions (Rust-native helper for testing)
    pub fn permissions_rs(&self) -> &[String] {
        &self.permissions
    }
}

/// Convert serde_json::Value to Python object
fn json_to_python(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    Ok(match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.into_pyobject(py)?.to_owned().into_any().unbind(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py)?.to_owned().into_any().unbind()
            } else if let Some(f) = n.as_f64() {
                f.into_pyobject(py)?.to_owned().into_any().unbind()
            } else {
                py.None()
            }
        }
        serde_json::Value::String(s) => s.into_pyobject(py)?.to_owned().into_any().unbind(),
        serde_json::Value::Array(arr) => {
            let items: Vec<PyObject> = arr
                .iter()
                .map(|v| json_to_python(py, v))
                .collect::<PyResult<_>>()?;
            let list = PyList::new(py, &items)?;
            list.into()
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_python(py, v)?)?;
            }
            dict.into()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // RequestContext Tests
    // =========================================================================

    #[test]
    fn test_request_context_creation() {
        let ctx = PyRequestContext::test("testOp");
        assert_eq!(ctx.operation_id, "testOp");
        assert_eq!(ctx.method, "GET");
    }

    #[test]
    fn test_request_context_with_path_params() {
        let mut path_params = HashMap::new();
        path_params.insert("userId".to_string(), "123".to_string());
        path_params.insert("orgId".to_string(), "org-456".to_string());

        let ctx = PyRequestContext::new(
            "getUser".to_string(),
            "GET".to_string(),
            "/orgs/org-456/users/123".to_string(),
            path_params,
            HashMap::new(),
            HashMap::new(),
            "trace-123".to_string(),
            "span-456".to_string(),
            None,
        );

        assert_eq!(ctx.operation_id, "getUser");
        assert_eq!(ctx.path, "/orgs/org-456/users/123");
    }

    #[test]
    fn test_request_context_with_query_params() {
        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), vec!["1".to_string()]);
        query_params.insert("tags".to_string(), vec!["a".to_string(), "b".to_string()]);

        let ctx = PyRequestContext::new(
            "listUsers".to_string(),
            "GET".to_string(),
            "/users".to_string(),
            HashMap::new(),
            query_params,
            HashMap::new(),
            "trace-123".to_string(),
            "span-456".to_string(),
            None,
        );

        assert_eq!(ctx.query("page"), Some("1".to_string()));
        assert_eq!(ctx.query_all("tags"), vec!["a", "b"]);
        assert_eq!(ctx.query("nonexistent"), None);
    }

    #[test]
    fn test_request_context_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());

        let ctx = PyRequestContext::new(
            "createUser".to_string(),
            "POST".to_string(),
            "/users".to_string(),
            HashMap::new(),
            HashMap::new(),
            headers,
            "trace-123".to_string(),
            "span-456".to_string(),
            None,
        );

        // Headers are case-insensitive
        assert_eq!(
            ctx.header("content-type"),
            Some("application/json".to_string())
        );
        assert_eq!(
            ctx.header("AUTHORIZATION"),
            Some("Bearer token123".to_string())
        );
        assert_eq!(ctx.header("X-Missing"), None);
    }

    #[test]
    fn test_request_context_without_identity() {
        let ctx = PyRequestContext::test("testOp");

        assert!(!ctx.is_authenticated());
        assert_eq!(ctx.subject_rs(), None);
    }

    #[test]
    fn test_request_context_with_identity() {
        let identity = PyIdentity::new(
            "user-123".to_string(),
            Some("https://auth.example.com".to_string()),
            None,
            None,
            HashMap::new(),
            vec!["admin".to_string()],
            vec!["users:read".to_string()],
        );

        let ctx = PyRequestContext::new(
            "getUser".to_string(),
            "GET".to_string(),
            "/users/123".to_string(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            "trace-123".to_string(),
            "span-456".to_string(),
            Some(identity),
        );

        assert!(ctx.is_authenticated());
        assert_eq!(ctx.subject_rs(), Some("user-123".to_string()));
    }

    // =========================================================================
    // Identity/Authorization Tests
    // =========================================================================

    #[test]
    fn test_identity_has_role() {
        let identity = PyIdentity::new(
            "user-123".to_string(),
            Some("issuer".to_string()),
            None,
            None,
            HashMap::new(),
            vec!["admin".to_string(), "user".to_string()],
            vec!["read".to_string(), "write".to_string()],
        );

        assert!(identity.has_role_rs("admin"));
        assert!(identity.has_role_rs("user"));
        assert!(!identity.has_role_rs("superadmin"));
    }

    #[test]
    fn test_identity_has_permission() {
        let identity = PyIdentity::new(
            "user-123".to_string(),
            None,
            None,
            None,
            HashMap::new(),
            vec![],
            vec!["users:read".to_string(), "users:write".to_string()],
        );

        assert!(identity.has_permission_rs("users:read"));
        assert!(!identity.has_permission_rs("admin:*"));
    }

    #[test]
    fn test_identity_expiration() {
        // Expired token
        let expired = PyIdentity::new(
            "user".to_string(),
            None,
            None,
            Some(0), // Expired at epoch
            HashMap::new(),
            vec![],
            vec![],
        );
        assert!(expired.is_expired_rs());

        // Future expiration
        let future_exp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            + 3600; // 1 hour from now

        let valid = PyIdentity::new(
            "user".to_string(),
            None,
            None,
            Some(future_exp),
            HashMap::new(),
            vec![],
            vec![],
        );
        assert!(!valid.is_expired_rs());
    }

    #[test]
    fn test_identity_without_expiration() {
        let no_exp = PyIdentity::new(
            "user".to_string(),
            None,
            None,
            None, // No expiration set
            HashMap::new(),
            vec![],
            vec![],
        );
        // No expiration means not expired
        assert!(!no_exp.is_expired_rs());
    }

    #[test]
    fn test_identity_claims() {
        let mut claims = HashMap::new();
        claims.insert("tenant_id".to_string(), serde_json::json!("tenant-123"));
        claims.insert("email".to_string(), serde_json::json!("user@example.com"));

        let identity = PyIdentity::new(
            "user-123".to_string(),
            None,
            None,
            None,
            claims.clone(),
            vec![],
            vec![],
        );

        // Test claims are stored correctly (using internal access)
        assert!(identity.has_claim("tenant_id"));
        assert!(identity.has_claim("email"));
        assert!(!identity.has_claim("missing"));
    }

    #[test]
    fn test_identity_multiple_roles() {
        let identity = PyIdentity::new(
            "user".to_string(),
            None,
            None,
            None,
            HashMap::new(),
            vec![
                "admin".to_string(),
                "developer".to_string(),
                "viewer".to_string(),
            ],
            vec![],
        );

        assert!(identity.has_role_rs("admin"));
        assert!(identity.has_role_rs("developer"));
        assert!(identity.has_role_rs("viewer"));
        assert!(!identity.has_role_rs("owner"));
    }

    #[test]
    fn test_identity_multiple_permissions() {
        let identity = PyIdentity::new(
            "user".to_string(),
            None,
            None,
            None,
            HashMap::new(),
            vec![],
            vec![
                "users:read".to_string(),
                "users:write".to_string(),
                "users:delete".to_string(),
                "admin:audit".to_string(),
            ],
        );

        assert!(identity.has_permission_rs("users:read"));
        assert!(identity.has_permission_rs("users:write"));
        assert!(identity.has_permission_rs("users:delete"));
        assert!(identity.has_permission_rs("admin:audit"));
        assert!(!identity.has_permission_rs("admin:delete"));
    }

    // =========================================================================
    // Authorization Pattern Tests
    // =========================================================================

    #[test]
    fn test_authorization_require_role() {
        // Simulates: if not ctx.identity.has_role_rs("admin"): raise 403
        let admin_identity = PyIdentity::new(
            "admin-user".to_string(),
            None,
            None,
            None,
            HashMap::new(),
            vec!["admin".to_string()],
            vec![],
        );

        let user_identity = PyIdentity::new(
            "regular-user".to_string(),
            None,
            None,
            None,
            HashMap::new(),
            vec!["user".to_string()],
            vec![],
        );

        // Admin has required role
        assert!(admin_identity.has_role_rs("admin"));

        // Regular user doesn't have required role
        assert!(!user_identity.has_role_rs("admin"));
    }

    #[test]
    fn test_authorization_require_permission() {
        // Simulates permission-based authorization check
        let writer = PyIdentity::new(
            "writer".to_string(),
            None,
            None,
            None,
            HashMap::new(),
            vec![],
            vec!["posts:write".to_string(), "posts:read".to_string()],
        );

        let reader = PyIdentity::new(
            "reader".to_string(),
            None,
            None,
            None,
            HashMap::new(),
            vec![],
            vec!["posts:read".to_string()],
        );

        // Writer can write
        assert!(writer.has_permission_rs("posts:write"));

        // Reader cannot write
        assert!(!reader.has_permission_rs("posts:write"));

        // Both can read
        assert!(writer.has_permission_rs("posts:read"));
        assert!(reader.has_permission_rs("posts:read"));
    }

    #[test]
    fn test_authorization_token_expired() {
        // Expired token should fail authorization
        let expired_identity = PyIdentity::new(
            "user".to_string(),
            None,
            None,
            Some(0), // Expired
            HashMap::new(),
            vec!["admin".to_string()],
            vec!["all:*".to_string()],
        );

        // Even with all roles and permissions, expired token should be rejected
        assert!(expired_identity.is_expired_rs());
        // Application should check: if identity.is_expired_rs(): raise 401
    }

    #[test]
    fn test_authorization_anonymous_user() {
        // Context without identity should be treated as anonymous
        let ctx = PyRequestContext::test("getPublicResource");

        assert!(!ctx.is_authenticated());
        // Application should handle: if not ctx.is_authenticated(): raise 401
    }

    // =========================================================================
    // Context Serialization Tests
    // =========================================================================

    #[test]
    fn test_context_repr() {
        let ctx = PyRequestContext::new(
            "getUser".to_string(),
            "POST".to_string(),
            "/api/users".to_string(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            "trace".to_string(),
            "span".to_string(),
            None,
        );

        let repr = ctx.__repr__();
        assert!(repr.contains("getUser"));
        assert!(repr.contains("POST"));
        assert!(repr.contains("/api/users"));
    }
}
