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

    #[test]
    fn test_request_context_creation() {
        let ctx = PyRequestContext::test("testOp");
        assert_eq!(ctx.operation_id, "testOp");
        assert_eq!(ctx.method, "GET");
    }

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

        assert!(identity.has_role("admin"));
        assert!(identity.has_role("user"));
        assert!(!identity.has_role("superadmin"));
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

        assert!(identity.has_permission("users:read"));
        assert!(!identity.has_permission("admin:*"));
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
        assert!(expired.is_expired());

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
        assert!(!valid.is_expired());
    }
}
