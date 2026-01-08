//! FFI-safe type definitions
//!
//! All types in this module use `#[repr(C)]` to ensure stable ABI across
//! different compilers and languages.

use std::os::raw::c_char;

/// Error codes returned by Archimedes FFI functions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchimedesError {
    /// Success - no error
    Ok = 0,
    /// Invalid configuration provided
    InvalidConfig = 1,
    /// Failed to load contract artifact
    ContractLoadError = 2,
    /// Failed to load policy bundle
    PolicyLoadError = 3,
    /// Failed to register handler
    HandlerRegistrationError = 4,
    /// Failed to start server
    ServerStartError = 5,
    /// Invalid operation ID
    InvalidOperation = 6,
    /// Handler returned an error
    HandlerError = 7,
    /// Validation failed
    ValidationError = 8,
    /// Authorization denied
    AuthorizationError = 9,
    /// Null pointer provided where non-null required
    NullPointer = 10,
    /// Invalid UTF-8 string
    InvalidUtf8 = 11,
    /// Internal error
    Internal = 99,
}

impl From<ArchimedesError> for i32 {
    fn from(err: ArchimedesError) -> Self {
        err as i32
    }
}

impl From<i32> for ArchimedesError {
    fn from(code: i32) -> Self {
        match code {
            0 => Self::Ok,
            1 => Self::InvalidConfig,
            2 => Self::ContractLoadError,
            3 => Self::PolicyLoadError,
            4 => Self::HandlerRegistrationError,
            5 => Self::ServerStartError,
            6 => Self::InvalidOperation,
            7 => Self::HandlerError,
            8 => Self::ValidationError,
            9 => Self::AuthorizationError,
            10 => Self::NullPointer,
            11 => Self::InvalidUtf8,
            _ => Self::Internal,
        }
    }
}

/// Opaque handle to an Archimedes application instance
///
/// This handle must be created with `archimedes_new()` and freed with
/// `archimedes_free()`. Do not attempt to dereference or modify this pointer.
#[repr(C)]
pub struct ArchimedesApp {
    _private: [u8; 0],
}

/// Request context passed to handlers
///
/// This struct provides read-only access to request metadata. All string
/// pointers are valid for the duration of the handler call.
#[repr(C)]
#[derive(Debug)]
pub struct ArchimedesRequestContext {
    /// Unique request ID (UUID v7 string)
    pub request_id: *const c_char,
    /// OpenTelemetry trace ID
    pub trace_id: *const c_char,
    /// OpenTelemetry span ID
    pub span_id: *const c_char,
    /// Matched operation ID from contract
    pub operation_id: *const c_char,
    /// HTTP method (GET, POST, etc.)
    pub method: *const c_char,
    /// Request path (e.g., "/users/123")
    pub path: *const c_char,
    /// Query string (without leading ?)
    pub query: *const c_char,
    /// JSON-encoded caller identity
    pub caller_identity_json: *const c_char,
    /// Number of path parameters
    pub path_params_count: usize,
    /// Path parameter names (array of C strings)
    pub path_param_names: *const *const c_char,
    /// Path parameter values (array of C strings)
    pub path_param_values: *const *const c_char,
    /// Number of headers
    pub headers_count: usize,
    /// Header names (array of C strings)
    pub header_names: *const *const c_char,
    /// Header values (array of C strings)
    pub header_values: *const *const c_char,
}

/// Response data returned by handlers
///
/// The handler must populate this struct with response information.
/// For `body`, the handler can either:
/// - Return a static string (set `body_owned` to false)
/// - Return memory allocated with `archimedes_alloc` (set `body_owned` to true)
#[repr(C)]
#[derive(Debug)]
pub struct ArchimedesResponseData {
    /// HTTP status code (e.g., 200, 404)
    pub status_code: i32,
    /// Response body (JSON or other content)
    pub body: *const c_char,
    /// Length of body in bytes
    pub body_len: usize,
    /// Whether Archimedes should free the body pointer
    pub body_owned: bool,
    /// Content-Type header value (null for default application/json)
    pub content_type: *const c_char,
    /// Number of response headers
    pub headers_count: usize,
    /// Header names (array of C strings)
    pub header_names: *const *const c_char,
    /// Header values (array of C strings)
    pub header_values: *const *const c_char,
}

impl Default for ArchimedesResponseData {
    fn default() -> Self {
        Self {
            status_code: 200,
            body: std::ptr::null(),
            body_len: 0,
            body_owned: false,
            content_type: std::ptr::null(),
            headers_count: 0,
            header_names: std::ptr::null(),
            header_values: std::ptr::null(),
        }
    }
}

/// Handler function signature
///
/// Handlers receive the request context, body, and user data, and must return
/// a response. The handler is called synchronously from Archimedes' async runtime.
///
/// # Parameters
///
/// - `ctx`: Read-only request context with metadata
/// - `body`: Request body bytes (may be null for requests without body)
/// - `body_len`: Length of body in bytes
/// - `user_data`: User-provided pointer from handler registration
///
/// # Returns
///
/// Response data with status code and body. Use default values for simple responses:
///
/// ```c
/// archimedes_response_data response = {0};
/// response.status_code = 200;
/// response.body = "{\"ok\": true}";
/// response.body_len = 12;
/// return response;
/// ```
pub type ArchimedesHandlerFn = extern "C" fn(
    ctx: *const ArchimedesRequestContext,
    body: *const u8,
    body_len: usize,
    user_data: *mut std::ffi::c_void,
) -> ArchimedesResponseData;

/// Async handler completion callback
///
/// For languages that need async handlers, this callback is invoked when the
/// handler completes. The callback receives the response data and user context.
pub type ArchimedesAsyncCallback =
    extern "C" fn(response: ArchimedesResponseData, callback_data: *mut std::ffi::c_void);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        assert_eq!(ArchimedesError::Ok as i32, 0);
        assert_eq!(ArchimedesError::InvalidConfig as i32, 1);
        assert_eq!(ArchimedesError::Internal as i32, 99);
    }

    #[test]
    fn test_error_conversion() {
        assert_eq!(ArchimedesError::from(0), ArchimedesError::Ok);
        assert_eq!(ArchimedesError::from(1), ArchimedesError::InvalidConfig);
        assert_eq!(ArchimedesError::from(999), ArchimedesError::Internal);
    }

    #[test]
    fn test_response_default() {
        let response = ArchimedesResponseData::default();
        assert_eq!(response.status_code, 200);
        assert!(response.body.is_null());
        assert_eq!(response.body_len, 0);
        assert!(!response.body_owned);
    }

    #[test]
    fn test_type_sizes() {
        // Ensure types have stable sizes for ABI compatibility
        assert!(std::mem::size_of::<ArchimedesError>() <= 4);
        assert!(std::mem::size_of::<ArchimedesRequestContext>() > 0);
        assert!(std::mem::size_of::<ArchimedesResponseData>() > 0);
    }
}
