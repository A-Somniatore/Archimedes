//! Radix tree node implementation.
//!
//! This module provides the core radix tree (compressed trie) data structure
//! used for efficient path matching.

use crate::method_router::MethodRouter;
use crate::params::Params;

/// Type of path segment in the radix tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentKind {
    /// Static path segment (e.g., "users", "api")
    Static,
    /// Named parameter (e.g., "{id}", "{userId}")
    Param(String),
    /// Catch-all wildcard (e.g., "*path")
    Wildcard(String),
}

/// A node in the radix tree.
///
/// Each node represents a path segment and may have children for
/// sub-paths. Leaf nodes (or nodes at route boundaries) contain
/// a [`MethodRouter`] for handling different HTTP methods.
#[derive(Debug, Clone)]
pub struct Node {
    /// The path segment this node represents
    pub segment: String,

    /// The kind of segment (static, param, or wildcard)
    pub kind: SegmentKind,

    /// Method router for this node (if it's a route endpoint)
    pub methods: Option<MethodRouter>,

    /// Static children, sorted by segment for binary search
    pub static_children: Vec<Node>,

    /// Parameter child (at most one per node)
    pub param_child: Option<Box<Node>>,

    /// Wildcard child (at most one per node, must be leaf)
    pub wildcard_child: Option<Box<Node>>,
}

impl Node {
    /// Creates a new static node.
    #[must_use]
    pub fn new_static(segment: impl Into<String>) -> Self {
        Self {
            segment: segment.into(),
            kind: SegmentKind::Static,
            methods: None,
            static_children: Vec::new(),
            param_child: None,
            wildcard_child: None,
        }
    }

    /// Creates a new parameter node.
    #[must_use]
    pub fn new_param(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            segment: format!("{{{name}}}"),
            kind: SegmentKind::Param(name),
            methods: None,
            static_children: Vec::new(),
            param_child: None,
            wildcard_child: None,
        }
    }

    /// Creates a new wildcard node.
    #[must_use]
    pub fn new_wildcard(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            segment: format!("*{name}"),
            kind: SegmentKind::Wildcard(name),
            methods: None,
            static_children: Vec::new(),
            param_child: None,
            wildcard_child: None,
        }
    }

    /// Creates a root node for the tree.
    #[must_use]
    pub fn root() -> Self {
        Self::new_static("")
    }

    /// Inserts a route into the tree.
    ///
    /// # Arguments
    ///
    /// * `path` - The path pattern (e.g., "/users/{id}")
    /// * `methods` - The method router for this path
    pub fn insert(&mut self, path: &str, methods: MethodRouter) {
        let segments = Self::parse_path(path);
        self.insert_segments(&segments, methods);
    }

    /// Parses a path into segments.
    fn parse_path(path: &str) -> Vec<(String, SegmentKind)> {
        path.split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                if let Some(name) = s.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
                    (s.to_string(), SegmentKind::Param(name.to_string()))
                } else if let Some(name) = s.strip_prefix('*') {
                    (s.to_string(), SegmentKind::Wildcard(name.to_string()))
                } else {
                    (s.to_string(), SegmentKind::Static)
                }
            })
            .collect()
    }

    /// Inserts segments into the tree recursively.
    fn insert_segments(&mut self, segments: &[(String, SegmentKind)], methods: MethodRouter) {
        if segments.is_empty() {
            // This is the target node - merge methods instead of replacing
            if let Some(existing) = &mut self.methods {
                existing.merge(methods);
            } else {
                self.methods = Some(methods);
            }
            return;
        }

        let (segment, kind) = &segments[0];
        let remaining = &segments[1..];

        match kind {
            SegmentKind::Static => {
                // Find or create static child
                if let Some(child) = self
                    .static_children
                    .iter_mut()
                    .find(|c| c.segment == *segment)
                {
                    child.insert_segments(remaining, methods);
                } else {
                    let mut child = Node::new_static(segment);
                    child.insert_segments(remaining, methods);
                    self.static_children.push(child);
                    // Keep sorted for binary search
                    self.static_children
                        .sort_by(|a, b| a.segment.cmp(&b.segment));
                }
            }
            SegmentKind::Param(name) => {
                // Create or reuse param child
                if self.param_child.is_none() {
                    self.param_child = Some(Box::new(Node::new_param(name)));
                }
                if let Some(child) = &mut self.param_child {
                    child.insert_segments(remaining, methods);
                }
            }
            SegmentKind::Wildcard(name) => {
                // Create or reuse wildcard child (must be last segment)
                assert!(
                    remaining.is_empty(),
                    "Wildcard must be the last segment in path"
                );
                if let Some(child) = &mut self.wildcard_child {
                    // Merge with existing wildcard
                    if let Some(existing) = &mut child.methods {
                        existing.merge(methods);
                    } else {
                        child.methods = Some(methods);
                    }
                } else {
                    let mut child = Node::new_wildcard(name);
                    child.methods = Some(methods);
                    self.wildcard_child = Some(Box::new(child));
                }
            }
        }
    }

    /// Matches a path against the tree.
    ///
    /// Returns the method router and extracted parameters if found.
    #[must_use]
    pub fn match_path(&self, path: &str) -> Option<(&MethodRouter, Params)> {
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut params = Params::new();
        self.match_segments(&segments, &mut params)
    }

    /// Matches segments against the tree recursively.
    fn match_segments<'a>(
        &'a self,
        segments: &[&str],
        params: &mut Params,
    ) -> Option<(&'a MethodRouter, Params)> {
        if segments.is_empty() {
            // Check if this node has methods
            return self.methods.as_ref().map(|m| (m, params.clone()));
        }

        let segment = segments[0];
        let remaining = &segments[1..];

        // Try static match first (highest priority)
        if let Some(child) = self.find_static_child(segment) {
            if let Some(result) = child.match_segments(remaining, params) {
                return Some(result);
            }
        }

        // Try parameter match
        if let Some(child) = &self.param_child {
            if let SegmentKind::Param(name) = &child.kind {
                params.push(name.clone(), segment.to_string());
                if let Some(result) = child.match_segments(remaining, params) {
                    return Some(result);
                }
                // Backtrack: remove the param we just added
                // Note: This is a simplified backtracking; for complex cases,
                // we'd need to clone params before trying each branch
            }
        }

        // Try wildcard match (lowest priority, catches all remaining)
        if let Some(child) = &self.wildcard_child {
            if let SegmentKind::Wildcard(name) = &child.kind {
                // Collect all remaining segments
                let remaining_path = segments.join("/");
                params.push(name.clone(), remaining_path);
                return child.methods.as_ref().map(|m| (m, params.clone()));
            }
        }

        None
    }

    /// Finds a static child by segment using binary search.
    fn find_static_child(&self, segment: &str) -> Option<&Node> {
        self.static_children
            .binary_search_by(|c| c.segment.as_str().cmp(segment))
            .ok()
            .map(|i| &self.static_children[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;

    #[test]
    fn test_node_new_static() {
        let node = Node::new_static("users");
        assert_eq!(node.segment, "users");
        assert_eq!(node.kind, SegmentKind::Static);
    }

    #[test]
    fn test_node_new_param() {
        let node = Node::new_param("id");
        assert_eq!(node.segment, "{id}");
        assert_eq!(node.kind, SegmentKind::Param("id".to_string()));
    }

    #[test]
    fn test_node_new_wildcard() {
        let node = Node::new_wildcard("path");
        assert_eq!(node.segment, "*path");
        assert_eq!(node.kind, SegmentKind::Wildcard("path".to_string()));
    }

    #[test]
    fn test_parse_path_static() {
        let segments = Node::parse_path("/users/list");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], ("users".to_string(), SegmentKind::Static));
        assert_eq!(segments[1], ("list".to_string(), SegmentKind::Static));
    }

    #[test]
    fn test_parse_path_param() {
        let segments = Node::parse_path("/users/{id}");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], ("users".to_string(), SegmentKind::Static));
        assert_eq!(
            segments[1],
            ("{id}".to_string(), SegmentKind::Param("id".to_string()))
        );
    }

    #[test]
    fn test_parse_path_wildcard() {
        let segments = Node::parse_path("/files/*path");
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], ("files".to_string(), SegmentKind::Static));
        assert_eq!(
            segments[1],
            (
                "*path".to_string(),
                SegmentKind::Wildcard("path".to_string())
            )
        );
    }

    #[test]
    fn test_insert_and_match_static() {
        let mut root = Node::root();
        root.insert("/users", MethodRouter::new().get("listUsers"));

        let result = root.match_path("/users");
        assert!(result.is_some());

        let (methods, params) = result.unwrap();
        assert_eq!(methods.get_operation(&Method::GET), Some("listUsers"));
        assert!(params.is_empty());
    }

    #[test]
    fn test_insert_and_match_param() {
        let mut root = Node::root();
        root.insert("/users/{id}", MethodRouter::new().get("getUser"));

        let result = root.match_path("/users/123");
        assert!(result.is_some());

        let (methods, params) = result.unwrap();
        assert_eq!(methods.get_operation(&Method::GET), Some("getUser"));
        assert_eq!(params.get("id"), Some("123"));
    }

    #[test]
    fn test_insert_and_match_wildcard() {
        let mut root = Node::root();
        root.insert("/files/*path", MethodRouter::new().get("serveFile"));

        let result = root.match_path("/files/images/logo.png");
        assert!(result.is_some());

        let (methods, params) = result.unwrap();
        assert_eq!(methods.get_operation(&Method::GET), Some("serveFile"));
        assert_eq!(params.get("path"), Some("images/logo.png"));
    }

    #[test]
    fn test_static_priority_over_param() {
        let mut root = Node::root();
        root.insert("/users/me", MethodRouter::new().get("getCurrentUser"));
        root.insert("/users/{id}", MethodRouter::new().get("getUser"));

        // Static "me" should take priority
        let result = root.match_path("/users/me");
        assert!(result.is_some());
        let (methods, _) = result.unwrap();
        assert_eq!(methods.get_operation(&Method::GET), Some("getCurrentUser"));

        // Other paths should match param
        let result = root.match_path("/users/123");
        assert!(result.is_some());
        let (methods, params) = result.unwrap();
        assert_eq!(methods.get_operation(&Method::GET), Some("getUser"));
        assert_eq!(params.get("id"), Some("123"));
    }

    #[test]
    fn test_multiple_params() {
        let mut root = Node::root();
        root.insert(
            "/orgs/{orgId}/users/{userId}",
            MethodRouter::new().get("getOrgUser"),
        );

        let result = root.match_path("/orgs/acme/users/123");
        assert!(result.is_some());

        let (methods, params) = result.unwrap();
        assert_eq!(methods.get_operation(&Method::GET), Some("getOrgUser"));
        assert_eq!(params.get("orgId"), Some("acme"));
        assert_eq!(params.get("userId"), Some("123"));
    }

    #[test]
    fn test_no_match() {
        let mut root = Node::root();
        root.insert("/users", MethodRouter::new().get("listUsers"));

        let result = root.match_path("/posts");
        assert!(result.is_none());
    }

    #[test]
    fn test_nested_routes() {
        let mut root = Node::root();
        root.insert("/api/v1/users", MethodRouter::new().get("listUsers"));
        root.insert(
            "/api/v1/users/{id}",
            MethodRouter::new().get("getUser").delete("deleteUser"),
        );
        root.insert("/api/v1/posts", MethodRouter::new().get("listPosts"));

        let result = root.match_path("/api/v1/users");
        assert!(result.is_some());
        let (methods, _) = result.unwrap();
        assert_eq!(methods.get_operation(&Method::GET), Some("listUsers"));

        let result = root.match_path("/api/v1/users/123");
        assert!(result.is_some());
        let (methods, params) = result.unwrap();
        assert_eq!(methods.get_operation(&Method::GET), Some("getUser"));
        assert_eq!(methods.get_operation(&Method::DELETE), Some("deleteUser"));
        assert_eq!(params.get("id"), Some("123"));

        let result = root.match_path("/api/v1/posts");
        assert!(result.is_some());
        let (methods, _) = result.unwrap();
        assert_eq!(methods.get_operation(&Method::GET), Some("listPosts"));
    }
}
