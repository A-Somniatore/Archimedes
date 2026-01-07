//! Request and response validation middleware.
//!
//! This middleware validates incoming requests and outgoing responses against
//! contract schemas. In the mock implementation, it uses simple schema definitions.
//! In production, this will integrate with Themis contract artifacts.
//!
//! # Pipeline Position
//!
//! Request validation runs after Authorization and before the Handler:
//!
//! ```text
//! Request → RequestId → Tracing → Identity → Authorization → [Validation] → Handler
//! ```
//!
//! Response validation runs after the Handler and before Telemetry:
//!
//! ```text
//! Handler → [ResponseValidation] → Telemetry → ErrorNormalization → Response
//! ```
//!
//! # Mock Implementation
//!
//! The mock validation middleware supports:
//!
//! - Allow-all mode (for development/testing)
//! - Reject-all mode (for testing validation errors)
//! - Operation-based schemas with field validation
//! - Required field checking
//! - Type validation (string, integer, boolean, array, object)
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_middleware::stages::ValidationMiddleware;
//!
//! // Allow all requests (development mode)
//! let allow_all = ValidationMiddleware::allow_all();
//!
//! // Reject all requests (testing)
//! let reject_all = ValidationMiddleware::reject_all();
//!
//! // Schema-based validation
//! let schema_based = ValidationMiddleware::with_schemas()
//!     .add_request_schema("createUser", user_schema())
//!     .add_response_schema("createUser", user_response_schema())
//!     .build();
//! ```
//!
//! # Production Integration
//!
//! In production, this middleware will:
//!
//! 1. Load schemas from Themis contract artifacts
//! 2. Validate request bodies against operation request schemas
//! 3. Validate response bodies against operation response schemas
//! 4. Return structured validation errors on failure

use crate::{
    context::MiddlewareContext,
    middleware::{BoxFuture, Middleware, Next},
    types::{Request, Response, ResponseExt},
};
use http::StatusCode;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "sentinel")]
use archimedes_sentinel::Sentinel;

/// Request validation middleware that validates against contract schemas.
///
/// This middleware supports multiple validation modes:
///
/// - **AllowAll**: Allow all requests (development mode)
/// - **RejectAll**: Reject all requests (testing)
/// - **Schema**: Mock schema-based validation
/// - **Sentinel**: Real contract validation via archimedes-sentinel (requires `sentinel` feature)
#[derive(Clone)]
pub struct ValidationMiddleware {
    /// The validation mode.
    mode: ValidationMode,
}

impl std::fmt::Debug for ValidationMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationMiddleware")
            .field("mode", &self.mode.name())
            .finish()
    }
}

/// Response validation middleware that validates handler responses.
#[derive(Clone)]
pub struct ResponseValidationMiddleware {
    /// The validation mode.
    mode: ValidationMode,
    /// Whether to enforce validation or just log.
    enforce: bool,
}

impl std::fmt::Debug for ResponseValidationMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResponseValidationMiddleware")
            .field("mode", &self.mode.name())
            .field("enforce", &self.enforce)
            .finish()
    }
}

/// Validation mode configuration.
#[derive(Clone)]
enum ValidationMode {
    /// Allow all requests/responses (development mode).
    AllowAll,
    /// Reject all requests/responses (testing).
    RejectAll,
    /// Schema-based validation.
    Schema(Arc<SchemaConfig>),
    /// Sentinel contract validation (requires `sentinel` feature).
    #[cfg(feature = "sentinel")]
    Sentinel(Arc<Sentinel>),
}

impl ValidationMode {
    fn name(&self) -> &'static str {
        match self {
            Self::AllowAll => "allow_all",
            Self::RejectAll => "reject_all",
            Self::Schema(_) => "schema",
            #[cfg(feature = "sentinel")]
            Self::Sentinel(_) => "sentinel",
        }
    }
}

/// Schema configuration for validation.
#[derive(Debug, Default)]
struct SchemaConfig {
    /// Request schemas by operation ID.
    request_schemas: HashMap<String, MockSchema>,
    /// Response schemas by operation ID.
    response_schemas: HashMap<String, MockSchema>,
}

/// A mock schema for validation.
///
/// This is a simplified schema that supports basic validation.
/// Production will use JSON Schema from Themis contracts.
#[derive(Debug, Clone)]
pub struct MockSchema {
    /// Required fields for the schema.
    required_fields: Vec<String>,
    /// Field types (field name -> expected type).
    field_types: HashMap<String, FieldType>,
    /// Whether to allow additional fields.
    allow_additional: bool,
}

/// Field type for mock schema validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    /// String type.
    String,
    /// Integer type.
    Integer,
    /// Number type (float).
    Number,
    /// Boolean type.
    Boolean,
    /// Array type.
    Array,
    /// Object type.
    Object,
    /// Any type (no validation).
    Any,
}

/// Validation result with details.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub valid: bool,
    /// Validation errors if any.
    pub errors: Vec<ValidationError>,
}

/// A single validation error.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// The field path that failed validation.
    pub field: String,
    /// The error message.
    pub message: String,
    /// The error code.
    pub code: String,
}

// ============================================================================
// ValidationMiddleware Implementation
// ============================================================================

impl ValidationMiddleware {
    /// Creates a new validation middleware that allows all requests.
    ///
    /// Use this for development or when validation is handled elsewhere.
    #[must_use]
    pub fn allow_all() -> Self {
        Self {
            mode: ValidationMode::AllowAll,
        }
    }

    /// Creates a new validation middleware that rejects all requests.
    ///
    /// Use this for testing validation error handling.
    #[must_use]
    pub fn reject_all() -> Self {
        Self {
            mode: ValidationMode::RejectAll,
        }
    }

    /// Creates a new schema-based validation middleware builder.
    #[must_use]
    pub fn with_schemas() -> ValidationBuilder {
        ValidationBuilder::default()
    }

    /// Creates a new validation middleware using Themis contract artifacts.
    ///
    /// This requires the `sentinel` feature to be enabled.
    ///
    /// # Arguments
    ///
    /// * `sentinel` - A pre-configured `Sentinel` from `archimedes-sentinel`
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use archimedes_sentinel::{Sentinel, ArtifactLoader};
    /// use archimedes_middleware::stages::ValidationMiddleware;
    ///
    /// let artifact = ArtifactLoader::from_file("contract.artifact.json").await?;
    /// let sentinel = Sentinel::with_defaults(artifact);
    /// let middleware = ValidationMiddleware::sentinel(sentinel);
    /// ```
    #[cfg(feature = "sentinel")]
    #[must_use]
    pub fn sentinel(sentinel: Sentinel) -> Self {
        Self {
            mode: ValidationMode::Sentinel(Arc::new(sentinel)),
        }
    }

    /// Validates the request body against the operation schema.
    fn validate_request(&self, operation_id: &str, body: &[u8]) -> ValidationResult {
        match &self.mode {
            ValidationMode::AllowAll => ValidationResult {
                valid: true,
                errors: vec![],
            },
            ValidationMode::RejectAll => ValidationResult {
                valid: false,
                errors: vec![ValidationError {
                    field: "".to_string(),
                    message: "Validation rejected (reject-all mode)".to_string(),
                    code: "VALIDATION_REJECTED".to_string(),
                }],
            },
            ValidationMode::Schema(config) => {
                if let Some(schema) = config.request_schemas.get(operation_id) {
                    Self::validate_body(body, schema)
                } else {
                    // No schema defined, allow by default
                    ValidationResult {
                        valid: true,
                        errors: vec![],
                    }
                }
            }
            #[cfg(feature = "sentinel")]
            ValidationMode::Sentinel(sentinel) => {
                Self::validate_with_sentinel(sentinel, operation_id, body)
            }
        }
    }

    /// Validates request body using Sentinel.
    #[cfg(feature = "sentinel")]
    fn validate_with_sentinel(
        sentinel: &Sentinel,
        operation_id: &str,
        body: &[u8],
    ) -> ValidationResult {
        // Parse body as JSON
        let json_body: serde_json::Value = if body.is_empty() {
            serde_json::Value::Null
        } else {
            match serde_json::from_slice(body) {
                Ok(v) => v,
                Err(e) => {
                    return ValidationResult {
                        valid: false,
                        errors: vec![ValidationError {
                            field: "".to_string(),
                            message: format!("Invalid JSON: {e}"),
                            code: "INVALID_JSON".to_string(),
                        }],
                    };
                }
            }
        };

        // Validate using sentinel
        match sentinel.validate_request(operation_id, &json_body) {
            Ok(result) => {
                if result.valid {
                    ValidationResult {
                        valid: true,
                        errors: vec![],
                    }
                } else {
                    ValidationResult {
                        valid: false,
                        errors: result
                            .errors
                            .into_iter()
                            .map(|e| ValidationError {
                                field: e.path,
                                message: e.message,
                                code: "SCHEMA_VALIDATION_ERROR".to_string(),
                            })
                            .collect(),
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Sentinel validation error");
                ValidationResult {
                    valid: false,
                    errors: vec![ValidationError {
                        field: "".to_string(),
                        message: format!("Validation error: {e}"),
                        code: "VALIDATION_ERROR".to_string(),
                    }],
                }
            }
        }
    }

    /// Validates a body against a schema.
    fn validate_body(body: &[u8], schema: &MockSchema) -> ValidationResult {
        // Empty body handling
        if body.is_empty() {
            if schema.required_fields.is_empty() {
                return ValidationResult {
                    valid: true,
                    errors: vec![],
                };
            }
            return ValidationResult {
                valid: false,
                errors: vec![ValidationError {
                    field: "".to_string(),
                    message: "Request body is required".to_string(),
                    code: "BODY_REQUIRED".to_string(),
                }],
            };
        }

        // Parse JSON
        let value: Value = match serde_json::from_slice(body) {
            Ok(v) => v,
            Err(e) => {
                return ValidationResult {
                    valid: false,
                    errors: vec![ValidationError {
                        field: "".to_string(),
                        message: format!("Invalid JSON: {e}"),
                        code: "INVALID_JSON".to_string(),
                    }],
                };
            }
        };

        // Must be an object
        let obj = match value.as_object() {
            Some(o) => o,
            None => {
                return ValidationResult {
                    valid: false,
                    errors: vec![ValidationError {
                        field: "".to_string(),
                        message: "Request body must be an object".to_string(),
                        code: "BODY_NOT_OBJECT".to_string(),
                    }],
                };
            }
        };

        let mut errors = Vec::new();

        // Check required fields
        for field in &schema.required_fields {
            if !obj.contains_key(field) {
                errors.push(ValidationError {
                    field: field.clone(),
                    message: format!("Missing required field: {field}"),
                    code: "FIELD_REQUIRED".to_string(),
                });
            }
        }

        // Check field types
        for (field, value) in obj {
            if let Some(expected_type) = schema.field_types.get(field) {
                if !Self::check_type(value, expected_type) {
                    errors.push(ValidationError {
                        field: field.clone(),
                        message: format!("Field '{field}' has invalid type, expected {expected_type:?}"),
                        code: "INVALID_TYPE".to_string(),
                    });
                }
            } else if !schema.allow_additional {
                errors.push(ValidationError {
                    field: field.clone(),
                    message: format!("Unexpected field: {field}"),
                    code: "UNEXPECTED_FIELD".to_string(),
                });
            }
        }

        ValidationResult {
            valid: errors.is_empty(),
            errors,
        }
    }

    /// Checks if a value matches the expected type.
    fn check_type(value: &Value, expected: &FieldType) -> bool {
        match expected {
            FieldType::String => value.is_string(),
            FieldType::Integer => value.is_i64() || value.is_u64(),
            FieldType::Number => value.is_number(),
            FieldType::Boolean => value.is_boolean(),
            FieldType::Array => value.is_array(),
            FieldType::Object => value.is_object(),
            FieldType::Any => true,
        }
    }
}

impl Middleware for ValidationMiddleware {
    fn name(&self) -> &'static str {
        "request_validation"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            let operation_id = ctx.operation_id().unwrap_or("unknown").to_string();

            // Get request body for validation
            // In a real implementation, we'd read and buffer the body
            // For mock, we'll use an empty body check or stored body
            let body = request
                .extensions()
                .get::<RequestBody>()
                .map(|b| b.0.as_slice())
                .unwrap_or(&[]);

            let result = self.validate_request(&operation_id, body);

            // Store validation result in context
            ctx.set_extension(result.clone());

            if !result.valid {
                // Return validation error response
                let first_error = result.errors.first();
                let code = first_error
                    .map(|e| e.code.as_str())
                    .unwrap_or("VALIDATION_ERROR");
                let message = first_error
                    .map(|e| e.message.as_str())
                    .unwrap_or("Request validation failed");

                return Response::json_error(StatusCode::BAD_REQUEST, code, message);
            }

            // Continue to next middleware/handler
            next.run(ctx, request).await
        })
    }
}

// ============================================================================
// ResponseValidationMiddleware Implementation
// ============================================================================

impl ResponseValidationMiddleware {
    /// Creates a new response validation middleware that allows all responses.
    #[must_use]
    pub fn allow_all() -> Self {
        Self {
            mode: ValidationMode::AllowAll,
            enforce: false,
        }
    }

    /// Creates a new response validation middleware that rejects all responses.
    #[must_use]
    pub fn reject_all() -> Self {
        Self {
            mode: ValidationMode::RejectAll,
            enforce: true,
        }
    }

    /// Creates a new schema-based response validation middleware builder.
    #[must_use]
    pub fn with_schemas() -> ResponseValidationBuilder {
        ResponseValidationBuilder::default()
    }

    /// Creates a new response validation middleware using Themis contract artifacts.
    ///
    /// This requires the `sentinel` feature to be enabled.
    #[cfg(feature = "sentinel")]
    #[must_use]
    pub fn sentinel(sentinel: Sentinel, enforce: bool) -> Self {
        Self {
            mode: ValidationMode::Sentinel(Arc::new(sentinel)),
            enforce,
        }
    }

    /// Sets whether to enforce validation (return error) or just log.
    #[must_use]
    pub fn enforce(mut self, enforce: bool) -> Self {
        self.enforce = enforce;
        self
    }

    /// Validates the response body against the operation schema.
    fn validate_response(&self, operation_id: &str, _status_code: u16, body: &[u8]) -> ValidationResult {
        match &self.mode {
            ValidationMode::AllowAll => ValidationResult {
                valid: true,
                errors: vec![],
            },
            ValidationMode::RejectAll => ValidationResult {
                valid: false,
                errors: vec![ValidationError {
                    field: "".to_string(),
                    message: "Response validation rejected (reject-all mode)".to_string(),
                    code: "RESPONSE_VALIDATION_REJECTED".to_string(),
                }],
            },
            ValidationMode::Schema(config) => {
                if let Some(schema) = config.response_schemas.get(operation_id) {
                    ValidationMiddleware::validate_body(body, schema)
                } else {
                    // No schema defined, allow by default
                    ValidationResult {
                        valid: true,
                        errors: vec![],
                    }
                }
            }
            #[cfg(feature = "sentinel")]
            ValidationMode::Sentinel(sentinel) => {
                Self::validate_response_with_sentinel(sentinel, operation_id, _status_code, body)
            }
        }
    }

    /// Validates response body using Sentinel.
    #[cfg(feature = "sentinel")]
    fn validate_response_with_sentinel(
        sentinel: &Sentinel,
        operation_id: &str,
        status_code: u16,
        body: &[u8],
    ) -> ValidationResult {
        // Parse body as JSON
        let json_body: serde_json::Value = if body.is_empty() {
            serde_json::Value::Null
        } else {
            match serde_json::from_slice(body) {
                Ok(v) => v,
                Err(e) => {
                    return ValidationResult {
                        valid: false,
                        errors: vec![ValidationError {
                            field: "".to_string(),
                            message: format!("Invalid JSON response: {e}"),
                            code: "INVALID_JSON".to_string(),
                        }],
                    };
                }
            }
        };

        // Validate using sentinel
        match sentinel.validate_response(operation_id, status_code, &json_body) {
            Ok(result) => {
                if result.valid {
                    ValidationResult {
                        valid: true,
                        errors: vec![],
                    }
                } else {
                    ValidationResult {
                        valid: false,
                        errors: result
                            .errors
                            .into_iter()
                            .map(|e| ValidationError {
                                field: e.path,
                                message: e.message,
                                code: "RESPONSE_SCHEMA_ERROR".to_string(),
                            })
                            .collect(),
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Sentinel response validation error");
                ValidationResult {
                    valid: false,
                    errors: vec![ValidationError {
                        field: "".to_string(),
                        message: format!("Response validation error: {e}"),
                        code: "VALIDATION_ERROR".to_string(),
                    }],
                }
            }
        }
    }
}

impl Middleware for ResponseValidationMiddleware {
    fn name(&self) -> &'static str {
        "response_validation"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            let operation_id = ctx.operation_id().unwrap_or("unknown").to_string();

            // Run the handler/next middleware first
            let response = next.run(ctx, request).await;

            // Only validate successful responses
            if !response.status().is_success() {
                return response;
            }

            // Get status code for sentinel validation
            let status_code = response.status().as_u16();

            // For mock implementation, we'd need to extract response body
            // In production, this would buffer and validate the response
            // For now, we'll use a placeholder that assumes valid responses
            let body: &[u8] = &[];

            let result = self.validate_response(&operation_id, status_code, body);

            // Store response validation result
            ctx.set_extension(ResponseValidationResult(result.clone()));

            if !result.valid && self.enforce {
                // Return internal error if response validation fails
                let first_error = result.errors.first();
                let code = first_error
                    .map(|e| e.code.as_str())
                    .unwrap_or("INTERNAL_ERROR");
                let message = first_error
                    .map(|e| e.message.as_str())
                    .unwrap_or("Response validation failed");

                return Response::json_error(StatusCode::INTERNAL_SERVER_ERROR, code, message);
            }

            response
        })
    }
}

// ============================================================================
// Builders
// ============================================================================

/// Builder for `ValidationMiddleware`.
#[derive(Debug, Default)]
pub struct ValidationBuilder {
    config: SchemaConfig,
}

impl ValidationBuilder {
    /// Adds a request schema for an operation.
    #[must_use]
    pub fn add_request_schema(mut self, operation_id: &str, schema: MockSchema) -> Self {
        self.config
            .request_schemas
            .insert(operation_id.to_string(), schema);
        self
    }

    /// Builds the validation middleware.
    #[must_use]
    pub fn build(self) -> ValidationMiddleware {
        ValidationMiddleware {
            mode: ValidationMode::Schema(Arc::new(self.config)),
        }
    }
}

/// Builder for `ResponseValidationMiddleware`.
#[derive(Debug, Default)]
pub struct ResponseValidationBuilder {
    config: SchemaConfig,
    enforce: bool,
}

impl ResponseValidationBuilder {
    /// Adds a response schema for an operation.
    #[must_use]
    pub fn add_response_schema(mut self, operation_id: &str, schema: MockSchema) -> Self {
        self.config
            .response_schemas
            .insert(operation_id.to_string(), schema);
        self
    }

    /// Sets whether to enforce validation.
    #[must_use]
    pub fn enforce(mut self, enforce: bool) -> Self {
        self.enforce = enforce;
        self
    }

    /// Builds the response validation middleware.
    #[must_use]
    pub fn build(self) -> ResponseValidationMiddleware {
        ResponseValidationMiddleware {
            mode: ValidationMode::Schema(Arc::new(self.config)),
            enforce: self.enforce,
        }
    }
}

// ============================================================================
// MockSchema Builder
// ============================================================================

impl MockSchema {
    /// Creates a new empty schema builder.
    #[must_use]
    pub fn builder() -> MockSchemaBuilder {
        MockSchemaBuilder::default()
    }

    /// Creates a schema that accepts any object.
    #[must_use]
    pub fn any() -> Self {
        Self {
            required_fields: vec![],
            field_types: HashMap::new(),
            allow_additional: true,
        }
    }
}

/// Builder for `MockSchema`.
#[derive(Debug, Default)]
pub struct MockSchemaBuilder {
    required_fields: Vec<String>,
    field_types: HashMap<String, FieldType>,
    allow_additional: bool,
}

impl MockSchemaBuilder {
    /// Adds a required field.
    #[must_use]
    pub fn required(mut self, field: &str) -> Self {
        self.required_fields.push(field.to_string());
        self
    }

    /// Adds a field with a specific type.
    #[must_use]
    pub fn field(mut self, name: &str, field_type: FieldType) -> Self {
        self.field_types.insert(name.to_string(), field_type);
        self
    }

    /// Sets whether additional fields are allowed.
    #[must_use]
    pub fn allow_additional(mut self, allow: bool) -> Self {
        self.allow_additional = allow;
        self
    }

    /// Builds the schema.
    #[must_use]
    pub fn build(self) -> MockSchema {
        MockSchema {
            required_fields: self.required_fields,
            field_types: self.field_types,
            allow_additional: self.allow_additional,
        }
    }
}

// ============================================================================
// Helper Types
// ============================================================================

/// Wrapper for request body stored in extensions.
#[derive(Debug, Clone)]
pub struct RequestBody(pub Vec<u8>);

/// Wrapper for response validation result stored in extensions.
#[derive(Debug, Clone)]
pub struct ResponseValidationResult(pub ValidationResult);

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::Next;
    use bytes::Bytes;
    use http::{Request as HttpRequest, Response as HttpResponse, StatusCode};
    use http_body_util::Full;

    fn make_test_request() -> Request {
        HttpRequest::builder()
            .method("POST")
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn make_request_with_body(body: &str) -> Request {
        let mut request = HttpRequest::builder()
            .method("POST")
            .uri("/test")
            .body(Full::new(Bytes::from(body.to_string())))
            .unwrap();

        // Store body in extensions for validation
        request
            .extensions_mut()
            .insert(RequestBody(body.as_bytes().to_vec()));

        request
    }

    fn success_response() -> Response {
        HttpResponse::builder()
            .status(StatusCode::OK)
            .body(Full::new(Bytes::from(r#"{"status":"ok"}"#)))
            .unwrap()
    }

    fn create_handler() -> impl FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> {
        |_ctx, _req| Box::pin(async { success_response() })
    }

    #[test]
    fn test_middleware_name() {
        let middleware = ValidationMiddleware::allow_all();
        assert_eq!(middleware.name(), "request_validation");

        let response_middleware = ResponseValidationMiddleware::allow_all();
        assert_eq!(response_middleware.name(), "response_validation");
    }

    #[tokio::test]
    async fn test_allow_all_permits_any_request() {
        let middleware = ValidationMiddleware::allow_all();
        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("testOp".to_string());

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_reject_all_rejects_any_request() {
        let middleware = ValidationMiddleware::reject_all();
        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("testOp".to_string());

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_schema_validates_required_fields() {
        let schema = MockSchema::builder()
            .required("name")
            .required("email")
            .field("name", FieldType::String)
            .field("email", FieldType::String)
            .allow_additional(true)
            .build();

        let middleware = ValidationMiddleware::with_schemas()
            .add_request_schema("createUser", schema)
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("createUser".to_string());

        // Missing required field
        let request = make_request_with_body(r#"{"name": "Alice"}"#);
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_schema_validates_field_types() {
        let schema = MockSchema::builder()
            .required("name")
            .required("age")
            .field("name", FieldType::String)
            .field("age", FieldType::Integer)
            .allow_additional(true)
            .build();

        let middleware = ValidationMiddleware::with_schemas()
            .add_request_schema("createUser", schema)
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("createUser".to_string());

        // Wrong type for age
        let request = make_request_with_body(r#"{"name": "Alice", "age": "twenty"}"#);
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_schema_passes_valid_request() {
        let schema = MockSchema::builder()
            .required("name")
            .required("email")
            .field("name", FieldType::String)
            .field("email", FieldType::String)
            .allow_additional(true)
            .build();

        let middleware = ValidationMiddleware::with_schemas()
            .add_request_schema("createUser", schema)
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("createUser".to_string());

        let request = make_request_with_body(r#"{"name": "Alice", "email": "alice@example.com"}"#);
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_schema_rejects_unexpected_fields() {
        let schema = MockSchema::builder()
            .required("name")
            .field("name", FieldType::String)
            .allow_additional(false)
            .build();

        let middleware = ValidationMiddleware::with_schemas()
            .add_request_schema("createUser", schema)
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("createUser".to_string());

        let request = make_request_with_body(r#"{"name": "Alice", "extra": "field"}"#);
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_no_schema_allows_request() {
        let middleware = ValidationMiddleware::with_schemas().build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("unknownOp".to_string());

        let request = make_request_with_body(r#"{"anything": "goes"}"#);
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_invalid_json_rejected() {
        let schema = MockSchema::builder()
            .required("name")
            .field("name", FieldType::String)
            .build();

        let middleware = ValidationMiddleware::with_schemas()
            .add_request_schema("createUser", schema)
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("createUser".to_string());

        let request = make_request_with_body(r#"{ invalid json }"#);
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_non_object_body_rejected() {
        let schema = MockSchema::builder()
            .required("name")
            .field("name", FieldType::String)
            .build();

        let middleware = ValidationMiddleware::with_schemas()
            .add_request_schema("createUser", schema)
            .build();

        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("createUser".to_string());

        let request = make_request_with_body(r#"["array", "not", "object"]"#);
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_response_validation_allow_all() {
        let middleware = ResponseValidationMiddleware::allow_all();
        let mut ctx = MiddlewareContext::new();
        ctx.set_operation_id("testOp".to_string());

        let request = make_test_request();
        let next = Next::handler(create_handler());

        let response = middleware.process(&mut ctx, request, next).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_field_type_validation() {
        // Test all field types
        let result = ValidationMiddleware::check_type(&serde_json::json!("test"), &FieldType::String);
        assert!(result);

        let result = ValidationMiddleware::check_type(&serde_json::json!(42), &FieldType::Integer);
        assert!(result);

        let result = ValidationMiddleware::check_type(&serde_json::json!(3.14), &FieldType::Number);
        assert!(result);

        let result = ValidationMiddleware::check_type(&serde_json::json!(true), &FieldType::Boolean);
        assert!(result);

        let result = ValidationMiddleware::check_type(&serde_json::json!([1, 2, 3]), &FieldType::Array);
        assert!(result);

        let result = ValidationMiddleware::check_type(&serde_json::json!({"key": "value"}), &FieldType::Object);
        assert!(result);

        let result = ValidationMiddleware::check_type(&serde_json::json!(null), &FieldType::Any);
        assert!(result);
    }

    #[test]
    fn test_mock_schema_any() {
        let schema = MockSchema::any();
        assert!(schema.required_fields.is_empty());
        assert!(schema.field_types.is_empty());
        assert!(schema.allow_additional);
    }

    #[test]
    fn test_validation_result_structure() {
        let result = ValidationResult {
            valid: false,
            errors: vec![ValidationError {
                field: "email".to_string(),
                message: "Invalid email format".to_string(),
                code: "INVALID_FORMAT".to_string(),
            }],
        };

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].field, "email");
    }
}
