//! Integration tests for OPA bundle format validation.
//!
//! These tests validate that our `BundleLoader` correctly parses bundles
//! in the format produced by `eunomia-compiler` and the standard OPA bundler.

use archimedes_authz::bundle::BundleLoader;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;

/// Create a tar.gz bundle in memory with the given files.
fn create_bundle(files: &[(&str, &str)]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());

    {
        let mut builder = Builder::new(&mut encoder);

        for (path, content) in files {
            let mut header = tar::Header::new_gnu();
            header.set_path(path).unwrap();
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();

            builder.append(&header, content.as_bytes()).unwrap();
        }

        builder.finish().unwrap();
    }

    encoder.finish().unwrap()
}

/// Test loading a minimal valid OPA bundle.
///
/// This matches the minimal bundle format from `opa build`.
#[test]
fn test_minimal_opa_bundle() {
    let manifest = r#"{
        "revision": "v1.0.0",
        "roots": ["authz"]
    }"#;

    let policy = r#"
package authz

default allow = false

allow {
    input.user.role == "admin"
}
"#;

    let bundle_data = create_bundle(&[(".manifest", manifest), ("authz/policy.rego", policy)]);

    let bundle = BundleLoader::from_tar_gz(&bundle_data, "test".to_string()).unwrap();

    assert_eq!(bundle.metadata.revision, "v1.0.0");
    assert_eq!(bundle.metadata.roots, vec!["authz"]);
    assert_eq!(bundle.policies.len(), 1);
    assert!(bundle.policies.contains_key("authz/policy.rego"));
}

/// Test loading a bundle with data files (common in Eunomia output).
///
/// Eunomia-compiler bundles often include data.json files for RBAC roles,
/// permissions, and other static data.
#[test]
fn test_bundle_with_data_files() {
    let manifest = r#"{
        "revision": "eunomia-v2.1.0",
        "roots": ["authz", "rbac"]
    }"#;

    let policy = r#"
package authz

import data.rbac.roles
import data.rbac.permissions

default allow = false

allow {
    some role in input.user.roles
    permissions[role][input.action]
}
"#;

    let data = r#"{
        "roles": {
            "admin": ["read", "write", "delete"],
            "user": ["read"]
        },
        "permissions": {
            "admin": {"read": true, "write": true, "delete": true},
            "user": {"read": true, "write": false, "delete": false}
        }
    }"#;

    let bundle_data = create_bundle(&[
        (".manifest", manifest),
        ("authz/policy.rego", policy),
        ("rbac/data.json", data),
    ]);

    let bundle = BundleLoader::from_tar_gz(&bundle_data, "test".to_string()).unwrap();

    assert_eq!(bundle.metadata.revision, "eunomia-v2.1.0");
    assert_eq!(bundle.metadata.roots.len(), 2);
    assert_eq!(bundle.policies.len(), 1);
    assert_eq!(bundle.data.len(), 1);

    // Verify data.json content is valid JSON
    let data_value = bundle.data.get("rbac/data.json").unwrap();
    assert!(data_value.get("roles").is_some());
    assert!(data_value.get("permissions").is_some());
}

/// Test loading a bundle with multiple policy files.
///
/// Production bundles often split policies across multiple files.
#[test]
fn test_bundle_with_multiple_policies() {
    let manifest = r#"{
        "revision": "multi-v1",
        "roots": ["policies"]
    }"#;

    let authz_policy = r#"
package policies.authz

default allow = false
allow { input.authenticated }
"#;

    let rate_limit_policy = r#"
package policies.rate_limit

import data.limits

default exceeded = false

exceeded {
    input.requests_per_minute > limits.max_requests
}
"#;

    let audit_policy = r#"
package policies.audit

log_event = event {
    event := {
        "user": input.user.id,
        "action": input.action,
        "timestamp": input.timestamp
    }
}
"#;

    let bundle_data = create_bundle(&[
        (".manifest", manifest),
        ("policies/authz.rego", authz_policy),
        ("policies/rate_limit.rego", rate_limit_policy),
        ("policies/audit.rego", audit_policy),
    ]);

    let bundle = BundleLoader::from_tar_gz(&bundle_data, "test".to_string()).unwrap();

    assert_eq!(bundle.policies.len(), 3);
    assert!(bundle.policies.contains_key("policies/authz.rego"));
    assert!(bundle.policies.contains_key("policies/rate_limit.rego"));
    assert!(bundle.policies.contains_key("policies/audit.rego"));
}

/// Test that bundles without manifest still load (OPA allows this).
///
/// Some legacy bundles may not have a manifest file.
#[test]
fn test_bundle_without_manifest() {
    let policy = r#"
package authz
default allow = true
"#;

    let bundle_data = create_bundle(&[("authz.rego", policy)]);

    let bundle = BundleLoader::from_tar_gz(&bundle_data, "test".to_string()).unwrap();

    // Should load with default/unknown revision
    assert!(bundle.metadata.revision.is_empty() || bundle.metadata.revision == "unknown");
    assert_eq!(bundle.policies.len(), 1);
}

/// Test the exact manifest format that OPA produces.
///
/// The OPA bundler produces `.manifest` with specific fields.
#[test]
fn test_opa_standard_manifest_format() {
    // This is the exact format OPA's `opa build` command produces
    let manifest = r#"{
        "revision": "abc123",
        "roots": ["app/authz"],
        "wasm": [],
        "metadata": {
            "created_at": "2024-01-15T10:30:00Z",
            "tools": {
                "opa": "0.60.0"
            }
        }
    }"#;

    let policy = r#"package app.authz
default allow = false
"#;

    let bundle_data = create_bundle(&[(".manifest", manifest), ("app/authz/policy.rego", policy)]);

    // Our loader should tolerate extra fields in manifest
    let bundle = BundleLoader::from_tar_gz(&bundle_data, "test".to_string()).unwrap();

    assert_eq!(bundle.metadata.revision, "abc123");
    assert_eq!(bundle.metadata.roots, vec!["app/authz"]);
}

/// Test bundle with nested directory structure.
///
/// Eunomia may produce deeply nested policy structures.
#[test]
fn test_nested_directory_structure() {
    let manifest = r#"{
        "revision": "nested-v1",
        "roots": ["services"]
    }"#;

    let api_policy = r#"
package services.api.v1.authz
allow { input.valid }
"#;

    let internal_policy = r#"
package services.internal.authz
allow { input.internal }
"#;

    let bundle_data = create_bundle(&[
        (".manifest", manifest),
        ("services/api/v1/authz.rego", api_policy),
        ("services/internal/authz.rego", internal_policy),
    ]);

    let bundle = BundleLoader::from_tar_gz(&bundle_data, "test".to_string()).unwrap();

    assert_eq!(bundle.policies.len(), 2);
    assert!(bundle.policies.contains_key("services/api/v1/authz.rego"));
    assert!(bundle.policies.contains_key("services/internal/authz.rego"));
}

/// Test that invalid manifest JSON returns proper error.
#[test]
fn test_invalid_manifest_json() {
    let invalid_manifest = "{ not valid json }";
    let policy = "package test\nallow = true";

    let bundle_data = create_bundle(&[(".manifest", invalid_manifest), ("test.rego", policy)]);

    let result = BundleLoader::from_tar_gz(&bundle_data, "test".to_string());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("invalid manifest"),
        "Error should mention invalid manifest: {}",
        err
    );
}

/// Test that invalid data.json returns proper error.
#[test]
fn test_invalid_data_json() {
    let manifest = r#"{"revision": "test", "roots": []}"#;
    let invalid_data = "{ invalid json }";

    let bundle_data =
        create_bundle(&[(".manifest", manifest), ("data.json", invalid_data)]);

    let result = BundleLoader::from_tar_gz(&bundle_data, "test".to_string());
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("invalid data.json"),
        "Error should mention invalid data.json: {}",
        err
    );
}

/// Test loading bundle with empty manifest (valid OPA behavior).
#[test]
fn test_empty_manifest() {
    let manifest = "{}";
    let policy = "package test\ndefault allow = false";

    let bundle_data = create_bundle(&[(".manifest", manifest), ("test.rego", policy)]);

    let bundle = BundleLoader::from_tar_gz(&bundle_data, "test".to_string()).unwrap();

    // Empty manifest should result in default values
    assert!(
        bundle.metadata.revision.is_empty(),
        "revision should be empty for empty manifest"
    );
    assert!(bundle.metadata.roots.is_empty());
    assert_eq!(bundle.policies.len(), 1);
}

/// Test that non-rego files are ignored.
#[test]
fn test_ignores_non_rego_files() {
    let manifest = r#"{"revision": "test", "roots": []}"#;
    let policy = "package test\nallow = true";
    let readme = "# README\nThis is documentation.";
    let config = "some_config = true";

    let bundle_data = create_bundle(&[
        (".manifest", manifest),
        ("policy.rego", policy),
        ("README.md", readme),
        ("config.txt", config),
    ]);

    let bundle = BundleLoader::from_tar_gz(&bundle_data, "test".to_string()).unwrap();

    // Should only have the .rego file
    assert_eq!(bundle.policies.len(), 1);
    assert!(bundle.policies.contains_key("policy.rego"));
    assert!(!bundle.policies.contains_key("README.md"));
    assert!(!bundle.policies.contains_key("config.txt"));
}

/// Test realistic Eunomia-style bundle structure.
///
/// This simulates what eunomia-compiler would produce for a service.
#[test]
fn test_eunomia_style_bundle() {
    let manifest = r#"{
        "revision": "user-service-v1.2.3-abc1234",
        "roots": ["user_service"]
    }"#;

    let authz_policy = r#"
package user_service.authz

import data.user_service.rbac

# Entry point for authorization decisions
default allow = false

# Allow if user has required permission
allow {
    some permission in required_permissions[input.operation]
    has_permission(input.identity.roles, permission)
}

# Required permissions per operation
required_permissions := {
    "getUser": ["user:read"],
    "createUser": ["user:write"],
    "deleteUser": ["user:admin"],
}

# Check if any role grants the permission
has_permission(roles, required) {
    some role in roles
    rbac.role_permissions[role][required]
}
"#;

    let rbac_data = r#"{
        "role_permissions": {
            "admin": {
                "user:read": true,
                "user:write": true,
                "user:admin": true
            },
            "editor": {
                "user:read": true,
                "user:write": true,
                "user:admin": false
            },
            "viewer": {
                "user:read": true,
                "user:write": false,
                "user:admin": false
            }
        }
    }"#;

    let bundle_data = create_bundle(&[
        (".manifest", manifest),
        ("user_service/authz.rego", authz_policy),
        ("user_service/rbac/data.json", rbac_data),
    ]);

    let bundle = BundleLoader::from_tar_gz(&bundle_data, "test".to_string()).unwrap();

    // Verify bundle structure
    assert_eq!(bundle.metadata.revision, "user-service-v1.2.3-abc1234");
    assert_eq!(bundle.metadata.roots, vec!["user_service"]);
    assert_eq!(bundle.policies.len(), 1);
    assert_eq!(bundle.data.len(), 1);

    // Verify RBAC data structure
    let rbac = bundle.data.get("user_service/rbac/data.json").unwrap();
    assert!(rbac.get("role_permissions").is_some());
    assert!(rbac["role_permissions"]["admin"]["user:admin"].as_bool().unwrap());
    assert!(!rbac["role_permissions"]["viewer"]["user:admin"].as_bool().unwrap());
}
