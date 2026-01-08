//! Request routing and path matching.
//!
//! This module provides routing functionality that maps incoming HTTP
//! requests to operation IDs defined in contracts. The router internally
//! uses a high-performance radix tree from [`archimedes_router`] for
//! O(k) path matching where k is the path length.
//!
//! # Architecture
//!
//! The router works in two stages:
//!
//! 1. **Path Resolution**: Match incoming path + method to an `operationId`
//! 2. **Handler Dispatch**: Look up and invoke the handler for that operation
//!
//! This contract-first approach ensures all routes are defined in the
//! API contract (OpenAPI/AsyncAPI) and validated at startup.
//!
//! # Performance
//!
//! The router uses a radix tree (compressed trie) data structure which provides:
//! - O(k) path matching where k is the path length
//! - Static > Parameter > Wildcard priority
//! - Minimal heap allocations via small-vector optimization
//!
//! # Example
//!
//! ```rust
//! use archimedes_server::{Router, RouteMatch};
//! use http::Method;
//!
//! let mut router = Router::new();
//!
//! // Register routes from contract
//! router.add_route(Method::GET, "/users/{userId}", "getUser");
//! router.add_route(Method::POST, "/users", "createUser");
//!
//! // Match incoming requests
//! let result = router.match_route(&Method::GET, "/users/123");
//! assert!(result.is_some());
//!
//! let route_match = result.unwrap();
//! assert_eq!(route_match.operation_id(), "getUser");
//! assert_eq!(route_match.params().get("userId"), Some(&"123".to_string()));
//! ```

use std::collections::HashMap;

use archimedes_router::MethodRouter;
use http::Method;

/// A matched route with extracted path parameters.
///
/// Returned by [`Router::match_route`] when a route is found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMatch {
    /// The operation ID from the contract
    operation_id: String,

    /// Extracted path parameters (e.g., `userId` from `/users/{userId}`)
    params: HashMap<String, String>,
}

impl RouteMatch {
    /// Creates a new route match.
    #[must_use]
    pub fn new(operation_id: impl Into<String>, params: HashMap<String, String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            params,
        }
    }

    /// Returns the operation ID for this route.
    #[must_use]
    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }

    /// Returns the extracted path parameters.
    #[must_use]
    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }

    /// Returns a specific path parameter by name.
    #[must_use]
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(String::as_str)
    }
}

/// HTTP request router.
///
/// Routes incoming requests to operation IDs based on method and path.
/// Supports path parameters using OpenAPI-style `{paramName}` syntax
/// and wildcards using `*paramName` syntax.
///
/// Internally uses a high-performance radix tree from [`archimedes_router`]
/// for O(k) path matching where k is the path length.
///
/// # Example
///
/// ```rust
/// use archimedes_server::Router;
/// use http::Method;
///
/// let mut router = Router::new();
///
/// // Add routes
/// router.add_route(Method::GET, "/users", "listUsers");
/// router.add_route(Method::GET, "/users/{userId}", "getUser");
/// router.add_route(Method::POST, "/users", "createUser");
/// router.add_route(Method::DELETE, "/users/{userId}", "deleteUser");
///
/// // Match a request
/// let result = router.match_route(&Method::GET, "/users/42");
/// assert!(result.is_some());
///
/// let m = result.unwrap();
/// assert_eq!(m.operation_id(), "getUser");
/// assert_eq!(m.param("userId"), Some("42"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct Router {
    /// Internal radix tree router
    inner: archimedes_router::Router,

    /// Track operation IDs for `has_operation` queries
    /// Maps `operation_id` -> route count (for tracking)
    operation_ids: HashMap<String, usize>,
}

impl Router {
    /// Creates a new empty router.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::Router;
    ///
    /// let router = Router::new();
    /// assert_eq!(router.route_count(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: archimedes_router::Router::new(),
            operation_ids: HashMap::new(),
        }
    }

    /// Adds a route to the router.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method for this route
    /// * `pattern` - Path pattern (e.g., "/users/{userId}")
    /// * `operation_id` - Operation ID from the contract
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::Router;
    /// use http::Method;
    ///
    /// let mut router = Router::new();
    /// router.add_route(Method::GET, "/health", "healthCheck");
    /// assert_eq!(router.route_count(), 1);
    /// ```
    pub fn add_route(
        &mut self,
        method: Method,
        pattern: impl AsRef<str>,
        operation_id: impl Into<String>,
    ) {
        let operation_id = operation_id.into();
        let method_router = MethodRouter::new().method(&method, &operation_id);
        self.inner.insert(pattern.as_ref(), method_router);

        // Track the operation ID
        *self.operation_ids.entry(operation_id).or_insert(0) += 1;
    }

    /// Returns the number of registered routes.
    #[must_use]
    pub fn route_count(&self) -> usize {
        self.inner.len()
    }

    /// Matches an incoming request to a route.
    ///
    /// Returns `Some(RouteMatch)` if a matching route is found,
    /// or `None` if no route matches.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method of the request
    /// * `path` - Request path (e.g., "/users/123")
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::Router;
    /// use http::Method;
    ///
    /// let mut router = Router::new();
    /// router.add_route(Method::GET, "/users/{userId}", "getUser");
    ///
    /// // Matching request
    /// let result = router.match_route(&Method::GET, "/users/abc");
    /// assert!(result.is_some());
    ///
    /// // Non-matching method
    /// let result = router.match_route(&Method::POST, "/users/abc");
    /// assert!(result.is_none());
    ///
    /// // Non-matching path
    /// let result = router.match_route(&Method::GET, "/products");
    /// assert!(result.is_none());
    /// ```
    #[must_use]
    pub fn match_route(&self, method: &Method, path: &str) -> Option<RouteMatch> {
        // Normalize path: ensure it has leading slash and trim trailing slash
        let path = normalize_path(path);

        // Use the radix tree router for O(k) matching
        let route_match = self.inner.match_route(method, &path)?;

        // Convert archimedes_router::Params to HashMap<String, String>
        let params: HashMap<String, String> = route_match
            .params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Some(RouteMatch::new(
            route_match.operation_id.to_string(),
            params,
        ))
    }

    /// Checks if a specific operation ID is registered.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID to check
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::Router;
    /// use http::Method;
    ///
    /// let mut router = Router::new();
    /// router.add_route(Method::GET, "/health", "healthCheck");
    ///
    /// assert!(router.has_operation("healthCheck"));
    /// assert!(!router.has_operation("unknown"));
    /// ```
    #[must_use]
    pub fn has_operation(&self, operation_id: &str) -> bool {
        self.operation_ids.contains_key(operation_id)
    }

    /// Returns all registered operation IDs.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_server::Router;
    /// use http::Method;
    ///
    /// let mut router = Router::new();
    /// router.add_route(Method::GET, "/users", "listUsers");
    /// router.add_route(Method::POST, "/users", "createUser");
    ///
    /// let ops: Vec<_> = router.operation_ids().collect();
    /// assert!(ops.contains(&"listUsers"));
    /// assert!(ops.contains(&"createUser"));
    /// ```
    pub fn operation_ids(&self) -> impl Iterator<Item = &str> {
        self.operation_ids.keys().map(String::as_str)
    }
}

/// Normalizes a path for routing.
///
/// - Ensures leading slash
/// - Removes trailing slash (except for root "/")
fn normalize_path(path: &str) -> String {
    let path = if path.is_empty() || !path.starts_with('/') {
        format!("/{path}")
    } else {
        path.to_string()
    };

    // Remove trailing slash (except for root)
    if path.len() > 1 && path.ends_with('/') {
        path[..path.len() - 1].to_string()
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_new() {
        let router = Router::new();
        assert_eq!(router.route_count(), 0);
    }

    #[test]
    fn test_router_add_route() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/health", "healthCheck");
        assert_eq!(router.route_count(), 1);
    }

    #[test]
    fn test_router_match_simple_path() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/health", "healthCheck");

        let result = router.match_route(&Method::GET, "/health");
        assert!(result.is_some());

        let m = result.unwrap();
        assert_eq!(m.operation_id(), "healthCheck");
        assert!(m.params().is_empty());
    }

    #[test]
    fn test_router_match_with_param() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users/{userId}", "getUser");

        let result = router.match_route(&Method::GET, "/users/123");
        assert!(result.is_some());

        let m = result.unwrap();
        assert_eq!(m.operation_id(), "getUser");
        assert_eq!(m.param("userId"), Some("123"));
    }

    #[test]
    fn test_router_match_with_multiple_params() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users/{userId}/posts/{postId}", "getUserPost");

        let result = router.match_route(&Method::GET, "/users/42/posts/99");
        assert!(result.is_some());

        let m = result.unwrap();
        assert_eq!(m.operation_id(), "getUserPost");
        assert_eq!(m.param("userId"), Some("42"));
        assert_eq!(m.param("postId"), Some("99"));
    }

    #[test]
    fn test_router_match_method_mismatch() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users", "listUsers");

        let result = router.match_route(&Method::POST, "/users");
        assert!(result.is_none());
    }

    #[test]
    fn test_router_match_path_mismatch() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users", "listUsers");

        let result = router.match_route(&Method::GET, "/products");
        assert!(result.is_none());
    }

    #[test]
    fn test_router_match_segment_count_mismatch() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users/{userId}", "getUser");

        // Too few segments
        let result = router.match_route(&Method::GET, "/users");
        assert!(result.is_none());

        // Too many segments
        let result = router.match_route(&Method::GET, "/users/123/extra");
        assert!(result.is_none());
    }

    #[test]
    fn test_router_multiple_routes_same_path_different_method() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users", "listUsers");
        router.add_route(Method::POST, "/users", "createUser");
        router.add_route(Method::DELETE, "/users/{userId}", "deleteUser");

        let get_result = router.match_route(&Method::GET, "/users");
        assert_eq!(get_result.unwrap().operation_id(), "listUsers");

        let post_result = router.match_route(&Method::POST, "/users");
        assert_eq!(post_result.unwrap().operation_id(), "createUser");

        let delete_result = router.match_route(&Method::DELETE, "/users/123");
        assert_eq!(delete_result.unwrap().operation_id(), "deleteUser");
    }

    #[test]
    fn test_router_has_operation() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/health", "healthCheck");

        assert!(router.has_operation("healthCheck"));
        assert!(!router.has_operation("unknown"));
    }

    #[test]
    fn test_router_operation_ids() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users", "listUsers");
        router.add_route(Method::POST, "/users", "createUser");
        router.add_route(Method::GET, "/health", "healthCheck");

        let ops: Vec<_> = router.operation_ids().collect();
        assert_eq!(ops.len(), 3);
        assert!(ops.contains(&"listUsers"));
        assert!(ops.contains(&"createUser"));
        assert!(ops.contains(&"healthCheck"));
    }

    #[test]
    fn test_route_match_params() {
        let params = [("userId".to_string(), "123".to_string())]
            .into_iter()
            .collect();
        let m = RouteMatch::new("getUser", params);

        assert_eq!(m.operation_id(), "getUser");
        assert_eq!(m.param("userId"), Some("123"));
        assert_eq!(m.param("nonexistent"), None);
        assert_eq!(m.params().len(), 1);
    }

    #[test]
    fn test_router_default() {
        let router = Router::default();
        assert_eq!(router.route_count(), 0);
    }

    #[test]
    fn test_path_with_leading_slash() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users", "listUsers");

        // Both with and without leading slash should work
        assert!(router.match_route(&Method::GET, "/users").is_some());
        assert!(router.match_route(&Method::GET, "users").is_some());
    }

    #[test]
    fn test_path_with_trailing_slash() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/users", "listUsers");

        // Path with trailing slash - our implementation normalizes trailing slashes
        let result = router.match_route(&Method::GET, "/users/");
        // The router implementation strips trailing slashes, so this should match
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id(), "listUsers");
    }

    #[test]
    fn test_empty_path() {
        let mut router = Router::new();
        router.add_route(Method::GET, "/", "root");

        let result = router.match_route(&Method::GET, "/");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id(), "root");
    }

    #[test]
    fn test_complex_path_pattern() {
        let mut router = Router::new();
        router.add_route(
            Method::GET,
            "/api/v1/organizations/{orgId}/users/{userId}/settings",
            "getUserSettings",
        );

        let result = router.match_route(
            &Method::GET,
            "/api/v1/organizations/acme/users/john/settings",
        );
        assert!(result.is_some());

        let m = result.unwrap();
        assert_eq!(m.operation_id(), "getUserSettings");
        assert_eq!(m.param("orgId"), Some("acme"));
        assert_eq!(m.param("userId"), Some("john"));
    }

    #[test]
    fn test_route_match_clone() {
        let params = [("id".to_string(), "42".to_string())].into_iter().collect();
        let m1 = RouteMatch::new("test", params);
        let m2 = m1.clone();

        assert_eq!(m1, m2);
        assert_eq!(m1.operation_id(), m2.operation_id());
    }

    #[test]
    fn test_router_clone() {
        let mut router1 = Router::new();
        router1.add_route(Method::GET, "/health", "healthCheck");

        let router2 = router1.clone();

        assert_eq!(router1.route_count(), router2.route_count());
        assert!(router2.match_route(&Method::GET, "/health").is_some());
    }
}
