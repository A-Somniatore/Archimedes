//! Router implementation for Python bindings
//!
//! Provides sub-router composition similar to Rust's `Router`:
//!
//! ```python,ignore
//! from archimedes import Router
//!
//! # Create sub-routers
//! users = Router().prefix("/users").tag("users")
//!
//! @users.handler("listUsers")
//! def list_users(ctx):
//!     return {"users": []}
//!
//! @users.handler("getUser")
//! def get_user(ctx):
//!     return {"id": ctx.path_params.get("userId")}
//!
//! # Create admin router
//! admin = Router().prefix("/admin").tag("admin")
//!
//! @admin.handler("adminStats")
//! def admin_stats(ctx):
//!     return {"stats": {}}
//!
//! # Compose in main app
//! app = App(config)
//! app.nest("/api/v1", users)
//! app.nest("/api/v1", admin)
//! # OR using merge
//! app.merge(users)
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use pyo3::prelude::*;

use crate::handlers::HandlerRegistry;

/// Route definition with handler and metadata
#[derive(Clone)]
pub struct RouteDefinition {
    /// The operation ID for this route
    pub operation_id: String,
    /// Path prefix for the route
    pub path_prefix: Option<String>,
    /// Tags for OpenAPI documentation
    pub tags: Vec<String>,
}

/// Python Router for grouping related routes
///
/// Provides methods similar to Rust's `archimedes_router::Router`:
/// - `prefix()` - Set a path prefix for all routes
/// - `tag()` - Add OpenAPI tags for documentation
/// - `nest()` - Nest another router at a path prefix
/// - `merge()` - Merge routes from another router
#[pyclass(name = "Router")]
#[derive(Clone)]
pub struct PyRouter {
    /// Path prefix for all routes in this router
    prefix: Option<String>,
    /// OpenAPI tags for all routes in this router
    tags: Vec<String>,
    /// Handler registry for this router's operations
    handlers: Arc<HandlerRegistry>,
    /// Route definitions (operation_id -> definition)
    routes: HashMap<String, RouteDefinition>,
}

#[pymethods]
impl PyRouter {
    /// Create a new empty router
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// from archimedes import Router
    ///
    /// router = Router()
    /// ```
    #[new]
    pub fn new() -> Self {
        Self {
            prefix: None,
            tags: Vec::new(),
            handlers: Arc::new(HandlerRegistry::new()),
            routes: HashMap::new(),
        }
    }

    /// Set a path prefix for all routes in this router
    ///
    /// This is a builder method that returns a new router with the prefix set.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The path prefix (e.g., "/api/v1" or "/users")
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// users_router = Router().prefix("/users")
    ///
    /// @users_router.handler("listUsers")
    /// def list_users(ctx):
    ///     # This handler will be mounted at /users
    ///     return {"users": []}
    /// ```
    fn prefix(&self, prefix: String) -> Self {
        let mut new_router = self.clone();
        new_router.prefix = Some(normalize_path(&prefix));
        new_router
    }

    /// Add an OpenAPI tag to all routes in this router
    ///
    /// Tags are used for grouping routes in API documentation.
    /// Multiple tags can be added by chaining `tag()` calls.
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag name for documentation
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// users_router = Router().tag("users").tag("public-api")
    /// ```
    fn tag(&self, tag: String) -> Self {
        let mut new_router = self.clone();
        new_router.tags.push(tag);
        new_router
    }

    /// Register a handler for an operation
    ///
    /// This method is typically used via the `@router.handler` decorator.
    ///
    /// # Arguments
    ///
    /// * `operation_id` - The operation ID from the contract
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// router = Router().prefix("/users").tag("users")
    ///
    /// @router.handler("listUsers")
    /// def list_users(ctx):
    ///     return {"users": []}
    /// ```
    fn handler(&self, operation_id: String) -> PyResult<RouterHandlerDecorator> {
        Ok(RouterHandlerDecorator {
            operation_id,
            registry: Arc::clone(&self.handlers),
            prefix: self.prefix.clone(),
            tags: self.tags.clone(),
        })
    }

    /// Nest another router at a path prefix
    ///
    /// All routes from the nested router will be available under the given prefix.
    /// The nested router's own prefix (if any) is combined with the nest prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The path prefix to nest under
    /// * `router` - The router to nest
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// users = Router()
    ///
    /// @users.handler("listUsers")
    /// def list_users(ctx):
    ///     return {"users": []}
    ///
    /// api = Router()
    /// api.nest("/api/v1/users", users)
    ///
    /// # listUsers is now at /api/v1/users
    /// ```
    fn nest(&mut self, prefix: String, router: &PyRouter) -> PyResult<()> {
        let normalized_prefix = normalize_path(&prefix);

        // Copy handlers from the nested router (already cloned by iter())
        for (op_id, handler) in router.handlers.iter() {
            self.handlers.register(op_id, handler)?;
        }

        // Copy route definitions with updated prefix
        for (op_id, def) in &router.routes {
            let new_prefix = match (&normalized_prefix, &def.path_prefix) {
                (p, Some(inner)) => Some(format!("{}{}", p, inner)),
                (p, None) => Some(p.clone()),
            };

            self.routes.insert(
                op_id.clone(),
                RouteDefinition {
                    operation_id: op_id.clone(),
                    path_prefix: new_prefix,
                    tags: def.tags.clone(),
                },
            );
        }

        Ok(())
    }

    /// Merge all routes from another router into this one
    ///
    /// Unlike `nest()`, this doesn't add a prefix - routes are added as-is.
    ///
    /// # Arguments
    ///
    /// * `router` - The router to merge
    ///
    /// # Example (Python)
    ///
    /// ```python,ignore
    /// users = Router()
    ///
    /// @users.handler("listUsers")
    /// def list_users(ctx):
    ///     return {"users": []}
    ///
    /// app = Router()
    /// app.merge(users)
    /// ```
    fn merge(&mut self, router: &PyRouter) -> PyResult<()> {
        // Copy handlers (already cloned by iter())
        for (op_id, handler) in router.handlers.iter() {
            self.handlers.register(op_id, handler)?;
        }

        // Copy route definitions as-is
        for (op_id, def) in &router.routes {
            self.routes.insert(op_id.clone(), def.clone());
        }

        Ok(())
    }

    /// Get the path prefix for this router
    #[getter]
    fn get_prefix(&self) -> Option<String> {
        self.prefix.clone()
    }

    /// Get the tags for this router
    #[getter]
    fn get_tags(&self) -> Vec<String> {
        self.tags.clone()
    }

    /// Get the list of registered operation IDs
    fn operation_ids(&self) -> Vec<String> {
        self.handlers.operation_ids()
    }

    /// Check if a handler is registered for an operation
    fn has_handler(&self, operation_id: &str) -> bool {
        self.handlers.has(operation_id)
    }

    /// Get the number of registered routes
    fn __len__(&self) -> usize {
        self.handlers.len()
    }

    /// String representation
    fn __repr__(&self) -> String {
        let prefix = self.prefix.as_deref().unwrap_or("");
        let tag_str = if self.tags.is_empty() {
            String::new()
        } else {
            format!(", tags={:?}", self.tags)
        };
        format!(
            "Router(prefix=\"{}\", routes={}{})",
            prefix,
            self.handlers.len(),
            tag_str
        )
    }
}

impl PyRouter {
    /// Get the handler registry (for internal use)
    pub fn handlers(&self) -> &Arc<HandlerRegistry> {
        &self.handlers
    }

    /// Get route definitions (for internal use)
    pub fn routes(&self) -> &HashMap<String, RouteDefinition> {
        &self.routes
    }
}

impl Default for PyRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Decorator helper for registering handlers in a router
#[pyclass]
pub struct RouterHandlerDecorator {
    operation_id: String,
    registry: Arc<HandlerRegistry>,
    prefix: Option<String>,
    tags: Vec<String>,
}

#[pymethods]
impl RouterHandlerDecorator {
    fn __call__(&self, py: Python<'_>, handler: PyObject) -> PyResult<PyObject> {
        let handler_clone = handler.clone_ref(py);
        self.registry
            .register(self.operation_id.clone(), handler_clone)?;
        Ok(handler)
    }
}

/// Normalize a path to ensure consistent formatting
fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return String::new();
    }

    // Ensure leading slash
    let with_leading = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{}", trimmed)
    };

    // Remove trailing slash
    if with_leading.ends_with('/') && with_leading.len() > 1 {
        with_leading[..with_leading.len() - 1].to_string()
    } else {
        with_leading
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_empty() {
        assert_eq!(normalize_path(""), "");
        assert_eq!(normalize_path("/"), "");
    }

    #[test]
    fn test_normalize_path_leading_slash() {
        assert_eq!(normalize_path("users"), "/users");
        assert_eq!(normalize_path("/users"), "/users");
    }

    #[test]
    fn test_normalize_path_trailing_slash() {
        assert_eq!(normalize_path("/users/"), "/users");
        assert_eq!(normalize_path("users/"), "/users");
    }

    #[test]
    fn test_normalize_path_nested() {
        assert_eq!(normalize_path("/api/v1/users"), "/api/v1/users");
        assert_eq!(normalize_path("api/v1/users/"), "/api/v1/users");
    }

    #[test]
    fn test_router_new() {
        let router = PyRouter::new();
        assert!(router.prefix.is_none());
        assert!(router.tags.is_empty());
        assert_eq!(router.handlers.len(), 0);
    }

    #[test]
    fn test_router_prefix() {
        let router = PyRouter::new();
        let prefixed = router.prefix("/api/v1".to_string());
        assert_eq!(prefixed.prefix, Some("/api/v1".to_string()));
    }

    #[test]
    fn test_router_tag() {
        let router = PyRouter::new();
        let tagged = router.tag("users".to_string());
        assert_eq!(tagged.tags, vec!["users"]);
    }

    #[test]
    fn test_router_chained_builder() {
        let router = PyRouter::new()
            .prefix("/api/v1".to_string())
            .tag("users".to_string())
            .tag("public".to_string());

        assert_eq!(router.prefix, Some("/api/v1".to_string()));
        assert_eq!(router.tags, vec!["users", "public"]);
    }
}
