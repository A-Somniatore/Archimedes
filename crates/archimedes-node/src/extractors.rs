//! TypeScript/Node.js extractors for Archimedes
//!
//! This module provides Form, Cookies, and Multipart extractors for Node.js.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;

/// URL-encoded form data extractor.
///
/// ## Example
///
/// ```typescript
/// import { Form } from '@archimedes/node';
///
/// app.operation('login', async (req) => {
///     const form = Form.parse(req.body);
///     const username = form.get('username');
///     const password = form.get('password');
///     return Response.ok({ authenticated: true });
/// });
/// ```
#[napi]
#[derive(Debug, Clone)]
pub struct Form {
    fields: HashMap<String, String>,
}

#[napi]
impl Form {
    /// Create a new Form instance.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    /// Parse form data from a URL-encoded string.
    #[napi(factory)]
    pub fn parse(data: String) -> Result<Self> {
        let fields: HashMap<String, String> =
            serde_urlencoded::from_str(&data).map_err(|e| {
                Error::new(Status::InvalidArg, format!("Invalid form data: {e}"))
            })?;
        Ok(Self { fields })
    }

    /// Create Form from a key-value object.
    #[napi(factory)]
    pub fn from_object(obj: HashMap<String, String>) -> Self {
        Self { fields: obj }
    }

    /// Get a field value by name.
    #[napi]
    pub fn get(&self, name: String) -> Option<String> {
        self.fields.get(&name).cloned()
    }

    /// Get a field value or return a default.
    #[napi]
    pub fn get_or(&self, name: String, default: String) -> String {
        self.fields.get(&name).cloned().unwrap_or(default)
    }

    /// Get a required field, throwing if not present.
    #[napi]
    pub fn require(&self, name: String) -> Result<String> {
        self.fields.get(&name).cloned().ok_or_else(|| {
            Error::new(Status::InvalidArg, format!("Missing required field: {name}"))
        })
    }

    /// Check if a field exists.
    #[napi]
    pub fn has(&self, name: String) -> bool {
        self.fields.contains_key(&name)
    }

    /// Get all field names.
    #[napi]
    pub fn keys(&self) -> Vec<String> {
        self.fields.keys().cloned().collect()
    }

    /// Get all field values.
    #[napi]
    pub fn values(&self) -> Vec<String> {
        self.fields.values().cloned().collect()
    }

    /// Get the number of fields.
    #[napi(getter)]
    pub fn length(&self) -> u32 {
        self.fields.len() as u32
    }

    /// Convert to a plain object.
    #[napi]
    pub fn to_object(&self) -> HashMap<String, String> {
        self.fields.clone()
    }
}

impl Default for Form {
    fn default() -> Self {
        Self::new()
    }
}

/// Cookie extractor for request cookies.
///
/// ## Example
///
/// ```typescript
/// import { Cookies } from '@archimedes/node';
///
/// app.operation('profile', async (req) => {
///     const cookies = Cookies.parse(req.headers.get('cookie') || '');
///     const sessionId = cookies.get('session_id');
///     const theme = cookies.getOr('theme', 'light');
///     return Response.ok({ theme });
/// });
/// ```
#[napi]
#[derive(Debug, Clone)]
pub struct Cookies {
    cookies: HashMap<String, String>,
}

#[napi]
impl Cookies {
    /// Create a new empty Cookies instance.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            cookies: HashMap::new(),
        }
    }

    /// Parse cookies from a Cookie header value.
    #[napi(factory)]
    pub fn parse(header_value: String) -> Self {
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

    /// Create Cookies from a key-value object.
    #[napi(factory)]
    pub fn from_object(obj: HashMap<String, String>) -> Self {
        Self { cookies: obj }
    }

    /// Get a cookie value by name.
    #[napi]
    pub fn get(&self, name: String) -> Option<String> {
        self.cookies.get(&name).cloned()
    }

    /// Get a cookie value or return a default.
    #[napi]
    pub fn get_or(&self, name: String, default: String) -> String {
        self.cookies.get(&name).cloned().unwrap_or(default)
    }

    /// Get a required cookie, throwing if not present.
    #[napi]
    pub fn require(&self, name: String) -> Result<String> {
        self.cookies.get(&name).cloned().ok_or_else(|| {
            Error::new(Status::InvalidArg, format!("Missing required cookie: {name}"))
        })
    }

    /// Check if a cookie exists.
    #[napi]
    pub fn has(&self, name: String) -> bool {
        self.cookies.contains_key(&name)
    }

    /// Get all cookie names.
    #[napi]
    pub fn names(&self) -> Vec<String> {
        self.cookies.keys().cloned().collect()
    }

    /// Get all cookie values.
    #[napi]
    pub fn cookie_values(&self) -> Vec<String> {
        self.cookies.values().cloned().collect()
    }

    /// Get the number of cookies.
    #[napi(getter)]
    pub fn length(&self) -> u32 {
        self.cookies.len() as u32
    }

    /// Convert to a plain object.
    #[napi]
    pub fn to_object(&self) -> HashMap<String, String> {
        self.cookies.clone()
    }
}

impl Default for Cookies {
    fn default() -> Self {
        Self::new()
    }
}

/// SameSite cookie attribute.
#[napi]
pub enum SameSite {
    /// Cookie is sent with cross-site requests.
    None,
    /// Cookie only sent with same-site requests and top-level navigation.
    Lax,
    /// Cookie only sent with same-site requests.
    Strict,
}

/// Set-Cookie response helper for setting cookies.
///
/// ## Example
///
/// ```typescript
/// import { SetCookie, SameSite, Response } from '@archimedes/node';
///
/// app.operation('login', async (req) => {
///     const cookie = new SetCookie('session_id', 'abc123')
///         .httpOnly(true)
///         .secure(true)
///         .sameSite(SameSite.Strict)
///         .maxAge(3600);
///     
///     const response = Response.ok({ authenticated: true });
///     response.setHeader('set-cookie', cookie.build());
///     return response;
/// });
/// ```
#[napi]
#[derive(Debug, Clone)]
pub struct SetCookie {
    name: String,
    value: String,
    domain: Option<String>,
    path: Option<String>,
    max_age: Option<i64>,
    expires: Option<String>,
    secure: bool,
    http_only: bool,
    same_site: Option<String>,
}

#[napi]
impl SetCookie {
    /// Create a new Set-Cookie builder.
    #[napi(constructor)]
    pub fn new(name: String, value: String) -> Self {
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

    /// Set the Domain attribute.
    #[napi]
    pub fn domain(&mut self, domain: String) -> &Self {
        self.domain = Some(domain);
        self
    }

    /// Set the Path attribute.
    #[napi]
    pub fn path(&mut self, path: String) -> &Self {
        self.path = Some(path);
        self
    }

    /// Set the Max-Age attribute (in seconds).
    #[napi]
    pub fn max_age(&mut self, seconds: i64) -> &Self {
        self.max_age = Some(seconds);
        self
    }

    /// Set the Expires attribute (RFC 7231 date format).
    #[napi]
    pub fn expires(&mut self, date: String) -> &Self {
        self.expires = Some(date);
        self
    }

    /// Set the Secure attribute.
    #[napi]
    pub fn secure(&mut self, secure: bool) -> &Self {
        self.secure = secure;
        self
    }

    /// Set the HttpOnly attribute.
    #[napi]
    pub fn http_only(&mut self, http_only: bool) -> &Self {
        self.http_only = http_only;
        self
    }

    /// Set the SameSite attribute.
    #[napi]
    pub fn same_site(&mut self, same_site: SameSite) -> &Self {
        let value = match same_site {
            SameSite::None => "None",
            SameSite::Lax => "Lax",
            SameSite::Strict => "Strict",
        };
        self.same_site = Some(value.to_string());
        self
    }

    /// Build the Set-Cookie header value.
    #[napi]
    pub fn build(&self) -> String {
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
        if let Some(ref same_site) = self.same_site {
            parts.push(format!("SameSite={same_site}"));
        }

        parts.join("; ")
    }

    /// Get the cookie name.
    #[napi(getter)]
    pub fn cookie_name(&self) -> String {
        self.name.clone()
    }

    /// Get the cookie value.
    #[napi(getter)]
    pub fn cookie_value(&self) -> String {
        self.value.clone()
    }
}

/// Uploaded file from multipart form data.
///
/// ## Example
///
/// ```typescript
/// import { Multipart } from '@archimedes/node';
///
/// app.operation('upload', async (req) => {
///     const multipart = await Multipart.parse(req);
///     for (const field of multipart.fields()) {
///         if (field.isFile()) {
///             const file = field.asFile();
///             await fs.writeFile(`/uploads/${file.filename}`, file.bytes());
///         }
///     }
///     return Response.noContent();
/// });
/// ```
#[napi]
#[derive(Debug, Clone)]
pub struct UploadedFile {
    filename: Option<String>,
    content_type: Option<String>,
    data: Vec<u8>,
}

#[napi]
impl UploadedFile {
    /// Create a new uploaded file.
    #[napi(constructor)]
    pub fn new(data: Buffer, filename: Option<String>, content_type: Option<String>) -> Self {
        Self {
            filename,
            content_type,
            data: data.to_vec(),
        }
    }

    /// Get the filename (if provided).
    #[napi(getter)]
    pub fn filename(&self) -> Option<String> {
        self.filename.clone()
    }

    /// Get the content type (if provided).
    #[napi(getter)]
    pub fn content_type(&self) -> Option<String> {
        self.content_type.clone()
    }

    /// Get the file size in bytes.
    #[napi(getter)]
    pub fn size(&self) -> u32 {
        self.data.len() as u32
    }

    /// Get the file content as a Buffer.
    #[napi]
    pub fn bytes(&self) -> Buffer {
        self.data.clone().into()
    }

    /// Get the file content as text (UTF-8).
    #[napi]
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.data.clone())
            .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid UTF-8: {e}")))
    }

    /// Save the file to disk.
    #[napi]
    pub fn save(&self, path: String) -> Result<()> {
        std::fs::write(&path, &self.data)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Failed to save file: {e}")))
    }
}

/// Multipart form field (either a regular field or file).
#[napi]
#[derive(Debug, Clone)]
pub struct MultipartField {
    name: String,
    filename: Option<String>,
    content_type: Option<String>,
    data: Vec<u8>,
}

#[napi]
impl MultipartField {
    /// Create a new multipart field.
    #[napi(constructor)]
    pub fn new(
        name: String,
        data: Buffer,
        filename: Option<String>,
        content_type: Option<String>,
    ) -> Self {
        Self {
            name,
            filename,
            content_type,
            data: data.to_vec(),
        }
    }

    /// Get the field name.
    #[napi(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Get the filename (if this is a file field).
    #[napi(getter)]
    pub fn filename(&self) -> Option<String> {
        self.filename.clone()
    }

    /// Get the content type.
    #[napi(getter)]
    pub fn content_type(&self) -> Option<String> {
        self.content_type.clone()
    }

    /// Check if this field is a file upload.
    #[napi]
    pub fn is_file(&self) -> bool {
        self.filename.is_some()
    }

    /// Get the field value as a Buffer.
    #[napi]
    pub fn bytes(&self) -> Buffer {
        self.data.clone().into()
    }

    /// Get the field value as text (UTF-8).
    #[napi]
    pub fn text(&self) -> Result<String> {
        String::from_utf8(self.data.clone())
            .map_err(|e| Error::new(Status::InvalidArg, format!("Invalid UTF-8: {e}")))
    }

    /// Get as UploadedFile (for file fields).
    #[napi]
    pub fn as_file(&self) -> UploadedFile {
        UploadedFile {
            filename: self.filename.clone(),
            content_type: self.content_type.clone(),
            data: self.data.clone(),
        }
    }
}

/// Multipart form data.
///
/// ## Example
///
/// ```typescript
/// import { Multipart, MultipartField } from '@archimedes/node';
///
/// const multipart = new Multipart();
/// multipart.addField(new MultipartField('name', Buffer.from('John'), null, null));
/// multipart.addField(new MultipartField('avatar', avatarData, 'avatar.png', 'image/png'));
/// ```
#[napi]
#[derive(Debug, Clone)]
pub struct Multipart {
    fields: Vec<MultipartField>,
}

#[napi]
impl Multipart {
    /// Create a new empty Multipart instance.
    #[napi(constructor)]
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Add a field to the multipart data.
    #[napi]
    pub fn add_field(&mut self, field: &MultipartField) {
        self.fields.push(field.clone());
    }

    /// Get all fields.
    #[napi]
    pub fn get_fields(&self) -> Vec<MultipartField> {
        self.fields.clone()
    }

    /// Get a field by name.
    #[napi]
    pub fn get(&self, name: String) -> Option<MultipartField> {
        self.fields.iter().find(|f| f.name == name).cloned()
    }

    /// Get all fields with a given name (for multiple file uploads).
    #[napi]
    pub fn get_all(&self, name: String) -> Vec<MultipartField> {
        self.fields.iter().filter(|f| f.name == name).cloned().collect()
    }

    /// Get all file fields.
    #[napi]
    pub fn files(&self) -> Vec<UploadedFile> {
        self.fields
            .iter()
            .filter(|f| f.is_file())
            .map(|f| UploadedFile {
                filename: f.filename.clone(),
                content_type: f.content_type.clone(),
                data: f.data.clone(),
            })
            .collect()
    }

    /// Get number of fields.
    #[napi(getter)]
    pub fn length(&self) -> u32 {
        self.fields.len() as u32
    }
}

impl Default for Multipart {
    fn default() -> Self {
        Self::new()
    }
}

/// File download response.
///
/// ## Example
///
/// ```typescript
/// import { FileResponse } from '@archimedes/node';
///
/// app.operation('download', async (req) => {
///     return FileResponse.fromPath('/path/to/file.pdf');
/// });
///
/// app.operation('export', async (req) => {
///     const csv = generateCsv();
///     return new FileResponse(Buffer.from(csv), 'export.csv', 'text/csv');
/// });
/// ```
#[napi]
#[derive(Debug, Clone)]
pub struct FileResponse {
    data: Vec<u8>,
    filename: Option<String>,
    content_type: String,
    inline: bool,
}

#[napi]
impl FileResponse {
    /// Create a new FileResponse from a buffer.
    #[napi(constructor)]
    pub fn new(
        data: Buffer,
        filename: Option<String>,
        content_type: Option<String>,
        inline: Option<bool>,
    ) -> Self {
        let content_type = content_type.unwrap_or_else(|| {
            filename
                .as_ref()
                .and_then(|f| guess_mime_type(f))
                .unwrap_or_else(|| "application/octet-stream".to_string())
        });

        Self {
            data: data.to_vec(),
            filename,
            content_type,
            inline: inline.unwrap_or(false),
        }
    }

    /// Create a FileResponse from a file path.
    #[napi(factory)]
    pub fn from_path(
        path: String,
        filename: Option<String>,
        content_type: Option<String>,
        inline: Option<bool>,
    ) -> Result<Self> {
        let data = std::fs::read(&path).map_err(|e| {
            Error::new(Status::GenericFailure, format!("Failed to read file: {e}"))
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
            inline: inline.unwrap_or(false),
        })
    }

    /// Create an attachment response (forces download).
    #[napi(factory)]
    pub fn attachment(data: Buffer, filename: String, content_type: Option<String>) -> Self {
        Self::new(data, Some(filename), content_type, Some(false))
    }

    /// Create an inline response (displays in browser).
    #[napi(factory)]
    pub fn inline(data: Buffer, filename: Option<String>, content_type: Option<String>) -> Self {
        Self::new(data, filename, content_type, Some(true))
    }

    /// Get the filename.
    #[napi(getter)]
    pub fn filename(&self) -> Option<String> {
        self.filename.clone()
    }

    /// Get the content type.
    #[napi(getter)]
    pub fn content_type(&self) -> String {
        self.content_type.clone()
    }

    /// Get the file size.
    #[napi(getter)]
    pub fn size(&self) -> u32 {
        self.data.len() as u32
    }

    /// Check if inline.
    #[napi(getter)]
    pub fn is_inline(&self) -> bool {
        self.inline
    }

    /// Get the Content-Disposition header value.
    #[napi]
    pub fn content_disposition(&self) -> String {
        let disposition_type = if self.inline { "inline" } else { "attachment" };
        match &self.filename {
            Some(name) => format!("{disposition_type}; filename=\"{name}\""),
            None => disposition_type.to_string(),
        }
    }

    /// Get the data as a Buffer.
    #[napi]
    pub fn bytes(&self) -> Buffer {
        self.data.clone().into()
    }

    /// Get response headers.
    #[napi]
    pub fn get_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), self.content_type.clone());
        headers.insert("content-disposition".to_string(), self.content_disposition());
        headers.insert("content-length".to_string(), self.data.len().to_string());
        headers
    }
}

/// Guess MIME type from filename.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_form_parse() {
        let form = Form::parse("username=alice&password=secret123".to_string()).unwrap();
        assert_eq!(form.get("username".to_string()), Some("alice".to_string()));
        assert_eq!(form.get("password".to_string()), Some("secret123".to_string()));
        assert_eq!(form.get("missing".to_string()), None);
    }

    #[test]
    fn test_form_get_or() {
        let form = Form::parse("name=test".to_string()).unwrap();
        assert_eq!(form.get_or("name".to_string(), "default".to_string()), "test");
        assert_eq!(form.get_or("missing".to_string(), "default".to_string()), "default");
    }

    #[test]
    fn test_form_require() {
        let form = Form::parse("name=test".to_string()).unwrap();
        assert!(form.require("name".to_string()).is_ok());
        assert!(form.require("missing".to_string()).is_err());
    }

    #[test]
    fn test_cookies_parse() {
        let cookies = Cookies::parse("session=abc123; theme=dark; lang=en".to_string());
        assert_eq!(cookies.get("session".to_string()), Some("abc123".to_string()));
        assert_eq!(cookies.get("theme".to_string()), Some("dark".to_string()));
        assert_eq!(cookies.get("lang".to_string()), Some("en".to_string()));
    }

    #[test]
    fn test_cookies_get_or() {
        let cookies = Cookies::parse("session=abc123".to_string());
        assert_eq!(cookies.get_or("session".to_string(), "default".to_string()), "abc123");
        assert_eq!(cookies.get_or("missing".to_string(), "default".to_string()), "default");
    }

    #[test]
    fn test_set_cookie_build() {
        let mut cookie = SetCookie::new("session".to_string(), "abc123".to_string());
        cookie.secure(true);
        cookie.http_only(true);
        cookie.same_site(SameSite::Strict);
        cookie.path("/".to_string());
        cookie.max_age(3600);

        let header = cookie.build();
        assert!(header.contains("session=abc123"));
        assert!(header.contains("Secure"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("SameSite=Strict"));
        assert!(header.contains("Path=/"));
        assert!(header.contains("Max-Age=3600"));
    }

    #[test]
    fn test_uploaded_file() {
        let file = UploadedFile::new(
            Buffer::from(b"file content".to_vec()),
            Some("test.txt".to_string()),
            Some("text/plain".to_string()),
        );

        assert_eq!(file.filename(), Some("test.txt".to_string()));
        assert_eq!(file.content_type(), Some("text/plain".to_string()));
        assert_eq!(file.size(), 12);
        assert_eq!(file.text().unwrap(), "file content");
    }

    #[test]
    fn test_multipart_field() {
        let field = MultipartField::new(
            "document".to_string(),
            Buffer::from(b"PDF content".to_vec()),
            Some("doc.pdf".to_string()),
            Some("application/pdf".to_string()),
        );

        assert_eq!(field.name(), "document");
        assert!(field.is_file());
        assert_eq!(field.filename(), Some("doc.pdf".to_string()));
    }

    #[test]
    fn test_multipart() {
        let mut multipart = Multipart::new();

        multipart.add_field(&MultipartField::new(
            "name".to_string(),
            Buffer::from(b"John".to_vec()),
            None,
            None,
        ));

        multipart.add_field(&MultipartField::new(
            "avatar".to_string(),
            Buffer::from(b"image data".to_vec()),
            Some("avatar.png".to_string()),
            Some("image/png".to_string()),
        ));

        assert_eq!(multipart.get_fields().len(), 2);
        assert_eq!(multipart.files().len(), 1);
        assert!(multipart.get("name".to_string()).is_some());
        assert!(multipart.get("avatar".to_string()).is_some());
    }

    #[test]
    fn test_file_response() {
        let file = FileResponse::new(
            Buffer::from(b"file content".to_vec()),
            Some("test.txt".to_string()),
            Some("text/plain".to_string()),
            Some(false),
        );

        assert_eq!(file.filename(), Some("test.txt".to_string()));
        assert_eq!(file.content_type(), "text/plain");
        assert_eq!(file.size(), 12);
        assert!(!file.is_inline());

        let disposition = file.content_disposition();
        assert!(disposition.contains("attachment"));
        assert!(disposition.contains("test.txt"));
    }

    #[test]
    fn test_file_response_inline() {
        let file = FileResponse::inline(
            Buffer::from(b"image data".to_vec()),
            Some("image.png".to_string()),
            None,
        );

        assert!(file.is_inline());
        assert_eq!(file.content_type(), "image/png"); // guessed from filename

        let disposition = file.content_disposition();
        assert!(disposition.contains("inline"));
    }

    #[test]
    fn test_file_response_headers() {
        let file = FileResponse::new(
            Buffer::from(b"test".to_vec()),
            Some("test.txt".to_string()),
            Some("text/plain".to_string()),
            Some(false),
        );

        let headers = file.get_headers();
        assert_eq!(headers.get("content-type"), Some(&"text/plain".to_string()));
        assert!(headers.get("content-disposition").unwrap().contains("attachment"));
        assert_eq!(headers.get("content-length"), Some(&"4".to_string()));
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
}
