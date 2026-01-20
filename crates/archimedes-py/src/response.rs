//! Python response types for Archimedes

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;

/// HTTP response returned from handlers
///
/// # Example (Python)
///
/// ```python,ignore
/// from archimedes import Response
///
/// @app.handler("getUser")
/// def get_user(ctx):
///     return Response(
///         status=200,
///         body={"id": "123", "name": "John"},
///         headers={"X-Custom": "value"}
///     )
/// ```
#[pyclass(name = "Response")]
#[derive(Clone, Debug)]
pub struct PyResponse {
    /// HTTP status code
    #[pyo3(get, set)]
    pub status: u16,

    /// Response body (will be JSON serialized)
    body: Option<serde_json::Value>,

    /// Response headers
    headers: HashMap<String, String>,
}

#[pymethods]
impl PyResponse {
    /// Create a new response
    ///
    /// Args:
    ///     status: HTTP status code (default: 200)
    ///     body: Response body (dict, list, or primitive)
    ///     headers: Response headers
    #[new]
    #[pyo3(signature = (status = 200, body = None, headers = None))]
    fn new(
        py: Python<'_>,
        status: u16,
        body: Option<PyObject>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        let body_json = if let Some(body) = body {
            Some(python_to_json(py, body)?)
        } else {
            None
        };

        Ok(Self {
            status,
            body: body_json,
            headers: headers.unwrap_or_default(),
        })
    }

    /// Get response body as Python object
    #[getter]
    fn body(&self, py: Python<'_>) -> PyResult<PyObject> {
        match &self.body {
            Some(json) => json_to_python(py, json),
            None => Ok(py.None()),
        }
    }

    /// Set response body from Python object
    fn set_body(&mut self, py: Python<'_>, value: PyObject) -> PyResult<()> {
        self.body = Some(python_to_json(py, value)?);
        Ok(())
    }

    /// Get response headers as dictionary
    #[getter]
    fn headers(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        for (k, v) in &self.headers {
            dict.set_item(k, v)?;
        }
        Ok(dict.into())
    }

    /// Set a header
    fn set_header(&mut self, name: String, value: String) {
        self.headers.insert(name, value);
    }

    /// Get a header
    fn get_header(&self, name: &str) -> Option<String> {
        self.headers.get(name).cloned()
    }

    /// Create an OK response (200)
    #[staticmethod]
    #[pyo3(signature = (body = None, headers = None))]
    fn ok(
        py: Python<'_>,
        body: Option<PyObject>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        Self::new(py, 200, body, headers)
    }

    /// Create a Created response (201)
    #[staticmethod]
    #[pyo3(signature = (body = None, headers = None))]
    fn created(
        py: Python<'_>,
        body: Option<PyObject>,
        headers: Option<HashMap<String, String>>,
    ) -> PyResult<Self> {
        Self::new(py, 201, body, headers)
    }

    /// Create a No Content response (204)
    #[staticmethod]
    fn no_content() -> PyResult<Self> {
        Ok(Self {
            status: 204,
            body: None,
            headers: HashMap::new(),
        })
    }

    /// Create a Bad Request response (400)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn bad_request(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 400,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create an Unauthorized response (401)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn unauthorized(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 401,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create a Forbidden response (403)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn forbidden(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 403,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create a Not Found response (404)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn not_found(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 404,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create an Internal Server Error response (500)
    #[staticmethod]
    #[pyo3(signature = (message = None))]
    fn internal_error(message: Option<String>) -> PyResult<Self> {
        let body = message.map(|m| serde_json::json!({"error": m}));
        Ok(Self {
            status: 500,
            body,
            headers: HashMap::new(),
        })
    }

    /// Create a JSON response
    #[staticmethod]
    #[pyo3(signature = (body, status = 200))]
    fn json(py: Python<'_>, body: PyObject, status: u16) -> PyResult<Self> {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        Self::new(py, status, Some(body), Some(headers))
    }

    /// Create a redirect response (302 Found)
    #[staticmethod]
    fn redirect(location: String) -> PyResult<Self> {
        let mut headers = HashMap::new();
        headers.insert("location".to_string(), location);
        Ok(Self {
            status: 302,
            body: None,
            headers,
        })
    }

    /// Create a permanent redirect response (301 Moved Permanently)
    #[staticmethod]
    fn permanent_redirect(location: String) -> PyResult<Self> {
        let mut headers = HashMap::new();
        headers.insert("location".to_string(), location);
        Ok(Self {
            status: 301,
            body: None,
            headers,
        })
    }

    /// Create a See Other redirect (303) - typically used after POST
    #[staticmethod]
    fn see_other(location: String) -> PyResult<Self> {
        let mut headers = HashMap::new();
        headers.insert("location".to_string(), location);
        Ok(Self {
            status: 303,
            body: None,
            headers,
        })
    }

    /// Create a temporary redirect (307) - preserves HTTP method
    #[staticmethod]
    fn temporary_redirect(location: String) -> PyResult<Self> {
        let mut headers = HashMap::new();
        headers.insert("location".to_string(), location);
        Ok(Self {
            status: 307,
            body: None,
            headers,
        })
    }

    /// Add a Set-Cookie to this response
    fn set_cookie(&mut self, cookie: &crate::extractors::PySetCookie) {
        // Multiple Set-Cookie headers need to be handled - for now store in headers map
        // In a real implementation, we'd need a proper header map that allows duplicates
        let key = format!("set-cookie-{}", self.headers.len());
        self.headers.insert(key, cookie.header_value());
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("Response(status={})", self.status)
    }

    /// Convert to dictionary
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("status", self.status)?;
        dict.set_item("body", self.body(py)?)?;
        dict.set_item("headers", self.headers(py)?)?;
        Ok(dict.into())
    }
}

impl PyResponse {
    /// Get the body as JSON
    pub fn body_json(&self) -> Option<&serde_json::Value> {
        self.body.as_ref()
    }

    /// Get headers as reference
    pub fn headers_ref(&self) -> &HashMap<String, String> {
        &self.headers
    }
}

/// File download response
///
/// # Example (Python)
///
/// ```python,ignore
/// from archimedes import FileResponse
///
/// @app.handler("download")
/// def download():
///     return FileResponse.from_path("/path/to/file.pdf")
///
/// @app.handler("export")
/// def export():
///     data = generate_csv()
///     return FileResponse(data, filename="export.csv", content_type="text/csv")
/// ```
#[pyclass(name = "FileResponse")]
#[derive(Clone, Debug)]
pub struct PyFileResponse {
    data: Vec<u8>,
    filename: Option<String>,
    content_type: String,
    inline: bool,
}

#[pymethods]
impl PyFileResponse {
    /// Create a new FileResponse from bytes
    #[new]
    #[pyo3(signature = (data, filename = None, content_type = None, inline = false))]
    fn new(
        data: Vec<u8>,
        filename: Option<String>,
        content_type: Option<String>,
        inline: bool,
    ) -> Self {
        let content_type = content_type.unwrap_or_else(|| {
            // Try to guess from filename
            filename
                .as_ref()
                .and_then(|f| guess_mime_type(f))
                .unwrap_or_else(|| "application/octet-stream".to_string())
        });

        Self {
            data,
            filename,
            content_type,
            inline,
        }
    }

    /// Create a FileResponse from a file path
    #[staticmethod]
    #[pyo3(signature = (path, filename = None, content_type = None, inline = false))]
    fn from_path(
        path: String,
        filename: Option<String>,
        content_type: Option<String>,
        inline: bool,
    ) -> PyResult<Self> {
        let data = std::fs::read(&path).map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to read file: {e}"))
        })?;

        let filename = filename.or_else(|| {
            std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from)
        });

        let content_type = content_type.unwrap_or_else(|| {
            filename
                .as_ref()
                .and_then(|f| guess_mime_type(f))
                .unwrap_or_else(|| "application/octet-stream".to_string())
        });

        Ok(Self {
            data,
            filename,
            content_type,
            inline,
        })
    }

    /// Create an attachment response (forces download)
    #[staticmethod]
    #[pyo3(signature = (data, filename, content_type = None))]
    fn attachment(data: Vec<u8>, filename: String, content_type: Option<String>) -> Self {
        Self::new(data, Some(filename), content_type, false)
    }

    /// Create an inline response (displays in browser)
    #[staticmethod]
    #[pyo3(signature = (data, filename = None, content_type = None))]
    fn inline(data: Vec<u8>, filename: Option<String>, content_type: Option<String>) -> Self {
        Self::new(data, filename, content_type, true)
    }

    /// Get the filename
    #[getter]
    fn filename(&self) -> Option<&str> {
        self.filename.as_deref()
    }

    /// Get the content type
    #[getter]
    fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Get the file size
    #[getter]
    fn size(&self) -> usize {
        self.data.len()
    }

    /// Check if inline
    #[getter]
    fn is_inline(&self) -> bool {
        self.inline
    }

    /// Get the Content-Disposition header value
    fn content_disposition(&self) -> String {
        let disposition_type = if self.inline { "inline" } else { "attachment" };
        match &self.filename {
            Some(name) => format!("{disposition_type}; filename=\"{name}\""),
            None => disposition_type.to_string(),
        }
    }

    /// Get the data as bytes
    fn bytes<'py>(&self, py: Python<'py>) -> pyo3::Bound<'py, pyo3::types::PyBytes> {
        pyo3::types::PyBytes::new(py, &self.data)
    }

    fn __repr__(&self) -> String {
        match &self.filename {
            Some(name) => format!("FileResponse({}, {} bytes)", name, self.data.len()),
            None => format!("FileResponse({} bytes)", self.data.len()),
        }
    }
}

impl PyFileResponse {
    /// Get raw data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get headers for HTTP response
    pub fn headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), self.content_type.clone());
        headers.insert("content-disposition".to_string(), self.content_disposition());
        headers.insert("content-length".to_string(), self.data.len().to_string());
        headers
    }
}

/// Guess MIME type from filename
fn guess_mime_type(filename: &str) -> Option<String> {
    let ext = filename.rsplit('.').next()?.to_lowercase();
    let mime = match ext.as_str() {
        // Text
        "txt" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "csv" => "text/csv",
        "xml" => "text/xml",
        // JavaScript
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        // Documents
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" | "gzip" => "application/gzip",
        // Audio/Video
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        // Fonts
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        _ => return None,
    };
    Some(mime.to_string())
}

/// Convert serde_json::Value to Python object
fn json_to_python(py: Python<'_>, value: &serde_json::Value) -> PyResult<PyObject> {
    Ok(match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.into_pyobject(py)?.to_owned().into_any().unbind(),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_pyobject(py)?.to_owned().into_any().unbind()
            } else if let Some(f) = n.as_f64() {
                f.into_pyobject(py)?.to_owned().into_any().unbind()
            } else {
                py.None()
            }
        }
        serde_json::Value::String(s) => s.into_pyobject(py)?.to_owned().into_any().unbind(),
        serde_json::Value::Array(arr) => {
            let items: Vec<PyObject> = arr
                .iter()
                .map(|v| json_to_python(py, v))
                .collect::<PyResult<_>>()?;
            let list = pyo3::types::PyList::new(py, &items)?;
            list.into()
        }
        serde_json::Value::Object(obj) => {
            let dict = PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_python(py, v)?)?;
            }
            dict.into()
        }
    })
}

/// Convert Python object to serde_json::Value
fn python_to_json(py: Python<'_>, obj: PyObject) -> PyResult<serde_json::Value> {
    let obj_ref = obj.bind(py);

    if obj_ref.is_none() {
        return Ok(serde_json::Value::Null);
    }

    // Try bool first (before int since bool is subclass of int in Python)
    if let Ok(b) = obj_ref.extract::<bool>() {
        return Ok(serde_json::Value::Bool(b));
    }

    // Try integer
    if let Ok(i) = obj_ref.extract::<i64>() {
        return Ok(serde_json::Value::Number(i.into()));
    }

    // Try float
    if let Ok(f) = obj_ref.extract::<f64>() {
        return Ok(serde_json::json!(f));
    }

    // Try string
    if let Ok(s) = obj_ref.extract::<String>() {
        return Ok(serde_json::Value::String(s));
    }

    // Try list
    if let Ok(list) = obj_ref.downcast::<pyo3::types::PyList>() {
        let mut arr = Vec::new();
        for item in list.iter() {
            arr.push(python_to_json(py, item.into())?);
        }
        return Ok(serde_json::Value::Array(arr));
    }

    // Try dict
    if let Ok(dict) = obj_ref.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key = k.extract::<String>()?;
            let value = python_to_json(py, v.into())?;
            map.insert(key, value);
        }
        return Ok(serde_json::Value::Object(map));
    }

    // Try tuple (convert to array)
    if let Ok(tuple) = obj_ref.downcast::<pyo3::types::PyTuple>() {
        let mut arr = Vec::new();
        for item in tuple.iter() {
            arr.push(python_to_json(py, item.into())?);
        }
        return Ok(serde_json::Value::Array(arr));
    }

    // Fallback: try to get string representation
    let repr = obj_ref.str()?.to_string();
    Ok(serde_json::Value::String(repr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_creation() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let response = PyResponse::new(py, 200, None, None).unwrap();
            assert_eq!(response.status, 200);
        });
    }

    #[test]
    fn test_response_with_body() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let body = PyDict::new(py);
            body.set_item("name", "test").unwrap();

            let response = PyResponse::new(py, 201, Some(body.into()), None).unwrap();
            assert_eq!(response.status, 201);

            let body_json = response.body_json().unwrap();
            assert_eq!(body_json["name"], "test");
        });
    }

    #[test]
    fn test_response_helpers() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let ok = PyResponse::ok(py, None, None).unwrap();
            assert_eq!(ok.status, 200);

            let created = PyResponse::created(py, None, None).unwrap();
            assert_eq!(created.status, 201);

            let no_content = PyResponse::no_content().unwrap();
            assert_eq!(no_content.status, 204);

            let bad_request = PyResponse::bad_request(Some("invalid".to_string())).unwrap();
            assert_eq!(bad_request.status, 400);

            let not_found = PyResponse::not_found(None).unwrap();
            assert_eq!(not_found.status, 404);
        });
    }

    #[test]
    fn test_response_headers() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let mut response = PyResponse::new(py, 200, None, None).unwrap();
            response.set_header("X-Custom".to_string(), "value".to_string());

            assert_eq!(response.get_header("X-Custom"), Some("value".to_string()));
            assert_eq!(response.get_header("X-Missing"), None);
        });
    }

    #[test]
    fn test_json_response() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let body = PyDict::new(py);
            body.set_item("data", "test").unwrap();

            let response = PyResponse::json(py, body.into(), 200).unwrap();

            assert_eq!(response.status, 200);
            assert_eq!(
                response.headers_ref().get("content-type"),
                Some(&"application/json".to_string())
            );
        });
    }

    #[test]
    fn test_redirect_responses() {
        pyo3::prepare_freethreaded_python();

        // 302 Found
        let redirect = PyResponse::redirect("/dashboard".to_string()).unwrap();
        assert_eq!(redirect.status, 302);
        assert_eq!(redirect.headers_ref().get("location"), Some(&"/dashboard".to_string()));

        // 301 Permanent
        let permanent = PyResponse::permanent_redirect("/new-url".to_string()).unwrap();
        assert_eq!(permanent.status, 301);
        assert_eq!(permanent.headers_ref().get("location"), Some(&"/new-url".to_string()));

        // 303 See Other
        let see_other = PyResponse::see_other("/result".to_string()).unwrap();
        assert_eq!(see_other.status, 303);

        // 307 Temporary
        let temp = PyResponse::temporary_redirect("/temp".to_string()).unwrap();
        assert_eq!(temp.status, 307);
    }

    #[test]
    fn test_file_response() {
        pyo3::prepare_freethreaded_python();

        let file = PyFileResponse::new(
            b"file content".to_vec(),
            Some("test.txt".to_string()),
            Some("text/plain".to_string()),
            false,
        );

        assert_eq!(file.filename(), Some("test.txt"));
        assert_eq!(file.content_type(), "text/plain");
        assert_eq!(file.size(), 12);
        assert!(!file.is_inline());

        let disposition = file.content_disposition();
        assert!(disposition.contains("attachment"));
        assert!(disposition.contains("test.txt"));
    }

    #[test]
    fn test_file_response_inline() {
        pyo3::prepare_freethreaded_python();

        let file = PyFileResponse::inline(
            b"image data".to_vec(),
            Some("image.png".to_string()),
            None,
        );

        assert!(file.is_inline());
        assert_eq!(file.content_type(), "image/png"); // guessed from filename

        let disposition = file.content_disposition();
        assert!(disposition.contains("inline"));
    }

    #[test]
    fn test_file_response_attachment() {
        pyo3::prepare_freethreaded_python();

        let file = PyFileResponse::attachment(
            b"pdf content".to_vec(),
            "report.pdf".to_string(),
            None,
        );

        assert!(!file.is_inline());
        assert_eq!(file.content_type(), "application/pdf"); // guessed from filename
        assert_eq!(file.filename(), Some("report.pdf"));
    }

    #[test]
    fn test_guess_mime_type() {
        assert_eq!(guess_mime_type("test.txt"), Some("text/plain".to_string()));
        assert_eq!(guess_mime_type("style.css"), Some("text/css".to_string()));
        assert_eq!(guess_mime_type("app.js"), Some("application/javascript".to_string()));
        assert_eq!(guess_mime_type("data.json"), Some("application/json".to_string()));
        assert_eq!(guess_mime_type("image.png"), Some("image/png".to_string()));
        assert_eq!(guess_mime_type("photo.jpg"), Some("image/jpeg".to_string()));
        assert_eq!(guess_mime_type("doc.pdf"), Some("application/pdf".to_string()));
        assert_eq!(guess_mime_type("archive.zip"), Some("application/zip".to_string()));
        assert_eq!(guess_mime_type("unknown.xyz"), None);
    }

    #[test]
    fn test_file_response_headers() {
        pyo3::prepare_freethreaded_python();

        let file = PyFileResponse::new(
            b"test".to_vec(),
            Some("test.txt".to_string()),
            Some("text/plain".to_string()),
            false,
        );

        let headers = file.headers();
        assert_eq!(headers.get("content-type"), Some(&"text/plain".to_string()));
        assert!(headers.get("content-disposition").unwrap().contains("attachment"));
        assert_eq!(headers.get("content-length"), Some(&"4".to_string()));
    }
}
