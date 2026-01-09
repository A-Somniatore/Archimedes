//! Extractor functions for FFI
//!
//! Provides helper functions for parsing request data in C/C++ handlers:
//! - Form data (URL-encoded)
//! - Cookies
//! - Multipart form data
//! - File responses
//!
//! ## Memory Management
//!
//! All returned strings and arrays must be freed by the caller using the
//! appropriate `archimedes_*_free` function.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

// ============================================================================
// Form Data
// ============================================================================

/// Parsed form data containing key-value pairs
#[repr(C)]
pub struct ArchimedesForm {
    /// Number of form fields
    pub count: usize,
    /// Field names (array of C strings)
    pub names: *mut *mut c_char,
    /// Field values (array of C strings)
    pub values: *mut *mut c_char,
}

impl Default for ArchimedesForm {
    fn default() -> Self {
        Self {
            count: 0,
            names: ptr::null_mut(),
            values: ptr::null_mut(),
        }
    }
}

/// Parse URL-encoded form data from request body
///
/// # Safety
///
/// - `body` must be a valid pointer to `body_len` bytes, or null if `body_len` is 0
/// - Returned form must be freed with `archimedes_form_free`
///
/// # Example (C)
///
/// ```c
/// archimedes_form form = archimedes_form_parse(body, body_len);
/// for (size_t i = 0; i < form.count; i++) {
///     printf("%s = %s\n", form.names[i], form.values[i]);
/// }
/// archimedes_form_free(&form);
/// ```
#[no_mangle]
pub unsafe extern "C" fn archimedes_form_parse(body: *const u8, body_len: usize) -> ArchimedesForm {
    if body.is_null() || body_len == 0 {
        return ArchimedesForm::default();
    }

    let body_slice = std::slice::from_raw_parts(body, body_len);
    let body_str = match std::str::from_utf8(body_slice) {
        Ok(s) => s,
        Err(_) => return ArchimedesForm::default(),
    };

    let parsed: HashMap<String, String> = match serde_urlencoded::from_str(body_str) {
        Ok(p) => p,
        Err(_) => return ArchimedesForm::default(),
    };

    if parsed.is_empty() {
        return ArchimedesForm::default();
    }

    let count = parsed.len();
    let mut names: Vec<*mut c_char> = Vec::with_capacity(count);
    let mut values: Vec<*mut c_char> = Vec::with_capacity(count);

    for (key, value) in parsed {
        let key_cstr = CString::new(key).unwrap_or_else(|_| CString::new("").unwrap());
        let value_cstr = CString::new(value).unwrap_or_else(|_| CString::new("").unwrap());
        names.push(key_cstr.into_raw());
        values.push(value_cstr.into_raw());
    }

    let names_ptr = names.as_mut_ptr();
    let values_ptr = values.as_mut_ptr();
    std::mem::forget(names);
    std::mem::forget(values);

    ArchimedesForm {
        count,
        names: names_ptr,
        values: values_ptr,
    }
}

/// Get a form field value by name
///
/// # Safety
///
/// - `form` must be a valid pointer to a form parsed with `archimedes_form_parse`
/// - `name` must be a valid null-terminated C string
/// - Returns null if field not found
/// - Returned string is borrowed from form, do NOT free it separately
#[no_mangle]
pub unsafe extern "C" fn archimedes_form_get(
    form: *const ArchimedesForm,
    name: *const c_char,
) -> *const c_char {
    if form.is_null() || name.is_null() {
        return ptr::null();
    }

    let form = &*form;
    let target = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null(),
    };

    if form.names.is_null() || form.values.is_null() || form.count == 0 {
        return ptr::null();
    }

    let names = std::slice::from_raw_parts(form.names, form.count);
    let values = std::slice::from_raw_parts(form.values, form.count);

    for i in 0..form.count {
        if !names[i].is_null() {
            if let Ok(key) = CStr::from_ptr(names[i]).to_str() {
                if key == target {
                    return values[i];
                }
            }
        }
    }

    ptr::null()
}

/// Free form data allocated by `archimedes_form_parse`
///
/// # Safety
///
/// - `form` must be a valid pointer to a form from `archimedes_form_parse`
/// - Do not use the form after calling this function
#[no_mangle]
pub unsafe extern "C" fn archimedes_form_free(form: *mut ArchimedesForm) {
    if form.is_null() {
        return;
    }

    let form = &mut *form;

    if !form.names.is_null() && !form.values.is_null() && form.count > 0 {
        let names = Vec::from_raw_parts(form.names, form.count, form.count);
        let values = Vec::from_raw_parts(form.values, form.count, form.count);

        for name in names {
            if !name.is_null() {
                drop(CString::from_raw(name));
            }
        }
        for value in values {
            if !value.is_null() {
                drop(CString::from_raw(value));
            }
        }
    }

    form.count = 0;
    form.names = ptr::null_mut();
    form.values = ptr::null_mut();
}

// ============================================================================
// Cookies
// ============================================================================

/// Parsed cookies from Cookie header
#[repr(C)]
pub struct ArchimedesCookies {
    /// Number of cookies
    pub count: usize,
    /// Cookie names (array of C strings)
    pub names: *mut *mut c_char,
    /// Cookie values (array of C strings)
    pub values: *mut *mut c_char,
}

impl Default for ArchimedesCookies {
    fn default() -> Self {
        Self {
            count: 0,
            names: ptr::null_mut(),
            values: ptr::null_mut(),
        }
    }
}

/// Parse cookies from Cookie header value
///
/// # Safety
///
/// - `cookie_header` must be a valid null-terminated C string
/// - Returned cookies must be freed with `archimedes_cookies_free`
///
/// # Example (C)
///
/// ```c
/// const char* cookie_header = archimedes_get_header(ctx, "Cookie");
/// if (cookie_header) {
///     archimedes_cookies cookies = archimedes_cookies_parse(cookie_header);
///     const char* session = archimedes_cookies_get(&cookies, "session");
///     archimedes_cookies_free(&cookies);
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn archimedes_cookies_parse(
    cookie_header: *const c_char,
) -> ArchimedesCookies {
    if cookie_header.is_null() {
        return ArchimedesCookies::default();
    }

    let header_str = match CStr::from_ptr(cookie_header).to_str() {
        Ok(s) => s,
        Err(_) => return ArchimedesCookies::default(),
    };

    let mut cookies = HashMap::new();

    for pair in header_str.split(';') {
        let pair = pair.trim();
        if let Some(eq_pos) = pair.find('=') {
            let (name, value) = pair.split_at(eq_pos);
            let value = &value[1..]; // Skip '='
            cookies.insert(name.trim().to_string(), value.trim().to_string());
        }
    }

    if cookies.is_empty() {
        return ArchimedesCookies::default();
    }

    let count = cookies.len();
    let mut names: Vec<*mut c_char> = Vec::with_capacity(count);
    let mut values: Vec<*mut c_char> = Vec::with_capacity(count);

    for (key, value) in cookies {
        let key_cstr = CString::new(key).unwrap_or_else(|_| CString::new("").unwrap());
        let value_cstr = CString::new(value).unwrap_or_else(|_| CString::new("").unwrap());
        names.push(key_cstr.into_raw());
        values.push(value_cstr.into_raw());
    }

    let names_ptr = names.as_mut_ptr();
    let values_ptr = values.as_mut_ptr();
    std::mem::forget(names);
    std::mem::forget(values);

    ArchimedesCookies {
        count,
        names: names_ptr,
        values: values_ptr,
    }
}

/// Get a cookie value by name
///
/// # Safety
///
/// - `cookies` must be a valid pointer from `archimedes_cookies_parse`
/// - `name` must be a valid null-terminated C string
/// - Returns null if cookie not found
/// - Returned string is borrowed, do NOT free it separately
#[no_mangle]
pub unsafe extern "C" fn archimedes_cookies_get(
    cookies: *const ArchimedesCookies,
    name: *const c_char,
) -> *const c_char {
    if cookies.is_null() || name.is_null() {
        return ptr::null();
    }

    let cookies = &*cookies;
    let target = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null(),
    };

    if cookies.names.is_null() || cookies.values.is_null() || cookies.count == 0 {
        return ptr::null();
    }

    let names = std::slice::from_raw_parts(cookies.names, cookies.count);
    let values = std::slice::from_raw_parts(cookies.values, cookies.count);

    for i in 0..cookies.count {
        if !names[i].is_null() {
            if let Ok(key) = CStr::from_ptr(names[i]).to_str() {
                if key == target {
                    return values[i];
                }
            }
        }
    }

    ptr::null()
}

/// Free cookies allocated by `archimedes_cookies_parse`
///
/// # Safety
///
/// - `cookies` must be a valid pointer from `archimedes_cookies_parse`
/// - Do not use the cookies after calling this function
#[no_mangle]
pub unsafe extern "C" fn archimedes_cookies_free(cookies: *mut ArchimedesCookies) {
    if cookies.is_null() {
        return;
    }

    let cookies = &mut *cookies;

    if !cookies.names.is_null() && !cookies.values.is_null() && cookies.count > 0 {
        let names = Vec::from_raw_parts(cookies.names, cookies.count, cookies.count);
        let values = Vec::from_raw_parts(cookies.values, cookies.count, cookies.count);

        for name in names {
            if !name.is_null() {
                drop(CString::from_raw(name));
            }
        }
        for value in values {
            if !value.is_null() {
                drop(CString::from_raw(value));
            }
        }
    }

    cookies.count = 0;
    cookies.names = ptr::null_mut();
    cookies.values = ptr::null_mut();
}

// ============================================================================
// Set-Cookie Builder
// ============================================================================

/// SameSite cookie attribute
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchimedesSameSite {
    /// None - cookie sent with all requests
    None = 0,
    /// Lax - cookie sent with top-level navigations
    Lax = 1,
    /// Strict - cookie only sent with same-site requests
    Strict = 2,
}

/// Set-Cookie header builder
#[repr(C)]
pub struct ArchimedesSetCookie {
    name: *mut c_char,
    value: *mut c_char,
    path: *mut c_char,
    domain: *mut c_char,
    expires: *mut c_char,
    max_age: i64,
    secure: bool,
    http_only: bool,
    same_site: ArchimedesSameSite,
    has_max_age: bool,
}

impl Default for ArchimedesSetCookie {
    fn default() -> Self {
        Self {
            name: ptr::null_mut(),
            value: ptr::null_mut(),
            path: ptr::null_mut(),
            domain: ptr::null_mut(),
            expires: ptr::null_mut(),
            max_age: 0,
            secure: false,
            http_only: false,
            same_site: ArchimedesSameSite::Lax,
            has_max_age: false,
        }
    }
}

/// Create a new Set-Cookie builder
///
/// # Safety
///
/// - `name` and `value` must be valid null-terminated C strings
/// - Returned builder must be freed with `archimedes_set_cookie_free`
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_new(
    name: *const c_char,
    value: *const c_char,
) -> *mut ArchimedesSetCookie {
    if name.is_null() || value.is_null() {
        return ptr::null_mut();
    }

    let name_str = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };
    let value_str = match CStr::from_ptr(value).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    let name_cstr = CString::new(name_str).unwrap_or_else(|_| CString::new("").unwrap());
    let value_cstr = CString::new(value_str).unwrap_or_else(|_| CString::new("").unwrap());

    let cookie = Box::new(ArchimedesSetCookie {
        name: name_cstr.into_raw(),
        value: value_cstr.into_raw(),
        ..Default::default()
    });

    Box::into_raw(cookie)
}

/// Set the Path attribute
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_path(
    cookie: *mut ArchimedesSetCookie,
    path: *const c_char,
) {
    if cookie.is_null() || path.is_null() {
        return;
    }

    let cookie = &mut *cookie;
    if !cookie.path.is_null() {
        drop(CString::from_raw(cookie.path));
    }

    let path_str = CStr::from_ptr(path).to_str().unwrap_or("/");
    cookie.path = CString::new(path_str)
        .unwrap_or_else(|_| CString::new("/").unwrap())
        .into_raw();
}

/// Set the Domain attribute
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_domain(
    cookie: *mut ArchimedesSetCookie,
    domain: *const c_char,
) {
    if cookie.is_null() || domain.is_null() {
        return;
    }

    let cookie = &mut *cookie;
    if !cookie.domain.is_null() {
        drop(CString::from_raw(cookie.domain));
    }

    let domain_str = CStr::from_ptr(domain).to_str().unwrap_or("");
    cookie.domain = CString::new(domain_str)
        .unwrap_or_else(|_| CString::new("").unwrap())
        .into_raw();
}

/// Set the Expires attribute (RFC 7231 date format)
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_expires(
    cookie: *mut ArchimedesSetCookie,
    expires: *const c_char,
) {
    if cookie.is_null() || expires.is_null() {
        return;
    }

    let cookie = &mut *cookie;
    if !cookie.expires.is_null() {
        drop(CString::from_raw(cookie.expires));
    }

    let expires_str = CStr::from_ptr(expires).to_str().unwrap_or("");
    cookie.expires = CString::new(expires_str)
        .unwrap_or_else(|_| CString::new("").unwrap())
        .into_raw();
}

/// Set the Max-Age attribute (in seconds)
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_max_age(
    cookie: *mut ArchimedesSetCookie,
    max_age: i64,
) {
    if cookie.is_null() {
        return;
    }

    let cookie = &mut *cookie;
    cookie.max_age = max_age;
    cookie.has_max_age = true;
}

/// Set the Secure attribute
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_secure(
    cookie: *mut ArchimedesSetCookie,
    secure: bool,
) {
    if cookie.is_null() {
        return;
    }

    let cookie = &mut *cookie;
    cookie.secure = secure;
}

/// Set the HttpOnly attribute
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_http_only(
    cookie: *mut ArchimedesSetCookie,
    http_only: bool,
) {
    if cookie.is_null() {
        return;
    }

    let cookie = &mut *cookie;
    cookie.http_only = http_only;
}

/// Set the SameSite attribute
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_same_site(
    cookie: *mut ArchimedesSetCookie,
    same_site: ArchimedesSameSite,
) {
    if cookie.is_null() {
        return;
    }

    let cookie = &mut *cookie;
    cookie.same_site = same_site;
}

/// Build the Set-Cookie header value
///
/// # Safety
///
/// - `cookie` must be a valid pointer from `archimedes_set_cookie_new`
/// - Returned string must be freed with `archimedes_free_string`
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_build(
    cookie: *const ArchimedesSetCookie,
) -> *mut c_char {
    if cookie.is_null() {
        return ptr::null_mut();
    }

    let cookie = &*cookie;
    if cookie.name.is_null() || cookie.value.is_null() {
        return ptr::null_mut();
    }

    let name = CStr::from_ptr(cookie.name).to_str().unwrap_or("");
    let value = CStr::from_ptr(cookie.value).to_str().unwrap_or("");

    let mut result = format!("{}={}", name, value);

    if !cookie.path.is_null() {
        let path = CStr::from_ptr(cookie.path).to_str().unwrap_or("");
        if !path.is_empty() {
            result.push_str(&format!("; Path={}", path));
        }
    }

    if !cookie.domain.is_null() {
        let domain = CStr::from_ptr(cookie.domain).to_str().unwrap_or("");
        if !domain.is_empty() {
            result.push_str(&format!("; Domain={}", domain));
        }
    }

    if !cookie.expires.is_null() {
        let expires = CStr::from_ptr(cookie.expires).to_str().unwrap_or("");
        if !expires.is_empty() {
            result.push_str(&format!("; Expires={}", expires));
        }
    }

    if cookie.has_max_age {
        result.push_str(&format!("; Max-Age={}", cookie.max_age));
    }

    if cookie.secure {
        result.push_str("; Secure");
    }

    if cookie.http_only {
        result.push_str("; HttpOnly");
    }

    match cookie.same_site {
        ArchimedesSameSite::None => result.push_str("; SameSite=None"),
        ArchimedesSameSite::Lax => result.push_str("; SameSite=Lax"),
        ArchimedesSameSite::Strict => result.push_str("; SameSite=Strict"),
    }

    CString::new(result)
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// Free a Set-Cookie builder
#[no_mangle]
pub unsafe extern "C" fn archimedes_set_cookie_free(cookie: *mut ArchimedesSetCookie) {
    if cookie.is_null() {
        return;
    }

    let cookie = Box::from_raw(cookie);

    if !cookie.name.is_null() {
        drop(CString::from_raw(cookie.name));
    }
    if !cookie.value.is_null() {
        drop(CString::from_raw(cookie.value));
    }
    if !cookie.path.is_null() {
        drop(CString::from_raw(cookie.path));
    }
    if !cookie.domain.is_null() {
        drop(CString::from_raw(cookie.domain));
    }
    if !cookie.expires.is_null() {
        drop(CString::from_raw(cookie.expires));
    }
}

// ============================================================================
// Multipart
// ============================================================================

/// A field in a multipart form
#[repr(C)]
pub struct ArchimedesMultipartField {
    /// Field name
    pub name: *mut c_char,
    /// Field value (for text fields) or filename (for files)
    pub value: *mut c_char,
    /// Content-Type of the field (may be null)
    pub content_type: *mut c_char,
    /// Whether this is a file upload
    pub is_file: bool,
    /// File data (only if is_file is true)
    pub data: *mut u8,
    /// File data length
    pub data_len: usize,
}

impl Default for ArchimedesMultipartField {
    fn default() -> Self {
        Self {
            name: ptr::null_mut(),
            value: ptr::null_mut(),
            content_type: ptr::null_mut(),
            is_file: false,
            data: ptr::null_mut(),
            data_len: 0,
        }
    }
}

/// Parsed multipart form data
#[repr(C)]
pub struct ArchimedesMultipart {
    /// Number of fields
    pub count: usize,
    /// Array of fields
    pub fields: *mut ArchimedesMultipartField,
}

impl Default for ArchimedesMultipart {
    fn default() -> Self {
        Self {
            count: 0,
            fields: ptr::null_mut(),
        }
    }
}

/// Parse multipart form data
///
/// # Safety
///
/// - `body` must be a valid pointer to `body_len` bytes
/// - `boundary` must be a valid null-terminated C string (the multipart boundary)
/// - Returned multipart must be freed with `archimedes_multipart_free`
///
/// # Note
///
/// The boundary can be extracted from the Content-Type header:
/// `Content-Type: multipart/form-data; boundary=----WebKitFormBoundary...`
#[no_mangle]
pub unsafe extern "C" fn archimedes_multipart_parse(
    body: *const u8,
    body_len: usize,
    boundary: *const c_char,
) -> ArchimedesMultipart {
    if body.is_null() || body_len == 0 || boundary.is_null() {
        return ArchimedesMultipart::default();
    }

    let body_slice = std::slice::from_raw_parts(body, body_len);
    let boundary_str = match CStr::from_ptr(boundary).to_str() {
        Ok(s) => s,
        Err(_) => return ArchimedesMultipart::default(),
    };

    // Simple multipart parser
    let delimiter = format!("--{}", boundary_str);
    let body_str = String::from_utf8_lossy(body_slice);

    let mut fields_vec: Vec<ArchimedesMultipartField> = Vec::new();

    for part in body_str.split(&delimiter) {
        let part = part.trim();
        if part.is_empty() || part == "--" {
            continue;
        }

        // Split headers and content
        if let Some(header_end) = part.find("\r\n\r\n") {
            let headers = &part[..header_end];
            let content = &part[header_end + 4..];

            // Parse Content-Disposition
            let mut name: Option<String> = None;
            let mut filename: Option<String> = None;
            let mut content_type: Option<String> = None;

            for line in headers.lines() {
                if line.to_lowercase().starts_with("content-disposition:") {
                    // Parse name and filename
                    if let Some(n) = extract_header_param(line, "name") {
                        name = Some(n);
                    }
                    if let Some(f) = extract_header_param(line, "filename") {
                        filename = Some(f);
                    }
                } else if line.to_lowercase().starts_with("content-type:") {
                    content_type = Some(line[13..].trim().to_string());
                }
            }

            if let Some(field_name) = name {
                let mut field = ArchimedesMultipartField::default();

                field.name = CString::new(field_name)
                    .map(|s| s.into_raw())
                    .unwrap_or(ptr::null_mut());

                if let Some(fname) = filename {
                    // File upload
                    field.is_file = true;
                    field.value = CString::new(fname)
                        .map(|s| s.into_raw())
                        .unwrap_or(ptr::null_mut());

                    // Strip trailing boundary markers
                    let content = content.trim_end_matches("\r\n");
                    let data = content.as_bytes().to_vec();
                    field.data_len = data.len();
                    let mut data = data.into_boxed_slice();
                    field.data = data.as_mut_ptr();
                    std::mem::forget(data);
                } else {
                    // Text field
                    field.is_file = false;
                    let value = content.trim_end_matches("\r\n");
                    field.value = CString::new(value)
                        .map(|s| s.into_raw())
                        .unwrap_or(ptr::null_mut());
                }

                if let Some(ct) = content_type {
                    field.content_type = CString::new(ct)
                        .map(|s| s.into_raw())
                        .unwrap_or(ptr::null_mut());
                }

                fields_vec.push(field);
            }
        }
    }

    if fields_vec.is_empty() {
        return ArchimedesMultipart::default();
    }

    let count = fields_vec.len();
    let fields_ptr = fields_vec.as_mut_ptr();
    std::mem::forget(fields_vec);

    ArchimedesMultipart {
        count,
        fields: fields_ptr,
    }
}

/// Helper to extract a parameter from a header line
fn extract_header_param(line: &str, param: &str) -> Option<String> {
    let search = format!("{}=\"", param);
    if let Some(start) = line.find(&search) {
        let rest = &line[start + search.len()..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }

    // Try without quotes
    let search = format!("{}=", param);
    if let Some(start) = line.find(&search) {
        let rest = &line[start + search.len()..];
        let end = rest.find(';').unwrap_or(rest.len());
        let end = rest[..end].find(' ').unwrap_or(end);
        return Some(rest[..end].trim().to_string());
    }

    None
}

/// Get a multipart field by name
///
/// # Safety
///
/// - `multipart` must be a valid pointer from `archimedes_multipart_parse`
/// - `name` must be a valid null-terminated C string
/// - Returns null if field not found
/// - Returned pointer is borrowed, do NOT free it separately
#[no_mangle]
pub unsafe extern "C" fn archimedes_multipart_get(
    multipart: *const ArchimedesMultipart,
    name: *const c_char,
) -> *const ArchimedesMultipartField {
    if multipart.is_null() || name.is_null() {
        return ptr::null();
    }

    let multipart = &*multipart;
    let target = match CStr::from_ptr(name).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null(),
    };

    if multipart.fields.is_null() || multipart.count == 0 {
        return ptr::null();
    }

    let fields = std::slice::from_raw_parts(multipart.fields, multipart.count);

    for field in fields {
        if !field.name.is_null() {
            if let Ok(field_name) = CStr::from_ptr(field.name).to_str() {
                if field_name == target {
                    return field;
                }
            }
        }
    }

    ptr::null()
}

/// Free multipart data
///
/// # Safety
///
/// - `multipart` must be a valid pointer from `archimedes_multipart_parse`
/// - Do not use the multipart after calling this function
#[no_mangle]
pub unsafe extern "C" fn archimedes_multipart_free(multipart: *mut ArchimedesMultipart) {
    if multipart.is_null() {
        return;
    }

    let multipart = &mut *multipart;

    if !multipart.fields.is_null() && multipart.count > 0 {
        let fields = Vec::from_raw_parts(multipart.fields, multipart.count, multipart.count);

        for field in fields {
            if !field.name.is_null() {
                drop(CString::from_raw(field.name));
            }
            if !field.value.is_null() {
                drop(CString::from_raw(field.value));
            }
            if !field.content_type.is_null() {
                drop(CString::from_raw(field.content_type));
            }
            if !field.data.is_null() && field.data_len > 0 {
                drop(Vec::from_raw_parts(field.data, field.data_len, field.data_len));
            }
        }
    }

    multipart.count = 0;
    multipart.fields = ptr::null_mut();
}

// ============================================================================
// File Response Helpers
// ============================================================================

/// Create a file response
///
/// # Safety
///
/// - `data` must be a valid pointer to `data_len` bytes
/// - `filename` must be a valid null-terminated C string
/// - `content_type` can be null (auto-detected from filename)
/// - Returns response data that should be used directly as handler return value
#[no_mangle]
pub unsafe extern "C" fn archimedes_file_response(
    data: *const u8,
    data_len: usize,
    filename: *const c_char,
    content_type: *const c_char,
    inline_disposition: bool,
) -> crate::types::ArchimedesResponseData {
    use crate::types::ArchimedesResponseData;

    if data.is_null() || data_len == 0 || filename.is_null() {
        return ArchimedesResponseData {
            status_code: 500,
            ..Default::default()
        };
    }

    let filename_str = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => {
            return ArchimedesResponseData {
                status_code: 500,
                ..Default::default()
            }
        }
    };

    // Copy data
    let mut body_data = Vec::with_capacity(data_len);
    body_data.extend_from_slice(std::slice::from_raw_parts(data, data_len));
    let body_ptr = body_data.as_ptr() as *const c_char;
    let body_len = body_data.len();
    std::mem::forget(body_data);

    // Determine content type
    let mime_type = if content_type.is_null() {
        guess_mime_type(filename_str)
    } else {
        CStr::from_ptr(content_type)
            .to_str()
            .unwrap_or("application/octet-stream")
            .to_string()
    };
    let content_type_ptr = CString::new(mime_type)
        .map(|s| s.into_raw() as *const c_char)
        .unwrap_or(ptr::null());

    // Build Content-Disposition header
    let disposition = if inline_disposition {
        format!("inline; filename=\"{}\"", filename_str)
    } else {
        format!("attachment; filename=\"{}\"", filename_str)
    };

    // Create headers array
    let header_name = CString::new("Content-Disposition")
        .unwrap()
        .into_raw() as *const c_char;
    let header_value = CString::new(disposition)
        .map(|s| s.into_raw() as *const c_char)
        .unwrap_or(ptr::null());

    let names = vec![header_name];
    let values = vec![header_value];

    let names_ptr = names.as_ptr() as *const *const c_char;
    let values_ptr = values.as_ptr() as *const *const c_char;
    std::mem::forget(names);
    std::mem::forget(values);

    ArchimedesResponseData {
        status_code: 200,
        body: body_ptr,
        body_len,
        body_owned: true,
        content_type: content_type_ptr,
        headers_count: 1,
        header_names: names_ptr,
        header_values: values_ptr,
    }
}

/// Create a redirect response
///
/// # Safety
///
/// - `location` must be a valid null-terminated C string
/// - Returns response data for a redirect
#[no_mangle]
pub unsafe extern "C" fn archimedes_redirect(
    location: *const c_char,
    status_code: i32,
) -> crate::types::ArchimedesResponseData {
    use crate::types::ArchimedesResponseData;

    if location.is_null() {
        return ArchimedesResponseData {
            status_code: 500,
            ..Default::default()
        };
    }

    // Build Location header
    let location_str = CStr::from_ptr(location).to_str().unwrap_or("");
    let header_name = CString::new("Location")
        .unwrap()
        .into_raw() as *const c_char;
    let header_value = CString::new(location_str)
        .map(|s| s.into_raw() as *const c_char)
        .unwrap_or(ptr::null());

    let names = vec![header_name];
    let values = vec![header_value];

    let names_ptr = names.as_ptr() as *const *const c_char;
    let values_ptr = values.as_ptr() as *const *const c_char;
    std::mem::forget(names);
    std::mem::forget(values);

    ArchimedesResponseData {
        status_code,
        body: ptr::null(),
        body_len: 0,
        body_owned: false,
        content_type: ptr::null(),
        headers_count: 1,
        header_names: names_ptr,
        header_values: values_ptr,
    }
}

/// Convenience: Create 302 Found redirect
#[no_mangle]
pub unsafe extern "C" fn archimedes_redirect_found(
    location: *const c_char,
) -> crate::types::ArchimedesResponseData {
    archimedes_redirect(location, 302)
}

/// Convenience: Create 301 Permanent redirect
#[no_mangle]
pub unsafe extern "C" fn archimedes_redirect_permanent(
    location: *const c_char,
) -> crate::types::ArchimedesResponseData {
    archimedes_redirect(location, 301)
}

/// Convenience: Create 303 See Other redirect
#[no_mangle]
pub unsafe extern "C" fn archimedes_redirect_see_other(
    location: *const c_char,
) -> crate::types::ArchimedesResponseData {
    archimedes_redirect(location, 303)
}

/// Convenience: Create 307 Temporary redirect
#[no_mangle]
pub unsafe extern "C" fn archimedes_redirect_temporary(
    location: *const c_char,
) -> crate::types::ArchimedesResponseData {
    archimedes_redirect(location, 307)
}

/// Guess MIME type from filename extension
fn guess_mime_type(filename: &str) -> String {
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        // Text
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "md" => "text/markdown",
        "yaml" | "yml" => "application/yaml",

        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",

        // Audio/Video
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",

        // Documents
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",

        // Archives
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" | "gzip" => "application/gzip",
        "rar" => "application/vnd.rar",
        "7z" => "application/x-7z-compressed",

        // Fonts
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",

        // Other
        "wasm" => "application/wasm",
        "bin" | _ => "application/octet-stream",
    }
    .to_string()
}

// ============================================================================
// Header Extraction Helper
// ============================================================================

/// Get a header value from request context
///
/// # Safety
///
/// - `ctx` must be a valid pointer to ArchimedesRequestContext
/// - `name` must be a valid null-terminated C string (case-insensitive)
/// - Returns null if header not found
/// - Returned string is borrowed from context, do NOT free it
#[no_mangle]
pub unsafe extern "C" fn archimedes_get_header(
    ctx: *const crate::types::ArchimedesRequestContext,
    name: *const c_char,
) -> *const c_char {
    if ctx.is_null() || name.is_null() {
        return ptr::null();
    }

    let ctx = &*ctx;
    let target = match CStr::from_ptr(name).to_str() {
        Ok(s) => s.to_lowercase(),
        Err(_) => return ptr::null(),
    };

    if ctx.header_names.is_null() || ctx.header_values.is_null() || ctx.headers_count == 0 {
        return ptr::null();
    }

    let names = std::slice::from_raw_parts(ctx.header_names, ctx.headers_count);
    let values = std::slice::from_raw_parts(ctx.header_values, ctx.headers_count);

    for i in 0..ctx.headers_count {
        if !names[i].is_null() {
            if let Ok(header_name) = CStr::from_ptr(names[i]).to_str() {
                if header_name.to_lowercase() == target {
                    return values[i];
                }
            }
        }
    }

    ptr::null()
}

/// Get the multipart boundary from Content-Type header
///
/// # Safety
///
/// - `content_type` must be a valid null-terminated C string
/// - Returns null if not a multipart content type or no boundary found
/// - Returned string must be freed with `archimedes_free_string`
#[no_mangle]
pub unsafe extern "C" fn archimedes_get_multipart_boundary(
    content_type: *const c_char,
) -> *mut c_char {
    if content_type.is_null() {
        return ptr::null_mut();
    }

    let ct = match CStr::from_ptr(content_type).to_str() {
        Ok(s) => s,
        Err(_) => return ptr::null_mut(),
    };

    if !ct.to_lowercase().contains("multipart/form-data") {
        return ptr::null_mut();
    }

    if let Some(boundary) = extract_header_param(ct, "boundary") {
        CString::new(boundary)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    } else {
        ptr::null_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_form_parse() {
        let body = b"name=John&age=30&city=NYC";
        unsafe {
            let form = archimedes_form_parse(body.as_ptr(), body.len());
            assert_eq!(form.count, 3);

            let name_key = CString::new("name").unwrap();
            let value = archimedes_form_get(&form, name_key.as_ptr());
            assert!(!value.is_null());
            assert_eq!(CStr::from_ptr(value).to_str().unwrap(), "John");

            let age_key = CString::new("age").unwrap();
            let value = archimedes_form_get(&form, age_key.as_ptr());
            assert!(!value.is_null());
            assert_eq!(CStr::from_ptr(value).to_str().unwrap(), "30");

            let mut form = form;
            archimedes_form_free(&mut form);
        }
    }

    #[test]
    fn test_form_parse_empty() {
        unsafe {
            let form = archimedes_form_parse(ptr::null(), 0);
            assert_eq!(form.count, 0);
            assert!(form.names.is_null());
        }
    }

    #[test]
    fn test_cookies_parse() {
        let header = CString::new("session=abc123; user=john; theme=dark").unwrap();
        unsafe {
            let cookies = archimedes_cookies_parse(header.as_ptr());
            assert_eq!(cookies.count, 3);

            let session_key = CString::new("session").unwrap();
            let value = archimedes_cookies_get(&cookies, session_key.as_ptr());
            assert!(!value.is_null());
            assert_eq!(CStr::from_ptr(value).to_str().unwrap(), "abc123");

            let user_key = CString::new("user").unwrap();
            let value = archimedes_cookies_get(&cookies, user_key.as_ptr());
            assert!(!value.is_null());
            assert_eq!(CStr::from_ptr(value).to_str().unwrap(), "john");

            let mut cookies = cookies;
            archimedes_cookies_free(&mut cookies);
        }
    }

    #[test]
    fn test_set_cookie_builder() {
        unsafe {
            let name = CString::new("session").unwrap();
            let value = CString::new("xyz789").unwrap();

            let cookie = archimedes_set_cookie_new(name.as_ptr(), value.as_ptr());
            assert!(!cookie.is_null());

            let path = CString::new("/api").unwrap();
            archimedes_set_cookie_path(cookie, path.as_ptr());
            archimedes_set_cookie_secure(cookie, true);
            archimedes_set_cookie_http_only(cookie, true);
            archimedes_set_cookie_max_age(cookie, 3600);
            archimedes_set_cookie_same_site(cookie, ArchimedesSameSite::Strict);

            let header = archimedes_set_cookie_build(cookie);
            assert!(!header.is_null());

            let header_str = CStr::from_ptr(header).to_str().unwrap();
            assert!(header_str.contains("session=xyz789"));
            assert!(header_str.contains("Path=/api"));
            assert!(header_str.contains("Secure"));
            assert!(header_str.contains("HttpOnly"));
            assert!(header_str.contains("Max-Age=3600"));
            assert!(header_str.contains("SameSite=Strict"));

            // Free
            drop(CString::from_raw(header));
            archimedes_set_cookie_free(cookie);
        }
    }

    #[test]
    fn test_mime_type_guessing() {
        assert_eq!(guess_mime_type("file.html"), "text/html");
        assert_eq!(guess_mime_type("style.css"), "text/css");
        assert_eq!(guess_mime_type("app.js"), "text/javascript");
        assert_eq!(guess_mime_type("data.json"), "application/json");
        assert_eq!(guess_mime_type("image.png"), "image/png");
        assert_eq!(guess_mime_type("photo.jpg"), "image/jpeg");
        assert_eq!(guess_mime_type("doc.pdf"), "application/pdf");
        assert_eq!(guess_mime_type("unknown.xyz"), "application/octet-stream");
    }

    #[test]
    fn test_redirect_responses() {
        let location = CString::new("https://example.com/new").unwrap();

        unsafe {
            let response = archimedes_redirect_found(location.as_ptr());
            assert_eq!(response.status_code, 302);
            assert_eq!(response.headers_count, 1);

            let response = archimedes_redirect_permanent(location.as_ptr());
            assert_eq!(response.status_code, 301);

            let response = archimedes_redirect_see_other(location.as_ptr());
            assert_eq!(response.status_code, 303);

            let response = archimedes_redirect_temporary(location.as_ptr());
            assert_eq!(response.status_code, 307);
        }
    }

    #[test]
    fn test_extract_header_param() {
        let line = r#"Content-Disposition: form-data; name="file"; filename="test.txt""#;
        assert_eq!(extract_header_param(line, "name"), Some("file".to_string()));
        assert_eq!(
            extract_header_param(line, "filename"),
            Some("test.txt".to_string())
        );
        assert_eq!(extract_header_param(line, "missing"), None);
    }

    #[test]
    fn test_multipart_boundary_extraction() {
        let ct = CString::new("multipart/form-data; boundary=----WebKitFormBoundary7MA").unwrap();
        unsafe {
            let boundary = archimedes_get_multipart_boundary(ct.as_ptr());
            assert!(!boundary.is_null());
            assert_eq!(
                CStr::from_ptr(boundary).to_str().unwrap(),
                "----WebKitFormBoundary7MA"
            );
            drop(CString::from_raw(boundary));
        }
    }
}
