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
///
/// # Sub-Router Nesting
///
/// Routers can be composed using the `nest()` method:
///
/// ```rust
/// use archimedes_router::{Router, MethodRouter};
/// use http::Method;
///
/// // Create a sub-router for users
/// let mut users = Router::new();
/// users.insert("/", MethodRouter::new().get("listUsers").post("createUser"));
/// users.insert("/{id}", MethodRouter::new().get("getUser"));
///
/// // Nest it under /api/v1/users
/// let mut api = Router::new();
/// api.nest("/api/v1/users", users);
///
/// // Routes are now available at the nested path
/// assert!(api.match_route(&Method::GET, "/api/v1/users").is_some());
/// assert!(api.match_route(&Method::GET, "/api/v1/users/123").is_some());
/// ```
#[derive(Debug, Clone)]
pub struct Router {
    /// Root node of the radix tree
    root: Node,
    /// Number of routes registered
    route_count: usize,
    /// Optional path prefix for all routes
    prefix: Option<String>,
    /// Optional `OpenAPI` tags for all routes
    tags: Vec<String>,
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
            prefix: None,
            tags: Vec::new(),
        }
    }

    /// Creates a new router with a path prefix.
    ///
    /// All routes added to this router will have this prefix prepended.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_router::{Router, MethodRouter};
    /// use http::Method;
    ///
    /// let mut router = Router::with_prefix("/api/v1");
    /// router.insert("/users", MethodRouter::new().get("listUsers"));
    ///
    /// // Route is available at /api/v1/users
    /// assert!(router.match_route(&Method::GET, "/api/v1/users").is_some());
    /// ```
    #[must_use]
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            root: Node::root(),
            route_count: 0,
            prefix: Some(normalize_path(&prefix.into())),
            tags: Vec::new(),
        }
    }

    /// Sets a path prefix for all routes in this router.
    ///
    /// This is a builder-style method that returns `Self`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_router::{Router, MethodRouter};
    /// use http::Method;
    ///
    /// let mut router = Router::new()
    ///     .prefix("/api/v1");
    /// router.insert("/users", MethodRouter::new().get("listUsers"));
    ///
    /// assert!(router.match_route(&Method::GET, "/api/v1/users").is_some());
    /// ```
    #[must_use]
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(normalize_path(&prefix.into()));
        self
    }

    /// Adds an `OpenAPI` tag to all routes in this router.
    ///
    /// Tags are used for grouping routes in `OpenAPI` documentation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_router::Router;
    ///
    /// let router = Router::new()
    ///     .tag("users")
    ///     .tag("admin");
    ///
    /// assert_eq!(router.tags(), &["users", "admin"]);
    /// ```
    #[must_use]
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Returns the tags associated with this router.
    #[must_use]
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Nests another router at the given path prefix.
    ///
    /// All routes from the nested router will be available under the given prefix.
    /// The nested router's own prefix (if any) is combined with the nest prefix.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_router::{Router, MethodRouter};
    /// use http::Method;
    ///
    /// // Create a users router
    /// let mut users = Router::new();
    /// users.insert("/", MethodRouter::new().get("listUsers"));
    /// users.insert("/{id}", MethodRouter::new().get("getUser"));
    ///
    /// // Create an orders router
    /// let mut orders = Router::new();
    /// orders.insert("/", MethodRouter::new().get("listOrders"));
    ///
    /// // Nest both under /api/v1
    /// let mut api = Router::new();
    /// api.nest("/api/v1/users", users);
    /// api.nest("/api/v1/orders", orders);
    ///
    /// // Routes are available at nested paths
    /// assert!(api.match_route(&Method::GET, "/api/v1/users").is_some());
    /// assert!(api.match_route(&Method::GET, "/api/v1/users/123").is_some());
    /// assert!(api.match_route(&Method::GET, "/api/v1/orders").is_some());
    /// ```
    pub fn nest(&mut self, prefix: &str, other: Router) {
        let prefix = normalize_path(prefix);

        // Iterate through all routes in the other router
        // We need to traverse the tree and collect all paths
        self.merge_with_prefix(&other.root, &prefix, "");
        self.route_count += other.route_count;
    }

    /// Merges all routes from another router into this one.
    ///
    /// Unlike `nest()`, this doesn't add a prefix - routes are added as-is.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_router::{Router, MethodRouter};
    /// use http::Method;
    ///
    /// let mut users = Router::new();
    /// users.insert("/users", MethodRouter::new().get("listUsers"));
    ///
    /// let mut api = Router::new();
    /// api.insert("/health", MethodRouter::new().get("health"));
    /// api.merge(users);
    ///
    /// // Both routes are available
    /// assert!(api.match_route(&Method::GET, "/health").is_some());
    /// assert!(api.match_route(&Method::GET, "/users").is_some());
    /// ```
    pub fn merge(&mut self, other: Router) {
        self.merge_with_prefix(&other.root, "", "");
        self.route_count += other.route_count;
    }

    /// Helper to recursively merge nodes with a prefix.
    fn merge_with_prefix(&mut self, node: &Node, prefix: &str, current_path: &str) {
        // Build the current full path
        let node_segment = node.segment();
        let full_path = if current_path.is_empty() && node_segment.is_empty() {
            prefix.to_string()
        } else if current_path.is_empty() {
            format!("{prefix}/{node_segment}")
        } else if node_segment.is_empty() {
            format!("{prefix}{current_path}")
        } else {
            format!("{prefix}{current_path}/{node_segment}")
        };

        // If this node has methods, add the route
        if let Some(methods) = node.methods() {
            let path = if full_path.is_empty() {
                "/".to_string()
            } else {
                normalize_path(&full_path)
            };
            self.root.insert(&path, methods.clone());
        }

        // Recursively process children
        for child in node.children() {
            let child_path = if current_path.is_empty() && node_segment.is_empty() {
                String::new()
            } else if current_path.is_empty() {
                format!("/{node_segment}")
            } else {
                format!("{current_path}/{node_segment}")
            };
            self.merge_with_prefix(child, prefix, &child_path);
        }
    }

    /// Inserts a route into the router.
    ///
    /// If this router has a prefix set, it will be prepended to the path.
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
        let full_path = match &self.prefix {
            Some(prefix) => {
                let normalized = normalize_path(path);
                if normalized == "/" {
                    prefix.clone()
                } else {
                    format!("{prefix}{normalized}")
                }
            }
            None => normalize_path(path),
        };
        self.root.insert(&full_path, methods);
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

    /// Returns the prefix for this router, if set.
    #[must_use]
    pub fn get_prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }
}

/// Normalizes a path by ensuring it starts with `/` and doesn't end with `/`.
fn normalize_path(path: &str) -> String {
    let path = path.trim();
    if path.is_empty() || path == "/" {
        return "/".to_string();
    }

    let mut normalized = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };

    // Remove trailing slash unless it's the root
    if normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }

    normalized
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

    // ============== Prefix Tests ==============

    #[test]
    fn test_router_with_prefix() {
        let mut router = Router::with_prefix("/api/v1");
        router.insert("/users", MethodRouter::new().get("listUsers"));

        let result = router.match_route(&Method::GET, "/api/v1/users");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id, "listUsers");
    }

    #[test]
    fn test_router_prefix_builder() {
        let mut router = Router::new().prefix("/api/v1");
        router.insert("/users", MethodRouter::new().get("listUsers"));

        let result = router.match_route(&Method::GET, "/api/v1/users");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id, "listUsers");
    }

    #[test]
    fn test_router_prefix_with_root() {
        let mut router = Router::with_prefix("/api/v1");
        router.insert("/", MethodRouter::new().get("apiRoot"));

        let result = router.match_route(&Method::GET, "/api/v1");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id, "apiRoot");
    }

    #[test]
    fn test_router_get_prefix() {
        let router = Router::with_prefix("/api/v1");
        assert_eq!(router.get_prefix(), Some("/api/v1"));

        let router_no_prefix = Router::new();
        assert_eq!(router_no_prefix.get_prefix(), None);
    }

    // ============== Tag Tests ==============

    #[test]
    fn test_router_tag() {
        let router = Router::new().tag("users").tag("admin");
        assert_eq!(router.tags(), &["users", "admin"]);
    }

    #[test]
    fn test_router_tags_empty() {
        let router = Router::new();
        assert!(router.tags().is_empty());
    }

    // ============== Nest Tests ==============

    #[test]
    fn test_router_nest_basic() {
        let mut users = Router::new();
        users.insert("/", MethodRouter::new().get("listUsers").post("createUser"));
        users.insert("/{id}", MethodRouter::new().get("getUser"));

        let mut api = Router::new();
        api.nest("/api/v1/users", users);

        // Check nested routes
        assert!(api.match_route(&Method::GET, "/api/v1/users").is_some());
        assert!(api.match_route(&Method::POST, "/api/v1/users").is_some());
        assert!(api.match_route(&Method::GET, "/api/v1/users/123").is_some());
    }

    #[test]
    fn test_router_nest_multiple() {
        let mut users = Router::new();
        users.insert("/", MethodRouter::new().get("listUsers"));

        let mut orders = Router::new();
        orders.insert("/", MethodRouter::new().get("listOrders"));

        let mut api = Router::new();
        api.nest("/api/v1/users", users);
        api.nest("/api/v1/orders", orders);

        assert!(api.match_route(&Method::GET, "/api/v1/users").is_some());
        assert!(api.match_route(&Method::GET, "/api/v1/orders").is_some());
    }

    #[test]
    fn test_router_nest_with_params() {
        let mut users = Router::new();
        users.insert("/{userId}/posts/{postId}", MethodRouter::new().get("getUserPost"));

        let mut api = Router::new();
        api.nest("/api/v1/users", users);

        let result = api.match_route(&Method::GET, "/api/v1/users/123/posts/456");
        assert!(result.is_some());

        let route_match = result.unwrap();
        assert_eq!(route_match.operation_id, "getUserPost");
        assert_eq!(route_match.params.get("userId"), Some("123"));
        assert_eq!(route_match.params.get("postId"), Some("456"));
    }

    #[test]
    fn test_router_nest_deep() {
        let mut posts = Router::new();
        posts.insert("/", MethodRouter::new().get("listPosts"));

        let mut users = Router::new();
        users.nest("/posts", posts);

        let mut api = Router::new();
        api.nest("/api/v1/users/{userId}", users);

        let result = api.match_route(&Method::GET, "/api/v1/users/123/posts");
        assert!(result.is_some());
        assert_eq!(result.unwrap().operation_id, "listPosts");
    }

    // ============== Merge Tests ==============

    #[test]
    fn test_router_merge_basic() {
        let mut users = Router::new();
        users.insert("/users", MethodRouter::new().get("listUsers"));

        let mut health = Router::new();
        health.insert("/health", MethodRouter::new().get("healthCheck"));

        let mut api = Router::new();
        api.merge(users);
        api.merge(health);

        assert!(api.match_route(&Method::GET, "/users").is_some());
        assert!(api.match_route(&Method::GET, "/health").is_some());
    }

    // ============== normalize_path Tests ==============

    #[test]
    fn test_normalize_path_empty() {
        assert_eq!(normalize_path(""), "/");
    }

    #[test]
    fn test_normalize_path_root() {
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_normalize_path_no_leading_slash() {
        assert_eq!(normalize_path("users"), "/users");
    }

    #[test]
    fn test_normalize_path_trailing_slash() {
        assert_eq!(normalize_path("/users/"), "/users");
    }

    #[test]
    fn test_normalize_path_normal() {
        assert_eq!(normalize_path("/api/v1/users"), "/api/v1/users");
    }

    #[test]
    fn test_normalize_path_whitespace() {
        assert_eq!(normalize_path("  /users  "), "/users");
    }
}
