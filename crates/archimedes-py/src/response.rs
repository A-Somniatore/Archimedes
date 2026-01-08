//! Python response types for Archimedes

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

/// HTTP response returned from handlers
///
/// # Example (Python)
///
/// ```python,ignore
/// from archimedes import Response
///
/// @app.handler("getUser")
/// def get_user(ctx):
///     return Response(
///         status=200,
///         body={"id": "123", "name": "John"},
///         headers={"X-Custom": "value"}
///     )
/// ```
#[pyclass(name = "Response")]
#[derive(Clone, Debug)]
pub struct PyResponse {
    /// HTTP status code
    #[pyo3(get, set)]
    pub status: u16,

    /// Response body (will be JSON serialized)
    body: Option<serde_json::Value>,

    /// Response headers
    headers: HashMap<String, String>,
}

#[pymethods]
impl PyResponse {
    /// Create a new response
    ///
    /// Args:
    ///     status: HTTP status code (default: 200)
    ///     body: Response body (dict, list, or primitive)
    ///     headers: Response headers
    #[new]
    #[pyo3(signature = (status = 200, body = None, headers = None))]
    fn new(
        py: Python<'_>,
        status: u16,
        body: Option<PyObject>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        let body_json = if let Some(body) = body {
            Some(python_to_json(py, body)?)
        } else {
            None
        };

        Ok(Self {
            status,
            body: body_json,
            headers: headers.unwrap_or_default(),
        })
    }

    /// Get response body as Python object
    #[getter]
    fn body(&self, py: Python<'_>) -> PyResult<PyObject> {
        match &self.body {
            Some(json) => json_to_python(py, json),
            None => Ok(py.None()),
        }
    }

    /// Set response body from Python object
    fn set_body(&mut self, py: Python<'_>, value: PyObject) -> PyResult<()> {
        self.body = Some(python_to_json(py, value)?);
        Ok(())
    }

    /// Get response headers as dictionary
    #[getter]
    fn headers(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.headers {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Set a header
    fn set_header(&mut self, name: String, value: String) {
        self.headers.insert(name, value);
    }

    /// Get a header
    fn get_header(&self, name: &str) -> Option<String> {
        self.headers.get(name).cloned()
    }

    /// Create an OK response (200)
    #[staticmethod]
    #[pyo3(signature = (body = None, headers = None))]
    fn ok(
        py: Python<'_>,
        body: Option<PyObject>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        Self::new(py, 200, body, headers)
    }

    /// Create a Created response (201)
    #[staticmethod]
    #[pyo3(signature = (body = None, headers = None))]
    fn created(
        py: Python<'_>,
        body: Option<PyObject>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        Self::new(py, 201, body, headers)
    }

    /// Create a No Content response (204)
    #[staticmethod]
    fn no_content() -> PyResult<Self> {
        Ok(Self {
            status: 204,
            body: None,
            headers: HashMap::new(),
        })
    }

    /// Create a Bad Request response (400)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn bad_request(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 400,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create an Unauthorized response (401)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn unauthorized(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 401,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create a Forbidden response (403)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn forbidden(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 403,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create a Not Found response (404)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn not_found(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 404,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create an Internal Server Error response (500)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn internal_error(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 500,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create a JSON response
    #[staticmethod]
    #[pyo3(signature = (body, status = 200))]
    fn json(py: Python<'_>, body: PyObject, status: u16) -> PyResult<Self> {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        Self::new(py, status, Some(body), Some(headers))
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("Response(status={})", self.status)
    }

    /// Convert to dictionary
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("status", self.status)?;
        dict.set_item("body", self.body(py)?)?;
        dict.set_item("headers", self.headers(py)?)?;
        Ok(dict.into())
    }
}

impl PyResponse {
    /// Get the body as JSON
    pub fn body_json(&self) -> Option<&serde_json::Value> {
        self.body.as_ref()
    }

    /// Get headers as reference
    pub fn headers_ref(&self) -> &HashMap<String, String> {
        &self.headers
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
            let list = pyo3::types::PyList::new(py, &items)?;
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

/// Convert Python object to serde_json::Value
fn python_to_json(py: Python<'_>, obj: PyObject) -> PyResult<serde_json::Value> {
    let obj_ref = obj.bind(py);

    if obj_ref.is_none() {
        return Ok(serde_json::Value::Null);
    }

    // Try bool first (before int since bool is subclass of int in Python)
    if let Ok(b) = obj_ref.extract::<bool>() {
        return Ok(serde_json::Value::Bool(b));
    }

    // Try integer
    if let Ok(i) = obj_ref.extract::<i64>() {
        return Ok(serde_json::Value::Number(i.into()));
    }

    // Try float
    if let Ok(f) = obj_ref.extract::<f64>() {
        return Ok(serde_json::json!(f));
    }

    // Try string
    if let Ok(s) = obj_ref.extract::<String>() {
        return Ok(serde_json::Value::String(s));
    }

    // Try list
    if let Ok(list) = obj_ref.downcast::<pyo3::types::PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(python_to_json(py, item.into())?);
        }
        return Ok(serde_json::Value::Array(arr));
    }

    // Try dict
    if let Ok(dict) = obj_ref.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key = k.extract::<String>()?;
            let value = python_to_json(py, v.into())?;
            map.insert(key, value);
        }
        return Ok(serde_json::Value::Object(map));
    }

    // Try tuple (convert to array)
    if let Ok(tuple) = obj_ref.downcast::<pyo3::types::PyTuple>() {
        let mut arr = Vec::new();
        for item in tuple.iter() {
            arr.push(python_to_json(py, item.into())?);
        }
        return Ok(serde_json::Value::Array(arr));
    }

    // Fallback: try to get string representation
    let repr = obj_ref.str()?.to_string();
    Ok(serde_json::Value::String(repr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_creation() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let response = PyResponse::new(py, 200, None, None).unwrap();
            assert_eq!(response.status, 200);
        });
    }

    #[test]
    fn test_response_with_body() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let body = PyDict::new(py);
            body.set_item("name", "test").unwrap();

            let response = PyResponse::new(py, 201, Some(body.into()), None).unwrap();
            assert_eq!(response.status, 201);

            let body_json = response.body_json().unwrap();
            assert_eq!(body_json["name"], "test");
        });
    }

    #[test]
    fn test_response_helpers() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let ok = PyResponse::ok(py, None, None).unwrap();
            assert_eq!(ok.status, 200);

            let created = PyResponse::created(py, None, None).unwrap();
            assert_eq!(created.status, 201);

            let no_content = PyResponse::no_content().unwrap();
            assert_eq!(no_content.status, 204);

            let bad_request = PyResponse::bad_request(Some("invalid".to_string())).unwrap();
            assert_eq!(bad_request.status, 400);

            let not_found = PyResponse::not_found(None).unwrap();
            assert_eq!(not_found.status, 404);
        });
    }

    #[test]
    fn test_response_headers() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut response = PyResponse::new(py, 200, None, None).unwrap();
            response.set_header("X-Custom".to_string(), "value".to_string());

            assert_eq!(response.get_header("X-Custom"), Some("value".to_string()));
            assert_eq!(response.get_header("X-Missing"), None);
        });
    }

    #[test]
    fn test_json_response() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let body = PyDict::new(py);
            body.set_item("data", "test").unwrap();

            let response = PyResponse::json(py, body.into(), 200).unwrap();

            assert_eq!(response.status, 200);
            assert_eq!(
                response.headers_ref().get("content-type"),
                Some(&"application/json".to_string())
            );
        });
    }
}
