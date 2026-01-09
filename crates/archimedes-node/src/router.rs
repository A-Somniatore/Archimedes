//! Sub-router support for TypeScript bindings.
//!
//! Provides a Router class that can compose handlers with shared prefixes and tags,
//! matching the Python Router API.
//!
//! ## Example (TypeScript)
//!
//! ```typescript
//! import { Router, Archimedes, Config, Response } from '@archimedes/node';
//!
//! // Create a router with prefix and tag
//! const usersRouter = new Router()
//!   .prefix('/users')
//!   .tag('users');
//!
//! usersRouter.operation('listUsers', async (ctx) => {
//!   return Response.json({ users: [] });
//! });
//!
//! usersRouter.operation('getUser', async (ctx) => {
//!   return Response.json({ user: { id: ctx.pathParams.userId } });
//! });
//!
//! // Create main app and merge router
//! const app = new Archimedes(config);
//! app.merge(usersRouter);
//! ```

use napi_derive::napi;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A single route definition within a router.
#[derive(Clone, Debug)]
#[allow(dead_code)] // Fields used for future route registration
pub struct RouteDefinition {
    /// Operation ID from the contract
    pub operation_id: String,
    /// HTTP status code for default response
    pub status_code: u16,
    /// JSON body for default response
    pub json_body: String,
}

/// Router for composing operation handlers with shared configuration.
///
/// Routers can have:
/// - A path prefix applied to all operations
/// - Tags for grouping operations
/// - Nested child routers
///
/// ## Example
///
/// ```typescript
/// const router = new Router()
///   .prefix('/api/v1')
///   .tag('v1');
///
/// router.operation('listItems', async (ctx) => {
///   return Response.json({ items: [] });
/// });
/// ```
#[napi]
#[derive(Clone)]
pub struct Router {
    prefix_path: Arc<RwLock<Option<String>>>,
    tags: Arc<RwLock<Vec<String>>>,
    routes: Arc<RwLock<Vec<RouteDefinition>>>,
    nested_routers: Arc<RwLock<Vec<Router>>>,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[napi]
impl Router {
    /// Create a new empty router.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            prefix_path: Arc::new(RwLock::new(None)),
            tags: Arc::new(RwLock::new(Vec::new())),
            routes: Arc::new(RwLock::new(Vec::new())),
            nested_routers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set a path prefix for all operations in this router.
    ///
    /// The prefix will be prepended to operation paths when the router is merged
    /// into the main application.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// const router = new Router().prefix('/users');
    /// ```
    #[napi]
    pub fn prefix(&self, path: String) -> Self {
        // Normalize the path
        let normalized = normalize_path(&path);

        if let Ok(mut prefix) = self.prefix_path.try_write() {
            *prefix = Some(normalized);
        }

        self.clone()
    }

    /// Add a tag to this router for grouping operations.
    ///
    /// Tags are used for OpenAPI documentation grouping.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// const router = new Router()
    ///   .prefix('/users')
    ///   .tag('users');
    /// ```
    #[napi]
    pub fn tag(&self, tag: String) -> Self {
        if let Ok(mut tags) = self.tags.try_write() {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        self.clone()
    }

    /// Nest another router under this router's prefix.
    ///
    /// The nested router's prefix will be appended to this router's prefix.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// const adminRouter = new Router().prefix('/admin');
    /// const usersRouter = new Router().prefix('/users');
    ///
    /// adminRouter.nest(usersRouter);
    /// // Results in /admin/users paths
    /// ```
    #[napi]
    pub fn nest(&self, router: &Router) -> Self {
        if let Ok(mut nested) = self.nested_routers.try_write() {
            nested.push(router.clone());
        }
        self.clone()
    }

    /// Merge another router's handlers into this router.
    ///
    /// Unlike nest(), merge() copies the handlers directly without
    /// combining prefixes.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// const mainRouter = new Router();
    /// const userRouter = new Router().prefix('/users');
    ///
    /// mainRouter.merge(userRouter);
    /// ```
    #[napi]
    pub fn merge(&self, router: &Router) -> Self {
        // Use tokio runtime for async operations
        let routes = router.routes.clone();
        let nested = router.nested_routers.clone();
        let self_routes = self.routes.clone();
        let self_nested = self.nested_routers.clone();

        if let (Ok(other_routes), Ok(mut my_routes)) =
            (routes.try_read(), self_routes.try_write())
        {
            my_routes.extend(other_routes.clone());
        }

        if let (Ok(other_nested), Ok(mut my_nested)) =
            (nested.try_read(), self_nested.try_write())
        {
            my_nested.extend(other_nested.clone());
        }

        self.clone()
    }

    /// Register an operation handler with a JSON response.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// router.operation('listUsers', 200, JSON.stringify({ users: [] }));
    /// ```
    #[napi]
    pub fn operation(&self, operation_id: String, status_code: u16, json_body: String) -> Self {
        if let Ok(mut routes) = self.routes.try_write() {
            routes.push(RouteDefinition {
                operation_id,
                status_code,
                json_body,
            });
        }
        self.clone()
    }

    /// Register an operation handler that returns 200 OK with JSON.
    ///
    /// ## Example
    ///
    /// ```typescript
    /// router.operationOk('listUsers', JSON.stringify({ users: [] }));
    /// ```
    #[napi]
    pub fn operation_ok(&self, operation_id: String, json_body: String) -> Self {
        self.operation(operation_id, 200, json_body)
    }

    /// Get the current prefix.
    #[napi(getter)]
    pub async fn current_prefix(&self) -> Option<String> {
        self.prefix_path.read().await.clone()
    }

    /// Get all tags.
    #[napi(getter)]
    pub async fn current_tags(&self) -> Vec<String> {
        self.tags.read().await.clone()
    }

    /// Get the number of routes in this router.
    #[napi]
    pub async fn route_count(&self) -> u32 {
        self.routes.read().await.len() as u32
    }

    /// Get the number of nested routers.
    #[napi]
    pub async fn nested_count(&self) -> u32 {
        self.nested_routers.read().await.len() as u32
    }

    /// Get all routes including from nested routers.
    ///
    /// Returns route definitions with effective prefixes applied.
    pub async fn all_routes(&self) -> Vec<RouteInfo> {
        let mut result = Vec::new();
        let prefix = self.prefix_path.read().await.clone();
        let tags = self.tags.read().await.clone();

        // Add direct routes
        for route in self.routes.read().await.iter() {
            result.push(RouteInfo {
                operation_id: route.operation_id.clone(),
                prefix: prefix.clone(),
                tags: tags.clone(),
            });
        }

        // Add nested router routes
        for nested in self.nested_routers.read().await.iter() {
            let nested_routes = Box::pin(nested.all_routes()).await;
            for mut route in nested_routes {
                // Combine prefixes
                route.prefix = combine_prefixes(&prefix, &route.prefix);
                // Combine tags
                for tag in &tags {
                    if !route.tags.contains(tag) {
                        route.tags.insert(0, tag.clone());
                    }
                }
                result.push(route);
            }
        }

        result
    }
}

/// Information about a route with effective configuration.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct RouteInfo {
    /// Operation ID
    pub operation_id: String,
    /// Effective path prefix
    pub prefix: Option<String>,
    /// Tags for this route
    pub tags: Vec<String>,
}

/// Create route info (for NAPI export).
#[napi]
pub fn create_route_info(
    operation_id: String,
    prefix: Option<String>,
    tags: Vec<String>,
) -> RouteInfo {
    RouteInfo {
        operation_id,
        prefix,
        tags,
    }
}

/// Normalize a path by ensuring it starts with / and doesn't end with /.
fn normalize_path(path: &str) -> String {
    let mut result = path.trim().to_string();

    // Ensure starts with /
    if !result.starts_with('/') {
        result = format!("/{}", result);
    }

    // Remove trailing slash (unless it's just "/")
    if result.len() > 1 && result.ends_with('/') {
        result.pop();
    }

    result
}

/// Combine two path prefixes.
fn combine_prefixes(parent: &Option<String>, child: &Option<String>) -> Option<String> {
    match (parent, child) {
        (Some(p), Some(c)) => Some(normalize_path(&format!("{}{}", p, c))),
        (Some(p), None) => Some(p.clone()),
        (None, Some(c)) => Some(c.clone()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_adds_leading_slash() {
        assert_eq!(normalize_path("users"), "/users");
    }

    #[test]
    fn test_normalize_path_removes_trailing_slash() {
        assert_eq!(normalize_path("/users/"), "/users");
    }

    #[test]
    fn test_normalize_path_keeps_root() {
        assert_eq!(normalize_path("/"), "/");
    }

    #[test]
    fn test_normalize_path_already_normalized() {
        assert_eq!(normalize_path("/users"), "/users");
    }

    #[test]
    fn test_combine_prefixes() {
        assert_eq!(
            combine_prefixes(&Some("/api".to_string()), &Some("/users".to_string())),
            Some("/api/users".to_string())
        );
        assert_eq!(
            combine_prefixes(&Some("/api".to_string()), &None),
            Some("/api".to_string())
        );
        assert_eq!(
            combine_prefixes(&None, &Some("/users".to_string())),
            Some("/users".to_string())
        );
        assert_eq!(combine_prefixes(&None, &None), None);
    }

    #[tokio::test]
    async fn test_router_creation() {
        let router = Router::new();
        assert!(router.current_prefix().await.is_none());
        assert!(router.current_tags().await.is_empty());
    }

    #[tokio::test]
    async fn test_router_prefix() {
        let router = Router::new().prefix("/users".to_string());
        assert_eq!(router.current_prefix().await, Some("/users".to_string()));
    }

    #[tokio::test]
    async fn test_router_tag() {
        let router = Router::new().tag("users".to_string());
        let tags = router.current_tags().await;
        assert!(tags.contains(&"users".to_string()));
    }

    #[tokio::test]
    async fn test_router_chaining() {
        let router = Router::new()
            .prefix("/api".to_string())
            .tag("api".to_string())
            .tag("v1".to_string());

        assert_eq!(router.current_prefix().await, Some("/api".to_string()));
        let tags = router.current_tags().await;
        assert!(tags.contains(&"api".to_string()));
        assert!(tags.contains(&"v1".to_string()));
    }

    #[tokio::test]
    async fn test_router_operation() {
        let router = Router::new().operation(
            "listUsers".to_string(),
            200,
            r#"{"users":[]}"#.to_string(),
        );
        assert_eq!(router.route_count().await, 1);
    }

    #[tokio::test]
    async fn test_router_operation_ok() {
        let router = Router::new().operation_ok("listUsers".to_string(), r#"{}"#.to_string());
        assert_eq!(router.route_count().await, 1);
    }

    #[tokio::test]
    async fn test_router_nest() {
        let child = Router::new().prefix("/users".to_string());
        let parent = Router::new().prefix("/api".to_string()).nest(&child);

        assert_eq!(parent.nested_count().await, 1);
    }

    #[tokio::test]
    async fn test_router_merge() {
        let router1 =
            Router::new().operation_ok("op1".to_string(), "{}".to_string());
        let router2 = Router::new()
            .operation_ok("op2".to_string(), "{}".to_string())
            .operation_ok("op3".to_string(), "{}".to_string());

        let merged = router1.merge(&router2);

        assert_eq!(merged.route_count().await, 3);
    }

    #[tokio::test]
    async fn test_all_routes() {
        let child = Router::new()
            .prefix("/users".to_string())
            .tag("users".to_string())
            .operation_ok("listUsers".to_string(), "{}".to_string());

        let parent = Router::new()
            .prefix("/api".to_string())
            .tag("api".to_string())
            .nest(&child);

        let routes = parent.all_routes().await;
        assert_eq!(routes.len(), 1);

        let route = &routes[0];
        assert_eq!(route.operation_id, "listUsers");
        assert_eq!(route.prefix, Some("/api/users".to_string()));
        assert!(route.tags.contains(&"api".to_string()));
        assert!(route.tags.contains(&"users".to_string()));
    }

    #[tokio::test]
    async fn test_duplicate_tags_not_added() {
        let router = Router::new()
            .tag("api".to_string())
            .tag("api".to_string())
            .tag("api".to_string());

        let tags = router.current_tags().await;
        assert_eq!(tags.len(), 1);
    }
}
