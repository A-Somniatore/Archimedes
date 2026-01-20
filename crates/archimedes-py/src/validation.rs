//! Contract validation integration for Python bindings
//!
//! This module provides Themis contract-based request/response validation
//! for Python handlers, ensuring parity with the Rust native implementation.
//!
//! ## Overview
//!
//! Sentinel validates requests and responses against Themis contract schemas:
//! 1. Resolve operation ID from HTTP method and path
//! 2. Validate request body against operation's request schema
//! 3. Validate response body against operation's response schema
//!
//! ## Example
//!
//! ```text
//! from archimedes import PySentinel, PyValidationResult
//!
//! # Create sentinel with contract artifact
//! sentinel = PySentinel.from_file("contract.artifact.json")
//!
//! # In middleware or handler
//! @app.handler("createUser")
//! async def create_user(ctx, body):
//!     # Validate request body
//!     result = sentinel.validate_request("createUser", body)
//!     if not result.valid:
//!         return Response.bad_request(result.errors_json())
//!     
//!     # ... handler logic
//!     response_data = {"id": "new-id", "name": body["name"]}
//!     
//!     # Validate response
//!     result = sentinel.validate_response("createUser", 200, response_data)
//!     if not result.valid:
//!         return Response.internal_error("Response validation failed")
//!     
//!     return response_data
//! ```

use std::collections::HashMap;

use archimedes_sentinel::{ArtifactLoader, Sentinel, SentinelConfig};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::Value;

use crate::error::ArchimedesError;

/// Python-exposed operation resolution result.
///
/// Contains the operation ID and extracted path parameters from route matching.
#[pyclass(name = "OperationResolution")]
#[derive(Debug, Clone)]
pub struct PyOperationResolution {
    /// The Themis operation ID.
    #[pyo3(get)]
    pub operation_id: String,

    /// HTTP method of the matched operation.
    #[pyo3(get)]
    pub method: String,

    /// Path template that was matched.
    #[pyo3(get)]
    pub path_template: String,

    /// Extracted path parameters.
    path_params: HashMap<String, String>,

    /// Whether the operation is deprecated.
    #[pyo3(get)]
    pub deprecated: bool,

    /// Tags from the operation.
    #[pyo3(get)]
    pub tags: Vec<String>,
}

#[pymethods]
impl PyOperationResolution {
    /// Get path parameters as a dictionary.
    #[getter]
    fn path_params(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.path_params {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Get a specific path parameter.
    fn get_param(&self, name: &str) -> Option<String> {
        self.path_params.get(name).cloned()
    }

    fn __repr__(&self) -> String {
        format!(
            "OperationResolution(operation_id='{}', method='{}', path='{}')",
            self.operation_id, self.method, self.path_template
        )
    }
}

/// Python-exposed validation error.
#[pyclass(name = "ValidationError")]
#[derive(Debug, Clone)]
pub struct PyValidationError {
    /// Error message.
    #[pyo3(get)]
    pub message: String,

    /// JSON path to the invalid field (e.g., "/users/0/email").
    #[pyo3(get)]
    pub path: Option<String>,

    /// Expected type or value.
    #[pyo3(get)]
    pub expected: Option<String>,

    /// Actual type or value.
    #[pyo3(get)]
    pub actual: Option<String>,
}

#[pymethods]
impl PyValidationError {
    fn __repr__(&self) -> String {
        if let Some(path) = &self.path {
            format!(
                "ValidationError(path='{}', message='{}')",
                path, self.message
            )
        } else {
            format!("ValidationError(message='{}')", self.message)
        }
    }

    fn __str__(&self) -> String {
        if let Some(path) = &self.path {
            format!("{}: {}", path, self.message)
        } else {
            self.message.clone()
        }
    }
}

/// Python-exposed validation result.
///
/// Contains the result of validating a request or response body.
#[pyclass(name = "ValidationResult")]
#[derive(Debug, Clone)]
pub struct PyValidationResult {
    /// Whether validation passed.
    #[pyo3(get)]
    pub valid: bool,

    /// List of validation errors.
    errors: Vec<PyValidationError>,

    /// Schema reference that was validated against.
    #[pyo3(get)]
    pub schema_ref: Option<String>,
}

#[pymethods]
impl PyValidationResult {
    /// Check if validation failed.
    fn is_invalid(&self) -> bool {
        !self.valid
    }

    /// Get the list of validation errors.
    #[getter]
    fn errors(&self) -> Vec<PyValidationError> {
        self.errors.clone()
    }

    /// Get the number of errors.
    fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get errors as a JSON-serializable list.
    fn errors_json(&self) -> Vec<HashMap<String, String>> {
        self.errors
            .iter()
            .map(|e| {
                let mut map = HashMap::new();
                map.insert("message".to_string(), e.message.clone());
                if let Some(path) = &e.path {
                    map.insert("path".to_string(), path.clone());
                }
                if let Some(expected) = &e.expected {
                    map.insert("expected".to_string(), expected.clone());
                }
                if let Some(actual) = &e.actual {
                    map.insert("actual".to_string(), actual.clone());
                }
                map
            })
            .collect()
    }

    /// Get first error message (convenience for single-error cases).
    fn first_error(&self) -> Option<String> {
        self.errors.first().map(|e| e.message.clone())
    }

    fn __repr__(&self) -> String {
        if self.valid {
            "ValidationResult(valid=True)".to_string()
        } else {
            format!(
                "ValidationResult(valid=False, errors={})",
                self.errors.len()
            )
        }
    }

    fn __bool__(&self) -> bool {
        self.valid
    }
}

/// Python-exposed contract sentinel.
///
/// Provides operation resolution and request/response validation
/// against Themis contract schemas.
#[pyclass(name = "Sentinel")]
pub struct PySentinel {
    /// The underlying Rust sentinel.
    sentinel: Sentinel,
}

#[pymethods]
impl PySentinel {
    /// Create a sentinel from a contract artifact file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the contract artifact file (JSON)
    ///
    /// # Example
    ///
    /// ```ignore
    /// sentinel = Sentinel.from_file("contract.artifact.json")
    /// ```
    #[staticmethod]
    pub fn from_file(py: Python<'_>, path: String) -> PyResult<Self> {
        // Load artifact synchronously by blocking on the async operation
        let sentinel = py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                ArchimedesError::new_err(format!("Failed to create runtime: {}", e))
            })?;

            rt.block_on(async {
                let artifact = ArtifactLoader::from_file(&path).await.map_err(|e| {
                    ArchimedesError::new_err(format!("Failed to load artifact: {}", e))
                })?;
                Ok::<Sentinel, PyErr>(Sentinel::with_defaults(artifact))
            })
        })?;

        Ok(Self { sentinel })
    }

    /// Create a sentinel from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json` - Contract artifact as JSON string
    #[staticmethod]
    pub fn from_json(json: &str) -> PyResult<Self> {
        let artifact = ArtifactLoader::from_json(json)
            .map_err(|e| ArchimedesError::new_err(format!("Failed to parse artifact: {}", e)))?;
        Ok(Self {
            sentinel: Sentinel::with_defaults(artifact),
        })
    }

    /// Create a sentinel with custom configuration.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the contract artifact file
    /// * `validate_requests` - Whether to validate request bodies
    /// * `validate_responses` - Whether to validate response bodies
    /// * `strict_mode` - Whether to fail on unknown fields
    #[staticmethod]
    #[pyo3(signature = (path, validate_requests = true, validate_responses = true, strict_mode = false))]
    pub fn with_config(
        py: Python<'_>,
        path: String,
        validate_requests: bool,
        validate_responses: bool,
        strict_mode: bool,
    ) -> PyResult<Self> {
        let sentinel = py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                ArchimedesError::new_err(format!("Failed to create runtime: {}", e))
            })?;

            rt.block_on(async {
                let artifact = ArtifactLoader::from_file(&path).await.map_err(|e| {
                    ArchimedesError::new_err(format!("Failed to load artifact: {}", e))
                })?;

                let mut config = SentinelConfig::default();
                config.validation.validate_requests = validate_requests;
                config.validation.validate_responses = validate_responses;
                config.validation.strict_mode = strict_mode;

                Ok::<Sentinel, PyErr>(Sentinel::new(artifact, config))
            })
        })?;

        Ok(Self { sentinel })
    }

    /// Get the service name from the contract.
    #[getter]
    fn service_name(&self) -> &str {
        self.sentinel.service_name()
    }

    /// Get the contract version.
    #[getter]
    fn version(&self) -> &str {
        self.sentinel.version()
    }

    /// Get the contract format (e.g., "openapi").
    #[getter]
    fn format(&self) -> &str {
        self.sentinel.format()
    }

    /// Get the number of operations in the contract.
    #[getter]
    fn operation_count(&self) -> usize {
        self.sentinel.operation_count()
    }

    /// Resolve an HTTP request to an operation.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method (GET, POST, etc.)
    /// * `path` - Request path (e.g., "/users/123")
    ///
    /// # Returns
    ///
    /// An `OperationResolution` with the matched operation ID and path params,
    /// or None if no match found.
    pub fn resolve(&self, method: &str, path: &str) -> PyResult<Option<PyOperationResolution>> {
        match self.sentinel.resolve(method, path) {
            Ok(resolution) => Ok(Some(PyOperationResolution {
                operation_id: resolution.operation_id,
                method: resolution.method,
                path_template: resolution.path_template,
                path_params: resolution.path_params,
                deprecated: resolution.deprecated,
                tags: resolution.tags,
            })),
            Err(_) => Ok(None),
        }
    }

    /// Check if an operation exists for the given method and path.
    pub fn has_operation(&self, method: &str, path: &str) -> bool {
        self.sentinel.has_operation(method, path)
    }

    /// Validate a request body against the operation schema.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID to validate against
    /// * `body` - The request body (Python dict will be converted to JSON)
    ///
    /// # Returns
    ///
    /// A `ValidationResult` indicating whether validation passed.
    pub fn validate_request(
        &self,
        py: Python<'_>,
        operation_id: &str,
        body: &Bound<'_, PyAny>,
    ) -> PyResult<PyValidationResult> {
        // Convert Python object to JSON
        let json_value = python_to_json(py, body)?;

        match self.sentinel.validate_request(operation_id, &json_value) {
            Ok(result) => Ok(PyValidationResult {
                valid: result.valid,
                errors: result
                    .errors
                    .iter()
                    .map(|e| PyValidationError {
                        message: e.to_string(),
                        path: None,
                        expected: None,
                        actual: None,
                    })
                    .collect(),
                schema_ref: result.schema_ref.map(|s| s.reference),
            }),
            Err(e) => Ok(PyValidationResult {
                valid: false,
                errors: vec![PyValidationError {
                    message: e.to_string(),
                    path: None,
                    expected: None,
                    actual: None,
                }],
                schema_ref: None,
            }),
        }
    }

    /// Validate a response body against the operation schema.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID to validate against
    /// * `status_code` - The HTTP status code of the response
    /// * `body` - The response body (Python dict will be converted to JSON)
    ///
    /// # Returns
    ///
    /// A `ValidationResult` indicating whether validation passed.
    pub fn validate_response(
        &self,
        py: Python<'_>,
        operation_id: &str,
        status_code: u16,
        body: &Bound<'_, PyAny>,
    ) -> PyResult<PyValidationResult> {
        // Convert Python object to JSON
        let json_value = python_to_json(py, body)?;

        match self
            .sentinel
            .validate_response(operation_id, status_code, &json_value)
        {
            Ok(result) => Ok(PyValidationResult {
                valid: result.valid,
                errors: result
                    .errors
                    .iter()
                    .map(|e| PyValidationError {
                        message: e.to_string(),
                        path: None,
                        expected: None,
                        actual: None,
                    })
                    .collect(),
                schema_ref: result.schema_ref.map(|s| s.reference),
            }),
            Err(e) => Ok(PyValidationResult {
                valid: false,
                errors: vec![PyValidationError {
                    message: e.to_string(),
                    path: None,
                    expected: None,
                    actual: None,
                }],
                schema_ref: None,
            }),
        }
    }

    /// Get all registered HTTP methods.
    fn methods(&self) -> Vec<String> {
        self.sentinel
            .methods()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get all routes for a specific method.
    fn routes_for_method(&self, method: &str) -> Vec<String> {
        self.sentinel
            .routes_for_method(method)
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "Sentinel(service='{}', version='{}', operations={})",
            self.sentinel.service_name(),
            self.sentinel.version(),
            self.sentinel.operation_count()
        )
    }
}

/// Convert a Python object to serde_json::Value.
fn python_to_json(py: Python<'_>, obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    // Use Python's json module for reliable conversion
    let json_module = py.import("json")?;
    let json_str: String = json_module.call_method1("dumps", (obj,))?.extract()?;

    serde_json::from_str(&json_str)
        .map_err(|e| ArchimedesError::new_err(format!("Failed to parse JSON: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_valid() {
        let result = PyValidationResult {
            valid: true,
            errors: vec![],
            schema_ref: Some("#/components/schemas/User".to_string()),
        };

        assert!(result.valid);
        assert!(!result.is_invalid());
        assert_eq!(result.error_count(), 0);
        assert!(result.__bool__());
    }

    #[test]
    fn test_validation_result_invalid() {
        let result = PyValidationResult {
            valid: false,
            errors: vec![PyValidationError {
                message: "required field missing".to_string(),
                path: Some("/name".to_string()),
                expected: Some("string".to_string()),
                actual: None,
            }],
            schema_ref: None,
        };

        assert!(!result.valid);
        assert!(result.is_invalid());
        assert_eq!(result.error_count(), 1);
        assert!(!result.__bool__());
        assert_eq!(
            result.first_error(),
            Some("required field missing".to_string())
        );
    }

    #[test]
    fn test_validation_error_repr() {
        let error = PyValidationError {
            message: "invalid type".to_string(),
            path: Some("/email".to_string()),
            expected: Some("string".to_string()),
            actual: Some("number".to_string()),
        };

        assert!(error.__repr__().contains("/email"));
        assert!(error.__str__().contains("/email: invalid type"));
    }

    #[test]
    fn test_operation_resolution_repr() {
        let resolution = PyOperationResolution {
            operation_id: "getUser".to_string(),
            method: "GET".to_string(),
            path_template: "/users/{userId}".to_string(),
            path_params: {
                let mut map = HashMap::new();
                map.insert("userId".to_string(), "123".to_string());
                map
            },
            deprecated: false,
            tags: vec!["users".to_string()],
        };

        assert!(resolution.__repr__().contains("getUser"));
        assert_eq!(resolution.get_param("userId"), Some("123".to_string()));
        assert_eq!(resolution.get_param("nonexistent"), None);
    }

    #[test]
    fn test_errors_json() {
        let result = PyValidationResult {
            valid: false,
            errors: vec![
                PyValidationError {
                    message: "field required".to_string(),
                    path: Some("/name".to_string()),
                    expected: None,
                    actual: None,
                },
                PyValidationError {
                    message: "type mismatch".to_string(),
                    path: Some("/age".to_string()),
                    expected: Some("number".to_string()),
                    actual: Some("string".to_string()),
                },
            ],
            schema_ref: None,
        };

        let errors_json = result.errors_json();
        assert_eq!(errors_json.len(), 2);
        assert_eq!(
            errors_json[0].get("message"),
            Some(&"field required".to_string())
        );
        assert_eq!(errors_json[1].get("expected"), Some(&"number".to_string()));
    }
}
