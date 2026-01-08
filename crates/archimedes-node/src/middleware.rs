//! Middleware processing for the request pipeline.

use crate::context::RequestContext;
use crate::response::Response;
use napi_derive::napi;
use serde::{Deserialize, Serialize};

/// Result of middleware processing (internal use).
#[derive(Debug, Clone)]
pub struct MiddlewareResult {
    /// Whether to continue processing
    pub continue_processing: bool,

    /// Early response (if processing should stop)
    pub response: Option<Response>,

    /// Updated request context
    pub context: RequestContext,

    /// Middleware that processed the request
    pub processed_by: Vec<String>,
}

/// Middleware result exposed to JavaScript.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareResultJs {
    /// Whether to continue processing
    pub continue_processing: bool,

    /// Whether there was an early response
    pub has_response: bool,

    /// HTTP status code of early response (if any)
    pub response_status: Option<u16>,

    /// Middleware that processed the request
    pub processed_by: Vec<String>,
}

impl From<&MiddlewareResult> for MiddlewareResultJs {
    fn from(result: &MiddlewareResult) -> Self {
        Self {
            continue_processing: result.continue_processing,
            has_response: result.response.is_some(),
            response_status: result.response.as_ref().map(Response::status_code),
            processed_by: result.processed_by.clone(),
        }
    }
}

/// Request ID middleware - adds unique ID to each request.
fn request_id_middleware_internal(mut ctx: RequestContext) -> MiddlewareResult {
    if ctx.request_id.is_empty() {
        ctx.request_id = uuid::Uuid::new_v4().to_string();
    }
    MiddlewareResult {
        continue_processing: true,
        response: None,
        context: ctx,
        processed_by: vec!["request_id".to_string()],
    }
}

/// Tracing middleware - adds trace context to request.
fn tracing_middleware_internal(mut ctx: RequestContext) -> MiddlewareResult {
    // Add trace ID if not present
    if !ctx.custom.contains_key("trace_id") {
        let trace_id = ctx
            .headers
            .get("x-trace-id")
            .cloned()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        ctx.custom.insert("trace_id".to_string(), trace_id);
    }

    // Add span ID
    if !ctx.custom.contains_key("span_id") {
        let span_id = uuid::Uuid::new_v4().to_string()[..16].to_string();
        ctx.custom.insert("span_id".to_string(), span_id);
    }

    MiddlewareResult {
        continue_processing: true,
        response: None,
        context: ctx,
        processed_by: vec!["tracing".to_string()],
    }
}

/// Identity extraction middleware - extracts identity from headers.
fn identity_middleware_internal(mut ctx: RequestContext) -> MiddlewareResult {
    use crate::context::Identity;

    // Try to extract identity from Authorization header
    if let Some(auth_header) = ctx.headers.get("authorization") {
        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            // In real implementation, would decode JWT and extract claims
            // For now, create a mock identity from the token
            if !token.is_empty() {
                ctx.identity = Some(Identity {
                    subject: Some(format!("token:{}", &token[..8.min(token.len())])),
                    issuer: None,
                    audience: None,
                    expires_at: None,
                    issued_at: None,
                    roles: None,
                    scopes: None,
                    claims: None,
                });
            }
        }
    }

    // Also check X-User-ID header for simple auth
    if ctx.identity.is_none() {
        if let Some(user_id) = ctx.headers.get("x-user-id") {
            ctx.identity = Some(Identity {
                subject: Some(user_id.clone()),
                issuer: Some("x-user-id".to_string()),
                audience: None,
                expires_at: None,
                issued_at: None,
                roles: ctx
                    .headers
                    .get("x-user-roles")
                    .map(|r| r.split(',').map(|s| s.trim().to_string()).collect()),
                scopes: None,
                claims: None,
            });
        }
    }

    MiddlewareResult {
        continue_processing: true,
        response: None,
        context: ctx,
        processed_by: vec!["identity".to_string()],
    }
}

/// Process a request through the standard middleware pipeline.
///
/// Order: `request_id` → tracing → identity
///
/// Returns the processed request context and a summary.
pub fn process_request(ctx: RequestContext) -> MiddlewareResult {
    let mut result = request_id_middleware_internal(ctx);
    if !result.continue_processing {
        return result;
    }

    let tracing_result = tracing_middleware_internal(result.context);
    result.context = tracing_result.context;
    result.processed_by.extend(tracing_result.processed_by);
    if !tracing_result.continue_processing {
        result.continue_processing = false;
        result.response = tracing_result.response;
        return result;
    }

    let identity_result = identity_middleware_internal(result.context);
    result.context = identity_result.context;
    result.processed_by.extend(identity_result.processed_by);
    result.continue_processing = identity_result.continue_processing;
    result.response = identity_result.response;

    result
}

/// Apply request ID middleware.
#[napi]
pub fn apply_request_id(ctx: RequestContext) -> RequestContext {
    let result = request_id_middleware_internal(ctx);
    result.context
}

/// Apply tracing middleware.
#[napi]
pub fn apply_tracing(ctx: RequestContext) -> RequestContext {
    let result = tracing_middleware_internal(ctx);
    result.context
}

/// Apply identity middleware.
#[napi]
pub fn apply_identity(ctx: RequestContext) -> RequestContext {
    let result = identity_middleware_internal(ctx);
    result.context
}

/// Apply all standard middleware and return processed context.
#[napi]
pub fn apply_all_middleware(ctx: RequestContext) -> RequestContext {
    let result = process_request(ctx);
    result.context
}

/// Get middleware processing summary.
#[napi]
pub fn get_middleware_summary(ctx: RequestContext) -> MiddlewareResultJs {
    let result = process_request(ctx);
    MiddlewareResultJs::from(&result)
}

/// Normalize error response - adds request ID header.
#[napi]
pub fn normalize_error_response_header(
    status_code: u16,
    request_id: String,
) -> std::collections::HashMap<String, String> {
    let mut headers = std::collections::HashMap::new();
    if status_code >= 400 {
        headers.insert("x-request-id".to_string(), request_id);
        headers.insert("content-type".to_string(), "application/json".to_string());
    }
    headers
}

/// Middleware configuration options.
#[napi(object)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    /// Enable request ID middleware
    pub enable_request_id: Option<bool>,

    /// Enable tracing middleware
    pub enable_tracing: Option<bool>,

    /// Enable identity extraction middleware
    pub enable_identity: Option<bool>,

    /// Enable validation middleware
    pub enable_validation: Option<bool>,

    /// Enable authorization middleware
    pub enable_authorization: Option<bool>,

    /// Custom middleware names to enable
    pub custom: Option<Vec<String>>,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            enable_request_id: Some(true),
            enable_tracing: Some(true),
            enable_identity: Some(true),
            enable_validation: Some(true),
            enable_authorization: Some(true),
            custom: None,
        }
    }
}

/// Create default middleware configuration.
#[napi]
pub fn default_middleware_config() -> MiddlewareConfig {
    MiddlewareConfig::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_context() -> RequestContext {
        RequestContext {
            request_id: String::new(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            operation_id: None,
            path_params: std::collections::HashMap::new(),
            query_params: std::collections::HashMap::new(),
            headers: std::collections::HashMap::new(),
            body: None,
            body_json: None,
            identity: None,
            client_ip: None,
            content_type: None,
            accept: None,
            custom: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_request_id_middleware() {
        let ctx = test_context();
        let result = request_id_middleware_internal(ctx);

        assert!(result.continue_processing);
        assert!(!result.context.request_id.is_empty());
        assert!(result.processed_by.contains(&"request_id".to_string()));
    }

    #[test]
    fn test_request_id_middleware_preserves_existing() {
        let mut ctx = test_context();
        ctx.request_id = "existing-id".to_string();

        let result = request_id_middleware_internal(ctx);
        assert_eq!(result.context.request_id, "existing-id");
    }

    #[test]
    fn test_tracing_middleware() {
        let ctx = test_context();
        let result = tracing_middleware_internal(ctx);

        assert!(result.continue_processing);
        assert!(result.context.custom.contains_key("trace_id"));
        assert!(result.context.custom.contains_key("span_id"));
    }

    #[test]
    fn test_tracing_middleware_uses_header() {
        let mut ctx = test_context();
        ctx.headers
            .insert("x-trace-id".to_string(), "custom-trace-id".to_string());

        let result = tracing_middleware_internal(ctx);

        assert_eq!(
            result.context.custom.get("trace_id"),
            Some(&"custom-trace-id".to_string())
        );
    }

    #[test]
    fn test_identity_middleware_bearer_token() {
        let mut ctx = test_context();
        ctx.headers.insert(
            "authorization".to_string(),
            "Bearer test-token-12345".to_string(),
        );

        let result = identity_middleware_internal(ctx);

        assert!(result.context.identity.is_some());
        let identity = result.context.identity.unwrap();
        assert!(identity.subject.unwrap().starts_with("token:"));
    }

    #[test]
    fn test_identity_middleware_x_user_id() {
        let mut ctx = test_context();
        ctx.headers
            .insert("x-user-id".to_string(), "user123".to_string());
        ctx.headers
            .insert("x-user-roles".to_string(), "admin, user".to_string());

        let result = identity_middleware_internal(ctx);

        assert!(result.context.identity.is_some());
        let identity = result.context.identity.unwrap();
        assert_eq!(identity.subject, Some("user123".to_string()));
        assert!(identity.roles.is_some());
        let roles = identity.roles.unwrap();
        assert!(roles.contains(&"admin".to_string()));
        assert!(roles.contains(&"user".to_string()));
    }

    #[test]
    fn test_identity_middleware_no_auth() {
        let ctx = test_context();
        let result = identity_middleware_internal(ctx);
        assert!(result.context.identity.is_none());
    }

    #[test]
    fn test_process_request() {
        let mut ctx = test_context();
        ctx.headers
            .insert("authorization".to_string(), "Bearer test-token".to_string());

        let result = process_request(ctx);

        assert!(result.continue_processing);
        assert!(!result.context.request_id.is_empty());
        assert!(result.context.custom.contains_key("trace_id"));
        assert!(result.context.identity.is_some());
        assert_eq!(result.processed_by.len(), 3);
    }

    #[test]
    fn test_apply_request_id() {
        let ctx = test_context();
        let processed = apply_request_id(ctx);
        assert!(!processed.request_id.is_empty());
    }

    #[test]
    fn test_apply_tracing() {
        let ctx = test_context();
        let processed = apply_tracing(ctx);
        assert!(processed.custom.contains_key("trace_id"));
    }

    #[test]
    fn test_apply_all_middleware() {
        let ctx = test_context();
        let processed = apply_all_middleware(ctx);

        assert!(!processed.request_id.is_empty());
        assert!(processed.custom.contains_key("trace_id"));
    }

    #[test]
    fn test_get_middleware_summary() {
        let ctx = test_context();
        let summary = get_middleware_summary(ctx);

        assert!(summary.continue_processing);
        assert!(!summary.has_response);
        assert_eq!(summary.processed_by.len(), 3);
    }

    #[test]
    fn test_middleware_config_default() {
        let config = default_middleware_config();
        assert_eq!(config.enable_request_id, Some(true));
        assert_eq!(config.enable_tracing, Some(true));
        assert_eq!(config.enable_identity, Some(true));
    }

    #[test]
    fn test_normalize_error_response_header() {
        let headers = normalize_error_response_header(404, "req-123".to_string());
        assert_eq!(headers.get("x-request-id"), Some(&"req-123".to_string()));
        assert_eq!(
            headers.get("content-type"),
            Some(&"application/json".to_string())
        );
    }

    #[test]
    fn test_normalize_error_response_header_success() {
        let headers = normalize_error_response_header(200, "req-123".to_string());
        assert!(headers.is_empty());
    }
}
