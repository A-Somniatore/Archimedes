# Archimedes – Development Roadmap

> **Version**: 2.1.0  
> **Created**: 2026-01-04  
> **Last Updated**: 2026-01-05  
> **Target Completion**: Week 48 (includes 4-week buffer)

---

## Key Decisions

| Decision                                                                     | Impact                                                 |
| ---------------------------------------------------------------------------- | ------------------------------------------------------ |
| [ADR-008](../../docs/decisions/008-archimedes-full-framework.md)             | **Archimedes as internal standardized framework** |
| [ADR-005](../../docs/decisions/005-kubernetes-ingress-over-custom-router.md) | Archimedes handles contract enforcement, not routing   |
| [ADR-006](../../docs/decisions/006-grpc-post-mvp.md)                         | MVP is HTTP/REST only, gRPC is post-MVP                |
| [ADR-004](../../docs/decisions/004-regorus-for-rego-parsing.md)              | Use Regorus for embedded OPA evaluation                |
| [ADR-007](../../docs/decisions/007-apache-2-license.md)                      | Apache 2.0 license                                     |

## Vision: Internal Standardization

Archimedes is an **internal platform** that standardizes how we build services:

| Challenge (Per-Team Choice) | Archimedes Solution |
|-----------------------------|---------------------|
| Each team picks different framework | One standard for all services |
| Auth implemented differently | OPA-based auth built-in |
| Validation varies | Contract-driven, automatic |
| Observability setup per service | Built-in, zero config |

**Archimedes Responsibilities (V1 Release):**

- **Own the HTTP layer** (direct Hyper for full control)
- High-performance custom router with radix tree matching
- Native request/response extractors
- Dependency injection system
- Contract-based validation (Themis)
- Embedded authorization (OPA/Eunomia)
- Full observability (OpenTelemetry)
- WebSocket and SSE support
- Background tasks and scheduled jobs

**Deferred to V1.1:**

- Multi-language SDK generation (Python, TS, Go, C++)
- GraphQL support

**NOT Archimedes Responsibilities:**

- External traffic routing (use K8s Ingress)
- HTTP/3 / QUIC (future consideration)
- Arbitrary middleware plugins (fixed pipeline)

---

## Overview

Archimedes is the async HTTP server framework for the Themis Platform. **Archimedes core can be developed in parallel** with Themis and Eunomia using mock contracts and policies.

### Parallel Development Strategy

```
Week 1: Shared Types (COORDINATION - themis-platform-types)
    ↓
Week 2-4: Core Framework (PARALLEL - uses shared types)
    ↓
Week 5-8: Server & Routing (PARALLEL - mock contracts)
    ↓
Week 9-12: Middleware Pipeline (PARALLEL - mock validation)
    ↓
Week 13-16: Observability & Config (PARALLEL)
    ↓
Week 17-20: Integration (AFTER Themis/Eunomia ready)
```

### Timeline Summary

| Phase                       | Duration | Weeks | Description                       | Dependencies            |
| --------------------------- | -------- | ----- | --------------------------------- | ----------------------- |
| **MVP (Weeks 1-20)**        |          |       |                                   |                         |
| A0: Shared Types            | 1 week   | 1     | Integrate `themis-platform-types` | Themis creates crate    |
| A1: Foundation              | 3 weeks  | 2-4   | Core types, server scaffold       | `themis-platform-types` |
| A2: Server & Routing        | 4 weeks  | 5-8   | HTTP server, routing, handlers    | None (mock contracts)   |
| A3: Middleware              | 4 weeks  | 9-12  | Middleware pipeline, validation   | None (mock validation)  |
| A4: Observability           | 4 weeks  | 13-16 | Metrics, tracing, logging, config | None                    |
| A5: Integration             | 4 weeks  | 17-20 | Themis + Eunomia integration      | Themis, Eunomia         |
| **Framework (Weeks 21-36)** |          |       |                                   |                         |
| A6: Core Framework          | 4 weeks  | 21-24 | Custom router, extractors, DI     | MVP complete            |
| A7: Handler Macros          | 4 weeks  | 25-28 | Handler macros, auto-docs         | A6                      |
| A8: Extended Features       | 4 weeks  | 29-32 | WebSocket, SSE, background tasks  | A7                      |
| A9: Developer Experience    | 4 weeks  | 33-36 | CLI tool, hot reload, templates   | A8                      |
| **Stoa (Weeks 37-44)**      |          |       |                                   |                         |
| Stoa UI                     | 8 weeks  | 37-44 | Visibility dashboard              | A9, Archimedes          |
| **Buffer (Weeks 45-48)**    |          |       |                                   |                         |
| Hardening & Buffer          | 4 weeks  | 45-48 | Performance tuning, contingency   | All                     |

**Total**: 48 weeks (12 months)
- MVP: Weeks 1-20
- Full Framework: Weeks 21-36
- Stoa UI: Weeks 37-44
- Buffer: Weeks 45-48

**Note**: Multi-language SDK generation (A10) deferred to V1.1 to ensure quality of core platform.

### Cross-Component Timeline Alignment

```
         Week: 1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20
 Themis:      [T0][---T1---][--T2--][------T3------][--T4--][--T5--]
 Eunomia:     [E0][---E1---][------E2------][------E3------]        (gap)        [------E4------]
 Archimedes:  [A0][---A1---][----------A2----------][----------A3----------][A4][------A5------]
```

**Key Integration Points**:

- Week 1: Shared types crate created (blocks all components)
- Week 14: Themis T5 ready → can start A5 integration
- Week 12: Eunomia E3 ready → bundles available for A5
- Weeks 17-20: All components integrate together

---

## Phase A0: Shared Platform Types (Week 1) ⭐ COORDINATION

> **Note**: The `themis-platform-types` crate is created by Themis team in Week 1.
> Archimedes integrates it to ensure schema compatibility with Eunomia.

### Week 1: Integrate Shared Types

> ✅ **Update (2026-01-04)**: `themis-platform-types` crate is now available!
> Local type implementations are ready for migration to shared crate.

- [ ] Add `themis-platform-types` dependency to `archimedes-core`
  > ⏳ **Ready**: Shared crate available, migration pending
- [x] Use shared `CallerIdentity` (not duplicate definition)
  > ✅ **Completed 2026-01-04**: Implemented locally in `archimedes-core::CallerIdentity`
  > Supports SPIFFE, User, ApiKey, Anonymous variants per spec. Ready for migration.
- [ ] Use shared `PolicyInput` for OPA evaluation
  > ⏳ **Ready**: To be implemented in Phase A3 (middleware)
- [ ] Use shared `PolicyDecision` from OPA response
  > ⏳ **Ready**: To be implemented in Phase A3 (middleware)
- [x] Use shared `ThemisErrorEnvelope` for error responses
  > ✅ **Completed 2026-01-04**: Implemented as `archimedes-core::ErrorEnvelope`
  > with ErrorDetail, ErrorCategory, and field-level errors. Ready for migration.
- [x] Use shared `RequestId` type
  > ✅ **Completed 2026-01-04**: Implemented as `archimedes-core::RequestId`
  > using UUID v7 for time-ordered IDs. Ready for migration.
- [ ] Verify JSON serialization matches integration spec
  > ⏳ **Ready**: Unit tests written, needs cross-crate verification.

### Phase A0 Milestone

**Criteria**: Archimedes uses `themis-platform-types` for all shared types

> ⏳ **Status**: Local implementations complete and tested. Migration to shared crate pending.

---

## Phase A1: Foundation (Weeks 2-4) — PARALLEL

### Week 2: Project Setup & Core Types

- [x] Create `archimedes` repository structure
  > ✅ **Completed 2026-01-04** (Week 1): Initialized git repository with `.gitignore`
- [x] Set up Cargo workspace:
  ```
  crates/
  ├── archimedes/           # Facade crate
  ├── archimedes-core/      # Core types (depends on themis-platform-types)
  ├── archimedes-server/    # Server implementation
  └── archimedes-middleware/# Middleware pipeline
  ```
  > ✅ **Completed 2026-01-04** (Week 1): Full workspace created with all 4 crates.
  > Used Rust 2024 edition, workspace dependencies, and workspace lints.
- [x] Configure CI pipeline (GitHub Actions)
  > ✅ **Completed 2026-01-04** (Week 1): `.github/workflows/ci.yml` with:
  >
  > - Format checking, Clippy linting, tests, docs build
  > - Security audit, code coverage
- [x] Implement `RequestContext` struct (uses shared types)
  > ✅ **Completed 2026-01-04** (Week 1): Full implementation in `archimedes-core`:
  >
  > - RequestId (UUID v7), CallerIdentity, trace/span IDs
  > - Operation ID, timing, builder pattern
- [x] Implement `Handler` trait
  > ✅ **Completed 2026-01-04** (Week 1): Async handler trait with:
  >
  > - Generic over Req/Res with Serde bounds
  > - FnHandler wrapper, Empty/NoContent unit types
- [x] Write initial documentation
  > ✅ **Completed 2026-01-04** (Week 1): Rustdoc on all public items with examples

### Week 3: Error Framework

- [x] Use `ThemisErrorEnvelope` from shared crate
  > ✅ **Completed 2026-01-04** (Week 1): Implemented as `ErrorEnvelope` locally
- [x] Implement error conversion traits
  > ✅ **Completed 2026-01-04** (Week 1): `ThemisError` with `thiserror` derive
- [x] Add error categorization (validation, auth, internal)
  > ✅ **Completed 2026-01-04** (Week 1): `ErrorCategory` enum with 9 categories
- [x] Implement error response serialization
  > ✅ **Completed 2026-01-04** (Week 1): `to_envelope()` method with JSON support
- [x] Test error scenarios match integration spec
  > ✅ **Completed 2026-01-04** (Week 1): Unit tests for all error types

### Week 4: Mock Contract Support

- [x] Create mock `Contract` type for testing
  > ✅ **Completed 2026-01-04**: `archimedes_core::contract::Contract`
  >
  > - Builder pattern for fluent construction
  > - Operation lookup by ID
  > - Path matching with parameter extraction
- [x] Create mock `Operation` type
  > ✅ **Completed 2026-01-04**: `archimedes_core::contract::Operation`
  >
  > - HTTP method, path pattern, request/response schemas
  > - Path parameter parsing ({userId} -> params map)
  > - Auth requirement flag, tags, descriptions
- [x] Implement mock schema validation
  > ✅ **Completed 2026-01-04**: `archimedes_core::contract::MockSchema`
  >
  > - String, Integer, Number, Boolean, Array, Object types
  > - Required field support with .required() modifier
  > - Min/max constraints for strings, numbers, arrays
  > - Nested object validation with JSON path error reporting
- [x] Write test fixtures
  > ✅ **Completed 2026-01-04**: `archimedes_core::fixtures` module
  >
  > - `user_service_contract()` - 5 CRUD operations
  > - `health_contract()` - health/readiness (no auth)
  > - `order_service_contract()` - nested resources
  > - Reusable schema helpers (user_schema, address_schema, etc.)
- [x] Document mock usage for parallel development
  > ✅ **Completed 2026-01-04**: Crate-level docs with examples
  >
  > - Contract builder usage
  > - Path matching examples
  > - Schema validation examples
  > - Fixtures usage guide

### Phase A1 Milestone

**Criteria**: Core types defined using shared crate, mock contracts work

> ✅ **Status (2026-01-04)**: Phase A1 COMPLETE
>
> - All core types implemented (RequestContext, RequestId, CallerIdentity, ThemisError)
> - Mock contracts fully functional with path matching and validation
> - Comprehensive test fixtures available
> - Ready to proceed to Phase A2: Server & Routing

---

## Phase A2: Server & Routing (Weeks 5-8) — PARALLEL

### Week 5: HTTP Server

- [x] Create `archimedes-server` crate
  > ✅ **Completed 2026-01-05**: Full server infrastructure
- [x] Implement basic Hyper server
  > ✅ **Completed 2026-01-05**: `archimedes_server::Server`
  >
  > - Hyper 1.6 HTTP/1.1 server with Tokio runtime
  > - Connection handling with per-connection tasks
  > - Service-based request handling
- [x] Add Tokio runtime setup
  > ✅ **Completed 2026-01-05**: Uses tokio::main and TcpListener
- [x] Implement graceful shutdown
  > ✅ **Completed 2026-01-05**: `archimedes_server::shutdown` module
  >
  > - ShutdownSignal for SIGTERM/SIGINT handling
  > - ConnectionTracker for in-flight request tracking
  > - Configurable shutdown timeout
  > - OS signal handling (Unix + Windows)
- [x] Add health check endpoint (`/health`)
  > ✅ **Completed 2026-01-05**: `archimedes_server::health` module
  >
  > - HealthCheck with service name, version, uptime
  > - ReadinessCheck with custom check functions
  > - /health and /ready built-in endpoints
  > - JSON response with proper content-type
- [x] Test server starts and accepts connections
  > ✅ **Completed 2026-01-05**: Comprehensive test coverage
  >
  > - Config builder tests
  > - Router path matching tests
  > - Shutdown signal tests
  > - Health/readiness endpoint tests
  > - Server start/shutdown integration tests

### Week 6: Request Routing

- [x] Implement `Router` struct
  > ✅ **Completed 2026-01-05**: `archimedes_server::Router`
  >
  > - Path segment parsing (literal and parameter)
  > - Route matching with parameter extraction
  > - Operation ID lookup
- [x] Add `operationId` → handler mapping
  > ✅ **Completed 2026-01-05**: Route stores operation_id
  >
  > - RouteMatch contains operation_id and params
  > - Server routes to matched handler (placeholder)
- [x] Implement path → operationId resolution
  > ✅ **Completed 2026-01-05**: PathSegment enum
  >
  > - Literal segment matching
  > - Parameter segment extraction ({userId})
  > - Multi-parameter path support
- [x] Add method matching
  > ✅ **Completed 2026-01-05**: Method stored per route
  >
  > - Same path, different methods = different routes
- [x] Handle 404 for unknown routes
  > ✅ **Completed 2026-01-05**: handle_not_found()
  >
  > - JSON error response with path
- [x] Test routing scenarios
  > ✅ **Completed 2026-01-05**: 20+ routing tests
  >
  > - Simple paths, parameter paths
  > - Method matching, path mismatch
  > - Multiple parameters, complex paths

### Week 7: Handler Registration

- [x] Implement handler registration API
  > ✅ **Completed 2026-01-05**: `archimedes_server::handler::HandlerRegistry`
  >
  > - Type-erased handlers with `ErasedHandler` type
  > - Generic `register<Req, Res, F>()` method
  > - `register_no_body<Res, F>()` for bodyless handlers
- [x] Add compile-time type checking
  > ✅ **Completed 2026-01-05**: Generic bounds enforce types
  >
  > - `Req: DeserializeOwned + Send + 'static`
  > - `Res: Serialize + Send + 'static`
  > - `HandlerRequest` and `HandlerResponse` marker traits
- [x] Validate handler signatures
  > ✅ **Completed 2026-01-05**: Enforced via trait bounds
  >
  > - Handlers must be `Fn(RequestContext, Req) -> Future`
  > - Return type must be `Result<Res, HandlerError>`
- [x] Implement request deserialization
  > ✅ **Completed 2026-01-05**: JSON deserialization in `register()`
  >
  > - `serde_json::from_slice()` with error handling
  > - `HandlerError::DeserializationError` on failure
- [x] Implement response serialization
  > ✅ **Completed 2026-01-05**: JSON serialization in `register()`
  >
  > - `serde_json::to_vec()` with error handling
  > - `HandlerError::SerializationError` on failure
- [x] Test handler invocation
  > ✅ **Completed 2026-01-05**: Comprehensive test coverage
  >
  > - Registry creation and registration tests
  > - Handler lookup and invocation tests
  > - Error handling tests

### Week 8: Handler Pipeline

- [x] Wire handlers to router
  > ✅ **Completed 2026-01-05**: Full handler integration
  >
  > - `Server` struct now contains `HandlerRegistry`
  > - `handle_matched_route()` invokes registered handlers
  > - Proper request context with operation_id
- [x] Add request body parsing
  > ✅ **Completed 2026-01-05**: Hyper body collection
  >
  > - `collect_body()` gathers Incoming body to Bytes
  > - Body passed to handler for deserialization
- [x] Add response body serialization
  > ✅ **Completed 2026-01-05**: JSON response handling
  >
  > - Handler responses serialized via serde_json
  > - Proper Content-Type headers
- [x] Implement timeout handling
  > ✅ **Completed 2026-01-05**: Request timeout support
  >
  > - Configurable `request_timeout` in ServerBuilder
  > - Body collection timeout
  > - Handler execution timeout
  > - 408 REQUEST_TIMEOUT / 504 GATEWAY_TIMEOUT responses
- [x] Add basic error responses
  > ✅ **Completed 2026-01-05**: Structured error handling
  >
  > - `handle_error()` for standard error responses
  > - `handle_handler_error()` for handler-specific errors
  > - Proper HTTP status codes for each error type
  > - JSON error envelopes with code and message
- [x] Integration tests with mock handlers
  > ✅ **Completed 2026-01-05**: Comprehensive test coverage
  >
  > - `test_handler_invocation` - full request/response cycle
  > - `test_handler_no_body_invocation` - bodyless handlers
  > - `test_handler_deserialization_error` - invalid JSON
  > - `test_handler_not_registered` - missing handlers

### Phase A2 Milestone

**Criteria**: HTTP server runs, routes requests, invokes handlers

> ✅ **Status (2026-01-05)**: Phase A2 COMPLETE
>
> - HTTP server with Hyper 1.6 and Tokio runtime
> - Path-based routing with parameter extraction
> - Type-erased handler registry with serialization
> - Handler invocation with timeout support
> - Health/readiness endpoints
> - Graceful shutdown with connection tracking
> - 90+ tests passing
> - Ready to proceed to Phase A3: Middleware Pipeline

---

## Phase A3: Middleware Pipeline (Weeks 9-12) — PARALLEL

### Week 9: Middleware Architecture

- [x] Create `archimedes-middleware` crate
  > ✅ **Completed 2026-01-05**: Full middleware infrastructure
- [x] Design middleware trait
  > ✅ **Completed 2026-01-05**: `archimedes_middleware::Middleware` trait
  >
  > - Async `process()` method with context, request, and next chain
  > - `BoxFuture<'a, Response>` return type for type erasure
  > - `name()` method for identification
- [x] Implement fixed-order pipeline
  > ✅ **Completed 2026-01-05**: `archimedes_middleware::Pipeline`
  >
  > - 8 fixed stages (RequestId through ErrorNormalization)
  > - `Stage` enum with pre/post handler categorization
  > - Cannot be reordered by users
- [x] Add middleware context passing
  > ✅ **Completed 2026-01-05**: `archimedes_middleware::MiddlewareContext`
  >
  > - Mutable context flows through pipeline
  > - Type-erased extensions via `HashMap<TypeId, Box<dyn Any>>`
  > - Converts to immutable `RequestContext` for handlers
- [x] Ensure middleware cannot be reordered
  > ✅ **Completed 2026-01-05**: `pub(crate)` methods prevent user modification
- [x] Document middleware constraints
  > ✅ **Completed 2026-01-05**: Crate-level docs with pipeline diagram

### Week 10: Core Middleware (Part 1)

- [x] Implement Request ID middleware
  > ✅ **Completed 2026-01-05**: `archimedes_middleware::RequestIdMiddleware`
  >
  > - UUID v7 generation for time-ordered IDs
  > - X-Request-ID header extraction (configurable trust)
  > - Sets request and response headers
- [x] Implement Tracing middleware (span creation)
  > ✅ **Completed 2026-01-05**: `archimedes_middleware::TracingMiddleware`
  >
  > - W3C Trace Context (traceparent header) parsing
  > - Trace ID and Span ID generation
  > - `SpanInfo` extension stored in context
- [x] Implement Identity extraction middleware
  > ✅ **Completed 2026-01-05**: `archimedes_middleware::IdentityMiddleware`
  >
  > - SPIFFE identity from `x-spiffe-id` header
  > - JWT identity from `authorization: Bearer` header
  > - API Key identity from `x-api-key` header
  > - Precedence: SPIFFE > JWT > ApiKey > Anonymous
- [x] Add SPIFFE identity parsing
  > ✅ **Completed 2026-01-05**: Trust domain validation, SPIFFE ID format
- [x] Add JWT identity parsing
  > ✅ **Completed 2026-01-05**: JWT structure parsing (header.payload.signature)
- [x] Test identity extraction
  > ✅ **Completed 2026-01-05**: 12 identity extraction tests

### Week 11: Core Middleware (Part 2)

- [x] Implement mock Authorization middleware
  > ✅ **Completed 2026-01-05**: `archimedes_middleware::AuthorizationMiddleware`
  >
  > - Allow-all, Deny-all, and RBAC modes
  > - Role-based access control with wildcard support
  > - Anonymous operation allowlist
  > - Custom `PolicyEvaluator` trait for extensibility
  > - 14 authorization tests
- [x] Implement mock Validation middleware
  > ✅ **Completed 2026-01-05**: `archimedes_middleware::ValidationMiddleware`
  >
  > - Request and Response validation middleware
  > - `MockSchema` with required fields and type checking
  > - Support for String, Integer, Number, Boolean, Array, Object types
  > - Allow-all, Reject-all, and Schema-based modes
  > - 14 validation tests
- [x] Add extension points (`pre_handler`, `post_handler`)
  > ✅ **Completed 2026-01-05**: `PipelineBuilder` with hooks
  >
  > - `pre_handler()` hook after identity, before authorization
  > - `post_handler()` hook after handler, before response validation
- [x] Test middleware ordering
  > ✅ **Completed 2026-01-05**: Stage ordering tests verify fixed sequence
- [x] Test extension points
  > ✅ **Completed 2026-01-05**: Pipeline builder tests with hooks

### Week 12: Response Pipeline

- [x] Implement response validation middleware (mock)
  > ✅ **Completed 2026-01-05**: `ResponseValidationMiddleware`
  >
  > - Schema-based response validation
  > - Configurable enforce mode (error vs log-only)
- [x] Implement telemetry emission middleware
  > ✅ **Completed 2026-01-05**: `TelemetryMiddleware`
  >
  > - Service name, version, environment tracking
  > - Request duration measurement
  > - Status code, operation ID, request/trace IDs
  > - `TelemetryData` stored in context for inspection
  > - 7 telemetry tests
- [x] Implement error normalization middleware
  > ✅ **Completed 2026-01-05**: `ErrorNormalizationMiddleware`
  >
  > - Converts all errors to standard JSON envelope
  > - Error code mapping (NOT_FOUND, UNAUTHORIZED, etc.)
  > - Request ID in all error responses
  > - Internal error message hiding (configurable)
  > - 9 error normalization tests
- [x] Wire complete request→response pipeline
  > ✅ **Completed 2026-01-05**: Full 8-stage pipeline
  >
  > - `PipelineBuilder` with public `add_pre_handler_stage` and `add_post_handler_stage`
  > - Stages: RequestId → Tracing → Identity → Authorization → Validation → Telemetry → ErrorNormalization
  > - Context flows through all stages
- [x] End-to-end pipeline tests
  > ✅ **Completed 2026-01-05**: `tests/pipeline_e2e.rs`
  >
  > - Full pipeline integration tests (19 tests)
  > - SPIFFE identity extraction test
  > - Trace context propagation test
  > - RBAC authorization tests
  > - Validation tests
  > - Error normalization tests

### Phase A3 Milestone

**Criteria**: Full middleware pipeline works with mock validation/auth

> ✅ **Status (2026-01-05)**: Phase A3 COMPLETE
>
> - All 8 middleware stages implemented and tested
> - 85 middleware tests + 19 end-to-end tests = 104 total middleware tests
> - RequestId, Tracing, Identity, Authorization, Validation, ResponseValidation, Telemetry, ErrorNormalization
> - Full pipeline wired and working
> - Ready to proceed to Phase A4: Observability & Config

---

## Phase A4: Observability & Config (Weeks 13-16) — PARALLEL

### Week 13: Metrics

- [ ] Create `archimedes-telemetry` crate
- [ ] Implement Prometheus metrics
- [ ] Add `archimedes_requests_total` counter
- [ ] Add `archimedes_request_duration_seconds` histogram
- [ ] Add `archimedes_in_flight_requests` gauge
- [ ] Expose `/metrics` endpoint

### Week 14: Tracing

- [ ] Integrate OpenTelemetry tracing
- [ ] Implement trace context propagation
- [ ] Add span attributes (operationId, service, etc.)
- [ ] Configure OTLP exporter
- [ ] Test trace correlation

### Week 15: Logging

- [ ] Implement structured JSON logging
- [ ] Add request_id, trace_id to all logs
- [ ] Configure log levels
- [ ] Add audit logging for authz decisions
- [ ] Test log output format

### Week 16: Configuration

- [ ] Create `archimedes-config` crate
- [ ] Design typed configuration schema
- [ ] Implement file-based config loading
- [ ] Add environment variable overrides
- [ ] Fail on unknown fields
- [ ] Document configuration options

### Phase A4 Milestone

**Criteria**: Full observability (metrics, traces, logs), typed config

---

## Phase A5: Integration (Weeks 17-20) — REQUIRES THEMIS/EUNOMIA

### Week 17: Themis Integration

- [ ] Create `archimedes-sentinel` crate
- [ ] Implement contract artifact loading
- [ ] Implement path → operationId resolution from real contracts
- [ ] Implement request validation against schemas
- [ ] Replace mock validation with real validation

### Week 18: Eunomia Integration

- [ ] Create `archimedes-authz` crate
- [ ] Integrate OPA evaluator
- [ ] Implement policy bundle loading
- [ ] Implement authorization decision caching
- [ ] Replace mock authorization with real OPA

### Week 19: Control Plane

- [ ] Implement policy update endpoint
- [ ] Add mTLS authentication
- [ ] Implement atomic policy swap
- [ ] Add rollback support
- [ ] Test hot-reload scenarios

### Week 20: End-to-End Testing

- [ ] Full integration tests with Themis contracts
- [ ] Full integration tests with Eunomia policies
- [ ] Performance benchmarks
- [ ] Load testing
- [ ] Documentation updates

### Phase A5 Milestone

**Criteria**: Full integration with Themis and Eunomia, production-ready MVP

---

## Phase A6: Core Framework (Weeks 21-24) 🚀 NEW

> **Goal**: Replace Axum as HTTP layer, own routing and extractors directly

### Week 21-22: Custom Router

- [ ] Remove Axum dependency, use Hyper directly
- [ ] Implement radix tree router for path matching
- [ ] Support path parameters (`/users/{id}`)
- [ ] Support wildcards (`/files/*path`)
- [ ] Support method-based routing (GET, POST, etc.)
- [ ] Benchmark: Match Axum's routing performance

```rust
// Target API
let router = Router::new()
    .route("/users", get(list_users).post(create_user))
    .route("/users/{id}", get(get_user).put(update_user).delete(delete_user))
    .route("/files/*path", get(serve_file));
```

### Week 23-24: Extractors and Response Building

- [ ] Implement `Path<T>` extractor
- [ ] Implement `Query<T>` extractor
- [ ] Implement `Json<T>` extractor with contract validation
- [ ] Implement `Form<T>` extractor
- [ ] Implement `Multipart` extractor
- [ ] Implement `Headers` extractor
- [ ] Response builders (Json, Html, Redirect, etc.)
- [ ] Error handling with structured responses

```rust
// Target handler signature
async fn create_user(
    Json(body): Json<CreateUserRequest>,  // Auto-validated against contract
    headers: Headers,
) -> Result<Json<User>, AppError> {
    // ...
}
```

### A6 Deliverables

- `archimedes-router` - High-performance radix tree router
- `archimedes-extract` - Request extractors
- Benchmark suite vs Axum/Actix

---

## Phase A7: FastAPI Parity (Weeks 25-28) 🐍 NEW

> **Goal**: Match FastAPI developer experience with handler macros

### Week 25-26: Handler Macros

- [ ] `#[archimedes::handler]` proc macro
- [ ] Automatic parameter extraction from signature
- [ ] Contract binding (which operation handles which contract endpoint)
- [ ] Dependency injection integration

```rust
// FastAPI-style handler definition
#[archimedes::handler(contract = "users.yaml", operation = "createUser")]
async fn create_user(
    db: Inject<Database>,      // DI injected
    auth: Auth,                // Current user from auth middleware
    body: CreateUserRequest,   // Auto-validated, auto-extracted
) -> User {
    db.insert_user(body, auth.user_id).await
}
```

### Week 27-28: Automatic Documentation

- [ ] OpenAPI spec generation from contracts + handlers
- [ ] Swagger UI endpoint (`/docs`)
- [ ] ReDoc endpoint (`/redoc`)
- [ ] Interactive API console
- [ ] Schema explorer

### A7 Deliverables

- `archimedes-macros` - Proc macros for handler definitions
- `archimedes-docs` - Auto-documentation system
- Documentation UI assets

---

## Phase A8: Extended Features (Weeks 29-32) 🔌 NEW

> **Goal**: Add WebSocket, SSE, background tasks, database integration

### Week 29-30: Real-Time Features

- [ ] WebSocket support with contract validation
- [ ] Server-Sent Events (SSE)
- [ ] Connection management and lifecycle
- [ ] Heartbeat and reconnection handling

```rust
#[archimedes::websocket(contract = "chat.yaml")]
async fn chat_handler(ws: WebSocket, auth: Auth) {
    while let Some(msg) = ws.recv().await {
        // Messages validated against contract
        ws.send(response).await;
    }
}
```

### Week 31-32: Background Processing

- [ ] Background task spawning
- [ ] Scheduled jobs (cron expressions)
- [ ] Task queues with retry logic
- [ ] Database connection pooling (SQLx)

```rust
#[archimedes::task(schedule = "0 0 * * *")]  // Daily at midnight
async fn cleanup_expired_sessions(db: Inject<Database>) {
    db.delete_expired_sessions().await;
}
```

### A8 Deliverables

- `archimedes-ws` - WebSocket support
- `archimedes-sse` - Server-Sent Events
- `archimedes-tasks` - Background task system
- `archimedes-db` - Database connection pooling

---

## Phase A9: Developer Experience (Weeks 33-36) 🛠️ NEW

> **Goal**: CLI tools, hot reload, and project scaffolding

### Week 33-34: CLI Tool

- [ ] `archimedes new <project>` - Scaffold new project
- [ ] `archimedes generate handler` - Generate handler from contract
- [ ] `archimedes generate client` - Generate client SDK
- [ ] `archimedes dev` - Development server with hot reload
- [ ] `archimedes build` - Production build

```bash
# Create new project
$ archimedes new my-service --contract api.yaml

# Generate handlers from contract
$ archimedes generate handler --contract api.yaml --output src/handlers/

# Run development server with hot reload
$ archimedes dev
```

### Week 35-36: Developer Tools

- [ ] Hot reload in development mode
- [ ] Request/response logging with pretty printing
- [ ] Error overlay in development
- [ ] Template engine integration (Askama/Tera)
- [ ] Static file serving

### A9 Deliverables

- `archimedes-cli` - Command-line tool
- `archimedes-dev` - Development server
- `archimedes-templates` - Template engine integration
- Project templates and examples

---

## Phase A10: Multi-Language SDKs (Weeks 37-40) 🌍 NEW

> **Goal**: Generate client SDKs for Python, TypeScript, Go, C++

### Week 37-38: Python and TypeScript SDKs

- [ ] Python SDK generator (asyncio-based)
- [ ] TypeScript SDK generator (fetch/axios)
- [ ] Type-safe request/response types
- [ ] Automatic retry and error handling
- [ ] Authentication handling

```python
# Generated Python client
from my_service import MyServiceClient, CreateUserRequest

client = MyServiceClient(base_url="https://api.example.com")
user = await client.create_user(CreateUserRequest(name="Alice", email="alice@example.com"))
```

```typescript
// Generated TypeScript client
import { MyServiceClient, CreateUserRequest } from "./my-service-client";

const client = new MyServiceClient({ baseUrl: "https://api.example.com" });
const user = await client.createUser({
  name: "Alice",
  email: "alice@example.com",
});
```

### Week 39-40: Go and C++ SDKs

- [ ] Go SDK generator
- [ ] C++ SDK generator (with Boost.Asio option)
- [ ] gRPC client generation (for services using gRPC)
- [ ] SDK versioning and compatibility

```go
// Generated Go client
client := myservice.NewClient("https://api.example.com")
user, err := client.CreateUser(ctx, &myservice.CreateUserRequest{
    Name: "Alice",
    Email: "alice@example.com",
})
```

### A10 Deliverables

- `archimedes-codegen-python` - Python SDK generator
- `archimedes-codegen-typescript` - TypeScript SDK generator
- `archimedes-codegen-go` - Go SDK generator
- `archimedes-codegen-cpp` - C++ SDK generator
- SDK templates and runtime libraries

---

## Milestones Summary

| Milestone             | Target  | Criteria                         | Dependencies            |
| --------------------- | ------- | -------------------------------- | ----------------------- |
| **MVP Release**       |         |                                  |                         |
| A0: Shared Types      | Week 1  | Using `themis-platform-types`    | Themis creates crate    |
| A1: Foundation        | Week 4  | Core types, mock contracts       | `themis-platform-types` |
| A2: Server            | Week 8  | HTTP server, routing, handlers   | None                    |
| A3: Middleware        | Week 12 | Full pipeline with mocks         | None                    |
| A4: Observability     | Week 16 | Metrics, traces, logs, config    | None                    |
| A5: Integrated        | Week 20 | Themis + Eunomia integration     | Themis, Eunomia         |
| **Framework Release** |         |                                  |                         |
| A6: Core Framework    | Week 24 | Router, extractors (Axum parity) | MVP                     |
| A7: FastAPI Parity    | Week 28 | Handler macros, auto-docs        | A6                      |
| A8: Extended Features | Week 32 | WebSocket, SSE, background tasks | A7                      |
| A9: Developer Exp     | Week 36 | CLI, hot reload, templates       | A8                      |
| A10: Multi-Lang SDKs  | Week 40 | Python, TS, Go, C++ generators   | A9                      |

---

## Deliverables

### Core Crates (MVP)

- `themis-platform-types` - **Shared types** (dependency, not owned)
- `archimedes` - Main facade crate (re-exports)
- `archimedes-core` - Core types and traits
- `archimedes-server` - HTTP server (Hyper-based)
- `archimedes-middleware` - Middleware pipeline
- `archimedes-sentinel` - Themis contract validation
- `archimedes-authz` - OPA/Eunomia integration
- `archimedes-telemetry` - OpenTelemetry integration
- `archimedes-config` - Configuration management

### Framework Crates (New)

- `archimedes-router` - High-performance radix tree router
- `archimedes-extract` - Request extractors (Path, Query, Json, etc.)
- `archimedes-macros` - Proc macros (`#[handler]`, etc.)
- `archimedes-docs` - Auto-documentation (Swagger UI, ReDoc)
- `archimedes-ws` - WebSocket support
- `archimedes-sse` - Server-Sent Events
- `archimedes-tasks` - Background tasks and scheduled jobs
- `archimedes-db` - Database connection pooling

### Tools

- `archimedes-cli` - Command-line scaffolding tool
- `archimedes-dev` - Development server with hot reload

### Code Generators

- `archimedes-codegen-rust` - Rust client/server generation
- `archimedes-codegen-python` - Python SDK generation
- `archimedes-codegen-typescript` - TypeScript SDK generation
- `archimedes-codegen-go` - Go SDK generation
- `archimedes-codegen-cpp` - C++ SDK generation

### Features (Full Release)

- HTTP/1.1 and HTTP/2 support
- gRPC support (post-MVP)
- Fixed-order middleware pipeline
- Contract validation (enforced/monitor modes)
- OPA authorization
- OpenTelemetry observability
- Typed configuration
- Graceful shutdown
- **Custom high-performance router**
- **Type-safe extractors**
- **Handler macros (FastAPI-style)**
- **Auto-generated documentation**
- **WebSocket and SSE**
- **Background tasks and cron jobs**
- **Database connection pooling**
- **CLI scaffolding**
- **Hot reload development**
- **Multi-language SDK generation**

---

## Parallel Development Details

### What Can Be Done Without Themis

| Component     | Mock Strategy                          |
| ------------- | -------------------------------------- |
| Server        | Use hardcoded routes                   |
| Router        | Mock operation definitions             |
| Validation    | Mock schema that accepts/rejects       |
| Handler types | Manually define request/response types |

### What Can Be Done Without Eunomia

| Component        | Mock Strategy                     |
| ---------------- | --------------------------------- |
| AuthZ middleware | Always allow / always deny config |
| Policy loading   | Load from local file              |
| Decision caching | Standard cache implementation     |

### Integration Points (Week 17+)

| Integration         | Archimedes Side       | External Dependency    |
| ------------------- | --------------------- | ---------------------- |
| Contract validation | `archimedes-sentinel` | Themis artifact format |
| Code generation     | Handler types         | Themis codegen         |
| Authorization       | `archimedes-authz`    | Eunomia bundle format  |
| Policy push         | Control endpoint      | Eunomia control plane  |

---

## Risk Mitigation

### Technical Risks

1. **Hyper Complexity**

   - _Mitigation_: Start simple, add features incrementally

2. **OPA Integration**

   - _Mitigation_: Use OPA CLI first, then native integration

3. **Performance**
   - _Mitigation_: Benchmark early, optimize incrementally

### Schedule Risks

1. **Themis/Eunomia Delays**

   - _Mitigation_: 16 weeks of work is independent
   - Only 4 weeks blocked on integration

2. **Integration Complexity**
   - _Mitigation_: Define interfaces early, use mocks
