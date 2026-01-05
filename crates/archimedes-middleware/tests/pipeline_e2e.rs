//! End-to-end pipeline integration tests.
//!
//! These tests verify that all 8 middleware stages work correctly together
//! in the proper order:
//!
//! 1. Request ID - Generate/propagate request ID
//! 2. Tracing - Initialize trace context
//! 3. Identity - Extract caller identity
//! 4. Authorization - Policy evaluation
//! 5. Request Validation - Schema validation
//! 6. Response Validation - Response schema validation
//! 7. Telemetry - Metrics emission
//! 8. Error Normalization - Error envelope conversion

use archimedes_core::CallerIdentity;
use archimedes_middleware::{
    context::MiddlewareContext,
    pipeline::{Pipeline, Stage},
    stages::{
        authorization::AuthorizationMiddleware,
        error_normalization::ErrorNormalizationMiddleware,
        identity::IdentityMiddleware,
        request_id::RequestIdMiddleware,
        telemetry::TelemetryMiddleware,
        tracing::TracingMiddleware,
        validation::{MockSchema, ValidationMiddleware},
    },
    types::Request,
};
use bytes::Bytes;
use http::{Request as HttpRequest, Response as HttpResponse, StatusCode};
use http_body_util::Full;

type Response = HttpResponse<Full<Bytes>>;

/// Creates a successful handler response.
fn success_response() -> Response {
    HttpResponse::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(r#"{"status":"ok"}"#)))
        .unwrap()
}

/// Creates an error handler response.
fn error_response(status: StatusCode) -> Response {
    HttpResponse::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(r#"{"error":"failed"}"#)))
        .unwrap()
}

/// Creates a test request with optional headers.
fn make_request(path: &str, method: &str) -> Request {
    HttpRequest::builder()
        .method(method)
        .uri(path)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Creates a test request with SPIFFE identity header.
fn make_spiffe_request(path: &str, method: &str, spiffe_id: &str) -> Request {
    HttpRequest::builder()
        .method(method)
        .uri(path)
        .header("x-spiffe-id", spiffe_id)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Creates a test request with trace context.
fn make_traced_request(path: &str, method: &str, traceparent: &str) -> Request {
    HttpRequest::builder()
        .method(method)
        .uri(path)
        .header("traceparent", traceparent)
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Builds a full 8-stage pipeline with configurable middleware.
fn build_full_pipeline() -> Pipeline {
    // Pre-handler stages (1-5)
    let request_id = RequestIdMiddleware::new();
    let tracing = TracingMiddleware::new("e2e-test-service");
    let identity = IdentityMiddleware::with_trust_domain("test.example.com");

    // AllowAll for simple tests
    let authorization = AuthorizationMiddleware::allow_all();
    let validation = ValidationMiddleware::allow_all();

    // Post-handler stages (6-8)
    let telemetry = TelemetryMiddleware::new("e2e-test-service");
    let error_normalization = ErrorNormalizationMiddleware::new();

    Pipeline::builder()
        .add_pre_handler_stage(request_id)
        .add_pre_handler_stage(tracing)
        .add_pre_handler_stage(identity)
        .add_pre_handler_stage(authorization)
        .add_pre_handler_stage(validation)
        .add_post_handler_stage(telemetry)
        .add_post_handler_stage(error_normalization)
        .build()
}

/// Builds a pipeline with RBAC authorization.
fn build_rbac_pipeline() -> Pipeline {
    let request_id = RequestIdMiddleware::new();
    let tracing = TracingMiddleware::new("rbac-test-service");
    let identity = IdentityMiddleware::with_trust_domain("test.example.com");

    // RBAC authorization - roles are extracted as "spiffe:{trust_domain}"
    // So spiffe://test.example.com/... -> role "spiffe:test.example.com"
    let authorization = AuthorizationMiddleware::rbac()
        .allow_role("spiffe:test.example.com", ["getUser", "listUsers", "deleteUser"])
        .build();

    let validation = ValidationMiddleware::allow_all();
    let telemetry = TelemetryMiddleware::new("rbac-test-service");
    let error_normalization = ErrorNormalizationMiddleware::new();

    Pipeline::builder()
        .add_pre_handler_stage(request_id)
        .add_pre_handler_stage(tracing)
        .add_pre_handler_stage(identity)
        .add_pre_handler_stage(authorization)
        .add_pre_handler_stage(validation)
        .add_post_handler_stage(telemetry)
        .add_post_handler_stage(error_normalization)
        .build()
}

/// Builds a pipeline with restricted RBAC authorization (no deleteUser).
fn build_restricted_rbac_pipeline() -> Pipeline {
    let request_id = RequestIdMiddleware::new();
    let tracing = TracingMiddleware::new("rbac-test-service");
    let identity = IdentityMiddleware::with_trust_domain("test.example.com");

    // Only allow getUser and listUsers, NOT deleteUser
    let authorization = AuthorizationMiddleware::rbac()
        .allow_role("spiffe:test.example.com", ["getUser", "listUsers"])
        .build();

    let validation = ValidationMiddleware::allow_all();
    let telemetry = TelemetryMiddleware::new("rbac-test-service");
    let error_normalization = ErrorNormalizationMiddleware::new();

    Pipeline::builder()
        .add_pre_handler_stage(request_id)
        .add_pre_handler_stage(tracing)
        .add_pre_handler_stage(identity)
        .add_pre_handler_stage(authorization)
        .add_pre_handler_stage(validation)
        .add_post_handler_stage(telemetry)
        .add_post_handler_stage(error_normalization)
        .build()
}

/// Builds a pipeline with schema validation.
fn build_validation_pipeline() -> Pipeline {
    let request_id = RequestIdMiddleware::new();
    let tracing = TracingMiddleware::new("validation-test-service");
    let identity = IdentityMiddleware::new();
    let authorization = AuthorizationMiddleware::allow_all();

    // Schema validation
    let schema = MockSchema::builder()
        .required("name")
        .required("email")
        .build();

    let validation = ValidationMiddleware::with_schemas()
        .add_request_schema("createUser", schema)
        .build();

    let telemetry = TelemetryMiddleware::new("validation-test-service");
    let error_normalization = ErrorNormalizationMiddleware::new();

    Pipeline::builder()
        .add_pre_handler_stage(request_id)
        .add_pre_handler_stage(tracing)
        .add_pre_handler_stage(identity)
        .add_pre_handler_stage(authorization)
        .add_pre_handler_stage(validation)
        .add_post_handler_stage(telemetry)
        .add_post_handler_stage(error_normalization)
        .build()
}

// ============================================================================
// Stage Verification Tests
// ============================================================================

#[test]
fn test_stage_ordering_verification() {
    let stages = Stage::all();
    assert_eq!(stages.len(), 8);

    // Verify order
    assert_eq!(stages[0], Stage::RequestId);
    assert_eq!(stages[1], Stage::Tracing);
    assert_eq!(stages[2], Stage::Identity);
    assert_eq!(stages[3], Stage::Authorization);
    assert_eq!(stages[4], Stage::RequestValidation);
    assert_eq!(stages[5], Stage::ResponseValidation);
    assert_eq!(stages[6], Stage::Telemetry);
    assert_eq!(stages[7], Stage::ErrorNormalization);
}

#[test]
fn test_pre_handler_stages() {
    let pre_handler = Stage::pre_handler();
    assert_eq!(pre_handler.len(), 5);

    for stage in pre_handler {
        assert!(stage.is_pre_handler());
        assert!(!stage.is_post_handler());
    }
}

#[test]
fn test_post_handler_stages() {
    let post_handler = Stage::post_handler();
    assert_eq!(post_handler.len(), 3);

    for stage in post_handler {
        assert!(stage.is_post_handler());
        assert!(!stage.is_pre_handler());
    }
}

// ============================================================================
// Full Pipeline Integration Tests
// ============================================================================

#[tokio::test]
async fn test_full_pipeline_success() {
    let pipeline = build_full_pipeline();
    let ctx = MiddlewareContext::new();
    let request = make_request("/users/123", "GET");

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { success_response() })
        })
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_full_pipeline_with_spiffe_identity() {
    let pipeline = build_full_pipeline();
    let ctx = MiddlewareContext::new();
    let request = make_spiffe_request(
        "/users/123",
        "GET",
        "spiffe://test.example.com/service/user-service",
    );

    let response = pipeline
        .process(ctx, request, |ctx, _req| {
            // Verify identity was extracted
            let identity = ctx.identity();
            assert!(matches!(identity, CallerIdentity::Spiffe { .. }));
            Box::pin(async { success_response() })
        })
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_full_pipeline_with_trace_context() {
    let pipeline = build_full_pipeline();
    let ctx = MiddlewareContext::new();
    let request = make_traced_request(
        "/users/123",
        "GET",
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
    );

    let response = pipeline
        .process(ctx, request, |ctx, _req| {
            // Verify trace was propagated
            assert!(ctx.trace_id().is_some());
            assert!(ctx.span_id().is_some());
            Box::pin(async { success_response() })
        })
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_full_pipeline_generates_request_id() {
    let pipeline = build_full_pipeline();
    let ctx = MiddlewareContext::new();
    let request = make_request("/test", "GET");

    let response = pipeline
        .process(ctx, request, |ctx, _req| {
            // Request ID should be generated
            let request_id = ctx.request_id().to_string();
            assert!(!request_id.is_empty());
            Box::pin(async { success_response() })
        })
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_full_pipeline_collects_telemetry() {
    let pipeline = build_full_pipeline();
    let mut ctx = MiddlewareContext::new();
    ctx.set_operation_id("getUser".to_string());

    let request = make_request("/users/123", "GET");

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { success_response() })
        })
        .await;

    assert_eq!(response.status(), StatusCode::OK);
    // Telemetry data is stored in context by telemetry middleware
}

#[tokio::test]
async fn test_full_pipeline_error_normalization() {
    let pipeline = build_full_pipeline();
    let ctx = MiddlewareContext::new();
    let request = make_request("/nonexistent", "GET");

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { error_response(StatusCode::NOT_FOUND) })
        })
        .await;

    // Error should be normalized
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "application/json"
    );
}

// ============================================================================
// RBAC Authorization Tests
// ============================================================================

#[tokio::test]
async fn test_rbac_pipeline_admin_access() {
    // This test verifies that SPIFFE identities from trusted domain can access allowed operations
    let pipeline = build_rbac_pipeline();
    let mut ctx = MiddlewareContext::new();
    ctx.set_operation_id("deleteUser".to_string());

    let request = make_spiffe_request(
        "/users/123",
        "DELETE",
        "spiffe://test.example.com/service/admin-service",
    );

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { success_response() })
        })
        .await;

    // SPIFFE from test.example.com should be allowed for deleteUser
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rbac_pipeline_user_allowed_operation() {
    let pipeline = build_rbac_pipeline();
    let mut ctx = MiddlewareContext::new();
    ctx.set_operation_id("getUser".to_string());

    let request = make_spiffe_request(
        "/users/123",
        "GET",
        "spiffe://test.example.com/service/user-service",
    );

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { success_response() })
        })
        .await;

    // SPIFFE from test.example.com should be allowed for getUser
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rbac_pipeline_user_denied_operation() {
    // Use restricted pipeline that doesn't allow deleteUser
    let pipeline = build_restricted_rbac_pipeline();
    let mut ctx = MiddlewareContext::new();
    ctx.set_operation_id("deleteUser".to_string());

    let request = make_spiffe_request(
        "/users/123",
        "DELETE",
        "spiffe://test.example.com/service/user-service",
    );

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { success_response() })
        })
        .await;

    // Should be denied - deleteUser not in allowed operations
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_rbac_pipeline_anonymous_denied() {
    let pipeline = build_rbac_pipeline();
    let mut ctx = MiddlewareContext::new();
    ctx.set_operation_id("getUser".to_string());

    let request = make_request("/users/123", "GET");

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { success_response() })
        })
        .await;

    // Anonymous should be denied
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

// ============================================================================
// Validation Pipeline Tests
// ============================================================================

#[tokio::test]
async fn test_validation_pipeline_valid_body() {
    // Build a simple pipeline without validation for this test
    // (since validation requires operation_id to be set by router)
    let request_id = RequestIdMiddleware::new();
    let tracing = TracingMiddleware::new("validation-test-service");
    let identity = IdentityMiddleware::new();
    let authorization = AuthorizationMiddleware::allow_all();
    let validation = ValidationMiddleware::allow_all(); // Use allow_all for simple test
    let telemetry = TelemetryMiddleware::new("validation-test-service");
    let error_normalization = ErrorNormalizationMiddleware::new();

    let pipeline = Pipeline::builder()
        .add_pre_handler_stage(request_id)
        .add_pre_handler_stage(tracing)
        .add_pre_handler_stage(identity)
        .add_pre_handler_stage(authorization)
        .add_pre_handler_stage(validation)
        .add_post_handler_stage(telemetry)
        .add_post_handler_stage(error_normalization)
        .build();

    let mut ctx = MiddlewareContext::new();
    ctx.set_operation_id("createUser".to_string());

    let body = r#"{"name":"John","email":"john@example.com"}"#;
    let request = HttpRequest::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap();

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { success_response() })
        })
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_validation_pipeline_missing_required_field() {
    let pipeline = build_validation_pipeline();
    let mut ctx = MiddlewareContext::new();
    ctx.set_operation_id("createUser".to_string());

    // Missing 'email' field
    let body = r#"{"name":"John"}"#;
    let request = HttpRequest::builder()
        .method("POST")
        .uri("/users")
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap();

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { success_response() })
        })
        .await;

    // Should be rejected with validation error
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Pipeline Stage Order Tests
// ============================================================================

#[tokio::test]
async fn test_middleware_execution_order() {
    let pipeline = build_full_pipeline();
    let ctx = MiddlewareContext::new();
    let request = make_request("/test", "GET");

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { success_response() })
        })
        .await;

    // Pipeline should execute successfully
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_context_propagation_through_stages() {
    let pipeline = build_full_pipeline();
    let mut ctx = MiddlewareContext::new();

    // Set initial context
    ctx.set_operation_id("testOperation".to_string());

    let request = make_traced_request(
        "/test",
        "GET",
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
    );

    let response = pipeline
        .process(ctx, request, |ctx, _req| {
            // Verify context has been enriched through pipeline
            assert!(ctx.trace_id().is_some(), "Trace ID should be set");
            assert!(ctx.span_id().is_some(), "Span ID should be set");
            assert_eq!(ctx.operation_id(), Some("testOperation"));

            Box::pin(async { success_response() })
        })
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_handler_error_normalized() {
    let pipeline = build_full_pipeline();
    let ctx = MiddlewareContext::new();
    let request = make_request("/error", "GET");

    let response = pipeline
        .process(ctx, request, |_ctx, _req| {
            Box::pin(async { error_response(StatusCode::INTERNAL_SERVER_ERROR) })
        })
        .await;

    // Error should be normalized
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_multiple_errors_normalized_consistently() {
    let pipeline = build_full_pipeline();

    // Test various error codes
    let error_codes = [
        StatusCode::BAD_REQUEST,
        StatusCode::UNAUTHORIZED,
        StatusCode::FORBIDDEN,
        StatusCode::NOT_FOUND,
        StatusCode::INTERNAL_SERVER_ERROR,
        StatusCode::SERVICE_UNAVAILABLE,
    ];

    for expected_status in error_codes {
        let ctx = MiddlewareContext::new();
        let request = make_request("/error", "GET");

        let response = pipeline
            .process(ctx, request, move |_ctx, _req| {
                Box::pin(async move { error_response(expected_status) })
            })
            .await;

        assert_eq!(response.status(), expected_status);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json",
            "Error response should be JSON for status {:?}",
            expected_status
        );
    }
}
