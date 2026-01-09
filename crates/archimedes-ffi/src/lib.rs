//! # Archimedes FFI
//!
//! C ABI foreign function interface for Archimedes, enabling native language
//! bindings for Python, Go, TypeScript, and C++.
//!
//! ## Overview
//!
//! This crate provides a stable C ABI that can be called from any language
//! that supports C FFI. It exposes the full Archimedes functionality including:
//!
//! - HTTP server with routing
//! - Contract validation (Themis)
//! - Authorization (OPA/Eunomia)
//! - Observability (OpenTelemetry)
//! - Request/Response handling
//!
//! ## Safety
//!
//! All FFI functions are marked `unsafe` because they deal with raw pointers
//! from foreign code. The caller is responsible for:
//!
//! - Ensuring pointers are valid and properly aligned
//! - Managing memory lifetimes correctly
//! - Not calling functions from multiple threads without synchronization
//!
//! ## Memory Management
//!
//! Memory ownership follows these rules:
//!
//! - Strings passed TO Archimedes are borrowed (not freed by Archimedes)
//! - Strings returned FROM Archimedes must be freed with `archimedes_free_string`
//! - Opaque handles must be freed with their respective `_free` functions
//!
//! ## Example (C)
//!
//! ```c
//! #include <archimedes.h>
//!
//! archimedes_response_data my_handler(
//!     const archimedes_request_context* ctx,
//!     const char* body,
//!     size_t body_len,
//!     void* user_data
//! ) {
//!     return (archimedes_response_data){
//!         .status_code = 200,
//!         .body = "{\"message\": \"Hello\"}",
//!         .body_len = 21,
//!     };
//! }
//!
//! int main() {
//!     archimedes_config config = {
//!         .contract_path = "contract.json",
//!         .listen_port = 8080,
//!     };
//!     
//!     archimedes_app app = archimedes_new(&config);
//!     archimedes_register_handler(app, "hello", my_handler, NULL);
//!     archimedes_run(app);
//!     archimedes_free(app);
//!     return 0;
//! }
//! ```

#![allow(unsafe_code)] // FFI requires unsafe
#![allow(clippy::missing_safety_doc)] // Safety docs in module-level

mod app;
mod config;
mod error;
mod handler;
mod lifecycle;
mod request;
mod response;
mod router;
mod runtime;
mod types;

// Public re-exports for FFI consumers
pub use app::{
    archimedes_free, archimedes_is_running, archimedes_load_contract, archimedes_new,
    archimedes_register_handler, archimedes_run, archimedes_stop, archimedes_version,
};
pub use config::ArchimedesConfig;
pub use error::FfiError;
pub use lifecycle::{
    archimedes_lifecycle_clear, archimedes_lifecycle_free, archimedes_lifecycle_has_shutdown,
    archimedes_lifecycle_has_startup, archimedes_lifecycle_new, archimedes_lifecycle_on_shutdown,
    archimedes_lifecycle_on_startup, archimedes_lifecycle_run_shutdown,
    archimedes_lifecycle_run_startup, archimedes_lifecycle_shutdown_count,
    archimedes_lifecycle_startup_count, ArchimedesLifecycle, ArchimedesLifecycleHook,
};
pub use router::{
    archimedes_router_count, archimedes_router_free, archimedes_router_get_prefix,
    archimedes_router_merge, archimedes_router_nest, archimedes_router_nested_count,
    archimedes_router_new, archimedes_router_operation_count, archimedes_router_prefix,
    archimedes_router_register, archimedes_router_tag, archimedes_router_tag_count,
    ArchimedesRouter,
};
pub use types::{
    ArchimedesAsyncCallback, ArchimedesError, ArchimedesHandlerFn, ArchimedesRequestContext,
    ArchimedesResponseData,
};

use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::OnceLock;

use parking_lot::Mutex;

/// Global last error message for FFI error reporting
static LAST_ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Set the last error message
pub(crate) fn set_last_error(err: impl std::fmt::Display) {
    let lock = LAST_ERROR.get_or_init(|| Mutex::new(None));
    *lock.lock() = Some(err.to_string());
}

/// Get the last error message as a C string
///
/// # Safety
///
/// The returned pointer is valid until the next call to any Archimedes function
/// that may set an error. The caller must not free this pointer.
#[no_mangle]
pub unsafe extern "C" fn archimedes_last_error() -> *const c_char {
    static ERROR_BUFFER: OnceLock<Mutex<Vec<u8>>> = OnceLock::new();

    let lock = LAST_ERROR.get_or_init(|| Mutex::new(None));
    let error = lock.lock();

    match error.as_ref() {
        Some(msg) => {
            let buffer = ERROR_BUFFER.get_or_init(|| Mutex::new(Vec::new()));
            let mut buf = buffer.lock();
            buf.clear();
            buf.extend_from_slice(msg.as_bytes());
            buf.push(0); // null terminator
            buf.as_ptr().cast()
        }
        None => std::ptr::null(),
    }
}

/// Helper to convert C string to Rust string
///
/// Returns None if the pointer is null or the string is not valid UTF-8.
fn c_str_to_rust(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok().map(|s| s.to_string()) }
}

/// Helper to convert C string to Rust &str
///
/// Returns None if the pointer is null or the string is not valid UTF-8.
fn c_str_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr).to_str().ok() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_last_error() {
        set_last_error("test error");
        unsafe {
            let ptr = archimedes_last_error();
            assert!(!ptr.is_null());
            let msg = CStr::from_ptr(ptr).to_str().unwrap();
            assert_eq!(msg, "test error");
        }
    }

    #[test]
    fn test_c_str_to_rust() {
        let s = std::ffi::CString::new("hello").unwrap();
        let result = c_str_to_rust(s.as_ptr());
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_c_str_null() {
        let result = c_str_to_rust(std::ptr::null());
        assert_eq!(result, None);
    }
}
