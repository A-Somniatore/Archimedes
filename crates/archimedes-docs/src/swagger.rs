//! Swagger UI serving and generation.
//!
//! This module provides the `SwaggerUi` type for serving an interactive
//! Swagger UI documentation interface for your API.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use archimedes_docs::{SwaggerUi, OpenApi};
//!
//! let spec: OpenApi = /* your OpenAPI spec */;
//! let swagger = SwaggerUi::new("/docs", &spec);
//!
//! // Get the HTML to serve
//! let html = swagger.html();
//!
//! // Get the JSON spec to serve at /docs/openapi.json
//! let json = swagger.spec_json();
//! ```

use crate::openapi::OpenApi;

/// Swagger UI configuration and HTML generation.
///
/// `SwaggerUi` generates a complete HTML page that loads Swagger UI
/// from a CDN and renders your OpenAPI specification.
#[derive(Debug, Clone)]
pub struct SwaggerUi {
    /// Base path where Swagger UI is served (e.g., "/docs").
    path: String,
    /// The OpenAPI specification to display.
    spec: OpenApi,
    /// Title for the HTML page.
    title: String,
    /// Whether to use deep linking (URL updates with operations).
    deep_linking: bool,
    /// Default expansion depth for operations.
    doc_expansion: DocExpansion,
    /// Whether to display the request duration.
    display_request_duration: bool,
    /// Swagger UI version to use from CDN.
    swagger_version: String,
}

/// Document expansion level for Swagger UI.
#[derive(Debug, Clone, Copy, Default)]
pub enum DocExpansion {
    /// Show all operations collapsed.
    None,
    /// Show only the list of operations.
    #[default]
    List,
    /// Expand all operations fully.
    Full,
}

impl DocExpansion {
    fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::List => "list",
            Self::Full => "full",
        }
    }
}

impl SwaggerUi {
    /// Create a new Swagger UI configuration.
    ///
    /// # Arguments
    ///
    /// * `path` - The base URL path where Swagger UI will be served
    /// * `spec` - The OpenAPI specification to display
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let swagger = SwaggerUi::new("/docs", &my_spec);
    /// ```
    #[must_use]
    pub fn new(path: impl Into<String>, spec: &OpenApi) -> Self {
        let path = path.into();
        let title = format!("{} - Swagger UI", spec.info.title);

        Self {
            path,
            spec: spec.clone(),
            title,
            deep_linking: true,
            doc_expansion: DocExpansion::List,
            display_request_duration: true,
            swagger_version: "5.18.2".to_string(),
        }
    }

    /// Set the page title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Enable or disable deep linking.
    #[must_use]
    pub fn deep_linking(mut self, enabled: bool) -> Self {
        self.deep_linking = enabled;
        self
    }

    /// Set the document expansion level.
    #[must_use]
    pub fn doc_expansion(mut self, expansion: DocExpansion) -> Self {
        self.doc_expansion = expansion;
        self
    }

    /// Enable or disable request duration display.
    #[must_use]
    pub fn display_request_duration(mut self, enabled: bool) -> Self {
        self.display_request_duration = enabled;
        self
    }

    /// Set the Swagger UI version to use.
    #[must_use]
    pub fn swagger_version(mut self, version: impl Into<String>) -> Self {
        self.swagger_version = version.into();
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

    /// Generate the HTML for Swagger UI.
    ///
    /// This returns a complete HTML page that can be served directly.
    /// The page loads Swagger UI from a CDN and initializes it with
    /// the embedded OpenAPI specification.
    #[must_use]
    pub fn html(&self) -> String {
        let spec_json = self.spec_json();
        let deep_linking = self.deep_linking.to_string();
        let doc_expansion = self.doc_expansion.as_str();
        let display_duration = self.display_request_duration.to_string();

        format!(
            r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@{version}/swagger-ui.css" />
    <style>
        html {{
            box-sizing: border-box;
            overflow: -moz-scrollbars-vertical;
            overflow-y: scroll;
        }}
        *,
        *:before,
        *:after {{
            box-sizing: inherit;
        }}
        body {{
            margin: 0;
            background: #fafafa;
        }}
    </style>
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@{version}/swagger-ui-bundle.js"></script>
    <script src="https://unpkg.com/swagger-ui-dist@{version}/swagger-ui-standalone-preset.js"></script>
    <script>
        window.onload = function() {{
            const spec = {spec_json};
            
            window.ui = SwaggerUIBundle({{
                spec: spec,
                dom_id: '#swagger-ui',
                deepLinking: {deep_linking},
                docExpansion: '{doc_expansion}',
                displayRequestDuration: {display_duration},
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIStandalonePreset
                ],
                plugins: [
                    SwaggerUIBundle.plugins.DownloadUrl
                ],
                layout: "StandaloneLayout"
            }});
        }};
    </script>
</body>
</html>"##,
            title = html_escape(&self.title),
            version = self.swagger_version,
            spec_json = spec_json,
            deep_linking = deep_linking,
            doc_expansion = doc_expansion,
            display_duration = display_duration,
        )
    }

    /// Generate the HTML as bytes for use in HTTP responses.
    #[must_use]
    pub fn html_bytes(&self) -> bytes::Bytes {
        bytes::Bytes::from(self.html())
    }
}

/// Simple HTML escape for XSS prevention in the title.
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
    fn test_swagger_ui_creation() {
        let spec = create_test_spec();
        let swagger = SwaggerUi::new("/docs", &spec);

        assert_eq!(swagger.path(), "/docs");
        assert_eq!(swagger.spec_path(), "/docs/openapi.json");
    }

    #[test]
    fn test_swagger_ui_customization() {
        let spec = create_test_spec();
        let swagger = SwaggerUi::new("/api/docs", &spec)
            .title("Custom Title")
            .deep_linking(false)
            .doc_expansion(DocExpansion::Full)
            .display_request_duration(false)
            .swagger_version("5.0.0");

        assert_eq!(swagger.title, "Custom Title");
        assert!(!swagger.deep_linking);
        assert!(!swagger.display_request_duration);
    }

    #[test]
    fn test_swagger_ui_html_generation() {
        let spec = create_test_spec();
        let swagger = SwaggerUi::new("/docs", &spec);
        let html = swagger.html();

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("swagger-ui"));
        assert!(html.contains("Test API"));
        assert!(html.contains("1.0.0"));
    }

    #[test]
    fn test_swagger_ui_spec_json() {
        let spec = create_test_spec();
        let swagger = SwaggerUi::new("/docs", &spec);
        let json = swagger.spec_json();

        assert!(json.contains("3.1.0"));
        assert!(json.contains("Test API"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("\"test\""), "&quot;test&quot;");
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }

    #[test]
    fn test_doc_expansion_as_str() {
        assert_eq!(DocExpansion::None.as_str(), "none");
        assert_eq!(DocExpansion::List.as_str(), "list");
        assert_eq!(DocExpansion::Full.as_str(), "full");
    }

    #[test]
    fn test_spec_path_trailing_slash() {
        let spec = create_test_spec();
        let swagger = SwaggerUi::new("/docs/", &spec);
        assert_eq!(swagger.spec_path(), "/docs/openapi.json");
    }

    #[test]
    fn test_html_bytes() {
        let spec = create_test_spec();
        let swagger = SwaggerUi::new("/docs", &spec);
        let bytes = swagger.html_bytes();

        assert!(!bytes.is_empty());
        assert!(bytes.len() > 100);
    }
}
