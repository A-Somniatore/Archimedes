//! Handler registration and invocation
//!
//! This module manages the mapping between operation IDs and FFI handler callbacks.

use crate::types::{ArchimedesHandlerFn, ArchimedesRequestContext, ArchimedesResponseData};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;

/// Registered handler with its callback and user data
#[derive(Clone)]
pub(crate) struct RegisteredHandler {
    /// The handler callback function
    pub callback: ArchimedesHandlerFn,
    /// User-provided data passed to handler
    pub user_data: *mut c_void,
}

// SAFETY: We require users to ensure user_data is thread-safe
unsafe impl Send for RegisteredHandler {}
unsafe impl Sync for RegisteredHandler {}

/// Handler registry managing all registered handlers
#[derive(Default)]
pub(crate) struct HandlerRegistry {
    handlers: RwLock<HashMap<String, RegisteredHandler>>,
}

impl HandlerRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a handler for an operation
    ///
    /// Returns an error if the operation already has a handler registered.
    pub fn register(
        &self,
        operation_id: &str,
        callback: ArchimedesHandlerFn,
        user_data: *mut c_void,
    ) -> Result<(), String> {
        let mut handlers = self.handlers.write();
        
        if handlers.contains_key(operation_id) {
            return Err(format!(
                "Handler already registered for operation '{operation_id}'"
            ));
        }

        handlers.insert(
            operation_id.to_string(),
            RegisteredHandler { callback, user_data },
        );

        tracing::debug!(operation_id, "Registered handler");
        Ok(())
    }

    /// Get a handler for an operation
    pub fn get(&self, operation_id: &str) -> Option<RegisteredHandler> {
        self.handlers.read().get(operation_id).cloned()
    }

    /// Check if an operation has a registered handler
    pub fn has_handler(&self, operation_id: &str) -> bool {
        self.handlers.read().contains_key(operation_id)
    }

    /// Get all registered operation IDs
    pub fn operation_ids(&self) -> Vec<String> {
        self.handlers.read().keys().cloned().collect()
    }

    /// Get the number of registered handlers
    pub fn len(&self) -> usize {
        self.handlers.read().len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.handlers.read().is_empty()
    }
}

/// Shared handler registry wrapped in Arc for thread-safe access
pub type SharedHandlerRegistry = Arc<HandlerRegistry>;

/// Invoke a registered handler
///
/// This function is called from the async runtime to execute a handler callback.
/// It constructs the request context and calls the foreign handler function.
///
/// # Safety
///
/// The caller must ensure:
/// - `ctx` points to valid memory for the duration of the call
/// - `body` is valid for `body_len` bytes (or null if body_len is 0)
/// - The handler callback is still valid
pub(crate) fn invoke_handler(
    handler: &RegisteredHandler,
    ctx: &ArchimedesRequestContext,
    body: &[u8],
) -> ArchimedesResponseData {
    let body_ptr = if body.is_empty() {
        std::ptr::null()
    } else {
        body.as_ptr()
    };

    // Call the foreign handler
    (handler.callback)(ctx, body_ptr, body.len(), handler.user_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::raw::c_char;

    // Test handler that returns a simple response
    extern "C" fn test_handler(
        _ctx: *const ArchimedesRequestContext,
        _body: *const u8,
        _body_len: usize,
        user_data: *mut c_void,
    ) -> ArchimedesResponseData {
        let counter = unsafe { &mut *(user_data as *mut i32) };
        *counter += 1;

        ArchimedesResponseData {
            status_code: 200,
            body: b"{\"ok\": true}\0".as_ptr().cast(),
            body_len: 12,
            body_owned: false,
            ..Default::default()
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = HandlerRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_register() {
        let registry = HandlerRegistry::new();
        let result = registry.register("listUsers", test_handler, std::ptr::null_mut());
        assert!(result.is_ok());
        assert!(registry.has_handler("listUsers"));
        assert!(!registry.has_handler("getUser"));
    }

    #[test]
    fn test_registry_duplicate() {
        let registry = HandlerRegistry::new();
        registry
            .register("op", test_handler, std::ptr::null_mut())
            .unwrap();

        let result = registry.register("op", test_handler, std::ptr::null_mut());
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_get() {
        let registry = HandlerRegistry::new();
        registry
            .register("op", test_handler, std::ptr::null_mut())
            .unwrap();

        let handler = registry.get("op");
        assert!(handler.is_some());

        let missing = registry.get("missing");
        assert!(missing.is_none());
    }

    #[test]
    fn test_invoke_handler() {
        let mut counter: i32 = 0;
        let handler = RegisteredHandler {
            callback: test_handler,
            user_data: &mut counter as *mut i32 as *mut c_void,
        };

        let ctx = ArchimedesRequestContext {
            request_id: std::ptr::null(),
            trace_id: std::ptr::null(),
            span_id: std::ptr::null(),
            operation_id: std::ptr::null(),
            method: std::ptr::null(),
            path: std::ptr::null(),
            query: std::ptr::null(),
            caller_identity_json: std::ptr::null(),
            path_params_count: 0,
            path_param_names: std::ptr::null(),
            path_param_values: std::ptr::null(),
            headers_count: 0,
            header_names: std::ptr::null(),
            header_values: std::ptr::null(),
        };

        let response = invoke_handler(&handler, &ctx, &[]);
        assert_eq!(response.status_code, 200);
        assert_eq!(counter, 1);

        // Invoke again
        let _ = invoke_handler(&handler, &ctx, &[]);
        assert_eq!(counter, 2);
    }
}
