//! Router FFI for C/C++ bindings.
//!
//! Provides a C-compatible Router API for organizing handlers with shared
//! prefixes and tags.
//!
//! ## Example (C)
//!
//! ```c
//! #include <archimedes.h>
//!
//! // Create a router with prefix and tag
//! archimedes_router* users_router = archimedes_router_new();
//! archimedes_router_prefix(users_router, "/users");
//! archimedes_router_tag(users_router, "users");
//!
//! // Register a handler on the router
//! archimedes_router_register(users_router, "listUsers", list_users_handler, NULL);
//!
//! // Merge router into main app
//! archimedes_merge(app, users_router);
//! archimedes_router_free(users_router);
//! ```

use std::ffi::{c_char, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Opaque router handle for FFI
#[repr(C)]
pub struct ArchimedesRouter {
    _opaque: [u8; 0],
}

/// Internal router state
pub(crate) struct RouterState {
    /// Path prefix for all routes
    prefix: Option<String>,
    /// Tags for grouping
    tags: Vec<String>,
    /// Registered operations
    operations: Vec<RouteEntry>,
    /// Nested routers
    nested: Vec<Box<RouterState>>,
}

/// A single route entry
#[derive(Clone)]
pub(crate) struct RouteEntry {
    /// Operation ID
    pub operation_id: String,
    /// User-provided data pointer
    pub user_data: *mut std::ffi::c_void,
}

impl RouterState {
    fn new() -> Self {
        Self {
            prefix: None,
            tags: Vec::new(),
            operations: Vec::new(),
            nested: Vec::new(),
        }
    }
}

/// Create a new router
///
/// # Safety
///
/// Returns a pointer to a router handle that must be freed with `archimedes_router_free`.
#[no_mangle]
pub extern "C" fn archimedes_router_new() -> *mut ArchimedesRouter {
    let state = Box::new(RouterState::new());
    Box::into_raw(state) as *mut ArchimedesRouter
}

/// Free a router
///
/// # Safety
///
/// - `router` must be a valid pointer returned by `archimedes_router_new`
/// - After calling this, `router` is no longer valid
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_free(router: *mut ArchimedesRouter) {
    if router.is_null() {
        return;
    }
    let _ = Box::from_raw(router as *mut RouterState);
}

/// Set a path prefix for the router
///
/// # Safety
///
/// - `router` must be a valid router pointer
/// - `prefix` must be a valid null-terminated UTF-8 string
///
/// Returns 0 on success, 1 on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_prefix(
    router: *mut ArchimedesRouter,
    prefix: *const c_char,
) -> i32 {
    if router.is_null() {
        crate::set_last_error("router pointer is null");
        return 1;
    }
    if prefix.is_null() {
        crate::set_last_error("prefix pointer is null");
        return 1;
    }

    let state = &mut *(router as *mut RouterState);

    let prefix_str = match CStr::from_ptr(prefix).to_str() {
        Ok(s) => normalize_path(s),
        Err(e) => {
            crate::set_last_error(format!("Invalid UTF-8 in prefix: {}", e));
            return 1;
        }
    };

    state.prefix = Some(prefix_str);
    0
}

/// Add a tag to the router
///
/// # Safety
///
/// - `router` must be a valid router pointer
/// - `tag` must be a valid null-terminated UTF-8 string
///
/// Returns 0 on success, 1 on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_tag(
    router: *mut ArchimedesRouter,
    tag: *const c_char,
) -> i32 {
    if router.is_null() {
        crate::set_last_error("router pointer is null");
        return 1;
    }
    if tag.is_null() {
        crate::set_last_error("tag pointer is null");
        return 1;
    }

    let state = &mut *(router as *mut RouterState);

    let tag_str = match CStr::from_ptr(tag).to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            crate::set_last_error(format!("Invalid UTF-8 in tag: {}", e));
            return 1;
        }
    };

    if !state.tags.contains(&tag_str) {
        state.tags.push(tag_str);
    }
    0
}

/// Register an operation on the router
///
/// # Safety
///
/// - `router` must be a valid router pointer
/// - `operation_id` must be a valid null-terminated UTF-8 string
/// - `user_data` is optional and passed to handlers
///
/// Returns 0 on success, 1 on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_register(
    router: *mut ArchimedesRouter,
    operation_id: *const c_char,
    user_data: *mut std::ffi::c_void,
) -> i32 {
    if router.is_null() {
        crate::set_last_error("router pointer is null");
        return 1;
    }
    if operation_id.is_null() {
        crate::set_last_error("operation_id pointer is null");
        return 1;
    }

    let state = &mut *(router as *mut RouterState);

    let op_id = match CStr::from_ptr(operation_id).to_str() {
        Ok(s) => s.to_string(),
        Err(e) => {
            crate::set_last_error(format!("Invalid UTF-8 in operation_id: {}", e));
            return 1;
        }
    };

    state.operations.push(RouteEntry {
        operation_id: op_id,
        user_data,
    });
    0
}

/// Nest another router under this router's prefix
///
/// # Safety
///
/// - `parent` and `child` must be valid router pointers
/// - Ownership of `child` is transferred to `parent`
///
/// Returns 0 on success, 1 on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_nest(
    parent: *mut ArchimedesRouter,
    child: *mut ArchimedesRouter,
) -> i32 {
    if parent.is_null() {
        crate::set_last_error("parent router pointer is null");
        return 1;
    }
    if child.is_null() {
        crate::set_last_error("child router pointer is null");
        return 1;
    }

    let parent_state = &mut *(parent as *mut RouterState);
    let child_state = Box::from_raw(child as *mut RouterState);

    parent_state.nested.push(child_state);
    0
}

/// Merge another router's routes into this router
///
/// # Safety
///
/// - `target` and `source` must be valid router pointers
/// - Routes are copied from source, source remains valid
///
/// Returns 0 on success, 1 on error.
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_merge(
    target: *mut ArchimedesRouter,
    source: *const ArchimedesRouter,
) -> i32 {
    if target.is_null() {
        crate::set_last_error("target router pointer is null");
        return 1;
    }
    if source.is_null() {
        crate::set_last_error("source router pointer is null");
        return 1;
    }

    let target_state = &mut *(target as *mut RouterState);
    let source_state = &*(source as *const RouterState);

    // Copy operations from source
    for op in &source_state.operations {
        target_state.operations.push(op.clone());
    }

    0
}

/// Get the current prefix of a router
///
/// # Safety
///
/// - `router` must be a valid router pointer
///
/// Returns a null-terminated string, or NULL if no prefix set.
/// The returned string must be freed with `archimedes_free_string`.
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_get_prefix(
    router: *const ArchimedesRouter,
) -> *mut c_char {
    if router.is_null() {
        return ptr::null_mut();
    }

    let state = &*(router as *const RouterState);

    match &state.prefix {
        Some(prefix) => match CString::new(prefix.as_str()) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        None => ptr::null_mut(),
    }
}

/// Get the number of tags on a router
///
/// # Safety
///
/// - `router` must be a valid router pointer
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_tag_count(router: *const ArchimedesRouter) -> usize {
    if router.is_null() {
        return 0;
    }

    let state = &*(router as *const RouterState);
    state.tags.len()
}

/// Get the number of operations on a router
///
/// # Safety
///
/// - `router` must be a valid router pointer
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_operation_count(
    router: *const ArchimedesRouter,
) -> usize {
    if router.is_null() {
        return 0;
    }

    let state = &*(router as *const RouterState);
    state.operations.len()
}

/// Get the number of nested routers
///
/// # Safety
///
/// - `router` must be a valid router pointer
#[no_mangle]
pub unsafe extern "C" fn archimedes_router_nested_count(router: *const ArchimedesRouter) -> usize {
    if router.is_null() {
        return 0;
    }

    let state = &*(router as *const RouterState);
    state.nested.len()
}

/// Normalize a path
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

/// Global router counter for testing
static ROUTER_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Get global router count (for testing)
#[no_mangle]
pub extern "C" fn archimedes_router_count() -> usize {
    ROUTER_COUNT.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("users"), "/users");
        assert_eq!(normalize_path("/users/"), "/users");
        assert_eq!(normalize_path("/"), "/");
        assert_eq!(normalize_path("/api/v1"), "/api/v1");
    }

    #[test]
    fn test_router_creation() {
        unsafe {
            let router = archimedes_router_new();
            assert!(!router.is_null());
            assert_eq!(archimedes_router_tag_count(router), 0);
            assert_eq!(archimedes_router_operation_count(router), 0);
            archimedes_router_free(router);
        }
    }

    #[test]
    fn test_router_prefix() {
        unsafe {
            let router = archimedes_router_new();

            let prefix = std::ffi::CString::new("/users").unwrap();
            let result = archimedes_router_prefix(router, prefix.as_ptr());
            assert_eq!(result, 0);

            let got_prefix = archimedes_router_get_prefix(router);
            assert!(!got_prefix.is_null());
            let prefix_str = CStr::from_ptr(got_prefix).to_str().unwrap();
            assert_eq!(prefix_str, "/users");

            // Free the returned string
            drop(CString::from_raw(got_prefix));
            archimedes_router_free(router);
        }
    }

    #[test]
    fn test_router_tag() {
        unsafe {
            let router = archimedes_router_new();

            let tag = std::ffi::CString::new("users").unwrap();
            let result = archimedes_router_tag(router, tag.as_ptr());
            assert_eq!(result, 0);
            assert_eq!(archimedes_router_tag_count(router), 1);

            // Adding same tag again shouldn't duplicate
            let result = archimedes_router_tag(router, tag.as_ptr());
            assert_eq!(result, 0);
            assert_eq!(archimedes_router_tag_count(router), 1);

            archimedes_router_free(router);
        }
    }

    #[test]
    fn test_router_register() {
        unsafe {
            let router = archimedes_router_new();

            let op_id = std::ffi::CString::new("listUsers").unwrap();
            let result = archimedes_router_register(router, op_id.as_ptr(), ptr::null_mut());
            assert_eq!(result, 0);
            assert_eq!(archimedes_router_operation_count(router), 1);

            archimedes_router_free(router);
        }
    }

    #[test]
    fn test_router_nest() {
        unsafe {
            let parent = archimedes_router_new();
            let child = archimedes_router_new();

            let prefix = std::ffi::CString::new("/users").unwrap();
            archimedes_router_prefix(child, prefix.as_ptr());

            let result = archimedes_router_nest(parent, child);
            assert_eq!(result, 0);
            assert_eq!(archimedes_router_nested_count(parent), 1);
            // child is now owned by parent, don't free it

            archimedes_router_free(parent);
        }
    }

    #[test]
    fn test_router_merge() {
        unsafe {
            let router1 = archimedes_router_new();
            let router2 = archimedes_router_new();

            let op1 = std::ffi::CString::new("op1").unwrap();
            let op2 = std::ffi::CString::new("op2").unwrap();
            archimedes_router_register(router2, op1.as_ptr(), ptr::null_mut());
            archimedes_router_register(router2, op2.as_ptr(), ptr::null_mut());

            assert_eq!(archimedes_router_operation_count(router1), 0);
            assert_eq!(archimedes_router_operation_count(router2), 2);

            let result = archimedes_router_merge(router1, router2);
            assert_eq!(result, 0);
            assert_eq!(archimedes_router_operation_count(router1), 2);

            archimedes_router_free(router1);
            archimedes_router_free(router2);
        }
    }

    #[test]
    fn test_null_safety() {
        unsafe {
            // All these should return error codes, not crash
            assert_eq!(
                archimedes_router_prefix(ptr::null_mut(), ptr::null()),
                1
            );
            assert_eq!(archimedes_router_tag(ptr::null_mut(), ptr::null()), 1);
            assert_eq!(
                archimedes_router_register(ptr::null_mut(), ptr::null(), ptr::null_mut()),
                1
            );
            assert_eq!(archimedes_router_nest(ptr::null_mut(), ptr::null_mut()), 1);
            assert_eq!(archimedes_router_merge(ptr::null_mut(), ptr::null()), 1);

            assert_eq!(archimedes_router_tag_count(ptr::null()), 0);
            assert_eq!(archimedes_router_operation_count(ptr::null()), 0);
            assert_eq!(archimedes_router_nested_count(ptr::null()), 0);
            assert!(archimedes_router_get_prefix(ptr::null()).is_null());
        }
    }
}
