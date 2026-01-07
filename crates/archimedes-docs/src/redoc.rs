//! ReDoc documentation serving and generation.
//!
//! This module provides the `ReDoc` type for serving beautiful,
//! responsive API documentation using ReDoc.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use archimedes_docs::{ReDoc, OpenApi};
//!
//! let spec: OpenApi = /* your OpenAPI spec */;
//! let redoc = ReDoc::new("/redoc", &spec);
//!
//! // Get the HTML to serve
//! let html = redoc.html();
//! ```
//!
//! ## Features
//!
//! - Clean, responsive three-panel design
//! - Search functionality
//! - Code samples with syntax highlighting
//! - Schema explorer with interactive examples

use crate::openapi::OpenApi;

/// ReDoc configuration and HTML generation.
///
/// `ReDoc` generates a complete HTML page that loads ReDoc from a CDN
/// and renders your OpenAPI specification in a beautiful, readable format.
#[derive(Debug, Clone)]
pub struct ReDoc {
    /// Base path where ReDoc is served (e.g., "/redoc").
    path: String,
    /// The OpenAPI specification to display.
    spec: OpenApi,
    /// Title for the HTML page.
    title: String,
    /// Theme configuration.
    theme: ReDocTheme,
    /// Whether to expand responses by default.
    expand_responses: ExpandResponses,
    /// Whether to hide download button.
    hide_download_button: bool,
    /// Whether to hide hostname in server URL.
    hide_hostname: bool,
    /// Whether to disable search.
    disable_search: bool,
    /// ReDoc version to use from CDN.
    redoc_version: String,
}

/// Response expansion configuration.
#[derive(Debug, Clone, Copy, Default)]
pub enum ExpandResponses {
    /// Expand all responses.
    All,
    /// Expand only success responses (2xx).
    #[default]
    Success,
    /// Don't expand any responses.
    None,
}

impl ExpandResponses {
    fn as_js(&self) -> &'static str {
        match self {
            Self::All => "\"all\"",
            Self::Success => "\"200,201\"",
            Self::None => "\"\"",
        }
    }
}

/// ReDoc theme configuration.
#[derive(Debug, Clone)]
pub struct ReDocTheme {
    /// Primary color (hex).
    pub primary_color: String,
    /// Success color (hex).
    pub success_color: String,
    /// Warning color (hex).
    pub warning_color: String,
    /// Error color (hex).
    pub error_color: String,
    /// Font family for text.
    pub font_family: String,
    /// Font family for code.
    pub code_font_family: String,
}

impl Default for ReDocTheme {
    fn default() -> Self {
        Self {
            primary_color: "#32329f".to_string(),
            success_color: "#00aa00".to_string(),
            warning_color: "#d4ac0d".to_string(),
            error_color: "#e74c3c".to_string(),
            font_family: "Roboto, sans-serif".to_string(),
            code_font_family: "'Source Code Pro', monospace".to_string(),
        }
    }
}

impl ReDoc {
    /// Create a new ReDoc configuration.
    ///
    /// # Arguments
    ///
    /// * `path` - The URL path where ReDoc will be served
    /// * `spec` - The OpenAPI specification to display
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let redoc = ReDoc::new("/redoc", &my_spec);
    /// ```
    #[must_use]
    pub fn new(path: impl Into<String>, spec: &OpenApi) -> Self {
        let path = path.into();
        let title = format!("{} - API Documentation", spec.info.title);

        Self {
            path,
            spec: spec.clone(),
            title,
            theme: ReDocTheme::default(),
            expand_responses: ExpandResponses::Success,
            hide_download_button: false,
            hide_hostname: false,
            disable_search: false,
            redoc_version: "2.1.5".to_string(),
        }
    }

    /// Set the page title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the theme.
    #[must_use]
    pub fn theme(mut self, theme: ReDocTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the primary color.
    #[must_use]
    pub fn primary_color(mut self, color: impl Into<String>) -> Self {
        self.theme.primary_color = color.into();
        self
    }

    /// Set response expansion behavior.
    #[must_use]
    pub fn expand_responses(mut self, expand: ExpandResponses) -> Self {
        self.expand_responses = expand;
        self
    }

    /// Hide the download button.
    #[must_use]
    pub fn hide_download_button(mut self, hide: bool) -> Self {
        self.hide_download_button = hide;
        self
    }

    /// Hide hostname in server URLs.
    #[must_use]
    pub fn hide_hostname(mut self, hide: bool) -> Self {
        self.hide_hostname = hide;
        self
    }

    /// Disable search functionality.
    #[must_use]
    pub fn disable_search(mut self, disable: bool) -> Self {
        self.disable_search = disable;
        self
    }

    /// Set the ReDoc version to use.
    #[must_use]
    pub fn redoc_version(mut self, version: impl Into<String>) -> Self {
        self.redoc_version = version.into();
        self
    }

    /// Get the base path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the path for the OpenAPI JSON specification.
    #[must_use]
    pub fn spec_path(&self) -> String {
        format!("{}/openapi.json", self.path.trim_end_matches('/'))
    }

    /// Get the OpenAPI specification as JSON.
    #[must_use]
    pub fn spec_json(&self) -> String {
        serde_json::to_string_pretty(&self.spec).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate the HTML for ReDoc.
    ///
    /// This returns a complete HTML page that can be served directly.
    /// The page loads ReDoc from a CDN and initializes it with
    /// the embedded OpenAPI specification.
    #[must_use]
    pub fn html(&self) -> String {
        let spec_json = self.spec_json();

        format!(
            r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link href="https://fonts.googleapis.com/css2?family=Roboto:wght@300;400;500;700&family=Source+Code+Pro:wght@400;600&display=swap" rel="stylesheet">
    <style>
        body {{
            margin: 0;
            padding: 0;
        }}
    </style>
</head>
<body>
    <div id="redoc-container"></div>
    <script src="https://cdn.redoc.ly/redoc/{version}/bundles/redoc.standalone.js"></script>
    <script>
        const spec = {spec_json};
        
        Redoc.init(spec, {{
            expandResponses: {expand_responses},
            hideDownloadButton: {hide_download},
            hideHostname: {hide_hostname},
            disableSearch: {disable_search},
            theme: {{
                colors: {{
                    primary: {{
                        main: "{primary_color}"
                    }},
                    success: {{
                        main: "{success_color}"
                    }},
                    warning: {{
                        main: "{warning_color}"
                    }},
                    error: {{
                        main: "{error_color}"
                    }}
                }},
                typography: {{
                    fontFamily: "{font_family}",
                    code: {{
                        fontFamily: "{code_font_family}"
                    }}
                }},
                sidebar: {{
                    backgroundColor: "#fafafa"
                }}
            }}
        }}, document.getElementById('redoc-container'));
    </script>
</body>
</html>"##,
            title = html_escape(&self.title),
            version = self.redoc_version,
            spec_json = spec_json,
            expand_responses = self.expand_responses.as_js(),
            hide_download = self.hide_download_button.to_string(),
            hide_hostname = self.hide_hostname.to_string(),
            disable_search = self.disable_search.to_string(),
            primary_color = self.theme.primary_color,
            success_color = self.theme.success_color,
            warning_color = self.theme.warning_color,
            error_color = self.theme.error_color,
            font_family = self.theme.font_family,
            code_font_family = self.theme.code_font_family,
        )
    }

    /// Generate the HTML as bytes for use in HTTP responses.
    #[must_use]
    pub fn html_bytes(&self) -> bytes::Bytes {
        bytes::Bytes::from(self.html())
    }
}

/// Simple HTML escape for XSS prevention.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openapi::Info;

    fn create_test_spec() -> OpenApi {
        OpenApi {
            openapi: "3.1.0".to_string(),
            info: Info {
                title: "Test API".to_string(),
                version: "1.0.0".to_string(),
                description: Some("A test API".to_string()),
                terms_of_service: None,
                contact: None,
                license: None,
            },
            servers: vec![],
            paths: indexmap::IndexMap::new(),
            components: None,
            tags: vec![],
            external_docs: None,
        }
    }

    #[test]
    fn test_redoc_creation() {
        let spec = create_test_spec();
        let redoc = ReDoc::new("/redoc", &spec);

        assert_eq!(redoc.path(), "/redoc");
        assert_eq!(redoc.spec_path(), "/redoc/openapi.json");
    }

    #[test]
    fn test_redoc_customization() {
        let spec = create_test_spec();
        let redoc = ReDoc::new("/api/redoc", &spec)
            .title("Custom Docs")
            .primary_color("#ff0000")
            .expand_responses(ExpandResponses::All)
            .hide_download_button(true)
            .disable_search(true);

        assert_eq!(redoc.title, "Custom Docs");
        assert_eq!(redoc.theme.primary_color, "#ff0000");
        assert!(redoc.hide_download_button);
        assert!(redoc.disable_search);
    }

    #[test]
    fn test_redoc_html_generation() {
        let spec = create_test_spec();
        let redoc = ReDoc::new("/redoc", &spec);
        let html = redoc.html();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("redoc"));
        assert!(html.contains("Test API"));
    }

    #[test]
    fn test_redoc_theme() {
        let theme = ReDocTheme::default();
        assert_eq!(theme.primary_color, "#32329f");
        assert!(theme.font_family.contains("Roboto"));
    }

    #[test]
    fn test_expand_responses_as_js() {
        assert_eq!(ExpandResponses::All.as_js(), "\"all\"");
        assert_eq!(ExpandResponses::Success.as_js(), "\"200,201\"");
        assert_eq!(ExpandResponses::None.as_js(), "\"\"");
    }

    #[test]
    fn test_html_bytes() {
        let spec = create_test_spec();
        let redoc = ReDoc::new("/redoc", &spec);
        let bytes = redoc.html_bytes();

        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_spec_json() {
        let spec = create_test_spec();
        let redoc = ReDoc::new("/redoc", &spec);
        let json = redoc.spec_json();

        assert!(json.contains("Test API"));
        assert!(json.contains("3.1.0"));
    }

    #[test]
    fn test_custom_theme() {
        let spec = create_test_spec();
        let theme = ReDocTheme {
            primary_color: "#123456".to_string(),
            success_color: "#00ff00".to_string(),
            warning_color: "#ffff00".to_string(),
            error_color: "#ff0000".to_string(),
            font_family: "Arial, sans-serif".to_string(),
            code_font_family: "Consolas, monospace".to_string(),
        };

        let redoc = ReDoc::new("/redoc", &spec).theme(theme);
        let html = redoc.html();

        assert!(html.contains("#123456"));
        assert!(html.contains("Arial"));
    }
}
