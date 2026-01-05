//! High-performance radix tree router for Archimedes.
//!
//! This crate provides a routing implementation using a radix tree (compressed trie)
//! data structure for efficient path matching. It's designed to replace linear
//! route matching with O(k) lookup time where k is the path length.
//!
//! # Features
//!
//! - **Radix Tree Matching**: O(k) path lookup vs O(n) linear scan
//! - **Path Parameters**: Extract named parameters from paths (`/users/{id}`)
//! - **Wildcards**: Catch-all routes (`/files/*path`)
//! - **Method-Based Routing**: Different handlers per HTTP method
//! - **Zero Allocations**: Path matching with minimal heap allocations
//!
//! # Example
//!
//! ```rust
//! use archimedes_router::{Router, MethodRouter};
//! use http::Method;
//!
//! let mut router = Router::new();
//!
//! // Add routes
//! router.insert("/users", MethodRouter::new().get("listUsers").post("createUser"));
//! router.insert("/users/{id}", MethodRouter::new().get("getUser").delete("deleteUser"));
//! router.insert("/files/*path", MethodRouter::new().get("serveFile"));
//!
//! // Match routes
//! let result = router.match_route(&Method::GET, "/users/123");
//! assert!(result.is_some());
//!
//! let route_match = result.unwrap();
//! assert_eq!(route_match.operation_id, "getUser");
//! assert_eq!(route_match.params.get("id"), Some("123"));
//! ```
//!
//! # Architecture
//!
//! The router uses a radix tree where each node represents a path segment:
//!
//! ```text
//!                    (root)
//!                      │
//!              ┌───────┴───────┐
//!              │               │
//!            "users"        "files"
//!              │               │
//!        ┌─────┴─────┐        "*path"
//!        │           │
//!       (leaf)    "{id}"
//!   [GET,POST]      │
//!                 (leaf)
//!              [GET,DELETE]
//! ```

mod node;
mod router;
mod method_router;
mod params;

pub use node::Node;
pub use router::Router;
pub use method_router::MethodRouter;
pub use params::Params;

/// A matched route with its operation ID and extracted parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMatch<'a> {
    /// The operation ID for the matched route
    pub operation_id: &'a str,
    /// Extracted path parameters
    pub params: Params,
}

impl<'a> RouteMatch<'a> {
    /// Creates a new route match.
    #[must_use]
    pub fn new(operation_id: &'a str, params: Params) -> Self {
        Self { operation_id, params }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;

    #[test]
    fn test_basic_routing() {
        let mut router = Router::new();
        router.insert("/users", MethodRouter::new().get("listUsers"));
        router.insert("/users/{id}", MethodRouter::new().get("getUser"));

        let result = router.match_route(&Method::GET, "/users");
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.operation_id, "listUsers");
        assert!(m.params.is_empty());

        let result = router.match_route(&Method::GET, "/users/123");
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.operation_id, "getUser");
        assert_eq!(m.params.get("id"), Some("123"));
    }

    #[test]
    fn test_method_routing() {
        let mut router = Router::new();
        router.insert("/users", MethodRouter::new().get("listUsers").post("createUser"));

        let get_result = router.match_route(&Method::GET, "/users");
        assert!(get_result.is_some());
        assert_eq!(get_result.unwrap().operation_id, "listUsers");

        let post_result = router.match_route(&Method::POST, "/users");
        assert!(post_result.is_some());
        assert_eq!(post_result.unwrap().operation_id, "createUser");

        let delete_result = router.match_route(&Method::DELETE, "/users");
        assert!(delete_result.is_none());
    }

    #[test]
    fn test_wildcard_routing() {
        let mut router = Router::new();
        router.insert("/files/*path", MethodRouter::new().get("serveFile"));

        let result = router.match_route(&Method::GET, "/files/images/logo.png");
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.operation_id, "serveFile");
        assert_eq!(m.params.get("path"), Some("images/logo.png"));
    }

    #[test]
    fn test_no_match() {
        let mut router = Router::new();
        router.insert("/users", MethodRouter::new().get("listUsers"));

        let result = router.match_route(&Method::GET, "/posts");
        assert!(result.is_none());
    }

    #[test]
    fn test_multiple_params() {
        let mut router = Router::new();
        router.insert("/orgs/{orgId}/users/{userId}", MethodRouter::new().get("getOrgUser"));

        let result = router.match_route(&Method::GET, "/orgs/acme/users/123");
        assert!(result.is_some());
        let m = result.unwrap();
        assert_eq!(m.operation_id, "getOrgUser");
        assert_eq!(m.params.get("orgId"), Some("acme"));
        assert_eq!(m.params.get("userId"), Some("123"));
    }
}
