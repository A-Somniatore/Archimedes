//! C FFI bindings for Archimedes TestClient.
//!
//! This module provides C ABI functions for testing Archimedes applications
//! without starting a real HTTP server.

use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};
use std::ptr;

/// Opaque test client handle.
#[repr(C)]
pub struct ArchimedesTestClient {
    default_headers: HashMap<String, String>,
    base_url: String,
}

/// Opaque test response handle.
#[repr(C)]
pub struct ArchimedesTestResponse {
    status_code: u16,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

// ============================================================================
// TestClient Functions
// ============================================================================

/// Creates a new test client.
///
/// # Safety
/// - `base_url` must be a valid null-terminated string or NULL for default.
/// - Caller must free the returned handle with `archimedes_test_client_free`.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_new(
    base_url: *const c_char,
) -> *mut ArchimedesTestClient {
    let base_url = if base_url.is_null() {
        "http://test".to_string()
    } else {
        match CStr::from_ptr(base_url).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return ptr::null_mut(),
        }
    };

    let client = Box::new(ArchimedesTestClient {
        default_headers: HashMap::new(),
        base_url,
    });
    Box::into_raw(client)
}

/// Frees a test client.
///
/// # Safety
/// - `client` must be a valid pointer from `archimedes_test_client_new`.
/// - Must only be called once per client.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_free(client: *mut ArchimedesTestClient) {
    if !client.is_null() {
        drop(Box::from_raw(client));
    }
}

/// Adds a default header to all requests.
///
/// # Safety
/// - `client` must be a valid test client pointer.
/// - `name` and `value` must be valid null-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_with_header(
    client: *mut ArchimedesTestClient,
    name: *const c_char,
    value: *const c_char,
) {
    if client.is_null() || name.is_null() || value.is_null() {
        return;
    }

    let client = &mut *client;
    let name = match CStr::from_ptr(name).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return,
    };
    let value = match CStr::from_ptr(value).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return,
    };

    client.default_headers.insert(name, value);
}

/// Sets a bearer token for all requests.
///
/// # Safety
/// - `client` must be a valid test client pointer.
/// - `token` must be a valid null-terminated string.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_with_bearer_token(
    client: *mut ArchimedesTestClient,
    token: *const c_char,
) {
    if client.is_null() || token.is_null() {
        return;
    }

    let client = &mut *client;
    let token = match CStr::from_ptr(token).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return,
    };

    client
        .default_headers
        .insert("Authorization".to_string(), format!("Bearer {}", token));
}

/// Makes a GET request.
///
/// # Safety
/// - `client` must be a valid test client pointer.
/// - `path` must be a valid null-terminated string.
/// - Caller must free the returned response with `archimedes_test_response_free`.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_get(
    client: *const ArchimedesTestClient,
    path: *const c_char,
) -> *mut ArchimedesTestResponse {
    archimedes_test_client_request(
        client,
        b"GET\0".as_ptr().cast(),
        path,
        ptr::null(),
        0,
    )
}

/// Makes a POST request.
///
/// # Safety
/// - `client` must be a valid test client pointer.
/// - `path` must be a valid null-terminated string.
/// - `body` can be NULL or a pointer to body bytes.
/// - Caller must free the returned response with `archimedes_test_response_free`.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_post(
    client: *const ArchimedesTestClient,
    path: *const c_char,
    body: *const u8,
    body_len: usize,
) -> *mut ArchimedesTestResponse {
    archimedes_test_client_request(
        client,
        b"POST\0".as_ptr().cast(),
        path,
        body,
        body_len,
    )
}

/// Makes a PUT request.
///
/// # Safety
/// - `client` must be a valid test client pointer.
/// - `path` must be a valid null-terminated string.
/// - `body` can be NULL or a pointer to body bytes.
/// - Caller must free the returned response with `archimedes_test_response_free`.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_put(
    client: *const ArchimedesTestClient,
    path: *const c_char,
    body: *const u8,
    body_len: usize,
) -> *mut ArchimedesTestResponse {
    archimedes_test_client_request(
        client,
        b"PUT\0".as_ptr().cast(),
        path,
        body,
        body_len,
    )
}

/// Makes a PATCH request.
///
/// # Safety
/// - `client` must be a valid test client pointer.
/// - `path` must be a valid null-terminated string.
/// - `body` can be NULL or a pointer to body bytes.
/// - Caller must free the returned response with `archimedes_test_response_free`.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_patch(
    client: *const ArchimedesTestClient,
    path: *const c_char,
    body: *const u8,
    body_len: usize,
) -> *mut ArchimedesTestResponse {
    archimedes_test_client_request(
        client,
        b"PATCH\0".as_ptr().cast(),
        path,
        body,
        body_len,
    )
}

/// Makes a DELETE request.
///
/// # Safety
/// - `client` must be a valid test client pointer.
/// - `path` must be a valid null-terminated string.
/// - Caller must free the returned response with `archimedes_test_response_free`.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_delete(
    client: *const ArchimedesTestClient,
    path: *const c_char,
) -> *mut ArchimedesTestResponse {
    archimedes_test_client_request(
        client,
        b"DELETE\0".as_ptr().cast(),
        path,
        ptr::null(),
        0,
    )
}

/// Makes a request with a custom method.
///
/// # Safety
/// - `client` must be a valid test client pointer.
/// - `method` and `path` must be valid null-terminated strings.
/// - `body` can be NULL or a pointer to body bytes.
/// - Caller must free the returned response with `archimedes_test_response_free`.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_client_request(
    client: *const ArchimedesTestClient,
    method: *const c_char,
    path: *const c_char,
    body: *const u8,
    body_len: usize,
) -> *mut ArchimedesTestResponse {
    if client.is_null() || method.is_null() || path.is_null() {
        return ptr::null_mut();
    }

    let client = &*client;
    let _method = match CStr::from_ptr(method).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return ptr::null_mut(),
    };
    let path = match CStr::from_ptr(path).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return ptr::null_mut(),
    };

    // Build full URL
    let _url = if path.starts_with("http://") || path.starts_with("https://") {
        path
    } else {
        format!("{}{}", client.base_url, path)
    };

    // Get body bytes
    let body_bytes = if body.is_null() || body_len == 0 {
        None
    } else {
        Some(std::slice::from_raw_parts(body, body_len).to_vec())
    };

    // For now, create a mock response
    let response = Box::new(ArchimedesTestResponse {
        status_code: 200,
        headers: client.default_headers.clone(),
        body: body_bytes.unwrap_or_default(),
    });
    Box::into_raw(response)
}

// ============================================================================
// TestResponse Functions
// ============================================================================

/// Frees a test response.
///
/// # Safety
/// - `response` must be a valid pointer from a test client request method.
/// - Must only be called once per response.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_free(response: *mut ArchimedesTestResponse) {
    if !response.is_null() {
        drop(Box::from_raw(response));
    }
}

/// Gets the status code from a test response.
///
/// # Safety
/// - `response` must be a valid test response pointer.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_status_code(
    response: *const ArchimedesTestResponse,
) -> u16 {
    if response.is_null() {
        return 0;
    }
    (*response).status_code
}

/// Returns true if the status is successful (2xx).
///
/// # Safety
/// - `response` must be a valid test response pointer.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_is_success(
    response: *const ArchimedesTestResponse,
) -> bool {
    if response.is_null() {
        return false;
    }
    (200..300).contains(&(*response).status_code)
}

/// Returns true if the status is a client error (4xx).
///
/// # Safety
/// - `response` must be a valid test response pointer.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_is_client_error(
    response: *const ArchimedesTestResponse,
) -> bool {
    if response.is_null() {
        return false;
    }
    (400..500).contains(&(*response).status_code)
}

/// Returns true if the status is a server error (5xx).
///
/// # Safety
/// - `response` must be a valid test response pointer.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_is_server_error(
    response: *const ArchimedesTestResponse,
) -> bool {
    if response.is_null() {
        return false;
    }
    (500..600).contains(&(*response).status_code)
}

/// Gets a header value by name (case-insensitive).
///
/// # Safety
/// - `response` must be a valid test response pointer.
/// - `name` must be a valid null-terminated string.
/// - Caller must free the returned string with `archimedes_string_free`.
/// - Returns NULL if the header is not found.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_get_header(
    response: *const ArchimedesTestResponse,
    name: *const c_char,
) -> *mut c_char {
    if response.is_null() || name.is_null() {
        return ptr::null_mut();
    }

    let response = &*response;
    let name = match CStr::from_ptr(name).to_str() {
        Ok(s) => s.to_lowercase(),
        Err(_) => return ptr::null_mut(),
    };

    for (k, v) in &response.headers {
        if k.to_lowercase() == name {
            return match CString::new(v.as_str()) {
                Ok(s) => s.into_raw(),
                Err(_) => ptr::null_mut(),
            };
        }
    }
    ptr::null_mut()
}

/// Gets the response body as a pointer and length.
///
/// # Safety
/// - `response` must be a valid test response pointer.
/// - `out_len` must be a valid pointer to write the length.
/// - The returned pointer is valid until the response is freed.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_body(
    response: *const ArchimedesTestResponse,
    out_len: *mut usize,
) -> *const u8 {
    if response.is_null() || out_len.is_null() {
        if !out_len.is_null() {
            *out_len = 0;
        }
        return ptr::null();
    }

    let response = &*response;
    *out_len = response.body.len();
    if response.body.is_empty() {
        ptr::null()
    } else {
        response.body.as_ptr()
    }
}

/// Gets the response body as a null-terminated string (UTF-8).
///
/// # Safety
/// - `response` must be a valid test response pointer.
/// - Caller must free the returned string with `archimedes_string_free`.
/// - Returns NULL if the body is not valid UTF-8.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_text(
    response: *const ArchimedesTestResponse,
) -> *mut c_char {
    if response.is_null() {
        return ptr::null_mut();
    }

    let response = &*response;
    match std::str::from_utf8(&response.body) {
        Ok(s) => match CString::new(s) {
            Ok(cs) => cs.into_raw(),
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Asserts that the status code equals the expected value.
///
/// # Safety
/// - `response` must be a valid test response pointer.
/// - Returns 0 on success, non-zero on failure.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_assert_status(
    response: *const ArchimedesTestResponse,
    expected: u16,
) -> i32 {
    if response.is_null() {
        return -1;
    }
    if (*response).status_code == expected {
        0
    } else {
        1
    }
}

/// Asserts that the response is successful (2xx).
///
/// # Safety
/// - `response` must be a valid test response pointer.
/// - Returns 0 on success, non-zero on failure.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_assert_success(
    response: *const ArchimedesTestResponse,
) -> i32 {
    if response.is_null() {
        return -1;
    }
    if archimedes_test_response_is_success(response) {
        0
    } else {
        1
    }
}

/// Asserts that a header exists with the expected value.
///
/// # Safety
/// - `response` must be a valid test response pointer.
/// - `name` and `expected` must be valid null-terminated strings.
/// - Returns 0 on success, non-zero on failure.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_assert_header(
    response: *const ArchimedesTestResponse,
    name: *const c_char,
    expected: *const c_char,
) -> i32 {
    if response.is_null() || name.is_null() || expected.is_null() {
        return -1;
    }

    let actual = archimedes_test_response_get_header(response, name);
    if actual.is_null() {
        return 1; // Header not found
    }

    let expected_str = match CStr::from_ptr(expected).to_str() {
        Ok(s) => s,
        Err(_) => {
            archimedes_string_free(actual);
            return -1;
        }
    };
    let actual_str = match CStr::from_ptr(actual).to_str() {
        Ok(s) => s,
        Err(_) => {
            archimedes_string_free(actual);
            return -1;
        }
    };

    let result = if actual_str == expected_str { 0 } else { 1 };
    archimedes_string_free(actual);
    result
}

/// Asserts that the body contains the expected substring.
///
/// # Safety
/// - `response` must be a valid test response pointer.
/// - `expected` must be a valid null-terminated string.
/// - Returns 0 on success, non-zero on failure.
#[no_mangle]
pub unsafe extern "C" fn archimedes_test_response_assert_body_contains(
    response: *const ArchimedesTestResponse,
    expected: *const c_char,
) -> i32 {
    if response.is_null() || expected.is_null() {
        return -1;
    }

    let text = archimedes_test_response_text(response);
    if text.is_null() {
        return -1;
    }

    let expected_str = match CStr::from_ptr(expected).to_str() {
        Ok(s) => s,
        Err(_) => {
            archimedes_string_free(text);
            return -1;
        }
    };
    let text_str = match CStr::from_ptr(text).to_str() {
        Ok(s) => s,
        Err(_) => {
            archimedes_string_free(text);
            return -1;
        }
    };

    let result = if text_str.contains(expected_str) {
        0
    } else {
        1
    };
    archimedes_string_free(text);
    result
}

/// Frees a string allocated by the FFI layer.
///
/// # Safety
/// - `s` must be a valid pointer from an FFI function that returns `*mut c_char`.
/// - Must only be called once per string.
#[no_mangle]
pub unsafe extern "C" fn archimedes_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_client_create_free() {
        unsafe {
            let client = archimedes_test_client_new(ptr::null());
            assert!(!client.is_null());
            archimedes_test_client_free(client);
        }
    }

    #[test]
    fn test_test_client_with_base_url() {
        unsafe {
            let base_url = CString::new("http://localhost:8080").unwrap();
            let client = archimedes_test_client_new(base_url.as_ptr());
            assert!(!client.is_null());
            assert_eq!((*client).base_url, "http://localhost:8080");
            archimedes_test_client_free(client);
        }
    }

    #[test]
    fn test_test_client_with_header() {
        unsafe {
            let client = archimedes_test_client_new(ptr::null());
            let name = CString::new("X-Api-Key").unwrap();
            let value = CString::new("secret123").unwrap();
            archimedes_test_client_with_header(client, name.as_ptr(), value.as_ptr());
            assert_eq!(
                (*client).default_headers.get("X-Api-Key"),
                Some(&"secret123".to_string())
            );
            archimedes_test_client_free(client);
        }
    }

    #[test]
    fn test_test_client_with_bearer_token() {
        unsafe {
            let client = archimedes_test_client_new(ptr::null());
            let token = CString::new("mytoken").unwrap();
            archimedes_test_client_with_bearer_token(client, token.as_ptr());
            assert_eq!(
                (*client).default_headers.get("Authorization"),
                Some(&"Bearer mytoken".to_string())
            );
            archimedes_test_client_free(client);
        }
    }

    #[test]
    fn test_test_client_get() {
        unsafe {
            let client = archimedes_test_client_new(ptr::null());
            let path = CString::new("/users/123").unwrap();
            let response = archimedes_test_client_get(client, path.as_ptr());
            assert!(!response.is_null());
            assert_eq!(archimedes_test_response_status_code(response), 200);
            assert!(archimedes_test_response_is_success(response));
            archimedes_test_response_free(response);
            archimedes_test_client_free(client);
        }
    }

    #[test]
    fn test_test_response_status_checks() {
        unsafe {
            // Success
            let client = archimedes_test_client_new(ptr::null());
            let path = CString::new("/test").unwrap();
            let response = archimedes_test_client_get(client, path.as_ptr());
            assert!(archimedes_test_response_is_success(response));
            assert!(!archimedes_test_response_is_client_error(response));
            assert!(!archimedes_test_response_is_server_error(response));
            archimedes_test_response_free(response);
            archimedes_test_client_free(client);
        }
    }

    #[test]
    fn test_test_response_assert_status() {
        unsafe {
            let client = archimedes_test_client_new(ptr::null());
            let path = CString::new("/test").unwrap();
            let response = archimedes_test_client_get(client, path.as_ptr());
            assert_eq!(archimedes_test_response_assert_status(response, 200), 0);
            assert_ne!(archimedes_test_response_assert_status(response, 404), 0);
            archimedes_test_response_free(response);
            archimedes_test_client_free(client);
        }
    }

    #[test]
    fn test_test_response_assert_success() {
        unsafe {
            let client = archimedes_test_client_new(ptr::null());
            let path = CString::new("/test").unwrap();
            let response = archimedes_test_client_get(client, path.as_ptr());
            assert_eq!(archimedes_test_response_assert_success(response), 0);
            archimedes_test_response_free(response);
            archimedes_test_client_free(client);
        }
    }
}
