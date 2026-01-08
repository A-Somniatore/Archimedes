//! Request context building
//!
//! Converts internal Archimedes request types to FFI-safe structs.

use crate::types::ArchimedesRequestContext;
use std::ffi::CString;
use std::os::raw::c_char;

/// Builder for FFI request context
///
/// This struct owns all the strings and arrays needed for the FFI context,
/// keeping them alive for the duration of the handler call.
pub(crate) struct RequestContextBuilder {
    // Owned strings (kept alive for FFI)
    request_id: CString,
    trace_id: CString,
    span_id: CString,
    operation_id: CString,
    method: CString,
    path: CString,
    query: CString,
    caller_identity_json: CString,

    // Path parameters
    path_param_names: Vec<CString>,
    path_param_values: Vec<CString>,
    path_param_name_ptrs: Vec<*const c_char>,
    path_param_value_ptrs: Vec<*const c_char>,

    // Headers
    header_names: Vec<CString>,
    header_values: Vec<CString>,
    header_name_ptrs: Vec<*const c_char>,
    header_value_ptrs: Vec<*const c_char>,
}

impl RequestContextBuilder {
    /// Create a new builder with required fields
    pub fn new(
        request_id: &str,
        operation_id: &str,
        method: &str,
        path: &str,
    ) -> Self {
        Self {
            request_id: CString::new(request_id).unwrap_or_default(),
            trace_id: CString::new("").unwrap_or_default(),
            span_id: CString::new("").unwrap_or_default(),
            operation_id: CString::new(operation_id).unwrap_or_default(),
            method: CString::new(method).unwrap_or_default(),
            path: CString::new(path).unwrap_or_default(),
            query: CString::new("").unwrap_or_default(),
            caller_identity_json: CString::new("null").unwrap_or_default(),
            path_param_names: Vec::new(),
            path_param_values: Vec::new(),
            path_param_name_ptrs: Vec::new(),
            path_param_value_ptrs: Vec::new(),
            header_names: Vec::new(),
            header_values: Vec::new(),
            header_name_ptrs: Vec::new(),
            header_value_ptrs: Vec::new(),
        }
    }

    /// Set trace context
    pub fn with_trace(mut self, trace_id: &str, span_id: &str) -> Self {
        self.trace_id = CString::new(trace_id).unwrap_or_default();
        self.span_id = CString::new(span_id).unwrap_or_default();
        self
    }

    /// Set query string
    pub fn with_query(mut self, query: &str) -> Self {
        self.query = CString::new(query).unwrap_or_default();
        self
    }

    /// Set caller identity as JSON
    pub fn with_caller_identity(mut self, identity_json: &str) -> Self {
        self.caller_identity_json = CString::new(identity_json).unwrap_or_default();
        self
    }

    /// Add path parameters
    pub fn with_path_params(mut self, params: &[(String, String)]) -> Self {
        self.path_param_names = params
            .iter()
            .map(|(k, _)| CString::new(k.as_str()).unwrap_or_default())
            .collect();
        self.path_param_values = params
            .iter()
            .map(|(_, v)| CString::new(v.as_str()).unwrap_or_default())
            .collect();
        self
    }

    /// Add headers
    pub fn with_headers(mut self, headers: &[(String, String)]) -> Self {
        self.header_names = headers
            .iter()
            .map(|(k, _)| CString::new(k.as_str()).unwrap_or_default())
            .collect();
        self.header_values = headers
            .iter()
            .map(|(_, v)| CString::new(v.as_str()).unwrap_or_default())
            .collect();
        self
    }

    /// Build the FFI context
    ///
    /// The returned context borrows from this builder, so the builder must
    /// be kept alive for the duration of the handler call.
    pub fn build(&mut self) -> ArchimedesRequestContext {
        // Build pointer arrays for path params
        self.path_param_name_ptrs = self
            .path_param_names
            .iter()
            .map(|s| s.as_ptr())
            .collect();
        self.path_param_value_ptrs = self
            .path_param_values
            .iter()
            .map(|s| s.as_ptr())
            .collect();

        // Build pointer arrays for headers
        self.header_name_ptrs = self
            .header_names
            .iter()
            .map(|s| s.as_ptr())
            .collect();
        self.header_value_ptrs = self
            .header_values
            .iter()
            .map(|s| s.as_ptr())
            .collect();

        ArchimedesRequestContext {
            request_id: self.request_id.as_ptr(),
            trace_id: self.trace_id.as_ptr(),
            span_id: self.span_id.as_ptr(),
            operation_id: self.operation_id.as_ptr(),
            method: self.method.as_ptr(),
            path: self.path.as_ptr(),
            query: self.query.as_ptr(),
            caller_identity_json: self.caller_identity_json.as_ptr(),
            path_params_count: self.path_param_names.len(),
            path_param_names: if self.path_param_name_ptrs.is_empty() {
                std::ptr::null()
            } else {
                self.path_param_name_ptrs.as_ptr()
            },
            path_param_values: if self.path_param_value_ptrs.is_empty() {
                std::ptr::null()
            } else {
                self.path_param_value_ptrs.as_ptr()
            },
            headers_count: self.header_names.len(),
            header_names: if self.header_name_ptrs.is_empty() {
                std::ptr::null()
            } else {
                self.header_name_ptrs.as_ptr()
            },
            header_values: if self.header_value_ptrs.is_empty() {
                std::ptr::null()
            } else {
                self.header_value_ptrs.as_ptr()
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_builder_basic() {
        let mut builder = RequestContextBuilder::new(
            "req-123",
            "listUsers",
            "GET",
            "/users",
        );
        let ctx = builder.build();

        unsafe {
            assert_eq!(
                CStr::from_ptr(ctx.request_id).to_str().unwrap(),
                "req-123"
            );
            assert_eq!(
                CStr::from_ptr(ctx.operation_id).to_str().unwrap(),
                "listUsers"
            );
            assert_eq!(CStr::from_ptr(ctx.method).to_str().unwrap(), "GET");
            assert_eq!(CStr::from_ptr(ctx.path).to_str().unwrap(), "/users");
        }
    }

    #[test]
    fn test_builder_with_trace() {
        let mut builder = RequestContextBuilder::new("req-1", "op", "GET", "/")
            .with_trace("trace-abc", "span-xyz");
        let ctx = builder.build();

        unsafe {
            assert_eq!(
                CStr::from_ptr(ctx.trace_id).to_str().unwrap(),
                "trace-abc"
            );
            assert_eq!(CStr::from_ptr(ctx.span_id).to_str().unwrap(), "span-xyz");
        }
    }

    #[test]
    fn test_builder_with_path_params() {
        let params = vec![
            ("userId".to_string(), "123".to_string()),
            ("postId".to_string(), "456".to_string()),
        ];

        let mut builder = RequestContextBuilder::new("req-1", "op", "GET", "/")
            .with_path_params(&params);
        let ctx = builder.build();

        assert_eq!(ctx.path_params_count, 2);
        assert!(!ctx.path_param_names.is_null());
        assert!(!ctx.path_param_values.is_null());

        unsafe {
            let names = std::slice::from_raw_parts(ctx.path_param_names, 2);
            let values = std::slice::from_raw_parts(ctx.path_param_values, 2);

            assert_eq!(CStr::from_ptr(names[0]).to_str().unwrap(), "userId");
            assert_eq!(CStr::from_ptr(values[0]).to_str().unwrap(), "123");
            assert_eq!(CStr::from_ptr(names[1]).to_str().unwrap(), "postId");
            assert_eq!(CStr::from_ptr(values[1]).to_str().unwrap(), "456");
        }
    }

    #[test]
    fn test_builder_with_headers() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Authorization".to_string(), "Bearer token".to_string()),
        ];

        let mut builder = RequestContextBuilder::new("req-1", "op", "GET", "/")
            .with_headers(&headers);
        let ctx = builder.build();

        assert_eq!(ctx.headers_count, 2);
        assert!(!ctx.header_names.is_null());
    }

    #[test]
    fn test_builder_empty_params() {
        let mut builder = RequestContextBuilder::new("req-1", "op", "GET", "/");
        let ctx = builder.build();

        assert_eq!(ctx.path_params_count, 0);
        assert!(ctx.path_param_names.is_null());
        assert!(ctx.path_param_values.is_null());
        assert_eq!(ctx.headers_count, 0);
        assert!(ctx.header_names.is_null());
    }
}
