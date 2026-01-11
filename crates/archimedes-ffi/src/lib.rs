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
mod extractors;
mod handler;
mod lifecycle;
mod middleware_config;
mod request;
mod response;
mod router;
mod runtime;
mod test_client;
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
pub use middleware_config::{
    archimedes_compression_config_add_content_type, archimedes_compression_config_brotli,
    archimedes_compression_config_deflate, archimedes_compression_config_free,
    archimedes_compression_config_get_level, archimedes_compression_config_get_min_size,
    archimedes_compression_config_gzip, archimedes_compression_config_is_brotli,
    archimedes_compression_config_is_gzip, archimedes_compression_config_level,
    archimedes_compression_config_min_size, archimedes_compression_config_new,
    archimedes_compression_config_should_compress, archimedes_compression_config_zstd,
    archimedes_cors_config_allow_any_origin, archimedes_cors_config_allow_credentials,
    archimedes_cors_config_allow_header, archimedes_cors_config_allow_method,
    archimedes_cors_config_allow_origin, archimedes_cors_config_expose_header,
    archimedes_cors_config_free, archimedes_cors_config_get_allow_credentials,
    archimedes_cors_config_get_max_age, archimedes_cors_config_is_method_allowed,
    archimedes_cors_config_is_origin_allowed, archimedes_cors_config_max_age,
    archimedes_cors_config_new, archimedes_rate_limit_config_burst,
    archimedes_rate_limit_config_enabled, archimedes_rate_limit_config_exempt_path,
    archimedes_rate_limit_config_free, archimedes_rate_limit_config_get_burst,
    archimedes_rate_limit_config_get_rps, archimedes_rate_limit_config_is_enabled,
    archimedes_rate_limit_config_is_exempt, archimedes_rate_limit_config_key_extractor,
    archimedes_rate_limit_config_new, archimedes_rate_limit_config_rps,
    archimedes_static_files_config_cache_max_age, archimedes_static_files_config_directory,
    archimedes_static_files_config_fallback, archimedes_static_files_config_free,
    archimedes_static_files_config_get_cache_max_age, archimedes_static_files_config_index,
    archimedes_static_files_config_is_precompressed, archimedes_static_files_config_new,
    archimedes_static_files_config_precompressed, archimedes_static_files_config_prefix,
    archimedes_static_files_config_resolve_path, ArchimedesCompressionAlgorithm,
    ArchimedesCompressionConfig, ArchimedesCorsConfig, ArchimedesRateLimitConfig,
    ArchimedesStaticFilesConfig,
};
pub use router::{
    archimedes_router_count, archimedes_router_free, archimedes_router_get_prefix,
    archimedes_router_merge, archimedes_router_nest, archimedes_router_nested_count,
    archimedes_router_new, archimedes_router_operation_count, archimedes_router_prefix,
    archimedes_router_register, archimedes_router_tag, archimedes_router_tag_count,
    ArchimedesRouter,
};
pub use extractors::{
    archimedes_cookies_free, archimedes_cookies_get, archimedes_cookies_parse,
    archimedes_file_response, archimedes_form_free, archimedes_form_get, archimedes_form_parse,
    archimedes_get_header, archimedes_get_multipart_boundary, archimedes_multipart_free,
    archimedes_multipart_get, archimedes_multipart_parse, archimedes_redirect,
    archimedes_redirect_found, archimedes_redirect_permanent, archimedes_redirect_see_other,
    archimedes_redirect_temporary, archimedes_set_cookie_build, archimedes_set_cookie_domain,
    archimedes_set_cookie_expires, archimedes_set_cookie_free, archimedes_set_cookie_http_only,
    archimedes_set_cookie_max_age, archimedes_set_cookie_new, archimedes_set_cookie_path,
    archimedes_set_cookie_same_site, archimedes_set_cookie_secure, ArchimedesCookies,
    ArchimedesForm, ArchimedesMultipart, ArchimedesMultipartField, ArchimedesSameSite,
    ArchimedesSetCookie,
};
pub use test_client::{
    archimedes_string_free, archimedes_test_client_delete, archimedes_test_client_free,
    archimedes_test_client_get, archimedes_test_client_new, archimedes_test_client_patch,
    archimedes_test_client_post, archimedes_test_client_put, archimedes_test_client_request,
    archimedes_test_client_with_bearer_token, archimedes_test_client_with_header,
    archimedes_test_response_assert_body_contains, archimedes_test_response_assert_header,
    archimedes_test_response_assert_status, archimedes_test_response_assert_success,
    archimedes_test_response_body, archimedes_test_response_free,
    archimedes_test_response_get_header, archimedes_test_response_is_client_error,
    archimedes_test_response_is_server_error, archimedes_test_response_is_success,
    archimedes_test_response_status_code, archimedes_test_response_text, ArchimedesTestClient,
    ArchimedesTestResponse,
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
