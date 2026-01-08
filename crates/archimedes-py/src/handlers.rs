//! Python handler registry for Archimedes

use crate::error::{handler_error, internal_error};
use crate::response::PyResponse;
use crate::PyRequestContext;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::RwLock;

/// Registry for Python handlers
pub struct HandlerRegistry {
    handlers: RwLock<HashMap<String, PyObject>>,
}

impl HandlerRegistry {
    /// Create a new handler registry
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a handler for an operation
    pub fn register(&self, operation_id: String, handler: PyObject) -> PyResult<()> {
        let mut handlers = self
            .handlers
            .write()
            .map_err(|e| internal_error(format!("Failed to acquire handler lock: {e}")))?;

        // Validate that handler is callable
        Python::with_gil(|py| {
            if !handler.bind(py).is_callable() {
                return Err(handler_error(format!(
                    "Handler for operation '{}' must be callable",
                    operation_id
                )));
            }
            Ok(())
        })?;

        handlers.insert(operation_id, handler);
        Ok(())
    }

    /// Get a handler for an operation
    pub fn get(&self, operation_id: &str) -> Option<PyObject> {
        self.handlers.read().ok().and_then(|h| {
            h.get(operation_id)
                .map(|obj| Python::with_gil(|py| obj.clone_ref(py)))
        })
    }

    /// Check if a handler is registered
    pub fn has(&self, operation_id: &str) -> bool {
        self.handlers
            .read()
            .ok()
            .map(|h| h.contains_key(operation_id))
            .unwrap_or(false)
    }

    /// Get all registered operation IDs
    pub fn operation_ids(&self) -> Vec<String> {
        self.handlers
            .read()
            .ok()
            .map(|h| h.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get the number of registered handlers
    pub fn len(&self) -> usize {
        self.handlers.read().ok().map(|h| h.len()).unwrap_or(0)
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Invoke a handler
    ///
    /// This calls the Python handler with the request context and optional body.
    pub fn invoke(
        &self,
        py: Python<'_>,
        operation_id: &str,
        ctx: PyRequestContext,
        body: Option<serde_json::Value>,
    ) -> PyResult<serde_json::Value> {
        let handler = self.get(operation_id).ok_or_else(|| {
            handler_error(format!(
                "No handler registered for operation '{operation_id}'"
            ))
        })?;

        let handler_ref = handler.bind(py);

        // Prepare arguments
        if let Some(body) = body {
            // Handler with body: handler(ctx, body)
            let body_py = json_to_python(py, &body)?;
            let args = (ctx, body_py);
            let result = handler_ref.call1(args)?;
            python_to_json(py, result.into())
        } else {
            // Handler without body: handler(ctx)
            let result = handler_ref.call1((ctx,))?;
            python_to_json(py, result.into())
        }
    }

    /// Invoke an async handler
    ///
    /// This calls the Python async handler and awaits the result.
    /// Note: Currently uses synchronous invocation. Full async support
    /// will be added in a future version.
    pub fn invoke_sync(
        &self,
        py: Python<'_>,
        operation_id: &str,
        ctx: PyRequestContext,
        body: Option<serde_json::Value>,
    ) -> PyResult<serde_json::Value> {
        // For now, use synchronous invocation
        self.invoke(py, operation_id, ctx, body)
    }
}

impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
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

    // Check for PyResponse first
    if let Ok(response) = obj_ref.extract::<PyResponse>() {
        let mut map = serde_json::Map::new();
        map.insert("status_code".to_string(), response.status.into());
        if let Some(body) = response.body_json() {
            map.insert("body".to_string(), body.clone());
        } else {
            map.insert("body".to_string(), serde_json::Value::Null);
        }
        return Ok(serde_json::Value::Object(map));
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
    fn test_registry_new() {
        let registry = HandlerRegistry::new();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_register_and_get() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            // Create a Python lambda
            let handler: PyObject = py
                .eval(
                    pyo3::ffi::c_str!("lambda ctx: {'status': 'ok'}"),
                    None,
                    None,
                )
                .unwrap()
                .into();

            registry.register("testOp".to_string(), handler).unwrap();

            assert!(registry.has("testOp"));
            assert!(!registry.has("otherOp"));
            assert_eq!(registry.len(), 1);
        });
    }

    #[test]
    fn test_registry_operation_ids() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            let handler1: PyObject = py
                .eval(pyo3::ffi::c_str!("lambda ctx: {}"), None, None)
                .unwrap()
                .into();
            let handler2: PyObject = py
                .eval(pyo3::ffi::c_str!("lambda ctx: {}"), None, None)
                .unwrap()
                .into();

            registry.register("op1".to_string(), handler1).unwrap();
            registry.register("op2".to_string(), handler2).unwrap();

            let ids = registry.operation_ids();
            assert!(ids.contains(&"op1".to_string()));
            assert!(ids.contains(&"op2".to_string()));
        });
    }

    #[test]
    fn test_json_to_python_and_back() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let original = serde_json::json!({
                "name": "test",
                "count": 42,
                "active": true,
                "items": [1, 2, 3],
                "nested": {
                    "key": "value"
                }
            });

            let py_obj = json_to_python(py, &original).unwrap();
            let back = python_to_json(py, py_obj).unwrap();

            assert_eq!(original, back);
        });
    }

    #[test]
    fn test_registry_replace_handler() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            // Register first handler
            let handler1: PyObject = py
                .eval(pyo3::ffi::c_str!("lambda ctx: {'version': 1}"), None, None)
                .unwrap()
                .into();
            registry.register("testOp".to_string(), handler1).unwrap();

            // Register second handler with same operation ID
            let handler2: PyObject = py
                .eval(pyo3::ffi::c_str!("lambda ctx: {'version': 2}"), None, None)
                .unwrap()
                .into();
            registry.register("testOp".to_string(), handler2).unwrap();

            // Should still have 1 handler (replaced)
            assert_eq!(registry.len(), 1);
            assert!(registry.has("testOp"));
        });
    }

    #[test]
    fn test_registry_reject_non_callable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            // Try to register a non-callable (string)
            let non_callable: PyObject = py
                .eval(pyo3::ffi::c_str!("'not a function'"), None, None)
                .unwrap()
                .into();

            let result = registry.register("testOp".to_string(), non_callable);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_json_null_handling() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let null = serde_json::Value::Null;
            let py_obj = json_to_python(py, &null).unwrap();
            let back = python_to_json(py, py_obj).unwrap();
            assert_eq!(back, serde_json::Value::Null);
        });
    }

    #[test]
    fn test_json_array_handling() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let arr = serde_json::json!([1, "two", 3.0, true, null]);
            let py_obj = json_to_python(py, &arr).unwrap();
            let back = python_to_json(py, py_obj).unwrap();

            // Compare elements (order matters)
            let arr_back = back.as_array().unwrap();
            assert_eq!(arr_back.len(), 5);
            assert_eq!(arr_back[0], 1);
            assert_eq!(arr_back[1], "two");
            assert_eq!(arr_back[3], true);
            assert!(arr_back[4].is_null());
        });
    }

    #[test]
    fn test_json_nested_object_handling() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let nested = serde_json::json!({
                "level1": {
                    "level2": {
                        "level3": {
                            "value": "deep"
                        }
                    }
                }
            });

            let py_obj = json_to_python(py, &nested).unwrap();
            let back = python_to_json(py, py_obj).unwrap();

            assert_eq!(back["level1"]["level2"]["level3"]["value"], "deep");
        });
    }

    #[test]
    fn test_invoke_simple_handler() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            // Handler that returns a dict
            let handler: PyObject = py
                .eval(
                    pyo3::ffi::c_str!("lambda ctx: {'status': 'ok', 'data': ctx.operation_id}"),
                    None,
                    None,
                )
                .unwrap()
                .into();

            registry.register("testOp".to_string(), handler).unwrap();

            // Create a test context
            let ctx = PyRequestContext::test("testOp");

            // Invoke the handler
            let result = registry.invoke(py, "testOp", ctx, None);
            assert!(result.is_ok());

            let response = result.unwrap();
            assert_eq!(response["status"], "ok");
            assert_eq!(response["data"], "testOp");
        });
    }

    #[test]
    fn test_invoke_handler_with_body() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            // Handler that uses the body
            let handler: PyObject = py
                .eval(
                    pyo3::ffi::c_str!(
                        "lambda ctx, body: {'received': body.get('name', 'unknown')}"
                    ),
                    None,
                    None,
                )
                .unwrap()
                .into();

            registry
                .register("createUser".to_string(), handler)
                .unwrap();

            let ctx = PyRequestContext::test("createUser");
            let body = serde_json::json!({"name": "Alice"});

            let result = registry.invoke(py, "createUser", ctx, Some(body));
            assert!(result.is_ok());

            let response = result.unwrap();
            assert_eq!(response["received"], "Alice");
        });
    }

    #[test]
    fn test_invoke_missing_handler() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();
            let ctx = PyRequestContext::test("missingOp");

            let result = registry.invoke(py, "missingOp", ctx, None);
            assert!(result.is_err());
        });
    }

    #[test]
    fn test_handler_returning_list() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            let handler: PyObject = py
                .eval(
                    pyo3::ffi::c_str!("lambda ctx: [{'id': 1}, {'id': 2}]"),
                    None,
                    None,
                )
                .unwrap()
                .into();

            registry.register("listItems".to_string(), handler).unwrap();

            let ctx = PyRequestContext::test("listItems");
            let result = registry.invoke(py, "listItems", ctx, None);
            assert!(result.is_ok());

            let response = result.unwrap();
            let arr = response.as_array().unwrap();
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0]["id"], 1);
            assert_eq!(arr[1]["id"], 2);
        });
    }

    // =========================================================================
    // Middleware Integration Pattern Tests
    // =========================================================================
    // These tests demonstrate how handlers integrate with middleware patterns

    #[test]
    fn test_handler_receives_context_with_identity() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            // Handler that checks for authenticated user
            let handler: PyObject = py.eval(
                pyo3::ffi::c_str!("lambda ctx: {'authenticated': ctx.is_authenticated(), 'subject': ctx.subject()}"),
                None,
                None,
            ).unwrap().into();

            registry
                .register("protectedOp".to_string(), handler)
                .unwrap();

            // Create context with identity (simulates middleware having set it)
            let identity = crate::context::PyIdentity::new(
                "user-123".to_string(),
                None,
                None,
                None,
                std::collections::HashMap::new(),
                vec![],
                vec![],
            );

            let ctx = PyRequestContext::new(
                "protectedOp".to_string(),
                "GET".to_string(),
                "/protected".to_string(),
                std::collections::HashMap::new(),
                std::collections::HashMap::new(),
                std::collections::HashMap::new(),
                "trace".to_string(),
                "span".to_string(),
                Some(identity),
            );

            let result = registry.invoke(py, "protectedOp", ctx, None);
            assert!(result.is_ok());

            let response = result.unwrap();
            assert_eq!(response["authenticated"], true);
            assert_eq!(response["subject"], "user-123");
        });
    }

    #[test]
    fn test_handler_uses_path_params() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            // Handler that extracts path params
            let handler: PyObject = py
                .eval(
                    pyo3::ffi::c_str!("lambda ctx: {'userId': ctx.path_params.get('userId')}"),
                    None,
                    None,
                )
                .unwrap()
                .into();

            registry.register("getUser".to_string(), handler).unwrap();

            // Create context with path params (simulates router having extracted them)
            let mut path_params = std::collections::HashMap::new();
            path_params.insert("userId".to_string(), "user-456".to_string());

            let ctx = PyRequestContext::new(
                "getUser".to_string(),
                "GET".to_string(),
                "/users/user-456".to_string(),
                path_params,
                std::collections::HashMap::new(),
                std::collections::HashMap::new(),
                "trace".to_string(),
                "span".to_string(),
                None,
            );

            let result = registry.invoke(py, "getUser", ctx, None);
            assert!(result.is_ok());

            let response = result.unwrap();
            assert_eq!(response["userId"], "user-456");
        });
    }

    #[test]
    fn test_handler_uses_trace_context() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let registry = HandlerRegistry::new();

            // Handler that returns trace context
            let handler: PyObject = py
                .eval(
                    pyo3::ffi::c_str!(
                        "lambda ctx: {'traceId': ctx.trace_id, 'spanId': ctx.span_id}"
                    ),
                    None,
                    None,
                )
                .unwrap()
                .into();

            registry.register("traceOp".to_string(), handler).unwrap();

            let ctx = PyRequestContext::new(
                "traceOp".to_string(),
                "GET".to_string(),
                "/trace".to_string(),
                std::collections::HashMap::new(),
                std::collections::HashMap::new(),
                std::collections::HashMap::new(),
                "abc123".to_string(),
                "def456".to_string(),
                None,
            );

            let result = registry.invoke(py, "traceOp", ctx, None);
            assert!(result.is_ok());

            let response = result.unwrap();
            assert_eq!(response["traceId"], "abc123");
            assert_eq!(response["spanId"], "def456");
        });
    }
}
