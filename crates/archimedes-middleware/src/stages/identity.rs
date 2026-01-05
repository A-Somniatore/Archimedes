//! Identity extraction middleware.
//!
//! This middleware extracts caller identity from incoming requests.
//! It supports multiple identity sources:
//!
//! - **SPIFFE**: From mTLS client certificates (service-to-service)
//! - **JWT**: From Authorization Bearer tokens (user requests)
//! - **API Key**: From X-API-Key header (external integrations)
//!
//! ## Identity Precedence
//!
//! When multiple identity sources are present, precedence is:
//! 1. SPIFFE (mTLS) - highest trust for internal services
//! 2. JWT - user authentication
//! 3. API Key - external access
//! 4. Anonymous - no credentials provided
//!
//! ## SPIFFE Identity
//!
//! For internal service-to-service communication, identity is extracted
//! from the client's mTLS certificate SPIFFE ID (typically via a header
//! set by the ingress/sidecar proxy).

use crate::context::MiddlewareContext;
use crate::middleware::{BoxFuture, Middleware, Next};
use crate::types::{Request, Response};
use archimedes_core::CallerIdentity;

/// Header for SPIFFE ID (set by ingress/sidecar).
pub const SPIFFE_ID_HEADER: &str = "x-spiffe-id";

/// Header for API key authentication.
pub const API_KEY_HEADER: &str = "x-api-key";

/// Authorization header for JWT tokens.
pub const AUTHORIZATION_HEADER: &str = "authorization";

/// Middleware that extracts caller identity from requests.
///
/// This middleware populates the [`MiddlewareContext::identity`] field
/// based on authentication credentials in the request.
///
/// # Behavior
///
/// 1. Check for SPIFFE ID header (highest precedence)
/// 2. Check for Authorization Bearer token (JWT)
/// 3. Check for X-API-Key header
/// 4. Default to Anonymous if no credentials
///
/// # Example
///
/// ```ignore
/// use archimedes_middleware::stages::identity::IdentityMiddleware;
///
/// let middleware = IdentityMiddleware::new();
/// // Add to pipeline...
/// ```
#[derive(Debug, Clone, Default)]
pub struct IdentityMiddleware {
    /// Trusted SPIFFE trust domain for validation.
    trusted_trust_domain: Option<String>,
}

impl IdentityMiddleware {
    /// Creates a new Identity middleware.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an Identity middleware with a trusted SPIFFE trust domain.
    ///
    /// SPIFFE IDs from other trust domains will be rejected.
    #[must_use]
    pub fn with_trust_domain(trust_domain: impl Into<String>) -> Self {
        Self {
            trusted_trust_domain: Some(trust_domain.into()),
        }
    }

    /// Extracts SPIFFE identity from headers.
    fn extract_spiffe_identity(&self, request: &Request) -> Option<CallerIdentity> {
        let spiffe_id = request.headers().get(SPIFFE_ID_HEADER)?.to_str().ok()?;

        // Validate SPIFFE ID format
        if !spiffe_id.starts_with("spiffe://") {
            return None;
        }

        // Validate trust domain if configured
        if let Some(ref trusted_domain) = self.trusted_trust_domain {
            let uri_part = &spiffe_id[9..]; // Skip "spiffe://"
            let domain = uri_part.split('/').next()?;
            if domain != trusted_domain {
                return None;
            }
        }

        Some(CallerIdentity::spiffe(spiffe_id))
    }

    /// Extracts JWT identity from Authorization header.
    fn extract_jwt_identity(&self, request: &Request) -> Option<CallerIdentity> {
        let auth_header = request.headers().get(AUTHORIZATION_HEADER)?.to_str().ok()?;

        // Check for Bearer token
        if !auth_header.starts_with("Bearer ") {
            return None;
        }

        let token = &auth_header[7..]; // Skip "Bearer "

        // In a real implementation, we would:
        // 1. Validate the JWT signature
        // 2. Check expiration
        // 3. Extract claims
        // For mock implementation, we extract a basic user identity

        // Parse mock JWT (base64 encoded JSON with user_id)
        // Real implementation would use a JWT library
        Some(self.parse_mock_jwt(token))
    }

    /// Parses a mock JWT for testing.
    ///
    /// In production, this would be replaced with proper JWT validation.
    fn parse_mock_jwt(&self, token: &str) -> CallerIdentity {
        // For mock implementation, use token as user ID
        // Real implementation would decode and validate the JWT
        CallerIdentity::User {
            user_id: format!("jwt:{}", &token[..std::cmp::min(16, token.len())]),
            email: None,
            name: None,
            roles: vec![],
        }
    }

    /// Extracts API key identity from headers.
    fn extract_api_key_identity(&self, request: &Request) -> Option<CallerIdentity> {
        let api_key = request.headers().get(API_KEY_HEADER)?.to_str().ok()?;

        // In a real implementation, we would:
        // 1. Look up the API key in a database
        // 2. Get associated scopes and metadata
        // For mock implementation, use the key as the ID

        Some(CallerIdentity::ApiKey {
            key_id: api_key.to_string(),
            name: None,
            scopes: vec![],
        })
    }
}

impl Middleware for IdentityMiddleware {
    fn name(&self) -> &'static str {
        "identity"
    }

    fn process<'a>(
        &'a self,
        ctx: &'a mut MiddlewareContext,
        request: Request,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            // Extract identity with precedence: SPIFFE > JWT > API Key > Anonymous
            let identity = self
                .extract_spiffe_identity(&request)
                .or_else(|| self.extract_jwt_identity(&request))
                .or_else(|| self.extract_api_key_identity(&request))
                .unwrap_or(CallerIdentity::Anonymous);

            // Store in context
            ctx.set_identity(identity);

            // Process request through remaining middleware
            next.run(ctx, request).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use http::{Request as HttpRequest, Response as HttpResponse, StatusCode};
    use http_body_util::Full;

    fn create_test_request() -> Request {
        HttpRequest::builder()
            .uri("/test")
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_request_with_spiffe(spiffe_id: &str) -> Request {
        HttpRequest::builder()
            .uri("/test")
            .header(SPIFFE_ID_HEADER, spiffe_id)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_request_with_jwt(token: &str) -> Request {
        HttpRequest::builder()
            .uri("/test")
            .header(AUTHORIZATION_HEADER, format!("Bearer {}", token))
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_request_with_api_key(key: &str) -> Request {
        HttpRequest::builder()
            .uri("/test")
            .header(API_KEY_HEADER, key)
            .body(Full::new(Bytes::new()))
            .unwrap()
    }

    fn create_handler()
    -> impl FnOnce(&mut MiddlewareContext, Request) -> BoxFuture<'static, Response> {
        |_ctx, _req| {
            Box::pin(async {
                HttpResponse::builder()
                    .status(StatusCode::OK)
                    .body(Full::new(Bytes::from("OK")))
                    .unwrap()
            })
        }
    }

    #[tokio::test]
    async fn test_anonymous_when_no_credentials() {
        let middleware = IdentityMiddleware::new();
        let mut ctx = MiddlewareContext::new();
        let request = create_test_request();

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        assert!(matches!(ctx.identity(), CallerIdentity::Anonymous));
    }

    #[tokio::test]
    async fn test_extracts_spiffe_identity() {
        let middleware = IdentityMiddleware::new();
        let mut ctx = MiddlewareContext::new();
        let request = create_request_with_spiffe("spiffe://example.org/service/users");

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        match ctx.identity() {
            CallerIdentity::Spiffe { spiffe_id } => {
                assert_eq!(spiffe_id, "spiffe://example.org/service/users");
            }
            _ => panic!("Expected SPIFFE identity"),
        }
    }

    #[tokio::test]
    async fn test_validates_trust_domain() {
        let middleware = IdentityMiddleware::with_trust_domain("trusted.org");
        let mut ctx = MiddlewareContext::new();
        let request = create_request_with_spiffe("spiffe://untrusted.org/service/bad");

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        // Should reject untrusted domain and fall back to anonymous
        assert!(matches!(ctx.identity(), CallerIdentity::Anonymous));
    }

    #[tokio::test]
    async fn test_accepts_trusted_domain() {
        let middleware = IdentityMiddleware::with_trust_domain("trusted.org");
        let mut ctx = MiddlewareContext::new();
        let request = create_request_with_spiffe("spiffe://trusted.org/service/good");

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        match ctx.identity() {
            CallerIdentity::Spiffe { spiffe_id } => {
                assert_eq!(spiffe_id, "spiffe://trusted.org/service/good");
            }
            _ => panic!("Expected SPIFFE identity"),
        }
    }

    #[tokio::test]
    async fn test_extracts_jwt_identity() {
        let middleware = IdentityMiddleware::new();
        let mut ctx = MiddlewareContext::new();
        let request = create_request_with_jwt("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test");

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        match ctx.identity() {
            CallerIdentity::User { user_id, .. } => {
                assert!(user_id.starts_with("jwt:"));
            }
            _ => panic!("Expected User identity"),
        }
    }

    #[tokio::test]
    async fn test_extracts_api_key_identity() {
        let middleware = IdentityMiddleware::new();
        let mut ctx = MiddlewareContext::new();
        let request = create_request_with_api_key("ak_test_12345");

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        match ctx.identity() {
            CallerIdentity::ApiKey { key_id, .. } => {
                assert_eq!(key_id, "ak_test_12345");
            }
            _ => panic!("Expected ApiKey identity"),
        }
    }

    #[tokio::test]
    async fn test_spiffe_takes_precedence_over_jwt() {
        let middleware = IdentityMiddleware::new();
        let mut ctx = MiddlewareContext::new();

        // Request with both SPIFFE and JWT
        let request = HttpRequest::builder()
            .uri("/test")
            .header(SPIFFE_ID_HEADER, "spiffe://example.org/service/users")
            .header(AUTHORIZATION_HEADER, "Bearer some-jwt-token")
            .body(Full::new(Bytes::new()))
            .unwrap();

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        // Should use SPIFFE (higher precedence)
        assert!(matches!(ctx.identity(), CallerIdentity::Spiffe { .. }));
    }

    #[tokio::test]
    async fn test_jwt_takes_precedence_over_api_key() {
        let middleware = IdentityMiddleware::new();
        let mut ctx = MiddlewareContext::new();

        // Request with both JWT and API key
        let request = HttpRequest::builder()
            .uri("/test")
            .header(AUTHORIZATION_HEADER, "Bearer some-jwt-token")
            .header(API_KEY_HEADER, "ak_test_12345")
            .body(Full::new(Bytes::new()))
            .unwrap();

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        // Should use JWT (higher precedence)
        assert!(matches!(ctx.identity(), CallerIdentity::User { .. }));
    }

    #[tokio::test]
    async fn test_rejects_invalid_spiffe_format() {
        let middleware = IdentityMiddleware::new();
        let mut ctx = MiddlewareContext::new();
        let request = create_request_with_spiffe("not-a-spiffe-id");

        let next = Next::handler(create_handler());
        let _response = middleware.process(&mut ctx, request, next).await;

        // Should fall back to anonymous
        assert!(matches!(ctx.identity(), CallerIdentity::Anonymous));
    }

    #[test]
    fn test_middleware_name() {
        let middleware = IdentityMiddleware::new();
        assert_eq!(middleware.name(), "identity");
    }
}
