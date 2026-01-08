//! Contract validation via Sentinel.

use crate::error::ArchimedesError;
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Validation error details.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Path to the invalid field (e.g., "body.email")
    pub path: String,

    /// Error message
    pub message: String,

    /// Expected type or format
    pub expected: Option<String>,

    /// Actual value received
    pub actual: Option<String>,
}

/// Result of validation.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,

    /// List of validation errors (if any)
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Create a successful validation result.
    pub fn success() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    /// Create a failed validation result.
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }

    /// Check if validation passed.
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Get the number of errors.
    pub fn error_count(&self) -> u32 {
        self.errors.len() as u32
    }

    /// Get a summary of all errors.
    pub fn error_summary(&self) -> String {
        self.errors
            .iter()
            .map(|e| format!("{}: {}", e.path, e.message))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

/// Create a successful validation result.
#[napi]
pub fn validation_success() -> ValidationResult {
    ValidationResult::success()
}

/// Create a failed validation result with a single error.
#[napi]
pub fn validation_failure(path: String, message: String) -> ValidationResult {
    ValidationResult::failure(vec![ValidationError {
        path,
        message,
        expected: None,
        actual: None,
    }])
}

/// Operation resolution result from route matching.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResolution {
    /// Operation ID from the contract
    pub operation_id: String,

    /// Extracted path parameters
    pub path_params: HashMap<String, String>,

    /// Whether the operation was found
    pub found: bool,
}

/// Contract validator (Sentinel).
///
/// Validates requests and responses against the contract schema.
#[napi]
#[derive(Debug, Clone)]
pub struct Sentinel {
    /// Contract JSON string
    contract_json: String,

    /// Whether the contract has been loaded
    loaded: bool,

    /// Operation definitions cache
    operations: HashMap<String, Operation>,
}

/// Internal operation definition.
#[derive(Debug, Clone)]
struct Operation {
    #[allow(dead_code)]
    operation_id: String,
    method: String,
    path_pattern: String,
}

#[napi]
impl Sentinel {
    /// Create a new Sentinel with a contract JSON string.
    #[napi(constructor)]
    pub fn new(contract_json: String) -> Self {
        Self {
            contract_json,
            loaded: false,
            operations: HashMap::new(),
        }
    }

    /// Load Sentinel from a contract file.
    #[napi(factory)]
    pub fn from_file(path: String) -> napi::Result<Self> {
        let content = std::fs::read_to_string(&path).map_err(|e| {
            napi::Error::new(
                napi::Status::GenericFailure,
                format!("Failed to read contract file '{}': {}", path, e),
            )
        })?;

        Ok(Self::new(content))
    }

    /// Initialize and parse the contract.
    #[napi]
    pub fn init(&mut self) -> napi::Result<()> {
        let contract: serde_json::Value =
            serde_json::from_str(&self.contract_json).map_err(|e| {
                napi::Error::new(
                    napi::Status::GenericFailure,
                    format!("Failed to parse contract JSON: {}", e),
                )
            })?;

        // Parse operations from contract
        if let Some(paths) = contract.get("paths").and_then(|p| p.as_object()) {
            for (path, methods) in paths {
                if let Some(methods_obj) = methods.as_object() {
                    for (method, op) in methods_obj {
                        if let Some(op_id) = op.get("operationId").and_then(|id| id.as_str()) {
                            self.operations.insert(
                                op_id.to_string(),
                                Operation {
                                    operation_id: op_id.to_string(),
                                    method: method.to_uppercase(),
                                    path_pattern: path.clone(),
                                },
                            );
                        }
                    }
                }
            }
        }

        self.loaded = true;
        Ok(())
    }

    /// Check if Sentinel is initialized.
    #[napi(getter)]
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Get the list of operation IDs.
    #[napi]
    pub fn operation_ids(&self) -> Vec<String> {
        self.operations.keys().cloned().collect()
    }

    /// Resolve a request to an operation.
    #[napi]
    pub fn resolve_operation(
        &self,
        method: String,
        path: String,
    ) -> napi::Result<OperationResolution> {
        if !self.loaded {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Sentinel not initialized. Call init() first.",
            ));
        }

        // Simple path matching (could be enhanced with regex)
        for op in self.operations.values() {
            if op.method == method.to_uppercase() {
                if let Some(params) = self.match_path(&op.path_pattern, &path) {
                    return Ok(OperationResolution {
                        operation_id: op.operation_id.clone(),
                        path_params: params,
                        found: true,
                    });
                }
            }
        }

        Ok(OperationResolution {
            operation_id: String::new(),
            path_params: HashMap::new(),
            found: false,
        })
    }

    /// Validate a request body against the operation schema.
    #[napi]
    pub fn validate_request(
        &self,
        operation_id: String,
        body: Option<String>,
    ) -> napi::Result<ValidationResult> {
        if !self.loaded {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Sentinel not initialized. Call init() first.",
            ));
        }

        if !self.operations.contains_key(&operation_id) {
            return Err(ArchimedesError::operation_not_found(operation_id).into());
        }

        // Basic validation - check if body is valid JSON when present
        if let Some(body_str) = body {
            if !body_str.is_empty() {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    return Ok(ValidationResult::failure(vec![ValidationError {
                        path: "body".to_string(),
                        message: format!("Invalid JSON: {}", e),
                        expected: Some("valid JSON".to_string()),
                        actual: Some(body_str),
                    }]));
                }
            }
        }

        Ok(ValidationResult::success())
    }

    /// Validate a response body against the operation schema.
    #[napi]
    pub fn validate_response(
        &self,
        operation_id: String,
        status_code: u16,
        body: Option<String>,
    ) -> napi::Result<ValidationResult> {
        if !self.loaded {
            return Err(napi::Error::new(
                napi::Status::GenericFailure,
                "Sentinel not initialized. Call init() first.",
            ));
        }

        if !self.operations.contains_key(&operation_id) {
            return Err(ArchimedesError::operation_not_found(operation_id).into());
        }

        // Basic validation - check status code range
        if !(100..600).contains(&status_code) {
            return Ok(ValidationResult::failure(vec![ValidationError {
                path: "statusCode".to_string(),
                message: format!("Invalid status code: {}", status_code),
                expected: Some("100-599".to_string()),
                actual: Some(status_code.to_string()),
            }]));
        }

        // Check if body is valid JSON when present
        if let Some(body_str) = body {
            if !body_str.is_empty() {
                if let Err(e) = serde_json::from_str::<serde_json::Value>(&body_str) {
                    return Ok(ValidationResult::failure(vec![ValidationError {
                        path: "body".to_string(),
                        message: format!("Invalid JSON: {}", e),
                        expected: Some("valid JSON".to_string()),
                        actual: Some(body_str),
                    }]));
                }
            }
        }

        Ok(ValidationResult::success())
    }

    /// Match a path pattern against an actual path, extracting parameters.
    fn match_path(&self, pattern: &str, path: &str) -> Option<HashMap<String, String>> {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        if pattern_parts.len() != path_parts.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (pat, actual) in pattern_parts.iter().zip(path_parts.iter()) {
            if pat.starts_with('{') && pat.ends_with('}') {
                // Parameter segment
                let param_name = &pat[1..pat.len() - 1];
                params.insert(param_name.to_string(), (*actual).to_string());
            } else if pat != actual {
                // Literal segment doesn't match
                return None;
            }
        }

        Some(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_contract() -> String {
        r#"{
            "paths": {
                "/users": {
                    "get": { "operationId": "listUsers" },
                    "post": { "operationId": "createUser" }
                },
                "/users/{userId}": {
                    "get": { "operationId": "getUser" },
                    "put": { "operationId": "updateUser" },
                    "delete": { "operationId": "deleteUser" }
                }
            }
        }"#
        .to_string()
    }

    #[test]
    fn test_sentinel_init() {
        let mut sentinel = Sentinel::new(sample_contract());
        assert!(!sentinel.is_loaded());

        sentinel.init().unwrap();
        assert!(sentinel.is_loaded());
    }

    #[test]
    fn test_sentinel_operation_ids() {
        let mut sentinel = Sentinel::new(sample_contract());
        sentinel.init().unwrap();

        let ops = sentinel.operation_ids();
        assert!(ops.contains(&"listUsers".to_string()));
        assert!(ops.contains(&"createUser".to_string()));
        assert!(ops.contains(&"getUser".to_string()));
    }

    #[test]
    fn test_resolve_operation() {
        let mut sentinel = Sentinel::new(sample_contract());
        sentinel.init().unwrap();

        let result = sentinel
            .resolve_operation("GET".to_string(), "/users".to_string())
            .unwrap();
        assert!(result.found);
        assert_eq!(result.operation_id, "listUsers");

        let result = sentinel
            .resolve_operation("GET".to_string(), "/users/123".to_string())
            .unwrap();
        assert!(result.found);
        assert_eq!(result.operation_id, "getUser");
        assert_eq!(result.path_params.get("userId"), Some(&"123".to_string()));
    }

    #[test]
    fn test_resolve_operation_not_found() {
        let mut sentinel = Sentinel::new(sample_contract());
        sentinel.init().unwrap();

        let result = sentinel
            .resolve_operation("GET".to_string(), "/nonexistent".to_string())
            .unwrap();
        assert!(!result.found);
    }

    #[test]
    fn test_validate_request_valid() {
        let mut sentinel = Sentinel::new(sample_contract());
        sentinel.init().unwrap();

        let result = sentinel
            .validate_request(
                "createUser".to_string(),
                Some(r#"{"name": "test"}"#.to_string()),
            )
            .unwrap();
        assert!(result.is_valid());
    }

    #[test]
    fn test_validate_request_invalid_json() {
        let mut sentinel = Sentinel::new(sample_contract());
        sentinel.init().unwrap();

        let result = sentinel
            .validate_request("createUser".to_string(), Some("not valid json".to_string()))
            .unwrap();
        assert!(!result.is_valid());
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_validation_result_success() {
        let result = ValidationResult::success();
        assert!(result.is_valid());
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_validation_result_failure() {
        let result = ValidationResult::failure(vec![ValidationError {
            path: "body.email".to_string(),
            message: "Invalid email format".to_string(),
            expected: Some("email".to_string()),
            actual: Some("not-an-email".to_string()),
        }]);
        assert!(!result.is_valid());
        assert_eq!(result.error_count(), 1);
        assert!(result.error_summary().contains("email"));
    }
}
