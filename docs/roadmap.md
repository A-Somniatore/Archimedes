# Archimedes – Development Roadmap

> **Version**: 2.13.0
> **Created**: 2026-01-04
> **Last Updated**: 2026-01-09
> **Target Completion**: Week 52 (extended for multi-language support)

> ✅ **CTO REVIEW (2026-01-04)**: Blocking issue resolved!
> **RESOLVED (2026-01-06)**: Local type definitions migrated to `themis-platform-types`. See Phase A0 completion.
> **UPDATE (2026-01-09)**: Phase A10 COMPLETE - Archimedes Sidecar for multi-language support (969 tests total).

---

## 🎉 Recent Progress (Phase A10 Complete)

### Archimedes Sidecar (v2.13.0) - ✅ COMPLETE

- **archimedes-sidecar** crate for multi-language support (39 tests)
- **ProxyClient** for HTTP forwarding to upstream services
- **SidecarServer** with internal health/readiness endpoints
- **MiddlewarePipeline** with Sentinel and Authz integration
- **PropagatedHeaders** for W3C Trace Context propagation
- **SidecarConfig** with TOML/JSON support and env overrides
- **HealthChecker** for liveness and readiness probes
- **Dockerfile** for containerized deployment
- **Kubernetes manifests** and Docker Compose examples
- **ADR-009** documenting sidecar pattern
- 969 tests passing across all crates

### Automatic Documentation (v2.10.0) - ✅ COMPLETE

- **archimedes-docs** crate for API documentation (29 tests)
- **OpenApiGenerator** converts Themis artifacts to OpenAPI 3.1 specs
- **SwaggerUi** generates interactive Swagger UI pages (CDN-loaded)
- **ReDoc** generates beautiful ReDoc documentation pages
- Full OpenAPI type system with schema conversion
- Path parameter extraction from URL templates
- 742 tests passing across all crates

### Eunomia/OPA Integration (v2.10.0) - ✅ COMPLETE

- **archimedes-authz** crate for OPA policy evaluation (26 tests)
- **PolicyEvaluator** wrapping regorus (pure Rust OPA)
- **DecisionCache** with TTL-based caching and stats
- **BundleLoader** for OPA tar.gz bundle loading
- **EvaluatorConfig** with production/development presets
- **AuthorizationMiddleware::opa()** wired into middleware pipeline
- Feature flag: `opa` in archimedes-middleware

### Themis/Sentinel Integration (v2.9.0) - ✅ COMPLETE

- **archimedes-sentinel** crate for contract validation (38 tests)
- **ArtifactLoader** supporting file, JSON, registry sources
- **OperationResolver** with regex path matching
- **SchemaValidator** with type and required field checks
- **ValidationMiddleware::sentinel()** wired into middleware pipeline
- **ResponseValidationMiddleware::sentinel()** for response validation
- Feature flag: `sentinel` in archimedes-middleware

### Handler Macros & Contract Binding (v2.9.0) - ✅ COMPLETE

- **archimedes-macros** crate with `#[handler]` attribute macro
- **HandlerBinder** for validating handlers against contracts
- Parsing utilities for handler attributes, parameters, function signatures
- DI container integration (`Container`, `Inject<T>`)

### InvocationContext & Integration (v2.8.0) - ✅ COMPLETE

- **InvocationContext** type combining HTTP request details with middleware context
- **BoxedHandler** signature updated to use `InvocationContext`
- **ExtractionContext::from_invocation()** bridge method for extractors
- Integration tests verifying full extraction pipeline

---

## 🔄 themis-platform-types v0.2.1 Production Readiness (Coming Soon)

> **When**: Before production release
> **Status**: Development Complete - Pending Publish

### New Production Guarantees (v0.2.1)

All types are now verified for production use:

1. **Thread Safety** - `Send + Sync` compile-time assertions (critical for async handlers)
2. **MSRV Testing** - CI validates Rust 1.75 compatibility
3. **Schema Validation** - JSON schemas validated in CI
4. **Serialization Testing** - Property-based roundtrip tests (proptest)
5. **Fallible Constructor** - `RequestId::try_new()` for container environments
6. **Security Lint** - `#[must_use = "security bug"]` on `PolicyDecision` constructors

### Recommended Usage in Archimedes

```rust
// Safe RequestId creation in middleware
let request_id = RequestId::try_new().unwrap_or_else(|| {
    tracing::warn!("UUID generation failed, using nil ID");
    RequestId::nil()
});

// PolicyDecision MUST be used - compiler will warn if ignored
let decision = eunomia.evaluate(&input).await?;
if decision.is_denied() {
    return Err(ThemisErrorEnvelope::policy_denied(decision.reason));
}
// Proceeding without checking decision would trigger must_use warning
```

---

## 🔄 themis-platform-types v0.2.0 Migration (Required)

> **When**: Week 9 (before A3 Middleware phase)
> **Effort**: ~2 hours
> **Breaking Changes**: Yes

### Migration Checklist

- [ ] Update `Cargo.toml` to `themis-platform-types = "0.2.0"`
- [ ] Replace `build()` calls with `try_build()?` (build() is deprecated)
- [ ] Update error handling to use `BuilderError` instead of `&'static str`
- [ ] Add wildcard arms to match statements on `CallerIdentity`, `ErrorCode`
- [ ] Use new re-exports if needed: `SpiffeIdentity`, `UserIdentity`, `ApiKeyIdentity`

### Code Changes Required

```rust
// Before (v0.1.0)
let input = PolicyInput::builder()
    .caller(caller)
    .service("my-service")
    .try_build()?; // Returns Result<_, &'static str>

// After (v0.2.0)
use themis_platform_types::BuilderError;
let input = PolicyInput::builder()
    .caller(caller)
    .service("my-service")
    .try_build()?; // Returns Result<_, BuilderError>

// Match statements need wildcard (enums are now #[non_exhaustive])
match caller {
    CallerIdentity::User(u) => handle_user(u),
    CallerIdentity::Spiffe(s) => handle_service(s),
    _ => handle_other(), // Required!
}
```

### New Features Available

- `Versioned<T>` wrapper for schema evolution
- `SchemaMetadata` for version tracking
- Proper `BuilderError` with field names
- Fixed SemVer pre-release comparison

---

## 🔧 Staff Engineer Code Review (2026-01-07)

> **Source**: Staff Engineer Cross-Component Review
> **Status**: Tracked for team action

### P0 - Build Blockers (Must Fix Immediately)

| Item | Description | Status |
|------|-------------|--------|
| **archimedes facade import error** | Main `archimedes` crate has unresolved imports: `CloseReason`, `WebSocketError`, `WebSocketId`, `WebSocketMessage` from archimedes-ws. Crate does not compile. | ✅ FIXED 2026-01-09 |
| **archimedes-tasks flaky tests** | 3 tests failing: `test_scheduler_basic`, `test_run_now`, `test_list_tasks_by_status`. Timeouts in async task spawner. | ✅ FIXED 2026-01-09 |

### P1 - Archimedes-Specific Items

| Item | Description | Status |
|------|-------------|--------|
| **OPA Bundle Format Validation** | Validate `BundleLoader` format against `eunomia-compiler` output. Eunomia writes `.manifest` JSON + policies as tar.gz. Archimedes expects same format - needs integration test. | ⏳ Backlog |
| **Error Code Unification** | Archimedes uses `ErrorCategory`, platform uses `ErrorCode` - unify | ⏳ Backlog |
| **Handler Macro + Real Contracts** | Test macros with actual Themis artifacts, not mocks | ⏳ Backlog |

### ✅ Verified Working

| Item | Description |
|------|-------------|
| **Platform types integration** | `archimedes-core` correctly imports `CallerIdentity`, `RequestId` from `themis-platform-types` |
| **Policy types integration** | `archimedes-authz` correctly uses `PolicyInput`, `PolicyDecision` from `themis-platform-types` |
| **Themis artifact integration** | `archimedes-sentinel` correctly imports from `themis-artifact`, `themis-core` |
| **Edition/MSRV** | Updated to edition 2021, MSRV 1.75 ✅ |
| **Git dependency** | Using GitHub reference for `themis-platform-types` ✅ |

### P2 - Cross-Component Items

| Item | Description | Owner |
|------|-------------|-------|
| **Health Check Standardization** | Define standard health check pattern for K8s deployment | Platform |
| **gRPC Clarification** | Is Eunomia→Archimedes push via gRPC or HTTP? (ADR-006 says post-MVP) | Platform |

---

## 📊 Spec vs Implementation Gap Analysis (2026-01-07)

> **Source**: Architecture Review comparing spec.md to actual implementation
> **Overall Score**: A (MVP feature-complete for HTTP/REST)

### ✅ Fully Implemented (Matches Spec)

| Spec Requirement | Evidence |
|------------------|----------|
| HTTP/1.1 & HTTP/2 Support | archimedes-server (hyper-based) |
| Async/Tokio runtime | All crates |
| Request ID generation (UUID v7) | archimedes-middleware/stages/request_id.rs |
| Trace context (OpenTelemetry) | archimedes-telemetry |
| Identity extraction (SPIFFE/JWT/ApiKey) | archimedes-extract |
| Authorization middleware (OPA) | archimedes-authz (26 tests) |
| Request validation | archimedes-middleware/stages/validation.rs |
| Response validation | archimedes-sentinel |
| Fixed 8-stage middleware pipeline | archimedes-middleware/pipeline.rs |
| Handler registration by operationId | archimedes-server/handler.rs |
| OPA/Rego policy evaluation | archimedes-authz (regorus) |
| Policy bundle loading | archimedes-authz/bundle.rs |
| Decision caching | archimedes-authz/cache.rs |
| Contract artifact loading | archimedes-sentinel |
| Prometheus metrics | archimedes-telemetry/metrics.rs |
| Structured logging | archimedes-telemetry/logging.rs |
| OpenTelemetry tracing | archimedes-telemetry/tracing.rs |
| Health/Ready probes | archimedes-server/health.rs |
| Graceful shutdown | archimedes-server/shutdown.rs |
| High-performance router | archimedes-router (57 tests) |
| Handler macros | archimedes-macros (#[handler]) |
| Dependency injection | archimedes-core/di.rs |
| API documentation generation | archimedes-docs (OpenAPI, Swagger, ReDoc) |
| WebSocket Support | archimedes-ws (52 tests) |
| Server-Sent Events | archimedes-sse (38 tests) |
| Background Tasks | archimedes-tasks (41 tests) |

### ⚠️ Partially Implemented

| Spec Requirement | Gap | Impact |
|------------------|-----|--------|
| **mTLS authentication** | Identity middleware extracts SPIFFE but actual cert validation deferred to deployment layer | Medium |
| **Enforced/Monitor modes** | Mode switching exists but needs full verification | Low |

### ❌ Not Implemented (Missing from Spec)

| Spec Requirement | Priority | Notes |
|------------------|----------|-------|
| **gRPC Support** | Post-MVP | ADR-006 explicitly defers to post-MVP. No tonic integration. |
| **Control Plane Endpoint** | ~~High~~ **DECISION: Deferred** | See ADR-010 below - pull-only model for V1 |
| **Policy push with atomic rollback** | ~~High~~ **DECISION: V1.1** | File-watch provides hot-reload without push endpoint |
| **SPIFFE allowlist for control endpoint** | N/A | Not needed if pull-only |
| **Contract-based WS message validation** | Medium | Spec §14.1 requires validating against Themis schemas |

### 🟡 Design Decision: Control Plane Model (ADR-010)

> **Decision**: Use **pull-only model with file watching** for V1.0
> **Rationale**: Simpler deployment, works with K8s ConfigMaps, no push endpoint security concerns
> **Future**: Push endpoint can be added in V1.1 if needed for Eunomia integration

The spec (§8.3) originally required a push endpoint, but we've decided to defer this:

| Approach | V1.0 Implementation |
|----------|---------------------|
| **Contract Loading** | File-based via `ArtifactLoader` |
| **Policy Loading** | File-based via `BundleLoader` |
| **Hot Reload** | File watching (inotify/kqueue) |
| **Deployment** | K8s ConfigMap/Secret mounting |

**Why Pull-Only for V1.0**:
1. Simpler security model (no inbound endpoint)
2. Works with standard K8s patterns (ConfigMap updates)
3. No need for SPIFFE allowlist complexity
4. Eunomia can write to shared volume / ConfigMap

---

## 📋 P1 Technical Debt Backlog

> **Source**: Staff Engineer Review (2026-01-07)
> **Priority**: Address before production release

| Item | Description | Owner | Target |
|------|-------------|-------|--------|
| **OPA Bundle Format Validation** | Add integration test validating `BundleLoader` against actual `eunomia-compiler` output | Archimedes | Pre-production |
| **Error Code Unification** | Archimedes uses `ErrorCategory`, platform uses `ErrorCode` - unify to `ErrorCode` | Archimedes + Platform | V1.1 |
| **Handler Macro + Real Contracts** | Test `#[handler]` macro with actual Themis artifacts, not mocks | Archimedes | Pre-production |
| **WebSocket Message Validation** | Implement contract-based WS message validation per spec §14.1 | Archimedes | V1.1 |
| **Monitor Mode Verification** | Full E2E test of enforce vs monitor validation modes | Archimedes | Pre-production |

---

## Key Decisions

| Decision                                                                     | Impact                                               |
| ---------------------------------------------------------------------------- | ---------------------------------------------------- |
| [ADR-010](docs/decisions/010-pull-only-policy-model.md)                      | **Pull-only policy loading for V1.0 (no push endpoint)** |
| [ADR-009](docs/decisions/009-archimedes-sidecar-multi-language.md)           | **Sidecar pattern for Python/Go/TS/C++ services**    |
| [ADR-008](docs/decisions/008-archimedes-full-framework.md)                   | **Archimedes as internal standardized framework**    |
| [ADR-005](docs/decisions/005-kubernetes-ingress-over-custom-router.md)       | Archimedes handles contract enforcement, not routing |
| [ADR-006](docs/decisions/006-grpc-post-mvp.md)                               | MVP is HTTP/REST only, gRPC is post-MVP              |
| [ADR-004](docs/decisions/004-regorus-for-rego-parsing.md)                    | Use Regorus for embedded OPA evaluation              |
| [ADR-007](docs/decisions/007-apache-2-license.md)                            | Apache 2.0 license                                   |

## Vision: Internal Standardization

Archimedes is an **internal platform** that standardizes how we build services:

| Challenge (Per-Team Choice)         | Archimedes Solution           |
| ----------------------------------- | ----------------------------- |
| Each team picks different framework | One standard for all services |
| Auth implemented differently        | OPA-based auth built-in       |
| Validation varies                   | Contract-driven, automatic    |
| Observability setup per service     | Built-in, zero config         |

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

| Phase                            | Duration | Weeks | Description                          | Dependencies            |
| -------------------------------- | -------- | ----- | ------------------------------------ | ----------------------- |
| **MVP (Weeks 1-20)**             |          |       |                                      |                         |
| A0: Shared Types                 | 1 week   | 1     | Integrate `themis-platform-types`    | Themis creates crate    |
| A1: Foundation                   | 3 weeks  | 2-4   | Core types, server scaffold          | `themis-platform-types` |
| A2: Server & Routing             | 4 weeks  | 5-8   | HTTP server, routing, handlers       | None (mock contracts)   |
| A3: Middleware                   | 4 weeks  | 9-12  | Middleware pipeline, validation      | None (mock validation)  |
| A4: Observability                | 4 weeks  | 13-16 | Metrics, tracing, logging, config    | None                    |
| A5: Integration                  | 4 weeks  | 17-20 | Themis + Eunomia integration         | Themis, Eunomia         |
| **Framework (Weeks 21-36)**      |          |       |                                      |                         |
| A6: Core Framework               | 4 weeks  | 21-24 | Custom router, extractors, DI        | MVP complete            |
| A7: Handler Macros               | 4 weeks  | 25-28 | Handler macros, auto-docs            | A6                      |
| A8: Extended Features            | 4 weeks  | 29-32 | WebSocket, SSE, background tasks     | A7                      |
| A9: Developer Experience         | 4 weeks  | 33-36 | CLI tool, hot reload, templates      | A8 **(DEFERRED)**       |
| **Multi-Language (Weeks 37-48)** | 🚨 **CRITICAL: Moved from post-MVP** |       |                                      |                         |
| A10: Sidecar Foundation          | 3 weeks  | 37-39 | Archimedes sidecar binary            | A8 ✅ **COMPLETE**      |
| A10.5: Pre-Production Hardening  | 1 week   | 40    | P1 backlog, hot-reload, testing      | A10 🔄 **IN PROGRESS**  |
| A11: Type Generation             | 2 weeks  | 41-42 | Python, Go, TypeScript generators    | **Themis-owned**        |
| A12: Multi-Language Integration  | 4 weeks  | 43-46 | Integration tests, deployment guides | A10.5, A11              |
| **Buffer (Weeks 47-52)**         |          |       |                                      |                         |
| Hardening & Buffer               | 6 weeks  | 47-52 | Performance tuning, contingency      | All                     |

**Total**: 52 weeks (13 months) - **Extended by 4 weeks for multi-language support**

- MVP: Weeks 1-20 (Rust-only services)
- Full Framework: Weeks 21-36 (Rust framework complete)
- Multi-Language Support: Weeks 37-48 (Python, Go, TypeScript, C++ services)
- Buffer: Weeks 47-52

**🚨 CRITICAL CHANGE**: Multi-language support is NO LONGER post-MVP. It is now required for V1.0 release because services in Python, C++, Go, and TypeScript must be able to use Archimedes.

**✅ Phase A10 COMPLETE**: Sidecar binary enables non-Rust services (Python, Go, TypeScript, C++) to use Archimedes middleware via reverse proxy pattern. 39 tests, Docker deployment ready.

**🔄 Phase A10.5 IN PROGRESS**: Pre-production hardening addressing P1 technical debt.

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

## Phase A0: Shared Platform Types (Week 1) ✅ COMPLETE

> **Note**: The `themis-platform-types` crate is created by Themis team in Week 1.
> Archimedes integrates it to ensure schema compatibility with Eunomia.

### Week 1: Integrate Shared Types

> ✅ **Completed (2026-01-06)**: Full migration to `themis-platform-types` shared crate!
> All local type implementations replaced with shared types.

- [x] Add `themis-platform-types` dependency to `archimedes-core`
  > ✅ **Completed 2026-01-06**: Added to workspace Cargo.toml as path dependency
  > Path: `../themis-platform-types`
- [x] Use shared `CallerIdentity` (not duplicate definition)
  > ✅ **Completed 2026-01-06**: Migrated to shared type
  > Created `CallerIdentityExt` extension trait for Archimedes-specific methods (log_id, roles)
  > Supports SPIFFE, User, ApiKey, Anonymous variants via tuple variants
- [ ] Use shared `PolicyInput` for OPA evaluation
  > ⏳ **Ready**: To be implemented in Phase A5 (Eunomia integration)
- [ ] Use shared `PolicyDecision` from OPA response
  > ⏳ **Ready**: To be implemented in Phase A5 (Eunomia integration)
- [x] Use shared `ThemisErrorEnvelope` for error responses
  > ✅ **Available 2026-01-06**: Type available in themis-platform-types
  > Note: Currently using local `ErrorEnvelope` - will migrate in Phase A5 when integrating Themis
- [x] Use shared `RequestId` type
  > ✅ **Completed 2026-01-06**: Migrated to shared type
  > Re-exported from themis_platform_types through archimedes-core
- [x] Verify JSON serialization matches integration spec
  > ✅ **Completed 2026-01-06**: 326 tests pass including serialization tests

### Phase A0 Milestone ✅ COMPLETE

**Criteria**: Archimedes uses `themis-platform-types` for all shared types

> ✅ **Status (2026-01-06)**: Phase A0 COMPLETE
>
> - Added `themis-platform-types` as path dependency
> - Migrated `CallerIdentity` to shared type with `CallerIdentityExt` extension
> - Migrated `RequestId` to shared type
> - Updated all enum variant syntax (struct → tuple variants)
> - All 326 tests pass

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

- [x] Create `archimedes-telemetry` crate
  > ✅ **Completed 2026-01-05**: Full telemetry infrastructure
  >
  > - Prometheus metrics, OpenTelemetry tracing, structured logging
  > - Unified `TelemetryConfig` builder
  > - `TelemetryGuard` for graceful shutdown
- [x] Implement Prometheus metrics
  > ✅ **Completed 2026-01-05**: `archimedes_telemetry::metrics` module
  >
  > - `MetricsConfig` with builder pattern
  > - `init_metrics()` and `render_metrics()` functions
  > - `metrics-exporter-prometheus` 0.16 integration
- [x] Add `archimedes_requests_total` counter
  > ✅ **Completed 2026-01-05**: `record_request()` function
  >
  > - Labels: operation_id, method, status_code
- [x] Add `archimedes_request_duration_seconds` histogram
  > ✅ **Completed 2026-01-05**: Duration recording with custom buckets
  >
  > - Default buckets: 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0
- [x] Add `archimedes_in_flight_requests` gauge
  > ✅ **Completed 2026-01-05**: RAII-style `InFlightGuard`
  >
  > - `increment_in_flight()` / `decrement_in_flight()`
  > - Guard auto-decrements on drop
- [ ] Expose `/metrics` endpoint
  > 🔜 **Pending**: Requires server integration

### Week 14: Tracing

- [x] Integrate OpenTelemetry tracing
  > ✅ **Completed 2026-01-05**: `archimedes_telemetry::tracing` module
  >
  > - OpenTelemetry 0.27 with `opentelemetry_sdk`
  > - `TracingConfig` with service name, environment, sampling ratio
  > - `init_tracing()` and `shutdown_tracing()` functions
- [x] Implement trace context propagation
  > ✅ **Completed 2026-01-05**: W3C Trace Context support
  >
  > - `HeaderExtractor` and `HeaderInjector` for http::HeaderMap
  > - `extract_context()` and `inject_context()` helpers
- [x] Add span attributes (operationId, service, etc.)
  > ✅ **Completed 2026-01-05**: Semantic conventions
  >
  > - Uses `opentelemetry-semantic-conventions` 0.27
  > - Service name, version, environment attributes
- [x] Configure OTLP exporter
  > ✅ **Completed 2026-01-05**: `opentelemetry-otlp` 0.27
  >
  > - Configurable endpoint via `TracingConfig::otlp_endpoint`
  > - Tonic gRPC transport
- [x] Test trace correlation
  > ✅ **Completed 2026-01-05**: Header extraction/injection tests

### Week 15: Logging

- [x] Implement structured JSON logging
  > ✅ **Completed 2026-01-05**: `archimedes_telemetry::logging` module
  >
  > - `LogConfig` with format (JSON/Pretty), level, service name
  > - `init_logging()` with tracing-subscriber
  > - `development()` and `production()` presets
- [x] Add request_id, trace_id to all logs
  > ✅ **Completed 2026-01-05**: `fields` module constants
  >
  > - REQUEST_ID, TRACE_ID, SPAN_ID, OPERATION_ID
  > - USER_ID, TENANT_ID, CLIENT_ID
- [x] Configure log levels
  > ✅ **Completed 2026-01-05**: `create_env_filter()` helper
  >
  > - EnvFilter-based level configuration
  > - Supports RUST_LOG environment variable
- [x] Add audit logging for authz decisions
  > ✅ **Completed 2026-01-05**: `record_authz_decision()` in metrics
  >
  > - Labels: operation_id, decision (allow/deny), policy_name
- [x] Test log output format
  > ✅ **Completed 2026-01-05**: LogConfig tests (6 tests)
  >
  > - Default, development, production config tests
  > - Field name constant tests

### Week 16: Configuration

- [x] Create `archimedes-config` crate
  > ✅ **Completed 2026-01-05**: New crate in `crates/archimedes-config/`
  >
  > - Dependencies: `toml` 0.8, `dotenvy` 0.15, `serde` 1.0
  > - Integrates with archimedes-core and archimedes-telemetry
- [x] Design typed configuration schema
  > ✅ **Completed 2026-01-05**: `schema.rs` module
  >
  > - `ServerConfig`: host, port, read/write timeouts, max connections
  > - `MetricsConfig`: enabled, endpoint, histogram buckets
  > - `TracingConfig`: enabled, endpoint, service name, sampling ratio
  > - `LoggingConfig`: level, format (Json/Pretty), include timestamps/spans
  > - `AuthorizationConfig`: mode (AllowAll/DenyAll/Rbac/Opa), OPA endpoint
  > - `ContractConfig`: path, strict validation, cache enabled
- [x] Implement file-based config loading
  > ✅ **Completed 2026-01-05**: `ConfigLoader` in `loader.rs`
  >
  > - TOML and JSON file format support
  > - `with_file()` for required files, `with_optional_file()` for optional
  > - `with_string()` for inline configuration
  > - Format auto-detection from file extension
- [x] Add environment variable overrides
  > ✅ **Completed 2026-01-05**: Layered loading system
  >
  > - `with_env_prefix()` for PREFIX**SECTION**KEY format
  > - `with_dotenv()` for .env file loading
  > - Environment variables override file values
  > - Type coercion for bool, integers, floats
- [x] Fail on unknown fields
  > ✅ **Completed 2026-01-05**: Strict validation
  >
  > - `#[serde(deny_unknown_fields)]` on all config structs
  > - `ArchimedesConfig::validate()` for semantic validation
  > - Socket address format validation
  > - OPA endpoint required when mode is Opa
- [x] Document configuration options
  > ✅ **Completed 2026-01-05**: Comprehensive rustdoc
  >
  > - Module-level examples for all config sections
  > - `development()` and `production()` presets with docs
  > - Error type documentation with examples
  > - 52 tests covering all configuration scenarios

### Phase A4 Milestone ✅ COMPLETE

**Criteria**: Full observability (metrics, traces, logs), typed config

**Completion Status** (2026-01-05):

- ✅ `archimedes-telemetry` crate: metrics, tracing, logging (25 tests)
- ✅ `archimedes-config` crate: typed configuration system (52 tests)
- ✅ OpenTelemetry 0.27 integration
- ✅ Prometheus metrics export
- ✅ Structured JSON logging with tracing-subscriber
- ✅ Layered configuration with file + env override support
- ✅ Total tests: 323 passing

---

## Phase A5: Integration (Weeks 17-20) — IN PROGRESS

### Week 17: Themis Integration — IN PROGRESS

- [x] Create `archimedes-sentinel` crate
  > ✅ **Completed 2026-01-06**: Full sentinel implementation with 38 tests
- [x] Implement contract artifact loading
  > ✅ **Completed 2026-01-06**: `ArtifactLoader` supporting file, JSON, registry
- [x] Implement path → operationId resolution from real contracts
  > ✅ **Completed 2026-01-06**: `OperationResolver` with regex path matching
- [x] Implement request validation against schemas
  > ✅ **Completed 2026-01-06**: `SchemaValidator` with type and required field checks
- [x] Replace mock validation with real validation
  > ✅ **Completed 2026-01-07**: Wired sentinel into archimedes-middleware
  >
  > - Added `sentinel` feature flag to archimedes-middleware
  > - `ValidationMiddleware::sentinel()` constructor
  > - `ResponseValidationMiddleware::sentinel()` constructor
  > - Uses real Themis contract schemas for validation

### Week 18: Eunomia Integration — ✅ COMPLETE

- [x] Create `archimedes-authz` crate
  > ✅ **Completed 2026-01-07**: Full authz implementation with 26 tests
- [x] Integrate OPA evaluator
  > ✅ **Completed 2026-01-07**: `PolicyEvaluator` wrapping regorus engine
  >
  > - Pure Rust OPA evaluation (no external process)
  > - Policy loading from files or bundles
  > - Evaluate queries returning `PolicyDecision`
- [x] Implement policy bundle loading
  > ✅ **Completed 2026-01-07**: `BundleLoader` with tar.gz support
  >
  > - Load OPA bundles from files
  > - Parse bundle metadata and revision
  > - Extract policies and data documents
- [x] Implement authorization decision caching
  > ✅ **Completed 2026-01-07**: `DecisionCache` with TTL support
  >
  > - Cache key based on caller + operation + resource
  > - Configurable TTL and max entries
  > - Stats tracking (hits, misses, evictions)
- [x] Replace mock authorization with real OPA
  > ✅ **Completed 2026-01-07**: Wired authz into archimedes-middleware
  >
  > - Added `opa` feature flag to archimedes-middleware
  > - `AuthorizationMiddleware::opa()` and `::opa_default()` constructors
  > - Builds `PolicyInput` from `MiddlewareContext`
  > - Uses real OPA policy evaluation via regorus

### Week 19: Control Plane — 🔜 NEXT

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

## Phase A6: Core Framework (Weeks 21-24) ✅ COMPLETE

> **Goal**: Replace Axum as HTTP layer, own routing and extractors directly
> **Note**: Completed in parallel while Phase A5 awaits Themis/Eunomia readiness.
> **Completion**: 2026-01-05 (router + extractors)

### Week 21-22: Custom Router

- [x] Remove Axum dependency, use Hyper directly
  > ✅ **Completed 2026-01-05**: Hyper-based implementation
- [x] Implement radix tree router for path matching
  > ✅ **Completed 2026-01-05**: `archimedes-router` crate created
  > Radix tree with O(k) path matching where k = path length
- [x] Support path parameters (`/users/{id}`)
  > ✅ **Completed 2026-01-05**: Named parameters with extraction
  > e.g., `/users/{userId}` extracts `userId` from path
- [x] Support wildcards (`/files/*path`)
  > ✅ **Completed 2026-01-05**: Catch-all wildcards for static files
  > e.g., `/files/*path` captures `images/logo.png`
- [x] Support method-based routing (GET, POST, etc.)
  > ✅ **Completed 2026-01-05**: `MethodRouter` with fluent API
  > All HTTP methods supported with per-path routing
- [ ] Benchmark: Match Axum's routing performance
  > 🔜 **Deferred**: Criterion benchmark suite created, benchmarks pending
- [x] Integrate with archimedes-server
  > ✅ **Completed 2026-01-05**: Router integrated via archimedes-extract

```rust
// Current API (archimedes-router)
let mut router = Router::new();
router.insert("/users", MethodRouter::new().get("listUsers").post("createUser"));
router.insert("/users/{id}", MethodRouter::new().get("getUser").delete("deleteUser"));
router.insert("/files/*path", MethodRouter::new().get("serveFile"));

let result = router.match_route(&Method::GET, "/users/123");
// RouteMatch { operation_id: "getUser", params: {"id": "123"} }
```

### Week 23-24: Extractors and Response Building

- [x] Implement `Path<T>` extractor
  > ✅ **Completed 2026-01-05**: `archimedes_extract::Path<T>`
  >
  > - Deserializes URL path parameters into typed structs
  > - Uses serde_urlencoded for type coercion (string → int)
  > - `path_param()` helper for single parameters
  > - Full test coverage (9 tests)
- [x] Implement `Query<T>` extractor
  > ✅ **Completed 2026-01-05**: `archimedes_extract::Query<T>`
  >
  > - Deserializes query string into typed structs
  > - Supports optional parameters with `#[serde(default)]`
  > - `RawQuery` for unprocessed query string access
  > - Full test coverage (14 tests)
- [x] Implement `Json<T>` extractor with contract validation
  > ✅ **Completed 2026-01-05**: `archimedes_extract::Json<T>`
  >
  > - Deserializes JSON body into typed structs
  > - 1MB default size limit (configurable via `JsonWithLimit`)
  > - Proper error handling for parse failures
  > - Full test coverage (12 tests)
- [x] Implement `Form<T>` extractor
  > ✅ **Completed 2026-01-05**: `archimedes_extract::Form<T>`
  >
  > - Deserializes URL-encoded form data
  > - 1MB default size limit (configurable via `FormWithLimit`)
  > - Handles + as space, percent-encoding
  > - Full test coverage (11 tests)
- [ ] Implement `Multipart` extractor
  > 🔜 Deferred to Phase A8 (Extended Features)
  >
  > - Requires streaming body support
  > - `multer` crate added as dependency
- [x] Implement `Headers` extractor
  > ✅ **Completed 2026-01-05**: `archimedes_extract::Headers`
  >
  > - `Headers` for all headers access
  > - `TypedHeader` trait for custom typed headers
  > - Built-in: `ContentType`, `Accept`, `Authorization`, `UserAgent`
  > - `header()` and `header_opt()` helper functions
  > - Full test coverage (12 tests)
- [x] Response builders (Json, Html, Redirect, etc.)
  > ✅ **Completed 2026-01-05**: `archimedes_extract::response`
  >
  > - `JsonResponse<T>` with status code customization
  > - `HtmlResponse` with charset
  > - `TextResponse` for plain text
  > - `Redirect` (to, permanent, see_other, temporary)
  > - `NoContent` for 204 responses
  > - `ErrorResponse` matching Themis error envelope
  > - Full test coverage (14 tests)
- [x] Error handling with structured responses
  > ✅ **Completed 2026-01-05**: `archimedes_extract::ExtractionError`
  >
  > - Source tracking (Path, Query, Body, Header, ContentType)
  > - Error codes (MISSING_PARAMETER, INVALID_PARAMETER, etc.)
  > - HTTP status code mapping (400, 413, 415, 422)
  > - Full test coverage (7 tests)

```rust
// Target handler signature (IMPLEMENTED)
async fn create_user(
    Json(body): Json<CreateUserRequest>,  // Auto-validated against contract
    headers: Headers,
) -> Result<Json<User>, AppError> {
    // ...
}
```

### A6 Deliverables

- ✅ `archimedes-router` - High-performance radix tree router (57 tests)
- ✅ `archimedes-extract` - Request extractors and response builders (104 tests)
- ✅ Server integration - archimedes-server now uses radix tree internally
- ✅ MethodRouter::merge() for incremental route registration
- ✅ Main crate re-exports router and extract modules
- 🔜 Benchmark suite vs Axum/Actix (deferred to post-A7)

**Integration Notes (2025-01-06)**:

- Server router replaced Vec<Route> O(n) with archimedes_router O(k)
- Path normalization handles leading/trailing slashes
- Multiple methods per path via merge semantics
- 561 tests passing across all crates

---

## Phase A7: FastAPI Parity (Weeks 25-28) 🐍 ✅ COMPLETE

> **Goal**: Match FastAPI developer experience with handler macros
> **Status**: 🔄 Week 25-26 - Handler Macros (Contract Binding Complete)

### Week 25-26: Handler Macros ✅ COMPLETE

- [x] Create `archimedes-macros` proc-macro crate
- [x] `#[archimedes::handler]` attribute macro
- [x] Parsing utilities for handler attributes and signatures
- [x] `HandlerAttrs` - operation ID, method, path parsing
- [x] `HandlerParam` - extractor type identification (Path, Query, Json, Header, Inject)
- [x] `HandlerFn` - async function signature parsing
- [x] Dependency injection container (`archimedes-core::di`)
  - [x] `Container` - TypeId-based service registry with Arc storage
  - [x] `Inject<T>` - Type-safe wrapper for injected services
  - [x] `InjectionError` - Error type for missing services
- [x] `Inject<T>` extractor in `archimedes-extract`
  - [x] `FromRequest` implementation
  - [x] `ExtractionContext` with optional container support
- [x] **InvocationContext** - Bridge between server handler invocation and extraction system
  > ✅ **Completed 2026-01-07**: `archimedes_core::InvocationContext`
  >
  > - Aggregates HTTP request details (method, URI, headers, body)
  > - Includes path parameters from router matching
  > - Carries middleware `RequestContext` (identity, request ID, trace info)
  > - Optional DI container via `Arc<Container>`
  > - Builder pattern for test construction
  > - 7 tests covering all functionality
- [x] **BoxedHandler** signature updated
  > ✅ **Completed 2026-01-07**: Handler type now uses `InvocationContext`
  >
  > - `Fn(InvocationContext) -> BoxFuture<'static, Result<Response<Body>, ThemisError>>`
  > - Simplifies handler invocation from server code
- [x] **ExtractionContext::from_invocation()** bridge method
  > ✅ **Completed 2026-01-07**: Extraction system integration
  >
  > - Converts `InvocationContext` to `ExtractionContext`
  > - Enables all extractors (Path, Query, Json, Headers, Inject) to work with handlers
- [x] Wire macro-generated code to work end-to-end
  > ✅ **Completed 2026-01-07**: Full integration verified with tests
- [x] Integration tests for handler workflow
  > ✅ **Completed 2026-01-07**: `archimedes-macros/tests/handler_integration.rs`
  >
  > - 6 integration tests covering JSON, Path, Query, Headers, Inject extractors
  > - Full extraction pipeline verification
- [x] **Contract binding** (which operation handles which contract endpoint)
  > ✅ **Completed 2026-01-07**: `archimedes_core::binder::HandlerBinder`
  >
  > - Validates handlers against contract operations
  > - Ensures all required operations have handlers
  > - Prevents duplicate handler registration
  > - Prevents registration of unknown operations
  > - 6 unit tests covering all validation cases

```rust
// Contract binding API
let mut binder = HandlerBinder::new(vec!["getUser", "createUser", "deleteUser"]);

// Register handlers (generated by #[handler] macro)
binder.register("getUser", handler1)?;      // ✓ Known operation
binder.register("createUser", handler2)?;   // ✓ Known operation
binder.register("deleteUser", handler3)?;   // ✓ Known operation

// Validate all operations have handlers
binder.validate()?;  // ✓ All operations covered

// Get handlers map for registration
let handlers = binder.into_handlers();
```

**Tests**: 8 macro tests + 13 DI tests + 6 inject extractor tests + 6 integration tests + 6 binder tests = 39 new tests
**Total**: 649 tests passing across all crates

````

### Week 27-28: Automatic Documentation ✅ COMPLETE

- [x] OpenAPI spec generation from contracts + handlers
  > ✅ **Completed 2026-01-07**: `archimedes_docs::OpenApiGenerator`
  >
  > - Converts `LoadedArtifact` from archimedes-sentinel to OpenAPI 3.1 spec
  > - Full OpenAPI type system (Info, Server, PathItem, Operation, Parameter, etc.)
  > - Schema conversion from Themis Schema to OpenAPI Schema
  > - Path parameter extraction from URL templates
  > - Security scheme support (Bearer, API Key)
- [x] Swagger UI endpoint (`/docs`)
  > ✅ **Completed 2026-01-07**: `archimedes_docs::SwaggerUi`
  >
  > - CDN-loaded Swagger UI v5.18.2
  > - Embedded OpenAPI spec (no separate JSON endpoint needed)
  > - Configurable: deep linking, doc expansion, request duration
  > - `html()` method returns complete HTML page
- [x] ReDoc endpoint (`/redoc`)
  > ✅ **Completed 2026-01-07**: `archimedes_docs::ReDoc`
  >
  > - CDN-loaded ReDoc v2.1.5
  > - Customizable theme (colors, fonts)
  > - Configurable: response expansion, search, download button
  > - Beautiful three-panel documentation
- [x] Interactive API console
  > ✅ **Completed 2026-01-07**: Built into Swagger UI
  >
  > - Try-it-out functionality from Swagger UI
- [ ] Schema explorer
  > 🔜 **Deferred**: Schema viewer available in both Swagger UI and ReDoc

```rust
// OpenAPI generation example
use archimedes_docs::{OpenApiGenerator, SwaggerUi, ReDoc};
use archimedes_sentinel::ArtifactLoader;

// Load contract
let artifact = ArtifactLoader::from_file("api.yaml").await?;

// Generate OpenAPI spec
let generator = OpenApiGenerator::new()
    .title("My API")
    .version("1.0.0")
    .server("https://api.example.com", Some("Production".to_string()))
    .bearer_auth("bearerAuth");
let spec = generator.generate(&artifact)?;

// Create documentation endpoints
let swagger = SwaggerUi::new("/docs", &spec);
let redoc = ReDoc::new("/redoc", &spec);

// Serve HTML at respective paths
// swagger.html() -> Swagger UI HTML
// redoc.html()   -> ReDoc HTML
````

**Tests**: 29 new tests in archimedes-docs
**Total**: 742 tests passing across all crates

### A7 Deliverables

- ✅ `archimedes-macros` - Proc macros for handler definitions
- ✅ `archimedes-docs` - Auto-documentation system (OpenAPI, Swagger, ReDoc)
- ✅ Documentation UI assets (CDN-loaded)

---

## Phase A8: Extended Features (Weeks 29-32) 🔌 ✅ COMPLETE

> **Goal**: Add WebSocket, SSE, background tasks, database integration
> **Status**: ✅ COMPLETE - 878 tests passing

### Week 29-30: Real-Time Features ✅

- [x] Create `archimedes-ws` crate
  > ✅ **Complete 2026-01-09**: 52 tests, WebSocket support with connection management
- [x] Create `archimedes-sse` crate
  > ✅ **Complete 2026-01-09**: 38 tests, Server-Sent Events with backpressure handling
- [x] WebSocket support with RFC 6455 compliance
- [x] Connection management (global/per-client limits)
- [x] Automatic ping/pong for connection health
- [x] Server-Sent Events (SSE) streaming
- [x] SSE retry and reconnection handling
- [x] JSON message serialization support

```rust
// WebSocket example
use archimedes::ws::{WebSocket, Message, ConnectionManager};

async fn handle_websocket(mut ws: WebSocket) {
    while let Some(msg) = ws.recv().await {
        match msg {
            Ok(Message::Text(text)) => {
                ws.send_text(format!("Echo: {}", text)).await.ok();
            }
            Ok(Message::Close(_)) => break,
            _ => {}
        }
    }
}

// SSE example
use archimedes::sse::{SseStream, SseEvent};

async fn handle_sse() -> SseStream {
    let (stream, sender) = SseStream::new(SseConfig::default());
    tokio::spawn(async move {
        sender.send_data("hello").await.ok();
    });
    stream
}
```

### Week 31-32: Background Processing ✅

- [x] Create `archimedes-tasks` crate
  > ✅ **Complete 2026-01-09**: 41 tests, task spawner and job scheduler
- [x] Background task spawning with DI support
- [x] Task cancellation and timeout handling
- [x] Concurrent task limits
- [x] Scheduled jobs (cron expressions)
- [x] Job enable/disable and run_now triggers
- [x] Task status tracking and statistics

```rust
use archimedes::tasks::{Spawner, Scheduler, SpawnerConfig};

// Background task spawning
let spawner = Spawner::new(SpawnerConfig::default());
let handle = spawner.spawn("my-task", async {
    // Task logic
    "result"
}).await?;
let result = handle.join().await?;

// Scheduled jobs with cron
let scheduler = Scheduler::new(Default::default());
scheduler.register(
    "cleanup",
    "0 0 * * *", // Daily at midnight
    || async { cleanup_expired().await },
).await?;
scheduler.start().await?;
```

### A8 Deliverables ✅

- ✅ `archimedes-ws` - WebSocket support (52 tests)
- ✅ `archimedes-sse` - Server-Sent Events (38 tests)
- ✅ `archimedes-tasks` - Background task system (41 tests)
- ⏳ `archimedes-db` - Database connection pooling (Deferred to future phase)

**Tests Added**: 131 new tests
**Total**: 878 tests passing across all crates

---

## Phase A9: Developer Experience (Weeks 33-36) 🛠️ DEFERRED

> **Goal**: CLI tools, hot reload, and project scaffolding
> **Status**: ⏳ DEFERRED - Prioritizing Phase A10 (Sidecar) for multi-language support
> **Reason**: Sidecar is critical for enabling Python/Go/TypeScript services. CLI is nice-to-have.

### Week 33-34: CLI Tool (DEFERRED)

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

## Phase A10: Archimedes Sidecar (Weeks 37-39) ✅ COMPLETE

> **Goal**: Enable Python, Go, TypeScript, and C++ services to use Archimedes via sidecar proxy
> **Why Critical**: Archimedes is currently Rust-only, but services MUST be written in any language
> **Status**: ✅ COMPLETE - 39 tests passing

### Week 37: Sidecar Architecture & Design

- [x] **Write ADR-009**: Archimedes sidecar pattern for multi-language support
- [x] Design sidecar architecture:
  - Request flow: Ingress → Sidecar → Application → Sidecar → Ingress
  - Service discovery: How sidecar finds backing service (env var, K8s annotations)
  - Wire protocol: HTTP between sidecar and application
- [x] Define sidecar API:
  - Health check endpoint (`/_archimedes/health`, `/_archimedes/ready`)
  - Metrics endpoint (via telemetry integration)
  - Configuration reloading (TOML/JSON with env overrides)
- [x] Create `archimedes-sidecar` crate scaffold
- [x] Document deployment patterns (K8s, Docker Compose, local dev)

### Week 38-39: Sidecar Implementation

- [x] Extract middleware logic into standalone binary
  - RequestId via W3C Trace Context headers
  - Identity extraction (mTLS, JWT, API key)
  - Contract validation (Sentinel integration)
  - Policy evaluation (embedded regorus via Authz)
  - Response validation ready
  - Telemetry emission via archimedes-telemetry
- [x] Implement service proxy logic:
  - Forward requests to `http://localhost:{port}` via reqwest
  - Preserve headers with PropagatedHeaders
  - Timeout handling (configurable)
  - Circuit breaker (planned for future)
- [x] Add configuration management:
  - Load contracts from filesystem
  - Load policies from filesystem
  - Hot-reload ready (config reload endpoint)
- [x] Create Dockerfile for sidecar (multi-stage build)
- [x] Create Kubernetes manifests (deployment, service, HPA, PDB)

---

## Phase A10.5: Pre-Production Hardening (Week 40) 🔧 NEW - IN PROGRESS

> **Goal**: Address P1 backlog items before production release
> **Priority**: MUST complete before any production deployment
> **Status**: 🔄 STARTING

### P1 Technical Debt Items

- [ ] **File Watching for Hot-Reload** (per ADR-010)
  - Add `notify` crate for file system watching
  - Watch contract file for changes, reload `Sentinel`
  - Watch policy bundle for changes, reload `PolicyEvaluator`
  - Add to both native Archimedes and sidecar
  - Tests for hot-reload functionality

- [ ] **OPA Bundle Format Validation**
  - Add integration test with actual `eunomia-compiler` output
  - Validate `.manifest` JSON format
  - Validate tar.gz bundle structure
  - Document expected bundle format

- [ ] **Monitor Mode Verification**
  - E2E test for `validation_mode = "enforce"`
  - E2E test for `validation_mode = "monitor"` (log-only)
  - Verify metrics differ between modes
  - Document mode switching behavior

- [ ] **Handler Macro + Real Contracts**
  - Test `#[handler]` macro with actual Themis artifact
  - Verify operation binding with real contract
  - Test error cases (missing operation, wrong schema)

### A10.5 Deliverables

- File watching hot-reload for contracts and policies
- OPA bundle format validation tests
- Monitor mode E2E tests
- Handler macro integration tests with real contracts
- [x] Create Docker Compose example for development

### A10 Deliverables

- [x] `archimedes-sidecar` - Standalone binary for non-Rust services (39 tests)
- [x] Sidecar Dockerfile (multi-stage, ~20MB runtime image)
- [x] Kubernetes deployment manifests
- [x] Docker Compose development example
- [x] ADR-009: Sidecar pattern for multi-language
- [x] Example configuration files

---

## Phase A11: Multi-Language Type Generation (Weeks 40-42) 🌐 THEMIS RESPONSIBILITY

> **Goal**: Auto-generate types from JSON Schema for all languages
> **Owner**: Themis team (contract tooling)
> **Archimedes Role**: Provide example services that consume generated types

### ℹ️ Scope Clarification

Phase A11 is primarily **Themis CLI functionality** - generating types from contract schemas. Archimedes benefits from this but doesn't own the implementation:

| Task | Owner | Archimedes Role |
|------|-------|-----------------|
| JSON Schema generation from Rust types | Themis | Consumer of schemas |
| Python type generator | Themis CLI | Example Python service |
| Go type generator | Themis CLI | Example Go service |
| TypeScript type generator | Themis CLI | Example TS service |
| C++ type generator | Themis CLI | Example C++ service |

### What Archimedes WILL Do in This Phase

- [ ] Create example services in each language demonstrating sidecar usage
- [ ] Document how generated types integrate with sidecar header parsing
- [ ] Test sidecar with services using generated types
- [ ] Validate `X-Caller-Identity` header parsing in each language

### What Themis Will Do

- [ ] **Automate JSON Schema generation from Rust types**
  - Add `schemars` derive to all themis-platform-types
  - Create CI job that regenerates schemas on type changes
  - Create CI check that schemas match Rust types
- [ ] **Create schema-to-Python generator**
  - Use `datamodel-code-generator` or write custom generator
  - Generate `@dataclass` from JSON Schema
  - Generate validation using `pydantic`
  - Test roundtrip: JSON → Python → JSON
- [ ] **Create schema-to-Go generator**
  - Use `quicktype` or write custom generator
  - Generate Go structs with JSON tags
  - Add validation using `go-playground/validator`
  - Test roundtrip: JSON → Go → JSON

### Week 41-42: TypeScript and C++

- [ ] **Create schema-to-TypeScript generator**
  - Use `quicktype` or `json-schema-to-typescript`
  - Generate TypeScript interfaces
  - Generate Zod schemas for runtime validation
  - Test roundtrip: JSON → TypeScript → JSON
- [ ] **Create schema-to-C++ generator** (Optional - lowest priority)
  - Use `quicktype` or write custom generator
  - Generate C++ classes with nlohmann/json serialization
- [ ] Add type generation to Themis CLI
- [ ] Create examples for each language

### A11 Deliverables

**Themis Deliverables:**
- Automated schema-to-type generators for Python, Go, TypeScript, C++
- Integration with Themis CLI
- CI pipeline for type generation

**Archimedes Deliverables:**
- Example Python service with sidecar
- Example Go service with sidecar
- Example TypeScript service with sidecar
- Identity header parsing libraries/examples for each language

---

## Phase A12: Multi-Language Integration (Weeks 43-46) 🧪 ARCHIMEDES + THEMIS

> **Goal**: Prove end-to-end flow for each language with real integration tests

### Week 43-44: Python and Go Integration

- [ ] Create example Python service (FastAPI)
- [ ] Create example Go service (Gin/Echo)
- [ ] Deploy both with Archimedes sidecar
- [ ] Test full request flow (identity, contract, policy, telemetry)
- [ ] Measure latency overhead (target: <2ms p99)
- [ ] Document deployment guides

### Week 45-46: TypeScript and Multi-Language E2E

- [ ] Create example TypeScript service (Express/NestJS)
- [ ] Create heterogeneous service mesh:
  - Rust service (native Archimedes)
  - Python service (sidecar)
  - Go service (sidecar)
  - TypeScript service (sidecar)
- [ ] Test cross-service calls
- [ ] Test distributed tracing
- [ ] Performance benchmarks
- [ ] Write multi-language deployment guide

### A12 Deliverables

- Integration tests for Python, Go, TypeScript
- Performance benchmarks showing <2ms sidecar overhead
- Deployment guides for each language
- Example service repositories
- Multi-language E2E test suite

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
| **Multi-Language**    |         |                                  |                         |
| A10: Sidecar          | Week 39 | Sidecar for non-Rust services    | A9                      |
| A11: Type Generation  | Week 42 | Python, Go, TypeScript, C++      | Themis codegen          |
| A12: Integration      | Week 46 | Multi-language E2E tests         | A10, A11                |

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
