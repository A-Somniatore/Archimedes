//! Cookie extraction and response helpers.
//!
//! This module provides extractors for reading cookies from requests
//! and helpers for setting cookies in responses.
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes_extract::{Cookies, Cookie, SetCookie, SameSite};
//!
//! async fn session_handler(cookies: Cookies) -> Response {
//!     // Read a cookie
//!     if let Some(session_id) = cookies.get("session_id") {
//!         // User has existing session
//!     }
//!     
//!     // Set a new cookie
//!     let cookie = SetCookie::new("session_id", "abc123")
//!         .http_only(true)
//!         .secure(true)
//!         .same_site(SameSite::Strict)
//!         .max_age(3600);
//!     
//!     Response::ok().with_cookie(cookie)
//! }
//! ```

use crate::{ExtractionContext, ExtractionError, ExtractionSource, FromRequest};
use http::header;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

/// Extractor for request cookies.
///
/// Parses all cookies from the `Cookie` header and provides
/// convenient access methods.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::{Cookies, FromRequest, ExtractionContext};
/// use archimedes_router::Params;
/// use http::{Method, Uri, HeaderMap, HeaderValue};
/// use bytes::Bytes;
///
/// let mut headers = HeaderMap::new();
/// headers.insert(
///     http::header::COOKIE,
///     HeaderValue::from_static("session=abc123; theme=dark"),
/// );
///
/// let ctx = ExtractionContext::new(
///     Method::GET,
///     Uri::from_static("/dashboard"),
///     headers,
///     Bytes::new(),
///     Params::new(),
/// );
///
/// let cookies = Cookies::from_request(&ctx).unwrap();
/// assert_eq!(cookies.get("session"), Some("abc123"));
/// assert_eq!(cookies.get("theme"), Some("dark"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct Cookies {
    cookies: HashMap<String, String>,
}

impl Cookies {
    /// Create an empty Cookies instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse cookies from a Cookie header value.
    fn parse(header_value: &str) -> Self {
        let mut cookies = HashMap::new();
        
        for cookie in header_value.split(';') {
            let cookie = cookie.trim();
            if let Some((name, value)) = cookie.split_once('=') {
                let name = name.trim();
                let value = value.trim();
                // Remove surrounding quotes if present
                let value = value.trim_matches('"');
                cookies.insert(name.to_string(), value.to_string());
            }
        }
        
        Self { cookies }
    }

    /// Get a cookie value by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(String::as_str)
    }

    /// Get a cookie value or return a default.
    #[must_use]
    pub fn get_or<'a>(&'a self, name: &str, default: &'a str) -> &'a str {
        self.get(name).unwrap_or(default)
    }

    /// Check if a cookie exists.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.cookies.contains_key(name)
    }

    /// Get all cookie names.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.cookies.keys().map(String::as_str)
    }

    /// Get an iterator over all cookies.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.cookies.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Get the number of cookies.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cookies.len()
    }

    /// Check if there are no cookies.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }

    /// Get a required cookie, returning an error if not present.
    ///
    /// # Errors
    ///
    /// Returns an error if the cookie is not found.
    pub fn require(&self, name: &str) -> Result<&str, ExtractionError> {
        self.get(name).ok_or_else(|| {
            ExtractionError::missing(ExtractionSource::Header, format!("cookie '{name}'"))
        })
    }
}

impl FromRequest for Cookies {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        let cookie_header = ctx.headers().get(header::COOKIE);
        
        match cookie_header {
            Some(value) => {
                let value_str = value.to_str().map_err(|_| {
                    ExtractionError::deserialization_failed(
                        ExtractionSource::Header,
                        "invalid UTF-8 in Cookie header",
                    )
                })?;
                Ok(Self::parse(value_str))
            }
            None => Ok(Self::new()),
        }
    }
}

/// A single parsed cookie from a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cookie {
    name: String,
    value: String,
}

impl Cookie {
    /// Create a new cookie.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Get the cookie name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the cookie value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// `SameSite` cookie attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SameSite {
    /// Cookie is sent with cross-site requests.
    None,
    /// Cookie is sent with same-site and cross-site top-level navigations.
    #[default]
    Lax,
    /// Cookie is only sent with same-site requests.
    Strict,
}

impl fmt::Display for SameSite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "None"),
            Self::Lax => write!(f, "Lax"),
            Self::Strict => write!(f, "Strict"),
        }
    }
}

/// Builder for Set-Cookie response header.
///
/// # Example
///
/// ```rust
/// use archimedes_extract::cookie::{SetCookie, SameSite};
///
/// let cookie = SetCookie::new("session", "abc123")
///     .http_only(true)
///     .secure(true)
///     .same_site(SameSite::Strict)
///     .max_age_secs(3600)
///     .path("/");
///
/// let header = cookie.to_header_value();
/// assert!(header.contains("session=abc123"));
/// assert!(header.contains("HttpOnly"));
/// assert!(header.contains("Secure"));
/// assert!(header.contains("SameSite=Strict"));
/// ```
#[derive(Debug, Clone)]
pub struct SetCookie {
    name: String,
    value: String,
    domain: Option<String>,
    path: Option<String>,
    max_age: Option<Duration>,
    expires: Option<String>,
    secure: bool,
    http_only: bool,
    same_site: Option<SameSite>,
}

impl SetCookie {
    /// Create a new Set-Cookie builder.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            domain: None,
            path: None,
            max_age: None,
            expires: None,
            secure: false,
            http_only: false,
            same_site: None,
        }
    }

    /// Create a cookie that will be removed (Max-Age=0).
    #[must_use]
    pub fn remove(name: impl Into<String>) -> Self {
        Self::new(name, "")
            .max_age_secs(0)
    }

    /// Set the Domain attribute.
    #[must_use]
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set the Path attribute.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the Max-Age attribute.
    #[must_use]
    pub fn max_age(mut self, duration: Duration) -> Self {
        self.max_age = Some(duration);
        self
    }

    /// Set the Max-Age attribute in seconds.
    #[must_use]
    pub fn max_age_secs(self, seconds: u64) -> Self {
        self.max_age(Duration::from_secs(seconds))
    }

    /// Set the Expires attribute (HTTP date format).
    #[must_use]
    pub fn expires(mut self, date: impl Into<String>) -> Self {
        self.expires = Some(date.into());
        self
    }

    /// Set the Secure attribute.
    #[must_use]
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Set the `HttpOnly` attribute.
    #[must_use]
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    /// Set the `SameSite` attribute.
    #[must_use]
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = Some(same_site);
        self
    }

    /// Get the cookie name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the cookie value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Convert to Set-Cookie header value.
    #[must_use]
    pub fn to_header_value(&self) -> String {
        let mut parts = vec![format!("{}={}", self.name, self.value)];

        if let Some(ref domain) = self.domain {
            parts.push(format!("Domain={domain}"));
        }

        if let Some(ref path) = self.path {
            parts.push(format!("Path={path}"));
        }

        if let Some(max_age) = self.max_age {
            parts.push(format!("Max-Age={}", max_age.as_secs()));
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
            parts.push(format!("SameSite={same_site}"));
        }

        parts.join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{HeaderMap, HeaderValue, Method, Uri};
    use archimedes_router::Params;

    fn create_ctx_with_cookie(cookie_value: &str) -> ExtractionContext {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_str(cookie_value).unwrap());
        
        ExtractionContext::new(
            Method::GET,
            Uri::from_static("/test"),
            headers,
            Bytes::new(),
            Params::new(),
        )
    }

    #[test]
    fn test_parse_single_cookie() {
        let ctx = create_ctx_with_cookie("session=abc123");
        let cookies = Cookies::from_request(&ctx).unwrap();

        assert_eq!(cookies.get("session"), Some("abc123"));
        assert_eq!(cookies.len(), 1);
    }

    #[test]
    fn test_parse_multiple_cookies() {
        let ctx = create_ctx_with_cookie("session=abc123; theme=dark; lang=en");
        let cookies = Cookies::from_request(&ctx).unwrap();

        assert_eq!(cookies.get("session"), Some("abc123"));
        assert_eq!(cookies.get("theme"), Some("dark"));
        assert_eq!(cookies.get("lang"), Some("en"));
        assert_eq!(cookies.len(), 3);
    }

    #[test]
    fn test_parse_cookie_with_spaces() {
        let ctx = create_ctx_with_cookie("  session  =  abc123  ");
        let cookies = Cookies::from_request(&ctx).unwrap();

        assert_eq!(cookies.get("session"), Some("abc123"));
    }

    #[test]
    fn test_parse_quoted_value() {
        let ctx = create_ctx_with_cookie("name=\"John Doe\"");
        let cookies = Cookies::from_request(&ctx).unwrap();

        assert_eq!(cookies.get("name"), Some("John Doe"));
    }

    #[test]
    fn test_missing_cookie_header() {
        let ctx = ExtractionContext::new(
            Method::GET,
            Uri::from_static("/test"),
            HeaderMap::new(),
            Bytes::new(),
            Params::new(),
        );
        
        let cookies = Cookies::from_request(&ctx).unwrap();
        assert!(cookies.is_empty());
    }

    #[test]
    fn test_cookies_contains() {
        let ctx = create_ctx_with_cookie("session=abc123");
        let cookies = Cookies::from_request(&ctx).unwrap();

        assert!(cookies.contains("session"));
        assert!(!cookies.contains("missing"));
    }

    #[test]
    fn test_cookies_get_or() {
        let ctx = create_ctx_with_cookie("theme=dark");
        let cookies = Cookies::from_request(&ctx).unwrap();

        assert_eq!(cookies.get_or("theme", "light"), "dark");
        assert_eq!(cookies.get_or("missing", "default"), "default");
    }

    #[test]
    fn test_cookies_require() {
        let ctx = create_ctx_with_cookie("session=abc123");
        let cookies = Cookies::from_request(&ctx).unwrap();

        assert!(cookies.require("session").is_ok());
        assert!(cookies.require("missing").is_err());
    }

    #[test]
    fn test_cookies_iter() {
        let ctx = create_ctx_with_cookie("a=1; b=2");
        let cookies = Cookies::from_request(&ctx).unwrap();

        let collected: Vec<_> = cookies.iter().collect();
        assert_eq!(collected.len(), 2);
    }

    #[test]
    fn test_set_cookie_simple() {
        let cookie = SetCookie::new("session", "abc123");
        assert_eq!(cookie.to_header_value(), "session=abc123");
    }

    #[test]
    fn test_set_cookie_full() {
        let cookie = SetCookie::new("session", "abc123")
            .domain("example.com")
            .path("/app")
            .max_age_secs(3600)
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Strict);

        let header = cookie.to_header_value();
        assert!(header.contains("session=abc123"));
        assert!(header.contains("Domain=example.com"));
        assert!(header.contains("Path=/app"));
        assert!(header.contains("Max-Age=3600"));
        assert!(header.contains("Secure"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("SameSite=Strict"));
    }

    #[test]
    fn test_set_cookie_remove() {
        let cookie = SetCookie::remove("session");
        let header = cookie.to_header_value();
        
        assert!(header.contains("session="));
        assert!(header.contains("Max-Age=0"));
    }

    #[test]
    fn test_set_cookie_same_site_values() {
        assert_eq!(
            SetCookie::new("a", "1").same_site(SameSite::None).to_header_value(),
            "a=1; SameSite=None"
        );
        assert_eq!(
            SetCookie::new("a", "1").same_site(SameSite::Lax).to_header_value(),
            "a=1; SameSite=Lax"
        );
        assert_eq!(
            SetCookie::new("a", "1").same_site(SameSite::Strict).to_header_value(),
            "a=1; SameSite=Strict"
        );
    }

    #[test]
    fn test_cookie_struct() {
        let cookie = Cookie::new("name", "value");
        assert_eq!(cookie.name(), "name");
        assert_eq!(cookie.value(), "value");
    }

    #[test]
    fn test_same_site_display() {
        assert_eq!(SameSite::None.to_string(), "None");
        assert_eq!(SameSite::Lax.to_string(), "Lax");
        assert_eq!(SameSite::Strict.to_string(), "Strict");
    }

    #[test]
    fn test_set_cookie_getters() {
        let cookie = SetCookie::new("session", "abc123");
        assert_eq!(cookie.name(), "session");
        assert_eq!(cookie.value(), "abc123");
    }
}
