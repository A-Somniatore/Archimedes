//! Artifact loading and transformation.
//!
//! This module provides types for loading Themis artifacts and transforming
//! them into a format suitable for runtime operation resolution.

use std::collections::HashMap;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use themis_artifact::{Artifact, ArtifactOperation};
use themis_core::Schema;
use tokio::fs;
use tracing::{debug, info};

use crate::error::{SentinelError, SentinelResult};

/// A loaded artifact ready for runtime use.
///
/// This is a processed form of a Themis artifact optimized for
/// fast operation lookup and validation.
#[derive(Debug, Clone)]
pub struct LoadedArtifact {
    /// Service name.
    pub service: String,
    /// Contract version.
    pub version: String,
    /// Contract format (e.g., "openapi", "protobuf").
    pub format: String,
    /// All operations in the contract.
    pub operations: Vec<LoadedOperation>,
    /// Named schemas for validation.
    pub schemas: IndexMap<String, Schema>,
}

/// A loaded operation ready for runtime use.
#[derive(Debug, Clone)]
pub struct LoadedOperation {
    /// Operation ID (e.g., "getUserById").
    pub id: String,
    /// HTTP method (uppercase).
    pub method: String,
    /// Path template (e.g., "/users/{userId}").
    pub path: String,
    /// Short summary.
    pub summary: Option<String>,
    /// Whether deprecated.
    pub deprecated: bool,
    /// Security requirements.
    pub security: Vec<String>,
    /// Request schema reference.
    pub request_schema: Option<SchemaRef>,
    /// Response schemas by status code.
    pub response_schemas: HashMap<String, SchemaRef>,
    /// Tags.
    pub tags: Vec<String>,
}

/// A reference to a schema for validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRef {
    /// Schema reference path (e.g., "#/components/schemas/User").
    pub reference: String,
    /// Schema type for quick type checking.
    pub schema_type: String,
    /// Required fields (for objects).
    pub required: Vec<String>,
}

/// Loads artifacts from various sources.
pub struct ArtifactLoader;

impl ArtifactLoader {
    /// Load an artifact from a file.
    pub async fn from_file(path: impl AsRef<Path>) -> SentinelResult<LoadedArtifact> {
        let path = path.as_ref();
        info!(path = %path.display(), "loading artifact from file");

        let content = fs::read_to_string(path).await.map_err(|e| {
            SentinelError::ArtifactLoad(format!(
                "failed to read artifact file {}: {}",
                path.display(),
                e
            ))
        })?;

        Self::from_json(&content)
    }

    /// Load an artifact from JSON string.
    pub fn from_json(json: &str) -> SentinelResult<LoadedArtifact> {
        let artifact: Artifact = serde_json::from_str(json).map_err(|e| {
            SentinelError::ArtifactLoad(format!("failed to parse artifact JSON: {}", e))
        })?;

        Self::from_artifact(artifact)
    }

    /// Load an artifact from a registry.
    pub async fn from_registry(
        registry_url: &str,
        service: &str,
        version: &str,
    ) -> SentinelResult<LoadedArtifact> {
        info!(
            registry = registry_url,
            service, version, "loading artifact from registry"
        );

        // Construct the registry URL for fetching the artifact
        let url = format!("{}/v1/artifacts/{}/{}", registry_url, service, version);

        // Use reqwest to fetch the artifact
        let response = reqwest::get(&url).await.map_err(|e| {
            SentinelError::ArtifactLoad(format!("failed to fetch from registry: {}", e))
        })?;

        if !response.status().is_success() {
            return Err(SentinelError::ArtifactLoad(format!(
                "registry returned status {}: {}",
                response.status(),
                service
            )));
        }

        let json = response.text().await.map_err(|e| {
            SentinelError::ArtifactLoad(format!("failed to read registry response: {}", e))
        })?;

        Self::from_json(&json)
    }

    /// Convert a Themis Artifact to a LoadedArtifact.
    pub fn from_artifact(artifact: Artifact) -> SentinelResult<LoadedArtifact> {
        // Verify checksum
        artifact.verify_checksum().map_err(|e| {
            SentinelError::ArtifactLoad(format!("artifact checksum verification failed: {}", e))
        })?;

        let operations = artifact
            .operations
            .iter()
            .map(Self::convert_operation)
            .collect();

        debug!(
            service = artifact.service,
            version = artifact.version,
            operations = artifact.operations.len(),
            schemas = artifact.schemas.len(),
            "artifact loaded successfully"
        );

        Ok(LoadedArtifact {
            service: artifact.service,
            version: artifact.version,
            format: artifact.format,
            operations,
            schemas: artifact.schemas,
        })
    }

    fn convert_operation(op: &ArtifactOperation) -> LoadedOperation {
        LoadedOperation {
            id: op.id.clone(),
            method: op.method.to_uppercase(),
            path: op.path.clone(),
            summary: op.summary.clone(),
            deprecated: op.deprecated,
            security: op.security.clone(),
            request_schema: op.request_schema.as_ref().map(Self::schema_to_ref),
            response_schemas: op
                .response_schemas
                .iter()
                .map(|(k, v)| (k.clone(), Self::schema_to_ref(v)))
                .collect(),
            tags: op.tags.clone(),
        }
    }

    fn schema_to_ref(schema: &Schema) -> SchemaRef {
        // Extract type information from the schema
        let (schema_type, required) = match schema {
            Schema::Object(obj) => ("object".to_string(), obj.required.clone()),
            Schema::Array(_) => ("array".to_string(), vec![]),
            Schema::String(_) => ("string".to_string(), vec![]),
            Schema::Integer(_) => ("integer".to_string(), vec![]),
            Schema::Number(_) => ("number".to_string(), vec![]),
            Schema::Boolean(_) => ("boolean".to_string(), vec![]),
            Schema::Ref(_) => ("ref".to_string(), vec![]),
            Schema::OneOf(_) => ("oneOf".to_string(), vec![]),
            Schema::AllOf(_) => ("allOf".to_string(), vec![]),
            Schema::AnyOf(_) => ("anyOf".to_string(), vec![]),
            Schema::Enum(_) => ("enum".to_string(), vec![]),
            Schema::Null => ("null".to_string(), vec![]),
        };

        // For ref schemas, use the reference, otherwise generate a placeholder
        let reference = if let Schema::Ref(r) = schema {
            r.reference.clone()
        } else {
            format!("#/inline/{}", schema_type)
        };

        SchemaRef {
            reference,
            schema_type,
            required,
        }
    }
}

impl From<Artifact> for LoadedArtifact {
    fn from(artifact: Artifact) -> Self {
        // Note: This doesn't verify checksum - use ArtifactLoader::from_artifact for that
        let operations = artifact
            .operations
            .iter()
            .map(ArtifactLoader::convert_operation)
            .collect();

        LoadedArtifact {
            service: artifact.service,
            version: artifact.version,
            format: artifact.format,
            operations,
            schemas: artifact.schemas,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_artifact_json() -> String {
        r#"{
            "$schema": "https://themis.somniatore.com/schemas/artifact.v1.json",
            "version": "1.0.0",
            "service": "test-service",
            "format": "openapi",
            "format_version": "3.1.0",
            "metadata": {
                "created_at": "2025-01-01T00:00:00Z"
            },
            "checksum": {
                "algorithm": "sha256",
                "value": "test"
            },
            "operations": [
                {
                    "id": "listUsers",
                    "method": "GET",
                    "path": "/users",
                    "summary": "List all users"
                },
                {
                    "id": "getUser",
                    "method": "GET",
                    "path": "/users/{userId}",
                    "summary": "Get a user by ID",
                    "deprecated": false
                }
            ],
            "schemas": {}
        }"#
        .to_string()
    }

    #[test]
    fn test_schema_ref_creation() {
        let schema_ref = SchemaRef {
            reference: "#/components/schemas/User".to_string(),
            schema_type: "object".to_string(),
            required: vec!["id".to_string(), "name".to_string()],
        };

        assert_eq!(schema_ref.schema_type, "object");
        assert_eq!(schema_ref.required.len(), 2);
    }

    // Note: Full parsing tests would require proper checksum validation
    // which is complex to set up in unit tests
}
