//! High-level router API.
//!
//! This module provides the main [`Router`] struct which is the primary
//! interface for building and matching routes.

use http::Method;

use crate::method_router::MethodRouter;
use crate::node::Node;
use crate::params::Params;
use crate::RouteMatch;

/// A high-performance radix tree router.
///
/// The router uses a radix tree (compressed trie) for efficient path matching.
/// Routes are matched in O(k) time where k is the length of the path.
///
/// # Example
///
/// ```rust
/// use archimedes_router::{Router, MethodRouter};
/// use http::Method;
///
/// let mut router = Router::new();
///
/// // Add routes using fluent API
/// router.insert("/users", MethodRouter::new().get("listUsers").post("createUser"));
/// router.insert("/users/{id}", MethodRouter::new().get("getUser").put("updateUser"));
///
/// // Match incoming requests
/// let result = router.match_route(&Method::GET, "/users/123");
/// assert!(result.is_some());
/// ```
///
/// # Route Priority
///
/// When multiple routes could match, the router uses the following priority:
///
/// 1. **Static segments** (e.g., `/users/me`)
/// 2. **Parameter segments** (e.g., `/users/{id}`)
/// 3. **Wildcard segments** (e.g., `/files/*path`)
///
/// This means `/users/me` will match before `/users/{id}` for the path `/users/me`.
#[derive(Debug, Clone)]
pub struct Router {
    /// Root node of the radix tree
    root: Node,
    /// Number of routes registered
    route_count: usize,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    /// Creates a new empty router.
    #[must_use]
    pub fn new() -> Self {
        Self {
            root: Node::root(),
            route_count: 0,
        }
    }

    /// Inserts a route into the router.
    ///
    /// # Arguments
    ///
    /// * `path` - The path pattern (e.g., "/users/{id}")
    /// * `methods` - The method router for this path
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_router::{Router, MethodRouter};
    ///
    /// let mut router = Router::new();
    /// router.insert("/users", MethodRouter::new().get("listUsers").post("createUser"));
    /// ```
    pub fn insert(&mut self, path: &str, methods: MethodRouter) {
        self.root.insert(path, methods);
        self.route_count += 1;
    }

    /// Convenience method to add a single-method route.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_router::Router;
    /// use http::Method;
    ///
    /// let mut router = Router::new();
    /// router.route(&Method::GET, "/users", "listUsers");
    /// ```
    pub fn route(&mut self, method: &Method, path: &str, operation_id: impl Into<String>) {
        // Check if path already exists, otherwise create new
        let methods = MethodRouter::new().method(method, operation_id);
        self.insert(path, methods);
    }

    /// Matches a path and method against the router.
    ///
    /// Returns a [`RouteMatch`] if a matching route is found.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_router::{Router, MethodRouter};
    /// use http::Method;
    ///
    /// let mut router = Router::new();
    /// router.insert("/users/{id}", MethodRouter::new().get("getUser"));
    ///
    /// let result = router.match_route(&Method::GET, "/users/123");
    /// assert!(result.is_some());
    ///
    /// let route_match = result.unwrap();
    /// assert_eq!(route_match.operation_id, "getUser");
    /// assert_eq!(route_match.params.get("id"), Some("123"));
    /// ```
    #[must_use]
    pub fn match_route(&self, method: &Method, path: &str) -> Option<RouteMatch<'_>> {
        let (methods, params) = self.root.match_path(path)?;
        let operation_id = methods.get_operation(method)?;
        Some(RouteMatch::new(operation_id, params))
    }

    /// Matches a path against the router (without method).
    ///
    /// Returns the method router and extracted parameters if a path matches.
    /// Useful for checking allowed methods or generating 405 responses.
    #[must_use]
    pub fn match_path(&self, path: &str) -> Option<(&MethodRouter, Params)> {
        self.root.match_path(path)
    }

    /// Returns the number of routes registered.
    #[must_use]
    pub fn len(&self) -> usize {
        self.route_count
    }

    /// Returns true if no routes are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.route_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_new() {
        let router = Router::new();
        assert!(router.is_empty());
        assert_eq!(router.len(), 0);
    }

    #[test]
    fn test_router_insert() {
        let mut router = Router::new();
        router.insert("/users", MethodRouter::new().get("listUsers"));
        assert_eq!(router.len(), 1);
        assert!(!router.is_empty());
    }

    #[test]
    fn test_router_match_static() {
        let mut router = Router::new();
        router.insert("/users", MethodRouter::new().get("listUsers"));

        let result = router.match_route(&Method::GET, "/users");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id, "listUsers");
    }

    #[test]
    fn test_router_match_param() {
        let mut router = Router::new();
        router.insert("/users/{id}", MethodRouter::new().get("getUser"));

        let result = router.match_route(&Method::GET, "/users/123");
        assert!(result.is_some());

        let route_match = result.unwrap();
        assert_eq!(route_match.operation_id, "getUser");
        assert_eq!(route_match.params.get("id"), Some("123"));
    }

    #[test]
    fn test_router_match_wildcard() {
        let mut router = Router::new();
        router.insert("/files/*path", MethodRouter::new().get("serveFile"));

        let result = router.match_route(&Method::GET, "/files/images/logo.png");
        assert!(result.is_some());

        let route_match = result.unwrap();
        assert_eq!(route_match.operation_id, "serveFile");
        assert_eq!(route_match.params.get("path"), Some("images/logo.png"));
    }

    #[test]
    fn test_router_method_not_allowed() {
        let mut router = Router::new();
        router.insert("/users", MethodRouter::new().get("listUsers"));

        // Path matches but method doesn't
        let result = router.match_route(&Method::POST, "/users");
        assert!(result.is_none());

        // Can still check if path exists
        let path_match = router.match_path("/users");
        assert!(path_match.is_some());
    }

    #[test]
    fn test_router_no_match() {
        let mut router = Router::new();
        router.insert("/users", MethodRouter::new().get("listUsers"));

        let result = router.match_route(&Method::GET, "/posts");
        assert!(result.is_none());
    }

    #[test]
    fn test_router_multiple_methods() {
        let mut router = Router::new();
        router.insert(
            "/users",
            MethodRouter::new()
                .get("listUsers")
                .post("createUser")
                .delete("deleteAllUsers"),
        );

        assert_eq!(
            router
                .match_route(&Method::GET, "/users")
                .map(|m| m.operation_id),
            Some("listUsers")
        );
        assert_eq!(
            router
                .match_route(&Method::POST, "/users")
                .map(|m| m.operation_id),
            Some("createUser")
        );
        assert_eq!(
            router
                .match_route(&Method::DELETE, "/users")
                .map(|m| m.operation_id),
            Some("deleteAllUsers")
        );
    }

    #[test]
    fn test_router_route_convenience() {
        let mut router = Router::new();
        router.route(&Method::GET, "/health", "healthCheck");

        let result = router.match_route(&Method::GET, "/health");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id, "healthCheck");
    }

    #[test]
    fn test_router_complex_paths() {
        let mut router = Router::new();
        router.insert("/api/v1/users", MethodRouter::new().get("listUsers"));
        router.insert("/api/v1/users/{userId}", MethodRouter::new().get("getUser"));
        router.insert(
            "/api/v1/users/{userId}/posts",
            MethodRouter::new().get("listUserPosts"),
        );
        router.insert(
            "/api/v1/users/{userId}/posts/{postId}",
            MethodRouter::new().get("getUserPost"),
        );

        let result = router.match_route(&Method::GET, "/api/v1/users/123/posts/456");
        assert!(result.is_some());

        let route_match = result.unwrap();
        assert_eq!(route_match.operation_id, "getUserPost");
        assert_eq!(route_match.params.get("userId"), Some("123"));
        assert_eq!(route_match.params.get("postId"), Some("456"));
    }

    #[test]
    fn test_router_default() {
        let router = Router::default();
        assert!(router.is_empty());
    }

    #[test]
    fn test_router_clone() {
        let mut router = Router::new();
        router.insert("/users", MethodRouter::new().get("listUsers"));

        let cloned = router.clone();
        let result = cloned.match_route(&Method::GET, "/users");
        assert!(result.is_some());
    }

    #[test]
    fn test_router_static_vs_param_priority() {
        let mut router = Router::new();
        router.insert("/users/me", MethodRouter::new().get("getCurrentUser"));
        router.insert("/users/{id}", MethodRouter::new().get("getUser"));

        // "/users/me" should match static route
        let result = router.match_route(&Method::GET, "/users/me");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id, "getCurrentUser");

        // "/users/123" should match param route
        let result = router.match_route(&Method::GET, "/users/123");
        assert!(result.is_some());
        let route_match = result.unwrap();
        assert_eq!(route_match.operation_id, "getUser");
        assert_eq!(route_match.params.get("id"), Some("123"));
    }

    #[test]
    fn test_router_trailing_slash() {
        let mut router = Router::new();
        router.insert("/users", MethodRouter::new().get("listUsers"));

        // Without trailing slash
        let result = router.match_route(&Method::GET, "/users");
        assert!(result.is_some());

        // With trailing slash - matches because empty segments are filtered
        // This is the desired behavior (trailing slashes are normalized)
        let result = router.match_route(&Method::GET, "/users/");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id, "listUsers");
    }

    #[test]
    fn test_router_empty_path() {
        let mut router = Router::new();
        router.insert("/", MethodRouter::new().get("root"));

        let result = router.match_route(&Method::GET, "/");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id, "root");
    }
}
