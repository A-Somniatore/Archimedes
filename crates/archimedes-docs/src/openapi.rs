//! OpenAPI specification types and generation.
//!
//! This module provides types that represent the OpenAPI 3.1 specification
//! and a generator that converts Themis artifacts into OpenAPI specs.
//!
//! ## OpenAPI 3.1 Compliance
//!
//! The types in this module follow the OpenAPI 3.1 specification:
//! <https://spec.openapis.org/oas/v3.1.0>

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use archimedes_sentinel::{LoadedArtifact, LoadedOperation};
use themis_core::Schema as ThemisSchema;

use crate::error::{DocsError, DocsResult};

/// OpenAPI document root object.
///
/// This is the root of an OpenAPI document, containing all API metadata,
/// paths, components, and server information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApi {
    /// OpenAPI version (should be "3.1.0").
    pub openapi: String,
    /// API metadata.
    pub info: Info,
    /// Available servers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub servers: Vec<Server>,
    /// API paths and operations.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub paths: IndexMap<String, PathItem>,
    /// Reusable components (schemas, responses, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
    /// Tags for API grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    /// External documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "externalDocs")]
    pub external_docs: Option<ExternalDocumentation>,
}

/// API metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    /// API title.
    pub title: String,
    /// API version.
    pub version: String,
    /// API description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Terms of service URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "termsOfService")]
    pub terms_of_service: Option<String>,
    /// Contact information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,
    /// License information.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
}

/// Contact information.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contact {
    /// Contact name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Contact URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Contact email.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// License information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    /// License name.
    pub name: String,
    /// License URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// SPDX identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
}

/// Server information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    /// Server URL.
    pub url: String,
    /// Server description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Server variables for URL templating.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub variables: HashMap<String, ServerVariable>,
}

/// Server variable for URL templating.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerVariable {
    /// Default value.
    pub default: String,
    /// Possible values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "enum")]
    pub enum_values: Vec<String>,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A path item containing operations for a single path.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathItem {
    /// Summary for all operations on this path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Description for all operations on this path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// GET operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    /// PUT operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    /// POST operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    /// DELETE operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
    /// OPTIONS operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Operation>,
    /// HEAD operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<Operation>,
    /// PATCH operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
    /// TRACE operation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<Operation>,
    /// Parameters common to all operations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
}

/// An API operation (endpoint).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    /// Unique operation identifier.
    #[serde(rename = "operationId")]
    pub operation_id: String,
    /// Short summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Full description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Whether deprecated.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deprecated: bool,
    /// Parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    /// Request body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "requestBody")]
    pub request_body: Option<RequestBody>,
    /// Responses.
    pub responses: IndexMap<String, Response>,
    /// Security requirements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<SecurityRequirement>,
}

/// Parameter location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterIn {
    /// Query string parameter.
    Query,
    /// URL path parameter.
    Path,
    /// HTTP header.
    Header,
    /// Cookie.
    Cookie,
}

/// An operation parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name.
    pub name: String,
    /// Parameter location.
    #[serde(rename = "in")]
    pub location: ParameterIn,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether required.
    #[serde(default)]
    pub required: bool,
    /// Whether deprecated.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deprecated: bool,
    /// Parameter schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
}

/// Request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether required.
    #[serde(default)]
    pub required: bool,
    /// Content by media type.
    pub content: IndexMap<String, MediaType>,
}

/// Media type content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    /// Schema for this media type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
    /// Example value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

/// Response definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Description (required).
    pub description: String,
    /// Response headers.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub headers: IndexMap<String, Header>,
    /// Response content by media type.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub content: IndexMap<String, MediaType>,
}

/// Response header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Header schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
}

/// Reusable components.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Components {
    /// Reusable schemas.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub schemas: IndexMap<String, Schema>,
    /// Reusable responses.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub responses: IndexMap<String, Response>,
    /// Reusable parameters.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub parameters: IndexMap<String, Parameter>,
    /// Security schemes.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    #[serde(rename = "securitySchemes")]
    pub security_schemes: IndexMap<String, SecurityScheme>,
}

/// Security scheme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScheme {
    /// Security scheme type.
    #[serde(rename = "type")]
    pub scheme_type: String,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// HTTP auth scheme name (for type=http).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    /// Bearer token format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "bearerFormat")]
    pub bearer_format: Option<String>,
    /// API key location (for type=apiKey).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "in")]
    pub location: Option<String>,
    /// API key name (for type=apiKey).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Security requirement.
pub type SecurityRequirement = HashMap<String, Vec<String>>;

/// API tag for grouping operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Tag name.
    pub name: String,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// External documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "externalDocs")]
    pub external_docs: Option<ExternalDocumentation>,
}

/// External documentation link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocumentation {
    /// URL.
    pub url: String,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// JSON Schema type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemaType {
    /// String type.
    String,
    /// Number type.
    Number,
    /// Integer type.
    Integer,
    /// Boolean type.
    Boolean,
    /// Array type.
    Array,
    /// Object type.
    Object,
    /// Null type.
    Null,
}

/// JSON Schema definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    /// Schema type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "type")]
    pub schema_type: Option<SchemaType>,
    /// Schema format (e.g., "date-time", "email").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Reference to another schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "$ref")]
    pub reference: Option<String>,
    /// Object properties.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub properties: IndexMap<String, Schema>,
    /// Required properties.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    /// Array item schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Schema>>,
    /// Enum values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "enum")]
    pub enum_values: Vec<serde_json::Value>,
    /// oneOf schemas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "oneOf")]
    pub one_of: Vec<Schema>,
    /// anyOf schemas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "anyOf")]
    pub any_of: Vec<Schema>,
    /// allOf schemas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "allOf")]
    pub all_of: Vec<Schema>,
    /// Minimum value (for numbers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// Maximum value (for numbers).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    /// Minimum length (for strings).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "minLength")]
    pub min_length: Option<u64>,
    /// Maximum length (for strings).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[serde(rename = "maxLength")]
    pub max_length: Option<u64>,
    /// Pattern regex (for strings).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Default value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Example value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    /// Whether nullable.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub nullable: bool,
}

impl Default for Schema {
    fn default() -> Self {
        Self {
            schema_type: None,
            format: None,
            description: None,
            reference: None,
            properties: IndexMap::new(),
            required: Vec::new(),
            items: None,
            enum_values: Vec::new(),
            one_of: Vec::new(),
            any_of: Vec::new(),
            all_of: Vec::new(),
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            default: None,
            example: None,
            nullable: false,
        }
    }
}

impl Schema {
    /// Create a string schema.
    #[must_use]
    pub fn string() -> Self {
        Self {
            schema_type: Some(SchemaType::String),
            ..Default::default()
        }
    }

    /// Create an integer schema.
    #[must_use]
    pub fn integer() -> Self {
        Self {
            schema_type: Some(SchemaType::Integer),
            ..Default::default()
        }
    }

    /// Create a number schema.
    #[must_use]
    pub fn number() -> Self {
        Self {
            schema_type: Some(SchemaType::Number),
            ..Default::default()
        }
    }

    /// Create a boolean schema.
    #[must_use]
    pub fn boolean() -> Self {
        Self {
            schema_type: Some(SchemaType::Boolean),
            ..Default::default()
        }
    }

    /// Create an array schema with the given item schema.
    #[must_use]
    pub fn array(items: Schema) -> Self {
        Self {
            schema_type: Some(SchemaType::Array),
            items: Some(Box::new(items)),
            ..Default::default()
        }
    }

    /// Create an object schema.
    #[must_use]
    pub fn object() -> Self {
        Self {
            schema_type: Some(SchemaType::Object),
            ..Default::default()
        }
    }

    /// Create a reference schema.
    #[must_use]
    pub fn reference(ref_path: impl Into<String>) -> Self {
        Self {
            reference: Some(ref_path.into()),
            ..Default::default()
        }
    }

    /// Add a description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a property to an object schema.
    #[must_use]
    pub fn property(mut self, name: impl Into<String>, schema: Schema) -> Self {
        self.properties.insert(name.into(), schema);
        self
    }

    /// Mark a property as required.
    #[must_use]
    pub fn required_property(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }
}

/// Generator for converting Themis artifacts to OpenAPI specs.
#[derive(Debug, Clone)]
pub struct OpenApiGenerator {
    title: Option<String>,
    version: Option<String>,
    description: Option<String>,
    servers: Vec<Server>,
    contact: Option<Contact>,
    license: Option<License>,
    external_docs: Option<ExternalDocumentation>,
    security_schemes: IndexMap<String, SecurityScheme>,
}

impl Default for OpenApiGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenApiGenerator {
    /// Create a new generator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            title: None,
            version: None,
            description: None,
            servers: Vec::new(),
            contact: None,
            license: None,
            external_docs: None,
            security_schemes: IndexMap::new(),
        }
    }

    /// Set the API title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the API version.
    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set the API description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a server.
    #[must_use]
    pub fn server(mut self, url: impl Into<String>, description: Option<String>) -> Self {
        self.servers.push(Server {
            url: url.into(),
            description,
            variables: HashMap::new(),
        });
        self
    }

    /// Set contact information.
    #[must_use]
    pub fn contact(mut self, contact: Contact) -> Self {
        self.contact = Some(contact);
        self
    }

    /// Set license information.
    #[must_use]
    pub fn license(mut self, name: impl Into<String>, url: Option<String>) -> Self {
        self.license = Some(License {
            name: name.into(),
            url,
            identifier: None,
        });
        self
    }

    /// Add a Bearer token security scheme.
    #[must_use]
    pub fn bearer_auth(mut self, name: impl Into<String>) -> Self {
        self.security_schemes.insert(
            name.into(),
            SecurityScheme {
                scheme_type: "http".to_string(),
                description: Some("JWT Bearer token authentication".to_string()),
                scheme: Some("bearer".to_string()),
                bearer_format: Some("JWT".to_string()),
                location: None,
                name: None,
            },
        );
        self
    }

    /// Add an API key security scheme.
    #[must_use]
    pub fn api_key_auth(
        mut self,
        name: impl Into<String>,
        header_name: impl Into<String>,
    ) -> Self {
        let name = name.into();
        self.security_schemes.insert(
            name.clone(),
            SecurityScheme {
                scheme_type: "apiKey".to_string(),
                description: Some(format!("API key authentication via {} header", header_name.into())),
                scheme: None,
                bearer_format: None,
                location: Some("header".to_string()),
                name: Some(name),
            },
        );
        self
    }

    /// Generate an OpenAPI spec from a loaded artifact.
    pub fn generate(&self, artifact: &LoadedArtifact) -> DocsResult<OpenApi> {
        let info = Info {
            title: self.title.clone().unwrap_or_else(|| artifact.service.clone()),
            version: self.version.clone().unwrap_or_else(|| artifact.version.clone()),
            description: self.description.clone(),
            terms_of_service: None,
            contact: self.contact.clone(),
            license: self.license.clone(),
        };

        let mut paths: IndexMap<String, PathItem> = IndexMap::new();
        let mut tags_set: std::collections::HashSet<String> = std::collections::HashSet::new();

        for operation in &artifact.operations {
            let path_item = paths.entry(operation.path.clone()).or_default();
            let openapi_op = self.convert_operation(operation)?;

            // Collect tags
            for tag in &openapi_op.tags {
                tags_set.insert(tag.clone());
            }

            // Assign to correct method
            match operation.method.to_uppercase().as_str() {
                "GET" => path_item.get = Some(openapi_op),
                "POST" => path_item.post = Some(openapi_op),
                "PUT" => path_item.put = Some(openapi_op),
                "DELETE" => path_item.delete = Some(openapi_op),
                "PATCH" => path_item.patch = Some(openapi_op),
                "OPTIONS" => path_item.options = Some(openapi_op),
                "HEAD" => path_item.head = Some(openapi_op),
                "TRACE" => path_item.trace = Some(openapi_op),
                _ => {
                    return Err(DocsError::InvalidOperation {
                        operation_id: operation.id.clone(),
                        reason: format!("unknown HTTP method: {}", operation.method),
                    });
                }
            }
        }

        // Convert tags to Tag structs
        let tags: Vec<Tag> = tags_set
            .into_iter()
            .map(|name| Tag {
                name,
                description: None,
                external_docs: None,
            })
            .collect();

        // Convert schemas
        let components = if !artifact.schemas.is_empty() || !self.security_schemes.is_empty() {
            let schemas = artifact
                .schemas
                .iter()
                .map(|(name, schema)| (name.clone(), convert_themis_schema(schema)))
                .collect();

            Some(Components {
                schemas,
                responses: IndexMap::new(),
                parameters: IndexMap::new(),
                security_schemes: self.security_schemes.clone(),
            })
        } else {
            None
        };

        Ok(OpenApi {
            openapi: "3.1.0".to_string(),
            info,
            servers: self.servers.clone(),
            paths,
            components,
            tags,
            external_docs: self.external_docs.clone(),
        })
    }

    fn convert_operation(&self, op: &LoadedOperation) -> DocsResult<Operation> {
        // Extract path parameters from path template
        let parameters = extract_path_parameters(&op.path);

        // Convert responses
        let mut responses: IndexMap<String, Response> = IndexMap::new();

        if op.response_schemas.is_empty() {
            // Add default 200 response
            responses.insert(
                "200".to_string(),
                Response {
                    description: "Successful response".to_string(),
                    headers: IndexMap::new(),
                    content: IndexMap::new(),
                },
            );
        } else {
            for (status, schema_ref) in &op.response_schemas {
                let mut content = IndexMap::new();
                content.insert(
                    "application/json".to_string(),
                    MediaType {
                        schema: Some(Schema::reference(&schema_ref.reference)),
                        example: None,
                    },
                );

                responses.insert(
                    status.clone(),
                    Response {
                        description: format!("{} response", status),
                        headers: IndexMap::new(),
                        content,
                    },
                );
            }
        }

        // Build request body if present
        let request_body = op.request_schema.as_ref().map(|schema_ref| {
            let mut content = IndexMap::new();
            content.insert(
                "application/json".to_string(),
                MediaType {
                    schema: Some(Schema::reference(&schema_ref.reference)),
                    example: None,
                },
            );

            RequestBody {
                description: None,
                required: true,
                content,
            }
        });

        // Build security requirements
        let security: Vec<SecurityRequirement> = if op.security.is_empty() {
            Vec::new()
        } else {
            op.security
                .iter()
                .map(|s| {
                    let mut req = HashMap::new();
                    req.insert(s.clone(), Vec::new());
                    req
                })
                .collect()
        };

        Ok(Operation {
            operation_id: op.id.clone(),
            summary: op.summary.clone(),
            description: None,
            tags: op.tags.clone(),
            deprecated: op.deprecated,
            parameters,
            request_body,
            responses,
            security,
        })
    }

    /// Generate the OpenAPI spec as JSON.
    pub fn generate_json(&self, artifact: &LoadedArtifact) -> DocsResult<String> {
        let spec = self.generate(artifact)?;
        serde_json::to_string_pretty(&spec).map_err(DocsError::from)
    }
}

/// Extract path parameters from a path template like `/users/{userId}`.
fn extract_path_parameters(path: &str) -> Vec<Parameter> {
    let mut params = Vec::new();
    let param_regex = regex::Regex::new(r"\{([^}]+)\}").expect("valid regex");

    for cap in param_regex.captures_iter(path) {
        if let Some(name) = cap.get(1) {
            params.push(Parameter {
                name: name.as_str().to_string(),
                location: ParameterIn::Path,
                description: None,
                required: true, // Path parameters are always required
                deprecated: false,
                schema: Some(Schema::string()),
            });
        }
    }

    params
}

/// Convert a Themis schema to an OpenAPI schema.
fn convert_themis_schema(schema: &ThemisSchema) -> Schema {
    match schema {
        ThemisSchema::String(s) => {
            let mut result = Schema::string();
            result.min_length = s.min_length.map(|v| v as u64);
            result.max_length = s.max_length.map(|v| v as u64);
            result.pattern = s.pattern.clone();
            result.format = s.format.clone();
            result
        }
        ThemisSchema::Integer(i) => {
            let mut result = Schema::integer();
            result.minimum = i.minimum.map(|v| v as f64);
            result.maximum = i.maximum.map(|v| v as f64);
            result
        }
        ThemisSchema::Number(n) => {
            let mut result = Schema::number();
            result.minimum = n.minimum;
            result.maximum = n.maximum;
            result
        }
        ThemisSchema::Boolean(_) => Schema::boolean(),
        ThemisSchema::Array(a) => {
            let items = convert_themis_schema(&a.items);
            Schema::array(items)
        }
        ThemisSchema::Object(o) => {
            let mut result = Schema::object();
            for (name, prop_schema) in &o.properties {
                result.properties.insert(name.clone(), convert_themis_schema(prop_schema));
            }
            result.required = o.required.clone();
            result
        }
        ThemisSchema::Ref(r) => Schema::reference(&r.reference),
        ThemisSchema::OneOf(one_of) => {
            let mut result = Schema::default();
            result.one_of = one_of.schemas.iter().map(convert_themis_schema).collect();
            result
        }
        ThemisSchema::AllOf(all_of) => {
            let mut result = Schema::default();
            result.all_of = all_of.schemas.iter().map(convert_themis_schema).collect();
            result
        }
        ThemisSchema::AnyOf(any_of) => {
            let mut result = Schema::default();
            result.any_of = any_of.schemas.iter().map(convert_themis_schema).collect();
            result
        }
        ThemisSchema::Enum(e) => {
            let mut result = Schema::string();
            result.enum_values = e.values.iter().map(|v| v.value.clone()).collect();
            result
        }
        ThemisSchema::Null => {
            let mut result = Schema::default();
            result.schema_type = Some(SchemaType::Null);
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_builders() {
        let string = Schema::string();
        assert_eq!(string.schema_type, Some(SchemaType::String));

        let integer = Schema::integer();
        assert_eq!(integer.schema_type, Some(SchemaType::Integer));

        let array = Schema::array(Schema::string());
        assert_eq!(array.schema_type, Some(SchemaType::Array));
        assert!(array.items.is_some());

        let object = Schema::object()
            .property("name", Schema::string())
            .required_property("name");
        assert_eq!(object.schema_type, Some(SchemaType::Object));
        assert!(object.properties.contains_key("name"));
        assert!(object.required.contains(&"name".to_string()));
    }

    #[test]
    fn test_schema_reference() {
        let schema = Schema::reference("#/components/schemas/User");
        assert_eq!(schema.reference, Some("#/components/schemas/User".to_string()));
    }

    #[test]
    fn test_extract_path_parameters() {
        let params = extract_path_parameters("/users/{userId}");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "userId");
        assert_eq!(params[0].location, ParameterIn::Path);
        assert!(params[0].required);

        let params = extract_path_parameters("/users/{userId}/orders/{orderId}");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "userId");
        assert_eq!(params[1].name, "orderId");

        let params = extract_path_parameters("/users");
        assert!(params.is_empty());
    }

    #[test]
    fn test_generator_builder() {
        let generator = OpenApiGenerator::new()
            .title("My API")
            .version("1.0.0")
            .description("API description")
            .server("https://api.example.com", Some("Production".to_string()))
            .bearer_auth("bearerAuth")
            .license("MIT", Some("https://opensource.org/licenses/MIT".to_string()));

        assert_eq!(generator.title, Some("My API".to_string()));
        assert_eq!(generator.version, Some("1.0.0".to_string()));
        assert_eq!(generator.servers.len(), 1);
        assert!(generator.security_schemes.contains_key("bearerAuth"));
    }

    #[test]
    fn test_info_serialization() {
        let info = Info {
            title: "Test API".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Test description".to_string()),
            terms_of_service: None,
            contact: None,
            license: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("Test API"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_operation_serialization() {
        let operation = Operation {
            operation_id: "getUser".to_string(),
            summary: Some("Get a user".to_string()),
            description: None,
            tags: vec!["users".to_string()],
            deprecated: false,
            parameters: vec![Parameter {
                name: "userId".to_string(),
                location: ParameterIn::Path,
                description: None,
                required: true,
                deprecated: false,
                schema: Some(Schema::string()),
            }],
            request_body: None,
            responses: IndexMap::new(),
            security: Vec::new(),
        };

        let json = serde_json::to_string(&operation).unwrap();
        assert!(json.contains("getUser"));
        assert!(json.contains("userId"));
    }

    #[test]
    fn test_path_item_serialization() {
        let mut path_item = PathItem::default();
        path_item.get = Some(Operation {
            operation_id: "listUsers".to_string(),
            summary: None,
            description: None,
            tags: vec![],
            deprecated: false,
            parameters: vec![],
            request_body: None,
            responses: IndexMap::new(),
            security: vec![],
        });

        let json = serde_json::to_string(&path_item).unwrap();
        assert!(json.contains("listUsers"));
    }

    #[test]
    fn test_parameter_in_serialization() {
        let param = Parameter {
            name: "id".to_string(),
            location: ParameterIn::Query,
            description: None,
            required: false,
            deprecated: false,
            schema: None,
        };

        let json = serde_json::to_string(&param).unwrap();
        assert!(json.contains("\"in\":\"query\""));
    }

    #[test]
    fn test_full_openapi_serialization() {
        let spec = OpenApi {
            openapi: "3.1.0".to_string(),
            info: Info {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                terms_of_service: None,
                contact: None,
                license: None,
            },
            servers: vec![Server {
                url: "https://api.example.com".to_string(),
                description: None,
                variables: HashMap::new(),
            }],
            paths: IndexMap::new(),
            components: None,
            tags: vec![],
            external_docs: None,
        };

        let json = serde_json::to_string_pretty(&spec).unwrap();
        assert!(json.contains("3.1.0"));
        assert!(json.contains("Test API"));
    }
}
