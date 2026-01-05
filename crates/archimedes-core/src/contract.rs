//! Mock contract types for parallel development.
//!
//! This module provides mock implementations of Themis contract types that allow
//! Archimedes to be developed and tested without requiring the actual Themis
//! contract compiler or runtime.
//!
//! These mocks will be replaced with real Themis contract integration in Phase A5.
//!
//! # Example
//!
//! ```
//! use archimedes_core::contract::{Contract, Operation, MockSchema};
//! use http::Method;
//!
//! // Create a mock contract for testing
//! let contract = Contract::builder("user-service")
//!     .version("1.0.0")
//!     .operation(
//!         Operation::builder("getUser")
//!             .method(Method::GET)
//!             .path("/users/{userId}")
//!             .build()
//!     )
//!     .operation(
//!         Operation::builder("createUser")
//!             .method(Method::POST)
//!             .path("/users")
//!             .request_schema(MockSchema::object(vec![
//!                 ("name", MockSchema::string().required()),
//!                 ("email", MockSchema::string().required()),
//!             ]))
//!             .build()
//!     )
//!     .build();
//!
//! assert_eq!(contract.operations().len(), 2);
//! ```

use http::Method;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A mock Themis contract for testing and parallel development.
///
/// This struct represents a simplified version of a Themis contract,
/// containing the service metadata and operation definitions.
///
/// # Note
///
/// This is a mock implementation. In production, contracts will be loaded
/// from compiled Themis contract artifacts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    /// The service name this contract defines.
    name: String,
    /// The contract version.
    version: String,
    /// Operations defined in this contract.
    operations: Vec<Operation>,
    /// Operation lookup by ID for fast access.
    #[serde(skip)]
    operation_index: HashMap<String, usize>,
}

impl Contract {
    /// Creates a new contract builder.
    ///
    /// # Arguments
    ///
    /// * `name` - The service name
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_core::contract::Contract;
    ///
    /// let contract = Contract::builder("my-service")
    ///     .version("1.0.0")
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder(name: impl Into<String>) -> ContractBuilder {
        ContractBuilder::new(name)
    }

    /// Returns the service name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the contract version.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns all operations defined in this contract.
    #[must_use]
    pub fn operations(&self) -> &[Operation] {
        &self.operations
    }

    /// Looks up an operation by its ID.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The unique operation identifier
    ///
    /// # Returns
    ///
    /// The operation if found, or `None` if no operation matches.
    #[must_use]
    pub fn get_operation(&self, operation_id: &str) -> Option<&Operation> {
        self.operation_index
            .get(operation_id)
            .map(|&idx| &self.operations[idx])
    }

    /// Finds an operation by HTTP method and path.
    ///
    /// This performs path matching including path parameters.
    ///
    /// # Arguments
    ///
    /// * `method` - The HTTP method
    /// * `path` - The request path (e.g., "/users/123")
    ///
    /// # Returns
    ///
    /// The matching operation and extracted path parameters, or `None`.
    #[must_use]
    pub fn match_operation(
        &self,
        method: &Method,
        path: &str,
    ) -> Option<(&Operation, HashMap<String, String>)> {
        for operation in &self.operations {
            if operation.method() == method {
                if let Some(params) = operation.match_path(path) {
                    return Some((operation, params));
                }
            }
        }
        None
    }

    /// Rebuilds the operation index after deserialization.
    fn rebuild_index(&mut self) {
        self.operation_index.clear();
        for (idx, op) in self.operations.iter().enumerate() {
            self.operation_index.insert(op.operation_id.clone(), idx);
        }
    }
}

/// Builder for creating [`Contract`] instances.
#[derive(Debug)]
pub struct ContractBuilder {
    name: String,
    version: String,
    operations: Vec<Operation>,
}

impl ContractBuilder {
    /// Creates a new contract builder.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: "0.0.0".to_string(),
            operations: Vec::new(),
        }
    }

    /// Sets the contract version.
    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Adds an operation to the contract.
    #[must_use]
    pub fn operation(mut self, operation: Operation) -> Self {
        self.operations.push(operation);
        self
    }

    /// Adds multiple operations to the contract.
    #[must_use]
    pub fn operations(mut self, operations: impl IntoIterator<Item = Operation>) -> Self {
        self.operations.extend(operations);
        self
    }

    /// Builds the contract.
    #[must_use]
    pub fn build(self) -> Contract {
        let mut contract = Contract {
            name: self.name,
            version: self.version,
            operations: self.operations,
            operation_index: HashMap::new(),
        };
        contract.rebuild_index();
        contract
    }
}

/// An operation defined in a contract.
///
/// Operations map to API endpoints and define the request/response schemas,
/// HTTP method, path pattern, and other metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique identifier for this operation (e.g., "getUser", "createOrder").
    operation_id: String,
    /// HTTP method for this operation.
    #[serde(with = "http_method_serde")]
    method: Method,
    /// Path pattern with parameter placeholders (e.g., "/users/{userId}").
    path: String,
    /// Parsed path segments for matching.
    #[serde(skip)]
    path_segments: Vec<PathSegment>,
    /// Request body schema (if any).
    request_schema: Option<MockSchema>,
    /// Response body schema.
    response_schema: Option<MockSchema>,
    /// Human-readable description.
    description: Option<String>,
    /// Tags for grouping operations.
    #[serde(default)]
    tags: Vec<String>,
    /// Whether this operation requires authentication.
    #[serde(default = "default_true")]
    requires_auth: bool,
}

fn default_true() -> bool {
    true
}

impl Operation {
    /// Creates a new operation builder.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - Unique identifier for the operation
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_core::contract::Operation;
    /// use http::Method;
    ///
    /// let operation = Operation::builder("getUser")
    ///     .method(Method::GET)
    ///     .path("/users/{userId}")
    ///     .description("Retrieves a user by ID")
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder(operation_id: impl Into<String>) -> OperationBuilder {
        OperationBuilder::new(operation_id)
    }

    /// Returns the operation ID.
    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    /// Returns the HTTP method.
    #[must_use]
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Returns the path pattern.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the request schema if defined.
    #[must_use]
    pub fn request_schema(&self) -> Option<&MockSchema> {
        self.request_schema.as_ref()
    }

    /// Returns the response schema if defined.
    #[must_use]
    pub fn response_schema(&self) -> Option<&MockSchema> {
        self.response_schema.as_ref()
    }

    /// Returns the operation description.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the operation tags.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Returns whether this operation requires authentication.
    #[must_use]
    pub fn requires_auth(&self) -> bool {
        self.requires_auth
    }

    /// Attempts to match a request path against this operation's path pattern.
    ///
    /// Returns the extracted path parameters if the path matches.
    ///
    /// # Arguments
    ///
    /// * `request_path` - The actual request path (e.g., "/users/123")
    ///
    /// # Returns
    ///
    /// A map of parameter names to values if the path matches, or `None`.
    #[must_use]
    pub fn match_path(&self, request_path: &str) -> Option<HashMap<String, String>> {
        let request_segments: Vec<&str> = request_path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if request_segments.len() != self.path_segments.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (pattern, actual) in self.path_segments.iter().zip(request_segments.iter()) {
            match pattern {
                PathSegment::Literal(lit) => {
                    if lit != *actual {
                        return None;
                    }
                }
                PathSegment::Parameter(name) => {
                    params.insert(name.clone(), (*actual).to_string());
                }
            }
        }

        Some(params)
    }

    /// Parses a path pattern into segments.
    fn parse_path(path: &str) -> Vec<PathSegment> {
        path.trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|segment| {
                if segment.starts_with('{') && segment.ends_with('}') {
                    PathSegment::Parameter(segment[1..segment.len() - 1].to_string())
                } else {
                    PathSegment::Literal(segment.to_string())
                }
            })
            .collect()
    }
}

/// Builder for creating [`Operation`] instances.
#[derive(Debug)]
pub struct OperationBuilder {
    operation_id: String,
    method: Method,
    path: String,
    request_schema: Option<MockSchema>,
    response_schema: Option<MockSchema>,
    description: Option<String>,
    tags: Vec<String>,
    requires_auth: bool,
}

impl OperationBuilder {
    /// Creates a new operation builder.
    #[must_use]
    pub fn new(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            method: Method::GET,
            path: "/".to_string(),
            request_schema: None,
            response_schema: None,
            description: None,
            tags: Vec::new(),
            requires_auth: true,
        }
    }

    /// Sets the HTTP method.
    #[must_use]
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Sets the path pattern.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Sets the request body schema.
    #[must_use]
    pub fn request_schema(mut self, schema: MockSchema) -> Self {
        self.request_schema = Some(schema);
        self
    }

    /// Sets the response body schema.
    #[must_use]
    pub fn response_schema(mut self, schema: MockSchema) -> Self {
        self.response_schema = Some(schema);
        self
    }

    /// Sets the operation description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds a tag to the operation.
    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Sets whether authentication is required.
    #[must_use]
    pub fn requires_auth(mut self, requires: bool) -> Self {
        self.requires_auth = requires;
        self
    }

    /// Marks this operation as not requiring authentication.
    #[must_use]
    pub fn no_auth(mut self) -> Self {
        self.requires_auth = false;
        self
    }

    /// Builds the operation.
    #[must_use]
    pub fn build(self) -> Operation {
        let path_segments = Operation::parse_path(&self.path);
        Operation {
            operation_id: self.operation_id,
            method: self.method,
            path: self.path,
            path_segments,
            request_schema: self.request_schema,
            response_schema: self.response_schema,
            description: self.description,
            tags: self.tags,
            requires_auth: self.requires_auth,
        }
    }
}

/// A path segment in an operation's path pattern.
#[derive(Debug, Clone)]
enum PathSegment {
    /// A literal path segment (e.g., "users").
    Literal(String),
    /// A path parameter (e.g., "{userId}").
    Parameter(String),
}

/// A mock JSON schema for request/response validation.
///
/// This provides a simplified schema system for testing. In production,
/// Themis contracts will provide full JSON Schema validation.
///
/// # Example
///
/// ```
/// use archimedes_core::contract::MockSchema;
///
/// // Define a user schema
/// let schema = MockSchema::object(vec![
///     ("id", MockSchema::string().required()),
///     ("name", MockSchema::string().required()),
///     ("age", MockSchema::integer()),
///     ("email", MockSchema::string()),
/// ]);
///
/// // Validate some JSON
/// let valid = serde_json::json!({
///     "id": "user-123",
///     "name": "Alice",
///     "age": 30
/// });
///
/// assert!(schema.validate(&valid).is_ok());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MockSchema {
    /// String type.
    String {
        /// Whether this field is required.
        #[serde(default)]
        required: bool,
        /// Minimum length.
        min_length: Option<usize>,
        /// Maximum length.
        max_length: Option<usize>,
        /// Pattern (regex) - stored but not enforced in mock.
        pattern: Option<String>,
    },
    /// Integer type.
    Integer {
        /// Whether this field is required.
        #[serde(default)]
        required: bool,
        /// Minimum value.
        minimum: Option<i64>,
        /// Maximum value.
        maximum: Option<i64>,
    },
    /// Number (float) type.
    Number {
        /// Whether this field is required.
        #[serde(default)]
        required: bool,
        /// Minimum value.
        minimum: Option<f64>,
        /// Maximum value.
        maximum: Option<f64>,
    },
    /// Boolean type.
    Boolean {
        /// Whether this field is required.
        #[serde(default)]
        required: bool,
    },
    /// Array type.
    Array {
        /// Whether this field is required.
        #[serde(default)]
        required: bool,
        /// Schema for array items.
        items: Box<MockSchema>,
        /// Minimum number of items.
        min_items: Option<usize>,
        /// Maximum number of items.
        max_items: Option<usize>,
    },
    /// Object type.
    Object {
        /// Whether this field is required.
        #[serde(default)]
        required: bool,
        /// Properties and their schemas.
        properties: HashMap<String, MockSchema>,
        /// List of required property names.
        #[serde(default)]
        required_properties: Vec<String>,
    },
    /// Any type (accepts anything).
    Any {
        /// Whether this field is required.
        #[serde(default)]
        required: bool,
    },
    /// Null type.
    Null,
}

impl MockSchema {
    /// Creates a string schema.
    #[must_use]
    pub fn string() -> Self {
        Self::String {
            required: false,
            min_length: None,
            max_length: None,
            pattern: None,
        }
    }

    /// Creates an integer schema.
    #[must_use]
    pub fn integer() -> Self {
        Self::Integer {
            required: false,
            minimum: None,
            maximum: None,
        }
    }

    /// Creates a number schema.
    #[must_use]
    pub fn number() -> Self {
        Self::Number {
            required: false,
            minimum: None,
            maximum: None,
        }
    }

    /// Creates a boolean schema.
    #[must_use]
    pub fn boolean() -> Self {
        Self::Boolean { required: false }
    }

    /// Creates an array schema.
    #[must_use]
    pub fn array(items: MockSchema) -> Self {
        Self::Array {
            required: false,
            items: Box::new(items),
            min_items: None,
            max_items: None,
        }
    }

    /// Creates an object schema from a list of property definitions.
    ///
    /// # Arguments
    ///
    /// * `properties` - List of (name, schema) pairs
    #[must_use]
    pub fn object(properties: Vec<(&str, MockSchema)>) -> Self {
        let required_properties: Vec<String> = properties
            .iter()
            .filter(|(_, schema)| schema.is_required())
            .map(|(name, _)| (*name).to_string())
            .collect();

        let props: HashMap<String, MockSchema> = properties
            .into_iter()
            .map(|(name, schema)| (name.to_string(), schema))
            .collect();

        Self::Object {
            required: false,
            properties: props,
            required_properties,
        }
    }

    /// Creates an "any" schema that accepts any value.
    #[must_use]
    pub fn any() -> Self {
        Self::Any { required: false }
    }

    /// Creates a null schema.
    #[must_use]
    pub fn null() -> Self {
        Self::Null
    }

    /// Marks this schema as required.
    #[must_use]
    pub fn required(self) -> Self {
        match self {
            Self::String {
                min_length,
                max_length,
                pattern,
                ..
            } => Self::String {
                required: true,
                min_length,
                max_length,
                pattern,
            },
            Self::Integer {
                minimum, maximum, ..
            } => Self::Integer {
                required: true,
                minimum,
                maximum,
            },
            Self::Number {
                minimum, maximum, ..
            } => Self::Number {
                required: true,
                minimum,
                maximum,
            },
            Self::Boolean { .. } => Self::Boolean { required: true },
            Self::Array {
                items,
                min_items,
                max_items,
                ..
            } => Self::Array {
                required: true,
                items,
                min_items,
                max_items,
            },
            Self::Object {
                properties,
                required_properties,
                ..
            } => Self::Object {
                required: true,
                properties,
                required_properties,
            },
            Self::Any { .. } => Self::Any { required: true },
            Self::Null => Self::Null,
        }
    }

    /// Returns whether this schema is marked as required.
    #[must_use]
    pub fn is_required(&self) -> bool {
        match self {
            Self::String { required, .. }
            | Self::Integer { required, .. }
            | Self::Number { required, .. }
            | Self::Boolean { required, .. }
            | Self::Array { required, .. }
            | Self::Object { required, .. }
            | Self::Any { required, .. } => *required,
            Self::Null => false,
        }
    }

    /// Sets the minimum length for string schemas.
    #[must_use]
    pub fn min_length(self, len: usize) -> Self {
        match self {
            Self::String {
                required,
                max_length,
                pattern,
                ..
            } => Self::String {
                required,
                min_length: Some(len),
                max_length,
                pattern,
            },
            other => other,
        }
    }

    /// Sets the maximum length for string schemas.
    #[must_use]
    pub fn max_length(self, len: usize) -> Self {
        match self {
            Self::String {
                required,
                min_length,
                pattern,
                ..
            } => Self::String {
                required,
                min_length,
                max_length: Some(len),
                pattern,
            },
            other => other,
        }
    }

    /// Sets the minimum value for integer schemas.
    #[must_use]
    pub fn minimum_int(self, min: i64) -> Self {
        match self {
            Self::Integer {
                required, maximum, ..
            } => Self::Integer {
                required,
                minimum: Some(min),
                maximum,
            },
            other => other,
        }
    }

    /// Sets the maximum value for integer schemas.
    #[must_use]
    pub fn maximum_int(self, max: i64) -> Self {
        match self {
            Self::Integer {
                required, minimum, ..
            } => Self::Integer {
                required,
                minimum,
                maximum: Some(max),
            },
            other => other,
        }
    }

    /// Sets the minimum items for array schemas.
    #[must_use]
    pub fn min_items(self, min: usize) -> Self {
        match self {
            Self::Array {
                required,
                items,
                max_items,
                ..
            } => Self::Array {
                required,
                items,
                min_items: Some(min),
                max_items,
            },
            other => other,
        }
    }

    /// Sets the maximum items for array schemas.
    #[must_use]
    pub fn max_items(self, max: usize) -> Self {
        match self {
            Self::Array {
                required,
                items,
                min_items,
                ..
            } => Self::Array {
                required,
                items,
                min_items,
                max_items: Some(max),
            },
            other => other,
        }
    }

    /// Validates a JSON value against this schema.
    ///
    /// # Arguments
    ///
    /// * `value` - The JSON value to validate
    ///
    /// # Returns
    ///
    /// `Ok(())` if validation passes, or `Err` with validation errors.
    ///
    /// # Example
    ///
    /// ```
    /// use archimedes_core::contract::MockSchema;
    ///
    /// let schema = MockSchema::string().min_length(1).required();
    /// assert!(schema.validate(&serde_json::json!("hello")).is_ok());
    /// assert!(schema.validate(&serde_json::json!("")).is_err());
    /// assert!(schema.validate(&serde_json::json!(null)).is_err());
    /// ```
    pub fn validate(&self, value: &serde_json::Value) -> Result<(), ValidationError> {
        self.validate_at_path(value, "$")
    }

    fn validate_at_path(
        &self,
        value: &serde_json::Value,
        path: &str,
    ) -> Result<(), ValidationError> {
        #[allow(unused_imports)]
        use serde_json::Value;

        // Handle null values
        if value.is_null() {
            if self.is_required() {
                return Err(ValidationError {
                    path: path.to_string(),
                    message: "required field is null".to_string(),
                });
            }
            return Ok(());
        }

        match self {
            Self::String {
                min_length,
                max_length,
                ..
            } => {
                let s = value.as_str().ok_or_else(|| ValidationError {
                    path: path.to_string(),
                    message: format!("expected string, got {}", value_type_name(value)),
                })?;

                if let Some(min) = min_length {
                    if s.len() < *min {
                        return Err(ValidationError {
                            path: path.to_string(),
                            message: format!(
                                "string length {} is less than minimum {}",
                                s.len(),
                                min
                            ),
                        });
                    }
                }

                if let Some(max) = max_length {
                    if s.len() > *max {
                        return Err(ValidationError {
                            path: path.to_string(),
                            message: format!(
                                "string length {} is greater than maximum {}",
                                s.len(),
                                max
                            ),
                        });
                    }
                }

                Ok(())
            }

            Self::Integer {
                minimum, maximum, ..
            } => {
                let n = value.as_i64().ok_or_else(|| ValidationError {
                    path: path.to_string(),
                    message: format!("expected integer, got {}", value_type_name(value)),
                })?;

                if let Some(min) = minimum {
                    if n < *min {
                        return Err(ValidationError {
                            path: path.to_string(),
                            message: format!("value {} is less than minimum {}", n, min),
                        });
                    }
                }

                if let Some(max) = maximum {
                    if n > *max {
                        return Err(ValidationError {
                            path: path.to_string(),
                            message: format!("value {} is greater than maximum {}", n, max),
                        });
                    }
                }

                Ok(())
            }

            Self::Number {
                minimum, maximum, ..
            } => {
                let n = value.as_f64().ok_or_else(|| ValidationError {
                    path: path.to_string(),
                    message: format!("expected number, got {}", value_type_name(value)),
                })?;

                if let Some(min) = minimum {
                    if n < *min {
                        return Err(ValidationError {
                            path: path.to_string(),
                            message: format!("value {} is less than minimum {}", n, min),
                        });
                    }
                }

                if let Some(max) = maximum {
                    if n > *max {
                        return Err(ValidationError {
                            path: path.to_string(),
                            message: format!("value {} is greater than maximum {}", n, max),
                        });
                    }
                }

                Ok(())
            }

            Self::Boolean { .. } => {
                if !value.is_boolean() {
                    return Err(ValidationError {
                        path: path.to_string(),
                        message: format!("expected boolean, got {}", value_type_name(value)),
                    });
                }
                Ok(())
            }

            Self::Array {
                items,
                min_items,
                max_items,
                ..
            } => {
                let arr = value.as_array().ok_or_else(|| ValidationError {
                    path: path.to_string(),
                    message: format!("expected array, got {}", value_type_name(value)),
                })?;

                if let Some(min) = min_items {
                    if arr.len() < *min {
                        return Err(ValidationError {
                            path: path.to_string(),
                            message: format!(
                                "array length {} is less than minimum {}",
                                arr.len(),
                                min
                            ),
                        });
                    }
                }

                if let Some(max) = max_items {
                    if arr.len() > *max {
                        return Err(ValidationError {
                            path: path.to_string(),
                            message: format!(
                                "array length {} is greater than maximum {}",
                                arr.len(),
                                max
                            ),
                        });
                    }
                }

                for (idx, item) in arr.iter().enumerate() {
                    let item_path = format!("{}[{}]", path, idx);
                    items.validate_at_path(item, &item_path)?;
                }

                Ok(())
            }

            Self::Object {
                properties,
                required_properties,
                ..
            } => {
                let obj = value.as_object().ok_or_else(|| ValidationError {
                    path: path.to_string(),
                    message: format!("expected object, got {}", value_type_name(value)),
                })?;

                // Check required properties
                for required in required_properties {
                    if !obj.contains_key(required) {
                        return Err(ValidationError {
                            path: format!("{}.{}", path, required),
                            message: format!("missing required property '{}'", required),
                        });
                    }
                }

                // Validate present properties
                for (key, prop_schema) in properties {
                    if let Some(prop_value) = obj.get(key) {
                        let prop_path = format!("{}.{}", path, key);
                        prop_schema.validate_at_path(prop_value, &prop_path)?;
                    }
                }

                Ok(())
            }

            Self::Any { .. } => Ok(()),

            Self::Null => {
                if !value.is_null() {
                    return Err(ValidationError {
                        path: path.to_string(),
                        message: format!("expected null, got {}", value_type_name(value)),
                    });
                }
                Ok(())
            }
        }
    }
}

/// Returns a human-readable name for a JSON value type.
fn value_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// A validation error from schema validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// The JSON path where the error occurred.
    pub path: String,
    /// The error message.
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "validation error at '{}': {}", self.path, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Serde support for HTTP methods.
mod http_method_serde {
    use http::Method;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(method: &Method, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(method.as_str())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Method, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ==================== Contract Tests ====================

    #[test]
    fn test_contract_builder() {
        let contract = Contract::builder("user-service")
            .version("1.0.0")
            .operation(
                Operation::builder("getUser")
                    .method(Method::GET)
                    .path("/users/{userId}")
                    .build(),
            )
            .build();

        assert_eq!(contract.name(), "user-service");
        assert_eq!(contract.version(), "1.0.0");
        assert_eq!(contract.operations().len(), 1);
    }

    #[test]
    fn test_contract_get_operation() {
        let contract = Contract::builder("test")
            .operation(Operation::builder("op1").build())
            .operation(Operation::builder("op2").build())
            .build();

        assert!(contract.get_operation("op1").is_some());
        assert!(contract.get_operation("op2").is_some());
        assert!(contract.get_operation("op3").is_none());
    }

    #[test]
    fn test_contract_match_operation() {
        let contract = Contract::builder("test")
            .operation(
                Operation::builder("getUser")
                    .method(Method::GET)
                    .path("/users/{userId}")
                    .build(),
            )
            .operation(
                Operation::builder("listUsers")
                    .method(Method::GET)
                    .path("/users")
                    .build(),
            )
            .operation(
                Operation::builder("createUser")
                    .method(Method::POST)
                    .path("/users")
                    .build(),
            )
            .build();

        // Match GET /users/123
        let (op, params) = contract.match_operation(&Method::GET, "/users/123").unwrap();
        assert_eq!(op.operation_id(), "getUser");
        assert_eq!(params.get("userId"), Some(&"123".to_string()));

        // Match GET /users
        let (op, params) = contract.match_operation(&Method::GET, "/users").unwrap();
        assert_eq!(op.operation_id(), "listUsers");
        assert!(params.is_empty());

        // Match POST /users
        let (op, _) = contract.match_operation(&Method::POST, "/users").unwrap();
        assert_eq!(op.operation_id(), "createUser");

        // No match for DELETE
        assert!(contract.match_operation(&Method::DELETE, "/users").is_none());
    }

    // ==================== Operation Tests ====================

    #[test]
    fn test_operation_builder() {
        let op = Operation::builder("createUser")
            .method(Method::POST)
            .path("/users")
            .description("Creates a new user")
            .tag("users")
            .request_schema(MockSchema::object(vec![
                ("name", MockSchema::string().required()),
            ]))
            .build();

        assert_eq!(op.operation_id(), "createUser");
        assert_eq!(op.method(), Method::POST);
        assert_eq!(op.path(), "/users");
        assert_eq!(op.description(), Some("Creates a new user"));
        assert_eq!(op.tags(), &["users".to_string()]);
        assert!(op.request_schema().is_some());
        assert!(op.requires_auth());
    }

    #[test]
    fn test_operation_no_auth() {
        let op = Operation::builder("health")
            .path("/health")
            .no_auth()
            .build();

        assert!(!op.requires_auth());
    }

    #[test]
    fn test_path_matching_simple() {
        let op = Operation::builder("test").path("/users").build();

        assert!(op.match_path("/users").is_some());
        assert!(op.match_path("/users/").is_some());
        assert!(op.match_path("/other").is_none());
    }

    #[test]
    fn test_path_matching_with_params() {
        let op = Operation::builder("test")
            .path("/users/{userId}/posts/{postId}")
            .build();

        let params = op.match_path("/users/123/posts/456").unwrap();
        assert_eq!(params.get("userId"), Some(&"123".to_string()));
        assert_eq!(params.get("postId"), Some(&"456".to_string()));

        assert!(op.match_path("/users/123").is_none());
        assert!(op.match_path("/users/123/posts").is_none());
    }

    // ==================== Schema Tests ====================

    #[test]
    fn test_string_schema_validation() {
        let schema = MockSchema::string().min_length(2).max_length(10);

        assert!(schema.validate(&json!("hello")).is_ok());
        assert!(schema.validate(&json!("a")).is_err()); // too short
        assert!(schema.validate(&json!("hello world!")).is_err()); // too long
        assert!(schema.validate(&json!(123)).is_err()); // wrong type
    }

    #[test]
    fn test_string_required() {
        let schema = MockSchema::string().required();

        assert!(schema.validate(&json!("hello")).is_ok());
        assert!(schema.validate(&json!(null)).is_err());
    }

    #[test]
    fn test_integer_schema_validation() {
        let schema = MockSchema::integer().minimum_int(0).maximum_int(100);

        assert!(schema.validate(&json!(50)).is_ok());
        assert!(schema.validate(&json!(0)).is_ok());
        assert!(schema.validate(&json!(100)).is_ok());
        assert!(schema.validate(&json!(-1)).is_err());
        assert!(schema.validate(&json!(101)).is_err());
        assert!(schema.validate(&json!("50")).is_err()); // wrong type
    }

    #[test]
    fn test_boolean_schema_validation() {
        let schema = MockSchema::boolean();

        assert!(schema.validate(&json!(true)).is_ok());
        assert!(schema.validate(&json!(false)).is_ok());
        assert!(schema.validate(&json!("true")).is_err());
        assert!(schema.validate(&json!(1)).is_err());
    }

    #[test]
    fn test_array_schema_validation() {
        let schema = MockSchema::array(MockSchema::integer())
            .min_items(1)
            .max_items(3);

        assert!(schema.validate(&json!([1, 2, 3])).is_ok());
        assert!(schema.validate(&json!([1])).is_ok());
        assert!(schema.validate(&json!([])).is_err()); // too few
        assert!(schema.validate(&json!([1, 2, 3, 4])).is_err()); // too many
        assert!(schema.validate(&json!([1, "two", 3])).is_err()); // wrong item type
    }

    #[test]
    fn test_object_schema_validation() {
        let schema = MockSchema::object(vec![
            ("name", MockSchema::string().required()),
            ("age", MockSchema::integer()),
            ("email", MockSchema::string()),
        ]);

        // Valid with all fields
        assert!(schema
            .validate(&json!({
                "name": "Alice",
                "age": 30,
                "email": "alice@example.com"
            }))
            .is_ok());

        // Valid with only required fields
        assert!(schema.validate(&json!({"name": "Bob"})).is_ok());

        // Invalid: missing required field
        assert!(schema.validate(&json!({"age": 30})).is_err());

        // Invalid: wrong type for field
        assert!(schema.validate(&json!({"name": 123})).is_err());

        // Invalid: not an object
        assert!(schema.validate(&json!("not an object")).is_err());
    }

    #[test]
    fn test_nested_object_validation() {
        let address_schema = MockSchema::object(vec![
            ("street", MockSchema::string().required()),
            ("city", MockSchema::string().required()),
        ]);

        let user_schema = MockSchema::object(vec![
            ("name", MockSchema::string().required()),
            ("address", address_schema.required()),
        ]);

        // Valid
        assert!(user_schema
            .validate(&json!({
                "name": "Alice",
                "address": {
                    "street": "123 Main St",
                    "city": "Springfield"
                }
            }))
            .is_ok());

        // Invalid: missing nested required field
        let result = user_schema.validate(&json!({
            "name": "Alice",
            "address": {
                "street": "123 Main St"
            }
        }));
        assert!(result.is_err());
        assert!(result.unwrap_err().path.contains("city"));
    }

    #[test]
    fn test_any_schema() {
        let schema = MockSchema::any();

        assert!(schema.validate(&json!("string")).is_ok());
        assert!(schema.validate(&json!(123)).is_ok());
        assert!(schema.validate(&json!({"any": "thing"})).is_ok());
        assert!(schema.validate(&json!([1, 2, 3])).is_ok());
    }

    #[test]
    fn test_null_schema() {
        let schema = MockSchema::null();

        assert!(schema.validate(&json!(null)).is_ok());
        assert!(schema.validate(&json!("not null")).is_err());
    }

    #[test]
    fn test_validation_error_paths() {
        let schema = MockSchema::object(vec![(
            "users",
            MockSchema::array(MockSchema::object(vec![(
                "name",
                MockSchema::string().required(),
            )])),
        )]);

        let result = schema.validate(&json!({
            "users": [
                {"name": "Alice"},
                {"name": 123}  // Invalid type
            ]
        }));

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.path.contains("users"));
        assert!(err.path.contains("[1]"));
        assert!(err.path.contains("name"));
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_contract_serialization() {
        let contract = Contract::builder("test-service")
            .version("1.0.0")
            .operation(
                Operation::builder("getItem")
                    .method(Method::GET)
                    .path("/items/{itemId}")
                    .build(),
            )
            .build();

        let json = serde_json::to_string(&contract).expect("serialization should work");
        assert!(json.contains("test-service"));
        assert!(json.contains("getItem"));
    }

    #[test]
    fn test_schema_serialization() {
        let schema = MockSchema::object(vec![
            ("name", MockSchema::string().required()),
            ("count", MockSchema::integer()),
        ]);

        let json = serde_json::to_string(&schema).expect("serialization should work");
        assert!(json.contains("\"type\":\"object\""));
        assert!(json.contains("\"name\""));
    }
}
