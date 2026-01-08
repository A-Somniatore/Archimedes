//! HTTP method-based routing.
//!
//! This module provides [`MethodRouter`] which maps HTTP methods to operation IDs
//! for a single path. It supports all standard HTTP methods and provides a
//! fluent builder API.

use http::Method;

/// Maps HTTP methods to operation IDs for a single route.
///
/// # Example
///
/// ```rust
/// use archimedes_router::MethodRouter;
/// use http::Method;
///
/// let router = MethodRouter::new()
///     .get("listUsers")
///     .post("createUser")
///     .options("userOptions");
///
/// assert_eq!(router.get_operation(&Method::GET), Some("listUsers"));
/// assert_eq!(router.get_operation(&Method::POST), Some("createUser"));
/// assert_eq!(router.get_operation(&Method::DELETE), None);
/// ```
#[derive(Debug, Clone, Default)]
pub struct MethodRouter {
    /// GET handler
    get: Option<String>,
    /// POST handler
    post: Option<String>,
    /// PUT handler
    put: Option<String>,
    /// DELETE handler
    delete: Option<String>,
    /// PATCH handler
    patch: Option<String>,
    /// HEAD handler
    head: Option<String>,
    /// OPTIONS handler
    options: Option<String>,
    /// TRACE handler
    trace: Option<String>,
    /// CONNECT handler
    connect: Option<String>,
}

impl MethodRouter {
    /// Creates a new empty method router.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a GET handler.
    #[must_use]
    pub fn get(mut self, operation_id: impl Into<String>) -> Self {
        self.get = Some(operation_id.into());
        self
    }

    /// Registers a POST handler.
    #[must_use]
    pub fn post(mut self, operation_id: impl Into<String>) -> Self {
        self.post = Some(operation_id.into());
        self
    }

    /// Registers a PUT handler.
    #[must_use]
    pub fn put(mut self, operation_id: impl Into<String>) -> Self {
        self.put = Some(operation_id.into());
        self
    }

    /// Registers a DELETE handler.
    #[must_use]
    pub fn delete(mut self, operation_id: impl Into<String>) -> Self {
        self.delete = Some(operation_id.into());
        self
    }

    /// Registers a PATCH handler.
    #[must_use]
    pub fn patch(mut self, operation_id: impl Into<String>) -> Self {
        self.patch = Some(operation_id.into());
        self
    }

    /// Registers a HEAD handler.
    #[must_use]
    pub fn head(mut self, operation_id: impl Into<String>) -> Self {
        self.head = Some(operation_id.into());
        self
    }

    /// Registers an OPTIONS handler.
    #[must_use]
    pub fn options(mut self, operation_id: impl Into<String>) -> Self {
        self.options = Some(operation_id.into());
        self
    }

    /// Registers a TRACE handler.
    #[must_use]
    pub fn trace(mut self, operation_id: impl Into<String>) -> Self {
        self.trace = Some(operation_id.into());
        self
    }

    /// Registers a CONNECT handler.
    #[must_use]
    pub fn connect(mut self, operation_id: impl Into<String>) -> Self {
        self.connect = Some(operation_id.into());
        self
    }

    /// Registers a handler for a specific method.
    #[must_use]
    pub fn method(mut self, method: &Method, operation_id: impl Into<String>) -> Self {
        let op = operation_id.into();
        match *method {
            Method::GET => self.get = Some(op),
            Method::POST => self.post = Some(op),
            Method::PUT => self.put = Some(op),
            Method::DELETE => self.delete = Some(op),
            Method::PATCH => self.patch = Some(op),
            Method::HEAD => self.head = Some(op),
            Method::OPTIONS => self.options = Some(op),
            Method::TRACE => self.trace = Some(op),
            Method::CONNECT => self.connect = Some(op),
            _ => {} // Ignore unknown methods
        }
        self
    }

    /// Returns the operation ID for a given HTTP method.
    #[must_use]
    pub fn get_operation(&self, method: &Method) -> Option<&str> {
        match *method {
            Method::GET => self.get.as_deref(),
            Method::POST => self.post.as_deref(),
            Method::PUT => self.put.as_deref(),
            Method::DELETE => self.delete.as_deref(),
            Method::PATCH => self.patch.as_deref(),
            Method::HEAD => self.head.as_deref(),
            Method::OPTIONS => self.options.as_deref(),
            Method::TRACE => self.trace.as_deref(),
            Method::CONNECT => self.connect.as_deref(),
            _ => None,
        }
    }

    /// Merges another method router into this one.
    ///
    /// Methods from the `other` router will be added to this router.
    /// If a method is already set in this router, it will NOT be overwritten.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_router::MethodRouter;
    /// use http::Method;
    ///
    /// let mut router = MethodRouter::new().get("getUsers");
    /// router.merge(MethodRouter::new().post("createUser"));
    ///
    /// assert_eq!(router.get_operation(&Method::GET), Some("getUsers"));
    /// assert_eq!(router.get_operation(&Method::POST), Some("createUser"));
    /// ```
    pub fn merge(&mut self, other: MethodRouter) {
        if self.get.is_none() {
            self.get = other.get;
        }
        if self.post.is_none() {
            self.post = other.post;
        }
        if self.put.is_none() {
            self.put = other.put;
        }
        if self.delete.is_none() {
            self.delete = other.delete;
        }
        if self.patch.is_none() {
            self.patch = other.patch;
        }
        if self.head.is_none() {
            self.head = other.head;
        }
        if self.options.is_none() {
            self.options = other.options;
        }
        if self.trace.is_none() {
            self.trace = other.trace;
        }
        if self.connect.is_none() {
            self.connect = other.connect;
        }
    }

    /// Returns true if any methods are registered.
    #[must_use]
    pub fn has_any_method(&self) -> bool {
        self.get.is_some()
            || self.post.is_some()
            || self.put.is_some()
            || self.delete.is_some()
            || self.patch.is_some()
            || self.head.is_some()
            || self.options.is_some()
            || self.trace.is_some()
            || self.connect.is_some()
    }

    /// Returns a list of allowed methods for this route.
    #[must_use]
    pub fn allowed_methods(&self) -> Vec<Method> {
        let mut methods = Vec::with_capacity(9);
        if self.get.is_some() {
            methods.push(Method::GET);
        }
        if self.post.is_some() {
            methods.push(Method::POST);
        }
        if self.put.is_some() {
            methods.push(Method::PUT);
        }
        if self.delete.is_some() {
            methods.push(Method::DELETE);
        }
        if self.patch.is_some() {
            methods.push(Method::PATCH);
        }
        if self.head.is_some() {
            methods.push(Method::HEAD);
        }
        if self.options.is_some() {
            methods.push(Method::OPTIONS);
        }
        if self.trace.is_some() {
            methods.push(Method::TRACE);
        }
        if self.connect.is_some() {
            methods.push(Method::CONNECT);
        }
        methods
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_router_new() {
        let router = MethodRouter::new();
        assert!(!router.has_any_method());
    }

    #[test]
    fn test_method_router_get() {
        let router = MethodRouter::new().get("listUsers");
        assert_eq!(router.get_operation(&Method::GET), Some("listUsers"));
        assert_eq!(router.get_operation(&Method::POST), None);
    }

    #[test]
    fn test_method_router_post() {
        let router = MethodRouter::new().post("createUser");
        assert_eq!(router.get_operation(&Method::POST), Some("createUser"));
    }

    #[test]
    fn test_method_router_put() {
        let router = MethodRouter::new().put("updateUser");
        assert_eq!(router.get_operation(&Method::PUT), Some("updateUser"));
    }

    #[test]
    fn test_method_router_delete() {
        let router = MethodRouter::new().delete("deleteUser");
        assert_eq!(router.get_operation(&Method::DELETE), Some("deleteUser"));
    }

    #[test]
    fn test_method_router_patch() {
        let router = MethodRouter::new().patch("patchUser");
        assert_eq!(router.get_operation(&Method::PATCH), Some("patchUser"));
    }

    #[test]
    fn test_method_router_head() {
        let router = MethodRouter::new().head("headUser");
        assert_eq!(router.get_operation(&Method::HEAD), Some("headUser"));
    }

    #[test]
    fn test_method_router_options() {
        let router = MethodRouter::new().options("optionsUser");
        assert_eq!(router.get_operation(&Method::OPTIONS), Some("optionsUser"));
    }

    #[test]
    fn test_method_router_multiple() {
        let router = MethodRouter::new()
            .get("getUser")
            .post("createUser")
            .put("updateUser")
            .delete("deleteUser");

        assert_eq!(router.get_operation(&Method::GET), Some("getUser"));
        assert_eq!(router.get_operation(&Method::POST), Some("createUser"));
        assert_eq!(router.get_operation(&Method::PUT), Some("updateUser"));
        assert_eq!(router.get_operation(&Method::DELETE), Some("deleteUser"));
    }

    #[test]
    fn test_method_router_generic() {
        let router = MethodRouter::new().method(&Method::GET, "getUser");
        assert_eq!(router.get_operation(&Method::GET), Some("getUser"));
    }

    #[test]
    fn test_method_router_has_any_method() {
        let empty = MethodRouter::new();
        assert!(!empty.has_any_method());

        let with_get = MethodRouter::new().get("test");
        assert!(with_get.has_any_method());
    }

    #[test]
    fn test_method_router_allowed_methods() {
        let router = MethodRouter::new().get("get").post("post").delete("delete");

        let allowed = router.allowed_methods();
        assert!(allowed.contains(&Method::GET));
        assert!(allowed.contains(&Method::POST));
        assert!(allowed.contains(&Method::DELETE));
        assert!(!allowed.contains(&Method::PUT));
    }

    #[test]
    fn test_method_router_clone() {
        let router = MethodRouter::new().get("getUser");
        let cloned = router.clone();
        assert_eq!(cloned.get_operation(&Method::GET), Some("getUser"));
    }

    #[test]
    fn test_method_router_merge_adds_methods() {
        let mut router = MethodRouter::new().get("getUsers");
        router.merge(MethodRouter::new().post("createUser"));

        assert_eq!(router.get_operation(&Method::GET), Some("getUsers"));
        assert_eq!(router.get_operation(&Method::POST), Some("createUser"));
    }

    #[test]
    fn test_method_router_merge_does_not_overwrite() {
        // Existing method should not be overwritten
        let mut router = MethodRouter::new().get("originalGet");
        router.merge(MethodRouter::new().get("newGet").post("createUser"));

        // Original GET should be preserved
        assert_eq!(router.get_operation(&Method::GET), Some("originalGet"));
        // New POST should be added
        assert_eq!(router.get_operation(&Method::POST), Some("createUser"));
    }

    #[test]
    fn test_method_router_merge_all_methods() {
        let mut router = MethodRouter::new();
        router.merge(
            MethodRouter::new()
                .get("get")
                .post("post")
                .put("put")
                .delete("delete")
                .patch("patch")
                .head("head")
                .options("options")
                .trace("trace")
                .connect("connect"),
        );

        assert_eq!(router.get_operation(&Method::GET), Some("get"));
        assert_eq!(router.get_operation(&Method::POST), Some("post"));
        assert_eq!(router.get_operation(&Method::PUT), Some("put"));
        assert_eq!(router.get_operation(&Method::DELETE), Some("delete"));
        assert_eq!(router.get_operation(&Method::PATCH), Some("patch"));
        assert_eq!(router.get_operation(&Method::HEAD), Some("head"));
        assert_eq!(router.get_operation(&Method::OPTIONS), Some("options"));
        assert_eq!(router.get_operation(&Method::TRACE), Some("trace"));
        assert_eq!(router.get_operation(&Method::CONNECT), Some("connect"));
    }
}
