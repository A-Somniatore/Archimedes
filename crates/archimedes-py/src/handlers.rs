//! Python handler registry for Archimedes

use crate::error::{handler_error, internal_error};
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
        let mut handlers = self.handlers.write().map_err(|e| {
            internal_error(format!("Failed to acquire handler lock: {e}"))
        })?;

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
        self.handlers
            .read()
            .ok()
            .and_then(|h| h.get(operation_id).map(|obj| Python::with_gil(|py| obj.clone_ref(py))))
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
            handler_error(format!("No handler registered for operation '{operation_id}'"))
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
            let handler: PyObject = py.eval(
                pyo3::ffi::c_str!("lambda ctx: {'status': 'ok'}"),
                None,
                None,
            ).unwrap().into();

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

            let handler1: PyObject = py.eval(
                pyo3::ffi::c_str!("lambda ctx: {}"),
                None,
                None,
            ).unwrap().into();
            let handler2: PyObject = py.eval(
                pyo3::ffi::c_str!("lambda ctx: {}"),
                None,
                None,
            ).unwrap().into();

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
}
