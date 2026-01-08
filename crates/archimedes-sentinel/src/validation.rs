//! Request and response validation against Themis schemas.
//!
//! This module provides validators that check HTTP requests and responses
//! against the JSON schemas defined in Themis contracts.

use std::collections::HashMap;

use indexmap::IndexMap;
use serde_json::Value;
use themis_core::Schema;
use tracing::{debug, warn};

use crate::artifact::{LoadedArtifact, SchemaRef};
use crate::config::ValidationConfig;
use crate::error::{SentinelResult, ValidationError};

/// Result of a validation operation.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub valid: bool,
    /// List of validation errors.
    pub errors: Vec<ValidationError>,
    /// Schema that was validated against.
    pub schema_ref: Option<SchemaRef>,
}

impl ValidationResult {
    /// Create a successful validation result.
    pub fn success(schema_ref: Option<SchemaRef>) -> Self {
        Self {
            valid: true,
            errors: vec![],
            schema_ref,
        }
    }

    /// Create a failed validation result.
    pub fn failure(errors: Vec<ValidationError>, schema_ref: Option<SchemaRef>) -> Self {
        Self {
            valid: false,
            errors,
            schema_ref,
        }
    }

    /// Check if any errors exist.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Validates requests and responses against Themis schemas.
#[derive(Debug)]
pub struct SchemaValidator {
    /// Validation configuration.
    config: ValidationConfig,
    /// Named schemas from the artifact.
    _schemas: IndexMap<String, Schema>,
}

impl SchemaValidator {
    /// Create a validator from a loaded artifact.
    pub fn from_artifact(artifact: &LoadedArtifact, config: ValidationConfig) -> Self {
        debug!(
            schema_count = artifact.schemas.len(),
            "schema validator initialized"
        );

        Self {
            config,
            _schemas: artifact.schemas.clone(),
        }
    }

    /// Validate a request body against an operation's request schema.
    pub fn validate_request(
        &self,
        operation_id: &str,
        artifact: &LoadedArtifact,
        body: &Value,
    ) -> SentinelResult<ValidationResult> {
        // Find the operation
        let operation = artifact.operations.iter().find(|op| op.id == operation_id);

        let operation = match operation {
            Some(op) => op,
            None => {
                warn!(operation_id, "operation not found for validation");
                return Ok(ValidationResult::success(None));
            }
        };

        // Check if operation has a request schema
        let schema_ref = match &operation.request_schema {
            Some(sr) => sr,
            None => {
                debug!(
                    operation_id,
                    "no request schema defined, skipping validation"
                );
                return Ok(ValidationResult::success(None));
            }
        };

        // Validate against the schema
        self.validate_against_schema_ref(schema_ref, body)
    }

    /// Validate a response body against an operation's response schema.
    pub fn validate_response(
        &self,
        operation_id: &str,
        artifact: &LoadedArtifact,
        status_code: u16,
        body: &Value,
    ) -> SentinelResult<ValidationResult> {
        // Find the operation
        let operation = artifact.operations.iter().find(|op| op.id == operation_id);

        let operation = match operation {
            Some(op) => op,
            None => {
                warn!(operation_id, "operation not found for validation");
                return Ok(ValidationResult::success(None));
            }
        };

        // Find schema for this status code
        let status_key = status_code.to_string();
        let schema_ref = operation
            .response_schemas
            .get(&status_key)
            .or_else(|| operation.response_schemas.get("default"));

        let schema_ref = match schema_ref {
            Some(sr) => sr,
            None => {
                debug!(
                    operation_id,
                    status_code, "no response schema for status code"
                );
                return Ok(ValidationResult::success(None));
            }
        };

        // Validate against the schema
        self.validate_against_schema_ref(schema_ref, body)
    }

    /// Validate path parameters against expected types.
    pub fn validate_path_params(
        &self,
        params: &HashMap<String, String>,
        expected: &HashMap<String, ParamType>,
    ) -> ValidationResult {
        let mut errors = Vec::new();

        for (name, param_type) in expected {
            if let Some(value) = params.get(name) {
                if !self.is_valid_param_type(value, param_type) {
                    errors.push(ValidationError {
                        path: format!("path.{}", name),
                        message: format!("expected {}, got '{}'", param_type.as_str(), value),
                        schema_path: None,
                        value: Some(value.clone()),
                    });
                }
            } else if !self.config.allow_missing_path_params {
                errors.push(ValidationError {
                    path: format!("path.{}", name),
                    message: format!("missing required path parameter '{}'", name),
                    schema_path: None,
                    value: None,
                });
            }
        }

        if errors.is_empty() {
            ValidationResult::success(None)
        } else {
            ValidationResult::failure(errors, None)
        }
    }

    /// Validate query parameters against expected schema.
    pub fn validate_query_params(
        &self,
        params: &HashMap<String, String>,
        required: &[String],
    ) -> ValidationResult {
        let mut errors = Vec::new();

        for name in required {
            if !params.contains_key(name) {
                errors.push(ValidationError {
                    path: format!("query.{}", name),
                    message: format!("missing required query parameter '{}'", name),
                    schema_path: None,
                    value: None,
                });
            }
        }

        if errors.is_empty() {
            ValidationResult::success(None)
        } else {
            ValidationResult::failure(errors, None)
        }
    }

    fn validate_against_schema_ref(
        &self,
        schema_ref: &SchemaRef,
        value: &Value,
    ) -> SentinelResult<ValidationResult> {
        // Perform basic type validation based on schema_ref
        let errors = self.validate_value_type(value, schema_ref, "");

        if errors.is_empty() {
            Ok(ValidationResult::success(Some(schema_ref.clone())))
        } else {
            Ok(ValidationResult::failure(errors, Some(schema_ref.clone())))
        }
    }

    fn validate_value_type(
        &self,
        value: &Value,
        schema_ref: &SchemaRef,
        path: &str,
    ) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Basic type checking based on schema_ref type
        match schema_ref.schema_type.as_str() {
            "object" => {
                if !value.is_object() && !value.is_null() {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: "expected object".to_string(),
                        schema_path: Some(schema_ref.reference.clone()),
                        value: Some(value.to_string()),
                    });
                }
            }
            "array" => {
                if !value.is_array() && !value.is_null() {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: "expected array".to_string(),
                        schema_path: Some(schema_ref.reference.clone()),
                        value: Some(value.to_string()),
                    });
                }
            }
            "string" => {
                if !value.is_string() && !value.is_null() {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: "expected string".to_string(),
                        schema_path: Some(schema_ref.reference.clone()),
                        value: Some(value.to_string()),
                    });
                }
            }
            "integer" | "number" => {
                if !value.is_number() && !value.is_null() {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: "expected number".to_string(),
                        schema_path: Some(schema_ref.reference.clone()),
                        value: Some(value.to_string()),
                    });
                }
            }
            "boolean" => {
                if !value.is_boolean() && !value.is_null() {
                    errors.push(ValidationError {
                        path: path.to_string(),
                        message: "expected boolean".to_string(),
                        schema_path: Some(schema_ref.reference.clone()),
                        value: Some(value.to_string()),
                    });
                }
            }
            _ => {
                // Unknown type, skip validation
                debug!(schema_type = schema_ref.schema_type, "unknown schema type");
            }
        }

        // Check required fields for objects
        if value.is_object() {
            if let Some(obj) = value.as_object() {
                for required_field in &schema_ref.required {
                    if !obj.contains_key(required_field) {
                        errors.push(ValidationError {
                            path: if path.is_empty() {
                                required_field.clone()
                            } else {
                                format!("{}.{}", path, required_field)
                            },
                            message: format!("missing required field '{}'", required_field),
                            schema_path: Some(schema_ref.reference.clone()),
                            value: None,
                        });
                    }
                }
            }
        }

        errors
    }

    fn is_valid_param_type(&self, value: &str, param_type: &ParamType) -> bool {
        match param_type {
            ParamType::String => true,
            ParamType::Integer => value.parse::<i64>().is_ok(),
            ParamType::Number => value.parse::<f64>().is_ok(),
            ParamType::Boolean => value == "true" || value == "false",
            ParamType::Uuid => uuid::Uuid::parse_str(value).is_ok(),
        }
    }
}

/// Parameter type for path/query validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamType {
    /// String type.
    String,
    /// Integer type.
    Integer,
    /// Number type.
    Number,
    /// Boolean type.
    Boolean,
    /// UUID type.
    Uuid,
}

impl ParamType {
    fn as_str(&self) -> &'static str {
        match self {
            ParamType::String => "string",
            ParamType::Integer => "integer",
            ParamType::Number => "number",
            ParamType::Boolean => "boolean",
            ParamType::Uuid => "uuid",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::LoadedOperation;

    fn create_test_config() -> ValidationConfig {
        ValidationConfig {
            validate_requests: true,
            validate_responses: true,
            strict_mode: false,
            allow_additional_properties: true,
            allow_missing_path_params: false,
        }
    }

    fn create_test_artifact() -> LoadedArtifact {
        let mut response_schemas = HashMap::new();
        response_schemas.insert(
            "200".to_string(),
            SchemaRef {
                reference: "#/components/schemas/User".to_string(),
                schema_type: "object".to_string(),
                required: vec!["id".to_string(), "name".to_string()],
            },
        );

        LoadedArtifact {
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            format: "openapi".to_string(),
            operations: vec![LoadedOperation {
                id: "createUser".to_string(),
                method: "POST".to_string(),
                path: "/users".to_string(),
                summary: None,
                deprecated: false,
                security: vec![],
                request_schema: Some(SchemaRef {
                    reference: "#/components/schemas/CreateUser".to_string(),
                    schema_type: "object".to_string(),
                    required: vec!["name".to_string(), "email".to_string()],
                }),
                response_schemas,
                tags: vec![],
            }],
            schemas: IndexMap::new(),
        }
    }

    #[test]
    fn test_validate_request_valid() {
        let artifact = create_test_artifact();
        let config = create_test_config();
        let validator = SchemaValidator::from_artifact(&artifact, config);

        let body = serde_json::json!({
            "name": "John Doe",
            "email": "john@example.com"
        });

        let result = validator
            .validate_request("createUser", &artifact, &body)
            .unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_request_missing_required() {
        let artifact = create_test_artifact();
        let config = create_test_config();
        let validator = SchemaValidator::from_artifact(&artifact, config);

        let body = serde_json::json!({
            "name": "John Doe"
            // missing email
        });

        let result = validator
            .validate_request("createUser", &artifact, &body)
            .unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("email")));
    }

    #[test]
    fn test_validate_request_wrong_type() {
        let artifact = create_test_artifact();
        let config = create_test_config();
        let validator = SchemaValidator::from_artifact(&artifact, config);

        // Send array instead of object
        let body = serde_json::json!([1, 2, 3]);

        let result = validator
            .validate_request("createUser", &artifact, &body)
            .unwrap();
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("object")));
    }

    #[test]
    fn test_validate_response_valid() {
        let artifact = create_test_artifact();
        let config = create_test_config();
        let validator = SchemaValidator::from_artifact(&artifact, config);

        let body = serde_json::json!({
            "id": "123",
            "name": "John Doe",
            "email": "john@example.com"
        });

        let result = validator
            .validate_response("createUser", &artifact, 200, &body)
            .unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_validate_path_params_valid() {
        let config = create_test_config();
        let artifact = create_test_artifact();
        let validator = SchemaValidator::from_artifact(&artifact, config);

        let params = HashMap::from([("userId".to_string(), "123".to_string())]);
        let expected = HashMap::from([("userId".to_string(), ParamType::Integer)]);

        let result = validator.validate_path_params(&params, &expected);
        assert!(result.valid);
    }

    #[test]
    fn test_validate_path_params_wrong_type() {
        let config = create_test_config();
        let artifact = create_test_artifact();
        let validator = SchemaValidator::from_artifact(&artifact, config);

        let params = HashMap::from([("userId".to_string(), "abc".to_string())]);
        let expected = HashMap::from([("userId".to_string(), ParamType::Integer)]);

        let result = validator.validate_path_params(&params, &expected);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.message.contains("integer")));
    }

    #[test]
    fn test_validate_query_params_missing_required() {
        let config = create_test_config();
        let artifact = create_test_artifact();
        let validator = SchemaValidator::from_artifact(&artifact, config);

        let params = HashMap::new();
        let required = vec!["page".to_string(), "limit".to_string()];

        let result = validator.validate_query_params(&params, &required);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_validate_uuid_param() {
        let config = create_test_config();
        let artifact = create_test_artifact();
        let validator = SchemaValidator::from_artifact(&artifact, config);

        let valid_uuid = HashMap::from([(
            "id".to_string(),
            "550e8400-e29b-41d4-a716-446655440000".to_string(),
        )]);
        let expected = HashMap::from([("id".to_string(), ParamType::Uuid)]);
        let result = validator.validate_path_params(&valid_uuid, &expected);
        assert!(result.valid);

        let invalid_uuid = HashMap::from([("id".to_string(), "not-a-uuid".to_string())]);
        let result = validator.validate_path_params(&invalid_uuid, &expected);
        assert!(!result.valid);
    }

    #[test]
    fn test_validation_result_has_errors() {
        let result = ValidationResult::success(None);
        assert!(!result.has_errors());

        let result = ValidationResult::failure(
            vec![ValidationError {
                path: "test".to_string(),
                message: "error".to_string(),
                schema_path: None,
                value: None,
            }],
            None,
        );
        assert!(result.has_errors());
    }
}
