//! Response handling
//!
//! Converts FFI response data to internal Archimedes types.

use crate::types::ArchimedesResponseData;
use std::ffi::CStr;

/// Convert FFI response to HTTP response bytes
///
/// # Safety
///
/// The response data must have valid pointers if body_len > 0.
pub(crate) fn response_to_bytes(response: &ArchimedesResponseData) -> (u16, Vec<u8>, String) {
    let status = response.status_code.try_into().unwrap_or(500);

    let body = if response.body.is_null() || response.body_len == 0 {
        Vec::new()
    } else {
        unsafe {
            std::slice::from_raw_parts(response.body.cast::<u8>(), response.body_len).to_vec()
        }
    };

    let content_type = if response.content_type.is_null() {
        "application/json".to_string()
    } else {
        unsafe {
            CStr::from_ptr(response.content_type)
                .to_str()
                .unwrap_or("application/json")
                .to_string()
        }
    };

    (status, body, content_type)
}

/// Extract headers from FFI response
///
/// # Safety
///
/// If headers_count > 0, header_names and header_values must be valid arrays.
pub(crate) fn extract_headers(response: &ArchimedesResponseData) -> Vec<(String, String)> {
    if response.headers_count == 0
        || response.header_names.is_null()
        || response.header_values.is_null()
    {
        return Vec::new();
    }

    unsafe {
        let names = std::slice::from_raw_parts(response.header_names, response.headers_count);
        let values = std::slice::from_raw_parts(response.header_values, response.headers_count);

        names
            .iter()
            .zip(values.iter())
            .filter_map(|(&name, &value)| {
                if name.is_null() || value.is_null() {
                    return None;
                }
                let name_str = CStr::from_ptr(name).to_str().ok()?;
                let value_str = CStr::from_ptr(value).to_str().ok()?;
                Some((name_str.to_string(), value_str.to_string()))
            })
            .collect()
    }
}

/// Free response body if it was allocated by the handler
///
/// # Safety
///
/// Only call this if body_owned is true and body was allocated with archimedes_alloc.
pub(crate) unsafe fn maybe_free_response_body(response: &ArchimedesResponseData) {
    if response.body_owned && !response.body.is_null() {
        // The body was allocated with archimedes_alloc, free it
        let ptr = response.body as *mut u8;
        let layout = std::alloc::Layout::from_size_align_unchecked(response.body_len, 1);
        std::alloc::dealloc(ptr, layout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::os::raw::c_char;

    #[test]
    fn test_response_to_bytes_empty() {
        let response = ArchimedesResponseData::default();
        let (status, body, content_type) = response_to_bytes(&response);

        assert_eq!(status, 200);
        assert!(body.is_empty());
        assert_eq!(content_type, "application/json");
    }

    #[test]
    fn test_response_to_bytes_with_body() {
        let body_str = b"{\"ok\": true}";
        let response = ArchimedesResponseData {
            status_code: 201,
            body: body_str.as_ptr().cast(),
            body_len: body_str.len(),
            body_owned: false,
            ..Default::default()
        };

        let (status, body, content_type) = response_to_bytes(&response);
        assert_eq!(status, 201);
        assert_eq!(body, body_str);
        assert_eq!(content_type, "application/json");
    }

    #[test]
    fn test_response_to_bytes_custom_content_type() {
        let ct = CString::new("text/plain").unwrap();
        let response = ArchimedesResponseData {
            status_code: 200,
            body: std::ptr::null(),
            body_len: 0,
            body_owned: false,
            content_type: ct.as_ptr(),
            ..Default::default()
        };

        let (_, _, content_type) = response_to_bytes(&response);
        assert_eq!(content_type, "text/plain");
    }

    #[test]
    fn test_extract_headers_empty() {
        let response = ArchimedesResponseData::default();
        let headers = extract_headers(&response);
        assert!(headers.is_empty());
    }

    #[test]
    fn test_extract_headers() {
        let name1 = CString::new("X-Custom").unwrap();
        let value1 = CString::new("value1").unwrap();
        let name2 = CString::new("X-Other").unwrap();
        let value2 = CString::new("value2").unwrap();

        let names: Vec<*const c_char> = vec![name1.as_ptr(), name2.as_ptr()];
        let values: Vec<*const c_char> = vec![value1.as_ptr(), value2.as_ptr()];

        let response = ArchimedesResponseData {
            headers_count: 2,
            header_names: names.as_ptr(),
            header_values: values.as_ptr(),
            ..Default::default()
        };

        let headers = extract_headers(&response);
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0], ("X-Custom".to_string(), "value1".to_string()));
        assert_eq!(headers[1], ("X-Other".to_string(), "value2".to_string()));
    }

    #[test]
    fn test_invalid_status_code() {
        let response = ArchimedesResponseData {
            status_code: -1,
            ..Default::default()
        };

        let (status, _, _) = response_to_bytes(&response);
        assert_eq!(status, 500); // Falls back to 500
    }
}
