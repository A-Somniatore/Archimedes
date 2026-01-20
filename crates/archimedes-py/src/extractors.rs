//! Python extractors for Archimedes
//!
//! This module provides Form, Cookies, and Multipart extractors for Python.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use std::collections::HashMap;

/// URL-encoded form data extractor
///
/// # Example (Python)
///
/// ```python,ignore
/// from archimedes import Form
///
/// @app.handler("login")
/// def login(form: Form):
///     username = form.get("username")
///     password = form.get("password")
///     return {"authenticated": True}
/// ```
#[pyclass(name = "Form")]
#[derive(Clone, Debug)]
pub struct PyForm {
    fields: HashMap<String, String>,
}

#[pymethods]
impl PyForm {
    /// Create a new Form from a dictionary
    #[new]
    #[pyo3(signature = (data = None))]
    fn new(data: Option<HashMap<String, String>>) -> Self {
        Self {
            fields: data.unwrap_or_default(),
        }
    }

    /// Parse form data from a URL-encoded string
    #[staticmethod]
    fn parse(data: &str) -> PyResult<Self> {
        let fields = serde_urlencoded::from_str(data).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid form data: {e}"))
        })?;
        Ok(Self { fields })
    }

    /// Get a field value by name
    fn get(&self, name: &str) -> Option<String> {
        self.fields.get(name).cloned()
    }

    /// Get a field value or return a default
    fn get_or(&self, name: &str, default: &str) -> String {
        self.fields.get(name).cloned().unwrap_or_else(|| default.to_string())
    }

    /// Get a required field, raising KeyError if not present
    fn require(&self, name: &str) -> PyResult<String> {
        self.fields.get(name).cloned().ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!("Missing required field: {name}"))
        })
    }

    /// Check if a field exists
    fn contains(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }

    /// Get all field names
    fn keys(&self) -> Vec<String> {
        self.fields.keys().cloned().collect()
    }

    /// Get all field values
    fn values(&self) -> Vec<String> {
        self.fields.values().cloned().collect()
    }

    /// Get all fields as items
    fn items(&self) -> Vec<(String, String)> {
        self.fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Get number of fields
    fn __len__(&self) -> usize {
        self.fields.len()
    }

    /// Check if field exists (for `in` operator)
    fn __contains__(&self, name: &str) -> bool {
        self.fields.contains_key(name)
    }

    /// Get field by subscript
    fn __getitem__(&self, name: &str) -> PyResult<String> {
        self.require(name)
    }

    /// Convert to dictionary
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.fields {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("Form({} fields)", self.fields.len())
    }
}

impl PyForm {
    /// Create from internal map
    pub fn from_map(fields: HashMap<String, String>) -> Self {
        Self { fields }
    }
}

/// Cookie extractor for request cookies
///
/// # Example (Python)
///
/// ```python,ignore
/// from archimedes import Cookies
///
/// @app.handler("profile")
/// def profile(cookies: Cookies):
///     session_id = cookies.get("session_id")
///     theme = cookies.get_or("theme", "light")
///     return {"theme": theme}
/// ```
#[pyclass(name = "Cookies")]
#[derive(Clone, Debug, Default)]
pub struct PyCookies {
    cookies: HashMap<String, String>,
}

#[pymethods]
impl PyCookies {
    /// Create a new Cookies instance
    #[new]
    #[pyo3(signature = (data = None))]
    fn new(data: Option<HashMap<String, String>>) -> Self {
        Self {
            cookies: data.unwrap_or_default(),
        }
    }

    /// Parse cookies from a Cookie header value
    #[staticmethod]
    fn parse(header_value: &str) -> Self {
        let mut cookies = HashMap::new();
        for cookie in header_value.split(';') {
            let cookie = cookie.trim();
            if let Some((name, value)) = cookie.split_once('=') {
                let name = name.trim();
                let value = value.trim().trim_matches('"');
                cookies.insert(name.to_string(), value.to_string());
            }
        }
        Self { cookies }
    }

    /// Get a cookie value by name
    fn get(&self, name: &str) -> Option<String> {
        self.cookies.get(name).cloned()
    }

    /// Get a cookie value or return a default
    fn get_or(&self, name: &str, default: &str) -> String {
        self.cookies.get(name).cloned().unwrap_or_else(|| default.to_string())
    }

    /// Get a required cookie, raising KeyError if not present
    fn require(&self, name: &str) -> PyResult<String> {
        self.cookies.get(name).cloned().ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!("Missing required cookie: {name}"))
        })
    }

    /// Check if a cookie exists
    fn contains(&self, name: &str) -> bool {
        self.cookies.contains_key(name)
    }

    /// Get all cookie names
    fn names(&self) -> Vec<String> {
        self.cookies.keys().cloned().collect()
    }

    /// Get all cookie values
    fn values(&self) -> Vec<String> {
        self.cookies.values().cloned().collect()
    }

    /// Get all cookies as items
    fn items(&self) -> Vec<(String, String)> {
        self.cookies.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Get number of cookies
    fn __len__(&self) -> usize {
        self.cookies.len()
    }

    /// Check if cookie exists (for `in` operator)
    fn __contains__(&self, name: &str) -> bool {
        self.cookies.contains_key(name)
    }

    /// Get cookie by subscript
    fn __getitem__(&self, name: &str) -> PyResult<String> {
        self.require(name)
    }

    /// Convert to dictionary
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.cookies {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    fn __repr__(&self) -> String {
        format!("Cookies({} cookies)", self.cookies.len())
    }
}

impl PyCookies {
    /// Create from internal map
    pub fn from_map(cookies: HashMap<String, String>) -> Self {
        Self { cookies }
    }
}

/// SameSite cookie attribute
#[pyclass(name = "SameSite", eq, eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PySameSite {
    /// Cookie is sent with cross-site requests
    None = 0,
    /// Cookie only sent with same-site requests and top-level navigation
    Lax = 1,
    /// Cookie only sent with same-site requests
    Strict = 2,
}

#[pymethods]
impl PySameSite {
    /// String representation
    fn __repr__(&self) -> &'static str {
        match self {
            PySameSite::None => "SameSite.None",
            PySameSite::Lax => "SameSite.Lax",
            PySameSite::Strict => "SameSite.Strict",
        }
    }
}

/// Set-Cookie response helper for setting cookies
///
/// # Example (Python)
///
/// ```python,ignore
/// from archimedes import SetCookie, SameSite, Response
///
/// @app.handler("login")
/// def login(form: Form):
///     # Create a secure session cookie
///     cookie = (SetCookie("session_id", "abc123")
///         .http_only(True)
///         .secure(True)
///         .same_site(SameSite.Strict)
///         .max_age(3600))
///     
///     response = Response.ok({"authenticated": True})
///     response.set_cookie(cookie)
///     return response
/// ```
#[pyclass(name = "SetCookie")]
#[derive(Clone, Debug)]
pub struct PySetCookie {
    name: String,
    value: String,
    domain: Option<String>,
    path: Option<String>,
    max_age: Option<i64>,
    expires: Option<String>,
    secure: bool,
    http_only: bool,
    same_site: Option<PySameSite>,
}

#[pymethods]
impl PySetCookie {
    /// Create a new Set-Cookie builder
    #[new]
    fn new(name: String, value: String) -> Self {
        Self {
            name,
            value,
            domain: None,
            path: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
        }
    }

    /// Set the Domain attribute
    fn domain(mut slf: PyRefMut<'_, Self>, domain: String) -> PyRefMut<'_, Self> {
        slf.domain = Some(domain);
        slf
    }

    /// Set the Path attribute
    fn path(mut slf: PyRefMut<'_, Self>, path: String) -> PyRefMut<'_, Self> {
        slf.path = Some(path);
        slf
    }

    /// Set the Max-Age attribute (in seconds)
    fn max_age(mut slf: PyRefMut<'_, Self>, seconds: i64) -> PyRefMut<'_, Self> {
        slf.max_age = Some(seconds);
        slf
    }

    /// Set the Expires attribute (RFC 7231 date format)
    fn expires(mut slf: PyRefMut<'_, Self>, date: String) -> PyRefMut<'_, Self> {
        slf.expires = Some(date);
        slf
    }

    /// Set the Secure attribute
    fn secure(mut slf: PyRefMut<'_, Self>, secure: bool) -> PyRefMut<'_, Self> {
        slf.secure = secure;
        slf
    }

    /// Set the HttpOnly attribute
    fn http_only(mut slf: PyRefMut<'_, Self>, http_only: bool) -> PyRefMut<'_, Self> {
        slf.http_only = http_only;
        slf
    }

    /// Set the SameSite attribute
    fn same_site(mut slf: PyRefMut<'_, Self>, same_site: PySameSite) -> PyRefMut<'_, Self> {
        slf.same_site = Some(same_site);
        slf
    }

    /// Build the Set-Cookie header value
    fn build(&self) -> String {
        let mut parts = vec![format!("{}={}", self.name, self.value)];

        if let Some(ref domain) = self.domain {
            parts.push(format!("Domain={domain}"));
        }
        if let Some(ref path) = self.path {
            parts.push(format!("Path={path}"));
        }
        if let Some(max_age) = self.max_age {
            parts.push(format!("Max-Age={max_age}"));
        }
        if let Some(ref expires) = self.expires {
            parts.push(format!("Expires={expires}"));
        }
        if self.secure {
            parts.push("Secure".to_string());
        }
        if self.http_only {
            parts.push("HttpOnly".to_string());
        }
        if let Some(same_site) = self.same_site {
            let value = match same_site {
                PySameSite::None => "None",
                PySameSite::Lax => "Lax",
                PySameSite::Strict => "Strict",
            };
            parts.push(format!("SameSite={value}"));
        }

        parts.join("; ")
    }

    /// Get the cookie name
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    /// Get the cookie value
    #[getter]
    fn value(&self) -> &str {
        &self.value
    }

    fn __repr__(&self) -> String {
        format!("SetCookie({}={})", self.name, self.value)
    }
}

impl PySetCookie {
    /// Get the header name
    pub fn header_name() -> &'static str {
        "set-cookie"
    }

    /// Get the built header value
    pub fn header_value(&self) -> String {
        self.build()
    }
}

/// Uploaded file from multipart form data
///
/// # Example (Python)
///
/// ```python,ignore
/// from archimedes import Multipart
///
/// @app.handler("upload")
/// async def upload(multipart: Multipart):
///     async for field in multipart:
///         if field.is_file():
///             filename = field.filename
///             content = field.bytes()
///             # Process file...
/// ```
#[pyclass(name = "UploadedFile")]
#[derive(Clone, Debug)]
pub struct PyUploadedFile {
    filename: Option<String>,
    content_type: Option<String>,
    data: Vec<u8>,
}

#[pymethods]
impl PyUploadedFile {
    /// Create a new uploaded file
    #[new]
    #[pyo3(signature = (data, filename = None, content_type = None))]
    fn new(data: Vec<u8>, filename: Option<String>, content_type: Option<String>) -> Self {
        Self {
            filename,
            content_type,
            data,
        }
    }

    /// Get the filename (if provided)
    #[getter]
    fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Get the content type (if provided)
    #[getter]
    fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Get the file size in bytes
    #[getter]
    fn size(&self) -> usize {
        self.data.len()
    }

    /// Get the file content as bytes
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }

    /// Get the file content as text (UTF-8)
    fn text(&self) -> PyResult<String> {
        String::from_utf8(self.data.clone()).map_err(|e| {
            pyo3::exceptions::PyUnicodeDecodeError::new_err(format!("Invalid UTF-8: {e}"))
        })
    }

    /// Save the file to disk
    fn save(&self, path: &str) -> PyResult<()> {
        std::fs::write(path, &self.data).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to save file: {e}"))
        })
    }

    fn __repr__(&self) -> String {
        match &self.filename {
            Some(name) => format!("UploadedFile({}, {} bytes)", name, self.data.len()),
            None => format!("UploadedFile({} bytes)", self.data.len()),
        }
    }
}

impl PyUploadedFile {
    /// Get the raw data
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

/// Multipart form field (either a regular field or file)
#[pyclass(name = "MultipartField")]
#[derive(Clone, Debug)]
pub struct PyMultipartField {
    name: String,
    filename: Option<String>,
    content_type: Option<String>,
    data: Vec<u8>,
}

#[pymethods]
impl PyMultipartField {
    /// Create a new multipart field
    #[new]
    #[pyo3(signature = (name, data, filename = None, content_type = None))]
    fn new(name: String, data: Vec<u8>, filename: Option<String>, content_type: Option<String>) -> Self {
        Self {
            name,
            filename,
            content_type,
            data,
        }
    }

    /// Get the field name
    #[getter]
    fn name(&self) -> &str {
        &self.name
    }

    /// Get the filename (if this is a file field)
    #[getter]
    fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Get the content type
    #[getter]
    fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// Check if this field is a file upload
    fn is_file(&self) -> bool {
        self.filename.is_some()
    }

    /// Get the field value as bytes
    fn bytes<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.data)
    }

    /// Get the field value as text (UTF-8)
    fn text(&self) -> PyResult<String> {
        String::from_utf8(self.data.clone()).map_err(|e| {
            pyo3::exceptions::PyUnicodeDecodeError::new_err(format!("Invalid UTF-8: {e}"))
        })
    }

    /// Get as UploadedFile (for file fields)
    fn as_file(&self) -> PyResult<PyUploadedFile> {
        Ok(PyUploadedFile::new(
            self.data.clone(),
            self.filename.clone(),
            self.content_type.clone(),
        ))
    }

    fn __repr__(&self) -> String {
        match &self.filename {
            Some(fname) => format!("MultipartField({}, file={})", self.name, fname),
            None => format!("MultipartField({})", self.name),
        }
    }
}

/// Multipart form data extractor
///
/// # Example (Python)
///
/// ```python,ignore
/// from archimedes import Multipart
///
/// @app.handler("upload")
/// async def upload(multipart: Multipart):
///     for field in multipart.fields():
///         if field.is_file():
///             file = field.as_file()
///             file.save(f"/uploads/{file.filename}")
///         else:
///             print(f"{field.name} = {field.text()}")
/// ```
#[pyclass(name = "Multipart")]
#[derive(Clone, Debug)]
pub struct PyMultipart {
    fields: Vec<PyMultipartField>,
}

#[pymethods]
impl PyMultipart {
    /// Create a new Multipart instance
    #[new]
    fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Add a field (internal use)
    fn add_field(&mut self, field: PyMultipartField) {
        self.fields.push(field);
    }

    /// Get all fields
    fn fields(&self) -> Vec<PyMultipartField> {
        self.fields.clone()
    }

    /// Get a field by name
    fn get(&self, name: &str) -> Option<PyMultipartField> {
        self.fields.iter().find(|f| f.name == name).cloned()
    }

    /// Get all fields with a given name (for multiple file uploads)
    fn get_all(&self, name: &str) -> Vec<PyMultipartField> {
        self.fields.iter().filter(|f| f.name == name).cloned().collect()
    }

    /// Get all file fields
    fn files(&self) -> Vec<PyUploadedFile> {
        self.fields
            .iter()
            .filter(|f| f.is_file())
            .map(|f| PyUploadedFile::new(
                f.data.clone(),
                f.filename.clone(),
                f.content_type.clone(),
            ))
            .collect()
    }

    /// Get number of fields
    fn __len__(&self) -> usize {
        self.fields.len()
    }

    /// Iterate over fields
    fn __iter__(slf: PyRef<'_, Self>) -> PyMultipartIterator {
        PyMultipartIterator {
            fields: slf.fields.clone(),
            index: 0,
        }
    }

    fn __repr__(&self) -> String {
        format!("Multipart({} fields)", self.fields.len())
    }
}

/// Iterator for Multipart fields
#[pyclass]
pub struct PyMultipartIterator {
    fields: Vec<PyMultipartField>,
    index: usize,
}

#[pymethods]
impl PyMultipartIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyMultipartField> {
        if slf.index < slf.fields.len() {
            let field = slf.fields[slf.index].clone();
            slf.index += 1;
            Some(field)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_parse() {
        let form = PyForm::parse("username=alice&password=secret123").unwrap();
        assert_eq!(form.get("username"), Some("alice".to_string()));
        assert_eq!(form.get("password"), Some("secret123".to_string()));
        assert_eq!(form.get("missing"), None);
    }

    #[test]
    fn test_form_require() {
        pyo3::prepare_freethreaded_python();

        let form = PyForm::parse("name=test").unwrap();
        assert!(form.require("name").is_ok());
        assert!(form.require("missing").is_err());
    }

    #[test]
    fn test_cookies_parse() {
        let cookies = PyCookies::parse("session=abc123; theme=dark; lang=en");
        assert_eq!(cookies.get("session"), Some("abc123".to_string()));
        assert_eq!(cookies.get("theme"), Some("dark".to_string()));
        assert_eq!(cookies.get("lang"), Some("en".to_string()));
    }

    #[test]
    fn test_cookies_get_or() {
        let cookies = PyCookies::parse("session=abc123");
        assert_eq!(cookies.get_or("session", "default"), "abc123");
        assert_eq!(cookies.get_or("missing", "default"), "default");
    }

    #[test]
    fn test_set_cookie_build() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut cookie = PySetCookie::new("session".to_string(), "abc123".to_string());
            
            // Use PyRefMut for method chaining
            {
                let mut cookie_ref = Bound::new(py, cookie.clone()).unwrap().borrow_mut();
                cookie_ref.secure = true;
                cookie_ref.http_only = true;
                cookie_ref.same_site = Some(PySameSite::Strict);
                cookie_ref.path = Some("/".to_string());
                cookie_ref.max_age = Some(3600);
                cookie = cookie_ref.clone();
            }

            let header = cookie.build();
            assert!(header.contains("session=abc123"));
            assert!(header.contains("Secure"));
            assert!(header.contains("HttpOnly"));
            assert!(header.contains("SameSite=Strict"));
            assert!(header.contains("Path=/"));
            assert!(header.contains("Max-Age=3600"));
        });
    }

    #[test]
    fn test_uploaded_file() {
        pyo3::prepare_freethreaded_python();

        let file = PyUploadedFile::new(
            b"file content".to_vec(),
            Some("test.txt".to_string()),
            Some("text/plain".to_string()),
        );

        assert_eq!(file.filename(), Some("test.txt"));
        assert_eq!(file.content_type(), Some("text/plain"));
        assert_eq!(file.size(), 12);
        assert_eq!(file.text().unwrap(), "file content");
    }

    #[test]
    fn test_multipart_field() {
        pyo3::prepare_freethreaded_python();

        let field = PyMultipartField::new(
            "document".to_string(),
            b"PDF content".to_vec(),
            Some("doc.pdf".to_string()),
            Some("application/pdf".to_string()),
        );

        assert_eq!(field.name(), "document");
        assert!(field.is_file());
        assert_eq!(field.filename(), Some("doc.pdf"));
    }

    #[test]
    fn test_multipart() {
        pyo3::prepare_freethreaded_python();

        let mut multipart = PyMultipart::new();
        
        multipart.add_field(PyMultipartField::new(
            "name".to_string(),
            b"John".to_vec(),
            None,
            None,
        ));
        
        multipart.add_field(PyMultipartField::new(
            "avatar".to_string(),
            b"image data".to_vec(),
            Some("avatar.png".to_string()),
            Some("image/png".to_string()),
        ));

        assert_eq!(multipart.fields().len(), 2);
        assert_eq!(multipart.files().len(), 1);
        assert!(multipart.get("name").is_some());
        assert!(multipart.get("avatar").is_some());
    }
}
