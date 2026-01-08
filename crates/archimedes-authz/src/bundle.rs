//! Policy bundle loading and management.
//!
//! Handles loading Eunomia policy bundles from files or the registry.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::error::{AuthzError, AuthzResult};

/// Metadata about a loaded policy bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    /// Bundle revision (version identifier).
    pub revision: String,
    /// Root paths in the bundle.
    pub roots: Vec<String>,
    /// When the bundle was created.
    pub created_at: Option<String>,
    /// Bundle checksum.
    pub checksum: Option<String>,
}

/// A loaded policy bundle.
#[derive(Debug)]
pub struct Bundle {
    /// Bundle metadata.
    pub metadata: BundleMetadata,
    /// Policy files (path -> content).
    pub policies: HashMap<String, String>,
    /// Data files (path -> JSON content).
    pub data: HashMap<String, serde_json::Value>,
}

impl Bundle {
    /// Create an empty bundle with the given revision.
    pub fn new(revision: impl Into<String>) -> Self {
        Self {
            metadata: BundleMetadata {
                revision: revision.into(),
                roots: vec![],
                created_at: None,
                checksum: None,
            },
            policies: HashMap::new(),
            data: HashMap::new(),
        }
    }

    /// Add a policy to the bundle.
    pub fn add_policy(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.policies.insert(path.into(), content.into());
    }

    /// Add data to the bundle.
    pub fn add_data(&mut self, path: impl Into<String>, content: serde_json::Value) {
        self.data.insert(path.into(), content);
    }

    /// Get all policy file contents.
    pub fn policy_sources(&self) -> impl Iterator<Item = (&str, &str)> {
        self.policies.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// Loads policy bundles from various sources.
pub struct BundleLoader;

impl BundleLoader {
    /// Load a bundle from a tar.gz file.
    pub async fn from_file(path: impl AsRef<Path>) -> AuthzResult<Bundle> {
        let path = path.as_ref();
        info!(path = %path.display(), "loading bundle from file");

        // Read file contents
        let content = tokio::fs::read(path)
            .await
            .map_err(|e| AuthzError::bundle_load(path, format!("failed to read file: {}", e)))?;

        Self::from_tar_gz(&content, path.to_string_lossy().to_string())
    }

    /// Load a bundle from tar.gz bytes.
    pub fn from_tar_gz(data: &[u8], source: String) -> AuthzResult<Bundle> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let decoder = GzDecoder::new(data);
        let mut archive = Archive::new(decoder);

        let mut bundle = Bundle::new("unknown");
        let mut found_manifest = false;

        for entry_result in archive
            .entries()
            .map_err(|e| AuthzError::BundleParse(format!("failed to read archive: {}", e)))?
        {
            let mut entry = entry_result
                .map_err(|e| AuthzError::BundleParse(format!("failed to read entry: {}", e)))?;

            let entry_path = entry
                .path()
                .map_err(|e| AuthzError::BundleParse(format!("invalid path in archive: {}", e)))?
                .to_string_lossy()
                .to_string();

            let mut content = String::new();
            entry.read_to_string(&mut content).map_err(|e| {
                AuthzError::BundleParse(format!("failed to read entry {}: {}", entry_path, e))
            })?;

            if entry_path.ends_with("/.manifest") || entry_path == ".manifest" {
                // Parse manifest
                let manifest: OpaManifest = serde_json::from_str(&content)
                    .map_err(|e| AuthzError::BundleParse(format!("invalid manifest: {}", e)))?;
                bundle.metadata.revision = manifest.revision.unwrap_or_default();
                bundle.metadata.roots = manifest.roots;
                found_manifest = true;
                debug!(revision = %bundle.metadata.revision, "found manifest");
            } else if entry_path.ends_with(".rego") {
                debug!(path = %entry_path, "loading policy");
                bundle.policies.insert(entry_path, content);
            } else if entry_path.ends_with("/data.json") || entry_path == "data.json" {
                let data: serde_json::Value = serde_json::from_str(&content)
                    .map_err(|e| AuthzError::BundleParse(format!("invalid data.json: {}", e)))?;
                debug!(path = %entry_path, "loading data");
                bundle.data.insert(entry_path, data);
            }
        }

        if !found_manifest {
            debug!(source = %source, "bundle has no manifest, using defaults");
        }

        info!(
            policies = bundle.policies.len(),
            data_files = bundle.data.len(),
            "bundle loaded"
        );

        Ok(bundle)
    }

    /// Load a bundle from the Eunomia registry.
    pub async fn from_registry(
        registry_url: &str,
        service: &str,
        version: &str,
    ) -> AuthzResult<Bundle> {
        info!(
            registry = registry_url,
            service, version, "loading bundle from registry"
        );

        let url = format!("{}/v1/bundles/{}/{}", registry_url, service, version);

        let response = reqwest::get(&url)
            .await
            .map_err(|e| AuthzError::Registry(format!("failed to fetch bundle: {}", e)))?;

        if !response.status().is_success() {
            return Err(AuthzError::Registry(format!(
                "registry returned status {}: {}",
                response.status(),
                service
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| AuthzError::Registry(format!("failed to read response: {}", e)))?;

        Self::from_tar_gz(&bytes, format!("{}:{}", service, version))
    }
}

/// OPA bundle manifest format.
#[derive(Debug, Deserialize)]
struct OpaManifest {
    /// Bundle revision.
    revision: Option<String>,
    /// Root paths.
    #[serde(default)]
    roots: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_creation() {
        let mut bundle = Bundle::new("v1.0.0");
        bundle.add_policy("authz.rego", "package authz\nallow = true");
        bundle.add_data("data.json", serde_json::json!({"roles": ["admin"]}));

        assert_eq!(bundle.metadata.revision, "v1.0.0");
        assert_eq!(bundle.policies.len(), 1);
        assert_eq!(bundle.data.len(), 1);
    }

    #[test]
    fn test_bundle_policy_sources() {
        let mut bundle = Bundle::new("test");
        bundle.add_policy("a.rego", "package a");
        bundle.add_policy("b.rego", "package b");

        let sources: Vec<_> = bundle.policy_sources().collect();
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn test_bundle_metadata() {
        let bundle = Bundle::new("rev-123");
        assert_eq!(bundle.metadata.revision, "rev-123");
        assert!(bundle.metadata.roots.is_empty());
    }
}
