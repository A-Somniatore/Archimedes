//! Operation resolution from HTTP requests.
//!
//! This module provides the `OperationResolver` which maps incoming HTTP
//! requests (method + path) to Themis operation IDs.

use std::collections::HashMap;

use regex::Regex;
use tracing::debug;

use crate::artifact::{LoadedArtifact, LoadedOperation};
use crate::error::{SentinelError, SentinelResult};

/// Result of resolving an HTTP request to an operation.
#[derive(Debug, Clone)]
pub struct OperationResolution {
    /// The Themis operation ID.
    pub operation_id: String,
    /// HTTP method of the matched operation.
    pub method: String,
    /// Path template that was matched.
    pub path_template: String,
    /// Extracted path parameters.
    pub path_params: HashMap<String, String>,
    /// Whether the operation is deprecated.
    pub deprecated: bool,
    /// Tags from the operation.
    pub tags: Vec<String>,
}

/// Resolves HTTP requests to Themis operations.
///
/// The resolver builds a routing table from the loaded artifact and provides
/// efficient path matching with parameter extraction.
#[derive(Debug)]
pub struct OperationResolver {
    /// Routes indexed by HTTP method.
    routes: HashMap<String, Vec<CompiledRoute>>,
}

/// A compiled route for efficient matching.
#[derive(Debug)]
struct CompiledRoute {
    /// Original path template.
    template: String,
    /// Regex for matching paths.
    pattern: Regex,
    /// Parameter names in order.
    param_names: Vec<String>,
    /// Operation ID.
    operation_id: String,
    /// Whether deprecated.
    deprecated: bool,
    /// Tags.
    tags: Vec<String>,
}

impl OperationResolver {
    /// Create a resolver from a loaded artifact.
    pub fn from_artifact(artifact: &LoadedArtifact) -> Self {
        let mut routes: HashMap<String, Vec<CompiledRoute>> = HashMap::new();

        for op in &artifact.operations {
            if op.path.is_empty() {
                continue;
            }

            let compiled = Self::compile_route(op);
            routes
                .entry(op.method.to_uppercase())
                .or_default()
                .push(compiled);
        }

        // Sort routes by specificity (more specific paths first)
        for method_routes in routes.values_mut() {
            method_routes.sort_by(|a, b| Self::route_specificity(&b.template, &a.template));
        }

        debug!(
            methods = routes.len(),
            total_routes = routes.values().map(Vec::len).sum::<usize>(),
            "operation resolver initialized"
        );

        Self { routes }
    }

    /// Resolve an HTTP request to an operation.
    pub fn resolve(&self, method: &str, path: &str) -> SentinelResult<OperationResolution> {
        let method_upper = method.to_uppercase();
        let routes = self.routes.get(&method_upper).ok_or_else(|| {
            SentinelError::OperationNotFound {
                method: method.to_string(),
                path: path.to_string(),
            }
        })?;

        // Try each route in order (already sorted by specificity)
        for route in routes {
            if let Some(captures) = route.pattern.captures(path) {
                let mut path_params = HashMap::new();
                for (i, name) in route.param_names.iter().enumerate() {
                    if let Some(value) = captures.get(i + 1) {
                        path_params.insert(name.clone(), value.as_str().to_string());
                    }
                }

                return Ok(OperationResolution {
                    operation_id: route.operation_id.clone(),
                    method: method_upper,
                    path_template: route.template.clone(),
                    path_params,
                    deprecated: route.deprecated,
                    tags: route.tags.clone(),
                });
            }
        }

        Err(SentinelError::OperationNotFound {
            method: method.to_string(),
            path: path.to_string(),
        })
    }

    /// Check if a route exists for the given method and path.
    pub fn has_route(&self, method: &str, path: &str) -> bool {
        self.resolve(method, path).is_ok()
    }

    /// Get all registered methods.
    pub fn methods(&self) -> Vec<&str> {
        self.routes.keys().map(String::as_str).collect()
    }

    /// Get all routes for a specific method.
    pub fn routes_for_method(&self, method: &str) -> Vec<&str> {
        self.routes
            .get(&method.to_uppercase())
            .map(|routes| routes.iter().map(|r| r.template.as_str()).collect())
            .unwrap_or_default()
    }

    fn compile_route(op: &LoadedOperation) -> CompiledRoute {
        let (pattern, param_names) = Self::compile_path(&op.path);

        CompiledRoute {
            template: op.path.clone(),
            pattern,
            param_names,
            operation_id: op.id.clone(),
            deprecated: op.deprecated,
            tags: op.tags.clone(),
        }
    }

    fn compile_path(template: &str) -> (Regex, Vec<String>) {
        let mut pattern = String::from("^");
        let mut param_names = Vec::new();

        for segment in template.split('/') {
            if segment.is_empty() {
                continue;
            }

            pattern.push('/');

            if segment.starts_with('{') && segment.ends_with('}') {
                // Path parameter
                let name = &segment[1..segment.len() - 1];
                param_names.push(name.to_string());
                // Match any non-slash characters
                pattern.push_str("([^/]+)");
            } else if segment.starts_with('*') {
                // Wildcard (catch-all)
                let name = &segment[1..];
                if !name.is_empty() {
                    param_names.push(name.to_string());
                }
                // Match remaining path
                pattern.push_str("(.+)");
            } else {
                // Literal segment - escape regex metacharacters
                pattern.push_str(&regex::escape(segment));
            }
        }

        // Handle root path
        if template == "/" {
            pattern = String::from("^/$");
        } else {
            pattern.push_str("/?$");
        }

        let regex = Regex::new(&pattern).expect("valid regex");
        (regex, param_names)
    }

    /// Compare route specificity for sorting.
    /// More specific routes (fewer parameters, longer literals) come first.
    fn route_specificity(a: &str, b: &str) -> std::cmp::Ordering {
        let a_params = a.matches('{').count();
        let b_params = b.matches('{').count();

        // Fewer parameters = more specific
        if a_params != b_params {
            return a_params.cmp(&b_params);
        }

        // Longer path = more specific (among same param count)
        b.len().cmp(&a.len())
    }
}

impl From<&LoadedArtifact> for OperationResolver {
    fn from(artifact: &LoadedArtifact) -> Self {
        Self::from_artifact(artifact)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::LoadedOperation;

    fn create_test_artifact() -> LoadedArtifact {
        LoadedArtifact {
            service: "test-service".to_string(),
            version: "1.0.0".to_string(),
            format: "openapi".to_string(),
            operations: vec![
                LoadedOperation {
                    id: "listUsers".to_string(),
                    method: "GET".to_string(),
                    path: "/users".to_string(),
                    summary: None,
                    deprecated: false,
                    security: vec![],
                    request_schema: None,
                    response_schemas: HashMap::new(),
                    tags: vec!["users".to_string()],
                },
                LoadedOperation {
                    id: "getUser".to_string(),
                    method: "GET".to_string(),
                    path: "/users/{userId}".to_string(),
                    summary: None,
                    deprecated: false,
                    security: vec![],
                    request_schema: None,
                    response_schemas: HashMap::new(),
                    tags: vec!["users".to_string()],
                },
                LoadedOperation {
                    id: "createUser".to_string(),
                    method: "POST".to_string(),
                    path: "/users".to_string(),
                    summary: None,
                    deprecated: false,
                    security: vec![],
                    request_schema: None,
                    response_schemas: HashMap::new(),
                    tags: vec!["users".to_string()],
                },
                LoadedOperation {
                    id: "getUserOrders".to_string(),
                    method: "GET".to_string(),
                    path: "/users/{userId}/orders".to_string(),
                    summary: None,
                    deprecated: false,
                    security: vec![],
                    request_schema: None,
                    response_schemas: HashMap::new(),
                    tags: vec!["users".to_string(), "orders".to_string()],
                },
                LoadedOperation {
                    id: "getOrder".to_string(),
                    method: "GET".to_string(),
                    path: "/orders/{orderId}".to_string(),
                    summary: None,
                    deprecated: true,
                    security: vec![],
                    request_schema: None,
                    response_schemas: HashMap::new(),
                    tags: vec!["orders".to_string()],
                },
            ],
            schemas: HashMap::new(),
        }
    }

    #[test]
    fn test_resolve_simple_path() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        let resolution = resolver.resolve("GET", "/users").unwrap();
        assert_eq!(resolution.operation_id, "listUsers");
        assert!(resolution.path_params.is_empty());
    }

    #[test]
    fn test_resolve_with_path_param() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        let resolution = resolver.resolve("GET", "/users/123").unwrap();
        assert_eq!(resolution.operation_id, "getUser");
        assert_eq!(resolution.path_params.get("userId"), Some(&"123".to_string()));
    }

    #[test]
    fn test_resolve_nested_path() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        let resolution = resolver.resolve("GET", "/users/456/orders").unwrap();
        assert_eq!(resolution.operation_id, "getUserOrders");
        assert_eq!(resolution.path_params.get("userId"), Some(&"456".to_string()));
    }

    #[test]
    fn test_resolve_different_methods() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        let get = resolver.resolve("GET", "/users").unwrap();
        assert_eq!(get.operation_id, "listUsers");

        let post = resolver.resolve("POST", "/users").unwrap();
        assert_eq!(post.operation_id, "createUser");
    }

    #[test]
    fn test_resolve_not_found() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        let result = resolver.resolve("GET", "/nonexistent");
        assert!(matches!(result, Err(SentinelError::OperationNotFound { .. })));
    }

    #[test]
    fn test_resolve_method_not_found() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        let result = resolver.resolve("DELETE", "/users");
        assert!(matches!(result, Err(SentinelError::OperationNotFound { .. })));
    }

    #[test]
    fn test_deprecated_flag() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        let resolution = resolver.resolve("GET", "/orders/123").unwrap();
        assert!(resolution.deprecated);
    }

    #[test]
    fn test_has_route() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        assert!(resolver.has_route("GET", "/users"));
        assert!(resolver.has_route("POST", "/users"));
        assert!(!resolver.has_route("DELETE", "/users"));
        assert!(!resolver.has_route("GET", "/nonexistent"));
    }

    #[test]
    fn test_case_insensitive_method() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        assert!(resolver.resolve("get", "/users").is_ok());
        assert!(resolver.resolve("Get", "/users").is_ok());
        assert!(resolver.resolve("GET", "/users").is_ok());
    }

    #[test]
    fn test_trailing_slash() {
        let artifact = create_test_artifact();
        let resolver = OperationResolver::from_artifact(&artifact);

        // Both with and without trailing slash should match
        assert!(resolver.resolve("GET", "/users").is_ok());
        assert!(resolver.resolve("GET", "/users/").is_ok());
    }
}
