# Archimedes – Development Roadmap

> **Version**: 3.6.0
> **Created**: 2026-01-04
> **Last Updated**: 2026-01-22
> **Target Completion**: Week 48 (MVP complete with sidecar pattern)

> ✅ **CTO REVIEW (2026-01-04)**: Blocking issue resolved!
> **RESOLVED (2026-01-06)**: Local type definitions migrated to `themis-platform-types`. See Phase A0 completion.
> **UPDATE (2026-01-09)**: Phase A10 COMPLETE - Archimedes Sidecar for multi-language support.
> **UPDATE (2026-01-09)**: Phase A10.5 COMPLETE - P1 items addressed. 1019 tests passing (964 executed, 55 ignored).
> **UPDATE (2026-01-10)**: Phase A12 (Example Services) STARTED - Created example services for Rust, Python, Go, TypeScript, and C++.
> **UPDATE (2026-01-10)**: Phase A13.1 (Core FFI Layer) COMPLETE - 44 tests, archimedes-ffi crate with C ABI.
> **📋 UPDATE (2026-01-21)**: Phase A13.2-A13.5 (Native Bindings) moved to POST-MVP. Sidecar pattern is sufficient for MVP.
> **📋 UPDATE (2026-01-21)**: Benchmarking (A13.6) deprioritized - not required for MVP.
> **✅ UPDATE (2026-01-21)**: **MVP SCOPE FINALIZED** - Sidecar pattern provides multi-language support. All pieces working together.
> **📊 UPDATE (2026-01-21)**: All 4 components building and passing tests. Themis build fixed (8 errors in themis-sdk resolved).
> **✅ UPDATE (2026-01-22)**: **Native Server Middleware Integration COMPLETE** - `archimedes-server` now supports middleware pipeline with `MiddlewareConfig` builder.

---

## 🎉 Recent Progress (Phase A12 In Progress → Phase A13 Planned)

### 🔧 Native Server Middleware Integration (v3.6.0) - ✅ COMPLETE

> **UPDATE 2026-01-22**: The `archimedes-server` crate now integrates the middleware pipeline directly.
> This enables native Rust services to use identity extraction, authorization, and validation without the sidecar.

**New Features:**
- ✅ `MiddlewareConfig` builder for configuring middleware stages
- ✅ `ServerBuilder::middleware()` method to enable middleware pipeline
- ✅ Identity extraction from HTTP headers (JWT, X-Caller-Identity, X-User-Id)
- ✅ Caller identity propagated to `RequestContext` for handlers
- ✅ Base64 JWT parsing for Bearer tokens (without cryptographic verification)
- ✅ Trusted identity header support for proxy/sidecar scenarios
- ✅ 141 tests passing in archimedes-server

**Usage Example:**
```rust
use archimedes_server::{Server, MiddlewareConfig};

let server = Server::builder()
    .http_addr("0.0.0.0:8080")
    .middleware(MiddlewareConfig::builder()
        .enable_identity()       // Extract identity from headers
        .enable_authorization()  // Enable OPA policy evaluation
        .service_name("my-service")
        .build())
    .build();
```

**Supported Identity Headers:**
- `Authorization: Bearer <jwt>` - JWT token parsing
- `X-Caller-Identity: <json>` - Trusted proxy identity
- `X-User-Id` + `X-User-Roles` - Simple identity headers

### 🚨 ARCHITECTURE DECISION: Multi-Language Support (v3.5.0)

**MVP Decision**: Archimedes Sidecar pattern provides multi-language support for MVP. Native bindings are POST-MVP.

**MVP (Sidecar Pattern)**:
- ✅ **Any language** can use Archimedes via sidecar reverse proxy
- ✅ **Teams keep existing frameworks** (FastAPI, Express, Gin, etc.)
- ✅ **Full middleware pipeline** (auth, validation, telemetry) handled by sidecar
- ✅ **39 tests passing**, Docker deployment ready

**POST-MVP (Native Bindings)** - Optional performance optimization:
- Native bindings for Python, Go, TypeScript, C++ via FFI
- Eliminates ~1-2ms sidecar latency overhead
- Only needed for ultra-low-latency services

| Language       | MVP (Sidecar)      | Post-MVP (Native) | Status              |
| -------------- | ------------------ | ----------------- | ------------------- |
| **Rust**       | N/A (native)       | N/A               | ✅ Complete         |
| **Python**     | ✅ FastAPI+Sidecar | PyO3 bindings     | 📋 Post-MVP         |
| **TypeScript** | ✅ Express+Sidecar | napi-rs bindings  | 📋 Post-MVP         |
| **C++**        | ✅ httplib+Sidecar | C ABI bindings    | 📋 Post-MVP         |
| **Go**         | ✅ net/http+Sidecar| cgo bindings      | 📋 Post-MVP         |

### Multi-Language Example Services (v2.16.0) - ✅ MVP COMPLETE

> **MVP Complete**: These examples use language-native frameworks with the sidecar pattern. This is the recommended approach for MVP. Native bindings are POST-MVP.

| Language       | Directory                     | Framework         | Status           | Port |
| -------------- | ----------------------------- | ----------------- | ---------------- | ---- |
| **Rust**       | `examples/rust-native`        | Archimedes        | ✅ MVP Complete  | 8001 |
| **Python**     | `examples/python-sidecar`     | FastAPI + Sidecar | ✅ MVP Complete  | 8002 |
| **TypeScript** | `examples/typescript-sidecar` | Express + Sidecar | ✅ MVP Complete  | 8004 |
| **C++**        | `examples/cpp-sidecar`        | cpp-httplib + Sidecar | ✅ MVP Complete | 8005 |
| **Go**         | `examples/go-sidecar`         | net/http + Sidecar| ✅ MVP Complete  | 8003 |

**Each example includes:**

- Complete User CRUD API (List, Get, Create, Update, Delete)
- Health check endpoint
- Sidecar header parsing (`X-Request-Id`, `X-Caller-Identity`, `X-Operation-Id`)
- Dockerfile for containerized deployment
- README with setup and testing instructions

**Shared resources:**

- `examples/contract.json` - Themis contract with 6 operations
- `examples/docker-compose.yml` - Unified deployment for all services

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

- **archimedes-authz** crate for OPA policy evaluation (26 → 26 tests after bundle validation)
- **PolicyEvaluator** wrapping regorus (pure Rust OPA)
- **DecisionCache** with TTL-based caching and stats
- **BundleLoader** for OPA tar.gz bundle loading with comprehensive format validation
- **EvaluatorConfig** with production/development presets
- **AuthorizationMiddleware::opa()** wired into middleware pipeline
- Feature flag: `opa` in archimedes-middleware
- ✅ **NEW (2026-01-09)**: Bundle integration tests (11 tests) validating eunomia-compiler format

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

### Configuration Hot-Reload (v2.15.0) - ✅ COMPLETE

- **archimedes-config** crate with FileWatcher for hot-reload (67 tests)
- **FileWatcher** with notify crate for cross-platform file monitoring
- **FileChangeEvent** and **FileChangeKind** for change notifications
- **FileWatcherBuilder** for flexible watcher configuration
- Debouncing to prevent reload storms
- Extension filtering (watch only specific file types)
- Recursive directory watching support
- Async event handling with poll() and next() methods
- Per ADR-010: Enables hot-reload of configuration and contracts without service restart

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

| Item                               | Description                                                                                                                                                    | Status              |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| **archimedes facade import error** | Main `archimedes` crate has unresolved imports: `CloseReason`, `WebSocketError`, `WebSocketId`, `WebSocketMessage` from archimedes-ws. Crate does not compile. | ✅ FIXED 2026-01-09 |
| **archimedes-tasks flaky tests**   | 3 tests failing: `test_scheduler_basic`, `test_run_now`, `test_list_tasks_by_status`. Timeouts in async task spawner.                                          | ✅ FIXED 2026-01-09 |

### P1 - Archimedes-Specific Items

| Item                               | Description                                                                                                                                                                      | Status                  |
| ---------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| **OPA Bundle Format Validation**   | Validate `BundleLoader` format against `eunomia-compiler` output. Eunomia writes `.manifest` JSON + policies as tar.gz. Archimedes expects same format - needs integration test. | ✅ DONE 2026-01-09 (11) |
| **Handler Macro + Real Contracts** | Test macros with actual Themis artifacts, not mocks                                                                                                                              | ✅ DONE 2026-01-09 (9)  |
| **Monitor Mode Verification**      | Full E2E test of enforce vs monitor validation modes                                                                                                                             | ✅ DONE 2026-01-09 (7)  |
| **Error Code Unification**         | Archimedes uses `ErrorCategory`, platform uses `ErrorCode` - unify                                                                                                               | 🔄 V1.1                 |

### ✅ Verified Working

| Item                            | Description                                                                                    |
| ------------------------------- | ---------------------------------------------------------------------------------------------- |
| **Platform types integration**  | `archimedes-core` correctly imports `CallerIdentity`, `RequestId` from `themis-platform-types` |
| **Policy types integration**    | `archimedes-authz` correctly uses `PolicyInput`, `PolicyDecision` from `themis-platform-types` |
| **Themis artifact integration** | `archimedes-sentinel` correctly imports from `themis-artifact`, `themis-core`                  |
| **Edition/MSRV**                | Updated to edition 2021, MSRV 1.75 ✅                                                          |
| **Git dependency**              | Using GitHub reference for `themis-platform-types` ✅                                          |

### P2 - Cross-Component Items

| Item                             | Description                                                          | Owner    |
| -------------------------------- | -------------------------------------------------------------------- | -------- |
| **Health Check Standardization** | Define standard health check pattern for K8s deployment              | Platform |
| **gRPC Clarification**           | Is Eunomia→Archimedes push via gRPC or HTTP? (ADR-006 says post-MVP) | Platform |

---

## 📊 Spec vs Implementation Gap Analysis (2026-01-07)

> **Source**: Architecture Review comparing spec.md to actual implementation
> **Overall Score**: A (MVP feature-complete for HTTP/REST)

### ✅ Fully Implemented (Matches Spec)

| Spec Requirement                        | Evidence                                   |
| --------------------------------------- | ------------------------------------------ |
| HTTP/1.1 & HTTP/2 Support               | archimedes-server (hyper-based)            |
| Async/Tokio runtime                     | All crates                                 |
| Request ID generation (UUID v7)         | archimedes-middleware/stages/request_id.rs |
| Trace context (OpenTelemetry)           | archimedes-telemetry                       |
| Identity extraction (SPIFFE/JWT/ApiKey) | archimedes-extract                         |
| Authorization middleware (OPA)          | archimedes-authz (26 tests)                |
| Request validation                      | archimedes-middleware/stages/validation.rs |
| Response validation                     | archimedes-sentinel                        |
| Fixed 8-stage middleware pipeline       | archimedes-middleware/pipeline.rs          |
| Handler registration by operationId     | archimedes-server/handler.rs               |
| OPA/Rego policy evaluation              | archimedes-authz (regorus)                 |
| Policy bundle loading                   | archimedes-authz/bundle.rs                 |
| Decision caching                        | archimedes-authz/cache.rs                  |
| Contract artifact loading               | archimedes-sentinel                        |
| Prometheus metrics                      | archimedes-telemetry/metrics.rs            |
| Structured logging                      | archimedes-telemetry/logging.rs            |
| OpenTelemetry tracing                   | archimedes-telemetry/tracing.rs            |
| Health/Ready probes                     | archimedes-server/health.rs                |
| Graceful shutdown                       | archimedes-server/shutdown.rs              |
| High-performance router                 | archimedes-router (57 tests)               |
| Handler macros                          | archimedes-macros (#[handler])             |
| Dependency injection                    | archimedes-core/di.rs                      |
| API documentation generation            | archimedes-docs (OpenAPI, Swagger, ReDoc)  |
| WebSocket Support                       | archimedes-ws (52 tests)                   |
| Server-Sent Events                      | archimedes-sse (38 tests)                  |
| Background Tasks                        | archimedes-tasks (41 tests)                |

### ⚠️ Partially Implemented

| Spec Requirement           | Gap                                                                                         | Impact |
| -------------------------- | ------------------------------------------------------------------------------------------- | ------ |
| **mTLS authentication**    | Identity middleware extracts SPIFFE but actual cert validation deferred to deployment layer | Medium |
| **Enforced/Monitor modes** | Mode switching exists but needs full verification                                           | Low    |

### ❌ Not Implemented (Missing from Spec)

| Spec Requirement                          | Priority                        | Notes                                                        |
| ----------------------------------------- | ------------------------------- | ------------------------------------------------------------ |
| **gRPC Support**                          | Post-MVP                        | ADR-006 explicitly defers to post-MVP. No tonic integration. |
| **Control Plane Endpoint**                | ~~High~~ **DECISION: Deferred** | See ADR-010 below - pull-only model for V1                   |
| **Policy push with atomic rollback**      | ~~High~~ **DECISION: V1.1**     | File-watch provides hot-reload without push endpoint         |
| **SPIFFE allowlist for control endpoint** | N/A                             | Not needed if pull-only                                      |
| **Contract-based WS message validation**  | Medium                          | Spec §14.1 requires validating against Themis schemas        |

### 🟡 Design Decision: Control Plane Model (ADR-010)

> **Decision**: Use **pull-only model with file watching** for V1.0
> **Rationale**: Simpler deployment, works with K8s ConfigMaps, no push endpoint security concerns
> **Future**: Push endpoint can be added in V1.1 if needed for Eunomia integration

The spec (§8.3) originally required a push endpoint, but we've decided to defer this:

| Approach             | V1.0 Implementation             |
| -------------------- | ------------------------------- |
| **Contract Loading** | File-based via `ArtifactLoader` |
| **Policy Loading**   | File-based via `BundleLoader`   |
| **Hot Reload**       | File watching (inotify/kqueue)  |
| **Deployment**       | K8s ConfigMap/Secret mounting   |

**Why Pull-Only for V1.0**:

1. Simpler security model (no inbound endpoint)
2. Works with standard K8s patterns (ConfigMap updates)
3. No need for SPIFFE allowlist complexity
4. Eunomia can write to shared volume / ConfigMap

---

## 📋 P1 Technical Debt Backlog

> **Source**: Staff Engineer Review (2026-01-07)
> **Priority**: Address before production release
> **Last Updated**: 2026-01-09 - 3 of 5 items complete

| Item                               | Description                                                                             | Owner                 | Status             |
| ---------------------------------- | --------------------------------------------------------------------------------------- | --------------------- | ------------------ |
| **OPA Bundle Format Validation**   | Add integration test validating `BundleLoader` against actual `eunomia-compiler` output | Archimedes            | ✅ DONE (11 tests) |
| **Handler Macro + Real Contracts** | Test `#[handler]` macro with actual Themis artifacts, not mocks                         | Archimedes            | ✅ DONE (9 tests)  |
| **Monitor Mode Verification**      | Full E2E test of enforce vs monitor validation modes                                    | Archimedes            | ✅ DONE (7 tests)  |
| **Error Code Unification**         | Archimedes uses `ErrorCategory`, platform uses `ErrorCode` - unify to `ErrorCode`       | Archimedes + Platform | 🔄 V1.1            |
| **WebSocket Message Validation**   | Implement contract-based WS message validation per spec §14.1                           | Archimedes            | ⏳ V1.1            |

---

## Key Decisions

| Decision                                                               | Impact                                                  |
| ---------------------------------------------------------------------- | ------------------------------------------------------- |
| [ADR-011](docs/decisions/011-native-language-bindings.md)              | **🆕 Native bindings replace FastAPI/Express/Gin/etc.** |
| [ADR-010](docs/decisions/010-pull-only-policy-model.md)                | Pull-only policy loading for V1.0 (no push endpoint)    |
| [ADR-009](docs/decisions/009-archimedes-sidecar-multi-language.md)     | Sidecar pattern (transitional, for migration)           |
| [ADR-008](docs/decisions/008-archimedes-full-framework.md)             | **Archimedes as internal standardized framework**       |
| [ADR-005](docs/decisions/005-kubernetes-ingress-over-custom-router.md) | Archimedes handles contract enforcement, not routing    |
| [ADR-006](docs/decisions/006-grpc-post-mvp.md)                         | MVP is HTTP/REST only, gRPC is post-MVP                 |
| [ADR-004](docs/decisions/004-regorus-for-rego-parsing.md)              | Use Regorus for embedded OPA evaluation                 |
| [ADR-007](docs/decisions/007-apache-2-license.md)                      | Apache 2.0 license                                      |

## Vision: Internal Standardization

Archimedes is an **internal platform** that standardizes how we build services:

| Challenge (Per-Team Choice)         | Archimedes Solution                        |
| ----------------------------------- | ------------------------------------------ |
| Each team picks different framework | **One framework for all languages**        |
| Python uses FastAPI, Go uses Gin    | **All use Archimedes native bindings**     |
| Auth implemented differently        | OPA-based auth built-in                    |
| Validation varies                   | Contract-driven, automatic                 |
| Observability setup per service     | Built-in, zero config                      |
| Different APIs per language         | **Consistent API across Python/Go/TS/C++** |

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
- **🆕 Native bindings for Python, Go, TypeScript, C++**

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

| Phase                             | Duration                             | Weeks | Description                          | Dependencies            |
| --------------------------------- | ------------------------------------ | ----- | ------------------------------------ | ----------------------- |
| **MVP (Weeks 1-20)**              |                                      |       |                                      |                         |
| A0: Shared Types                  | 1 week                               | 1     | Integrate `themis-platform-types`    | Themis creates crate    |
| A1: Foundation                    | 3 weeks                              | 2-4   | Core types, server scaffold          | `themis-platform-types` |
| A2: Server & Routing              | 4 weeks                              | 5-8   | HTTP server, routing, handlers       | None (mock contracts)   |
| A3: Middleware                    | 4 weeks                              | 9-12  | Middleware pipeline, validation      | None (mock validation)  |
| A4: Observability                 | 4 weeks                              | 13-16 | Metrics, tracing, logging, config    | None                    |
| A5: Integration                   | 4 weeks                              | 17-20 | Themis + Eunomia integration         | Themis, Eunomia         |
| **Framework (Weeks 21-36)**       |                                      |       |                                      |                         |
| A6: Core Framework                | 4 weeks                              | 21-24 | Custom router, extractors, DI        | MVP complete            |
| A7: Handler Macros                | 4 weeks                              | 25-28 | Handler macros, auto-docs            | A6                      |
| A8: Extended Features             | 4 weeks                              | 29-32 | WebSocket, SSE, background tasks     | A7                      |
| A9: Developer Experience          | 4 weeks                              | 33-36 | CLI tool, hot reload, templates      | A8 **(DEFERRED)**       |
| **Multi-Language (Weeks 37-48)**  | 🚨 **CRITICAL: Moved from post-MVP** |       |                                      |                         |
| A10: Sidecar Foundation           | 3 weeks                              | 37-39 | Archimedes sidecar binary            | A8 ✅ **COMPLETE**      |
| A10.5: Pre-Production Hardening   | 1 week                               | 40    | P1 backlog, hot-reload, testing      | A10 ✅ **COMPLETE**     |
| A11: Type Generation              | 2 weeks                              | 41-42 | Python, Go, TypeScript generators    | **Themis-owned**        |
| A12: Multi-Language Integration   | 4 weeks                              | 43-46 | Integration tests, deployment guides | A10.5, A11              |
| **Buffer & Hardening**            |                                      |       |                                      |                         |
| Hardening & Buffer                | 2 weeks                              | 47-48 | Performance tuning, contingency      | All                     |
| **POST-MVP: Native Bindings**     | 📋 **Optional: Native language support** |   |                                      |                         |
| A13.1: Core FFI Layer             | 4 weeks                              | -     | C ABI, memory-safe bindings          | Post-MVP                |
| A13.2: Python Bindings (PyO3)     | 4 weeks                              | -     | archimedes-py package                | Post-MVP                |
| A13.3: Go Bindings (cgo)          | 3 weeks                              | -     | archimedes-go module                 | Post-MVP                |
| A13.4: TypeScript Bindings        | 3 weeks                              | -     | @archimedes/node package             | Post-MVP                |
| A13.5: C++ Bindings               | 2 weeks                              | -     | libarchimedes headers                | Post-MVP                |

**Total**: 48 weeks (12 months) - **MVP complete with sidecar pattern**

- MVP: Weeks 1-20 (Rust-only services)
- Full Framework: Weeks 21-36 (Rust framework complete)
- Multi-Language Support: Weeks 37-46 (Sidecar pattern - **MVP COMPLETE**)
- Buffer: Weeks 47-48
- Post-MVP: Native bindings (optional performance optimization)

**✅ MVP SCOPE FINALIZED**: Multi-language support via sidecar pattern is complete. Services in Python, C++, Go, and TypeScript can use Archimedes middleware via the sidecar. Native bindings are POST-MVP for teams needing ultra-low-latency.

**✅ Phase A10 COMPLETE**: Sidecar binary enables non-Rust services (Python, Go, TypeScript, C++) to use Archimedes middleware via reverse proxy pattern. 39 tests, Docker deployment ready.

**✅ Phase A10.5 COMPLETE**: Pre-production hardening addressing P1 technical debt.

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

## Phase A10.5: Pre-Production Hardening (Week 40) ✅ COMPLETE

> **Goal**: Address P1 backlog items before production release
> **Priority**: MUST complete before any production deployment
> **Status**: ✅ COMPLETE (2026-01-09)
> **Tests Added**: 27 new tests (9 sentinel, 11 bundle, 7 monitor mode)
> **Total Tests**: 1019 (964 passed, 55 ignored)

### P1 Technical Debt Items

- [x] **File Watching for Hot-Reload** (per ADR-010)

  - Add `notify` crate for file system watching
  - Watch contract file for changes, reload `Sentinel`
  - Watch policy bundle for changes, reload `PolicyEvaluator`
  - Add to both native Archimedes and sidecar
  - Tests for hot-reload functionality (archimedes-config: 67 tests)

- [x] **OPA Bundle Format Validation** ✅ 2026-01-09

  - Add integration test with actual `eunomia-compiler` output
  - Validate `.manifest` JSON format
  - Validate tar.gz bundle structure
  - Document expected bundle format
  - **Tests**: `bundle_integration.rs` (11 tests)

- [x] **Monitor Mode Verification** ✅ 2026-01-09

  - E2E test for `validation_mode = "enforce"`
  - E2E test for `validation_mode = "monitor"` (allow-all)
  - Verify reject/allow behavior differs between modes
  - Document mode switching behavior
  - **Tests**: `pipeline_e2e.rs` (7 tests)

- [x] **Handler Macro + Real Contracts** ✅ 2026-01-09
  - Test `#[handler]` macro with actual Themis artifact
  - Verify operation binding with real contract
  - Test error cases (missing operation, deprecated)
  - **Tests**: `sentinel_integration.rs` (9 tests)

### A10.5 Deliverables

- [x] File watching hot-reload for contracts and policies
- [x] OPA bundle format validation tests (11 tests)
- [x] Monitor mode E2E tests (7 tests)
- [x] Handler macro integration tests with real contracts (9 tests)
- [x] Create Docker Compose example for development
- [x] Fixed unreachable pattern warnings in CallerIdentity matches

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
> **Status**: ⏳ WAITING ON THEMIS

### ℹ️ Scope Clarification

Phase A11 is primarily **Themis CLI functionality** - generating types from contract schemas. Archimedes benefits from this but doesn't own the implementation:

| Task                                   | Owner      | Archimedes Role        |
| -------------------------------------- | ---------- | ---------------------- |
| JSON Schema generation from Rust types | Themis     | Consumer of schemas    |
| Python type generator                  | Themis CLI | Example Python service |
| Go type generator                      | Themis CLI | Example Go service     |
| TypeScript type generator              | Themis CLI | Example TS service     |
| C++ type generator                     | Themis CLI | Example C++ service    |

### What Archimedes WILL Do in This Phase

- [x] Create example services in each language demonstrating sidecar usage
  - ✅ Rust native service (`examples/rust-native`)
  - ✅ Python FastAPI service (`examples/python-sidecar`)
  - ✅ Go net/http service (`examples/go-sidecar`)
  - ✅ TypeScript Express service (`examples/typescript-sidecar`)
  - ✅ C++ cpp-httplib service (`examples/cpp-sidecar`)
- [x] Document how generated types integrate with sidecar header parsing
- [ ] Test sidecar with services using generated types (waiting on Themis)
- [x] Validate `X-Caller-Identity` header parsing in each language

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
> **Status**: 🔄 IN PROGRESS - Example services created, integration testing pending

### Week 43-44: Python and Go Integration

- [x] Create example Python service (FastAPI)
- [x] Create example Go service (net/http)
- [ ] Deploy both with Archimedes sidecar
- [ ] Test full request flow (identity, contract, policy, telemetry)
- [ ] Measure latency overhead (target: <2ms p99)
- [x] Document deployment guides

### Week 45-46: TypeScript and Multi-Language E2E

- [x] Create example TypeScript service (Express)
- [x] Create heterogeneous service mesh:
  - ✅ Rust service (native Archimedes) - `examples/rust-native`
  - ✅ Python service (sidecar) - `examples/python-sidecar`
  - ✅ Go service (sidecar) - `examples/go-sidecar`
  - ✅ TypeScript service (sidecar) - `examples/typescript-sidecar`
  - ✅ C++ service (sidecar) - `examples/cpp-sidecar`
- [ ] Test cross-service calls
- [ ] Test distributed tracing
- [ ] Performance benchmarks
- [x] Write multi-language deployment guide (`examples/README.md`)

### A12 Deliverables

- [x] Example services for Python, Go, TypeScript, C++, Rust
- [ ] Integration tests for Python, Go, TypeScript
- [ ] Performance benchmarks showing <2ms sidecar overhead
- [x] Deployment guides for each language
- [ ] Example service repositories (currently in monorepo)
- [ ] Multi-language E2E test suite

---

## Phase A13: Native Language Bindings (Weeks 47-64) 🚀 NEW

> **Goal**: Archimedes becomes THE framework for all languages - replacing FastAPI, Flask, Express, Gin, etc.
> **Status**: 📋 PLANNED
> **Decision**: [ADR-011](docs/decisions/011-native-language-bindings.md) (to be written)

### Why Native Bindings?

The sidecar pattern (Phase A10) works but has limitations:

| Sidecar Limitations                        | Native Bindings Solution  |
| ------------------------------------------ | ------------------------- |
| Extra network hop (latency)                | In-process function calls |
| Separate process (memory, deployment)      | Single binary             |
| Header parsing required in each language   | Direct struct access      |
| Framework-specific code (FastAPI, Express) | One consistent API        |
| Two things to maintain (sidecar + service) | One unified codebase      |

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         ARCHIMEDES CORE (Rust)                           │
│                                                                          │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  HTTP Server │ Router │ Middleware │ Validation │ Auth │ Telemetry │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                    │                                     │
│                     ┌──────────────┼──────────────┐                     │
│                     ▼              ▼              ▼                     │
│              ┌─────────────┐ ┌─────────────┐ ┌─────────────┐           │
│              │  C ABI      │ │  C ABI      │ │  C ABI      │           │
│              │  (stable)   │ │  (stable)   │ │  (stable)   │           │
│              └─────────────┘ └─────────────┘ └─────────────┘           │
└─────────────────────────────────────────────────────────────────────────┘
                     │              │              │
        ┌────────────┘              │              └────────────┐
        ▼                          ▼                           ▼
┌───────────────┐          ┌───────────────┐          ┌───────────────┐
│ archimedes-py │          │ archimedes-go │          │ @archimedes/  │
│   (PyO3)      │          │   (cgo)       │          │    node       │
│               │          │               │          │  (napi-rs)    │
└───────────────┘          └───────────────┘          └───────────────┘
        │                          │                           │
        ▼                          ▼                           ▼
┌───────────────┐          ┌───────────────┐          ┌───────────────┐
│ Python App    │          │   Go App      │          │  Node.js App  │
│               │          │               │          │               │
│ @app.get("/") │          │ app.Get("/")  │          │ app.get("/")  │
│ def handler() │          │ func handler()│          │ (req) => {}   │
└───────────────┘          └───────────────┘          └───────────────┘
```

### Phase A13.1: Core FFI Layer (Weeks 47-50) ✅ COMPLETE

> **Goal**: Create stable C ABI for Archimedes core functionality
> **Status**: COMPLETE - 44 tests passing

#### Week 47-48: FFI Foundation ✅

- [x] Create `archimedes-ffi` crate with C ABI exports
- [x] Define stable memory layout for cross-language types:

  ```rust
  // C-compatible types
  #[repr(C)]
  pub struct ArchimedesConfig {
      pub listen_addr: *const c_char,
      pub listen_port: u16,
      pub contract_path: *const c_char,
      // ...
  }

  #[repr(C)]
  pub struct RequestContext {
      pub request_id: [u8; 16],  // UUID as bytes
      pub method: *const c_char,
      pub path: *const c_char,
      pub caller_identity: *const CallerIdentity,
      // ...
  }
  ```

- [x] Implement callback-based handler registration:

  ```rust
  pub type HandlerCallback = extern "C" fn(
      ctx: *const RequestContext,
      body: *const u8,
      body_len: usize,
      response_out: *mut *mut u8,
      response_len_out: *mut usize,
  ) -> i32;

  #[no_mangle]
  pub extern "C" fn archimedes_register_handler(
      operation_id: *const c_char,
      callback: HandlerCallback,
  ) -> i32;
  ```

- [x] Memory management functions (alloc/free)
- [x] Error handling with error codes and messages

#### Week 49-50: FFI Testing & Stability ✅

- [x] Create C header file (`archimedes.h`) with cbindgen
- [x] Create FFI integration tests (44 tests)
- [x] Document memory ownership rules
- [ ] Benchmark FFI call overhead (target: <100ns)
- [x] Version the ABI (semver for C ABI)

### Phase A13.2: Python Bindings - Full Rust Parity (Weeks 51-58) 🔄 IN PROGRESS

> **Goal**: `pip install archimedes` - Python developers use Archimedes directly **with FULL Rust parity**
> **Technology**: PyO3 (Rust-Python bindings)
> **Status**: HTTP server working, middleware integration pending
> **Tests**: 69 passing tests (context: 18, handlers: 16, config: 13, server: 22)
> **UPDATE 2026-01-08**: Server module complete, comprehensive test coverage added

#### CRITICAL: Full Rust Parity Requirements

Python bindings MUST have the same behavior as native Rust Archimedes. The goal is **exact parity** - every feature in `examples/rust-native` must work identically in `examples/python-native`.

| Feature                   | Rust Status | Python Status | Priority | Notes                             |
| ------------------------- | ----------- | ------------- | -------- | --------------------------------- |
| HTTP Server (hyper)       | ✅ Complete | ✅ Complete   | P0       | server.rs with graceful shutdown  |
| Handler Registration      | ✅ Complete | ✅ Complete   | P0       | @app.handler decorator            |
| Request Context           | ✅ Complete | ✅ Complete   | P0       | PyRequestContext with all fields  |
| Response Builder          | ✅ Complete | ✅ Complete   | P0       | PyResponse with status methods    |
| Path Parameter Extraction | ✅ Complete | ✅ Complete   | P0       | ctx.path_params["userId"]         |
| Query Parameters          | ✅ Complete | ✅ Complete   | P0       | ctx.query() and ctx.query_all()   |
| Header Access             | ✅ Complete | ✅ Complete   | P0       | ctx.header() case-insensitive     |
| Health/Ready Endpoints    | ✅ Complete | ✅ Complete   | P0       | Built-in /health, /ready          |
| Request ID Generation     | ✅ Complete | ⚠️ Basic      | P0       | Generated but not middleware      |
| Identity (PyIdentity)     | ✅ Complete | ✅ Complete   | P0       | Roles, permissions, claims        |
| Authorization Checks      | ✅ Complete | ✅ Types only | P0       | has_role(), has_permission()      |
| Request ID Middleware     | ✅ Complete | ❌ Missing    | P0       | Add to pipeline                   |
| Tracing/OpenTelemetry     | ✅ Complete | ❌ Missing    | P0       | Wire to archimedes-telemetry      |
| Identity Extraction       | ✅ Complete | ❌ Missing    | P0       | Wire to archimedes-extract        |
| Authorization (OPA)       | ✅ Complete | ❌ Missing    | P0       | Wire to archimedes-authz          |
| Request Validation        | ✅ Complete | ❌ Missing    | P0       | Wire to archimedes-sentinel       |
| Response Validation       | ✅ Complete | ❌ Missing    | P0       | Wire to archimedes-sentinel       |
| Error Normalization       | ✅ Complete | ⚠️ Basic      | P0       | Need ThemisError envelope         |
| Telemetry Collection      | ✅ Complete | ❌ Missing    | P0       | Wire to archimedes-telemetry      |
| Contract-based Routing    | ✅ Complete | ❌ Missing    | P1       | Wire to archimedes-router         |
| Graceful Shutdown         | ✅ Complete | ✅ Complete   | P1       | watch::channel shutdown           |
| Configuration             | ✅ Complete | ✅ Complete   | P1       | YAML/JSON, env vars               |

#### Rust→Python Mapping (For Parity)

| Rust Crate             | Python Equivalent                | Status      |
| ---------------------- | -------------------------------- | ----------- |
| archimedes-server      | archimedes-py/server.rs          | ✅ Complete |
| archimedes-core        | archimedes-py/context.rs         | ✅ Complete |
| archimedes-extract     | Not needed (Python dynamic)      | N/A         |
| archimedes-middleware  | archimedes-py/middleware.rs      | ❌ Missing  |
| archimedes-sentinel    | Wire via FFI or re-implement     | ❌ Missing  |
| archimedes-authz       | Wire via FFI or re-implement     | ❌ Missing  |
| archimedes-telemetry   | Wire via FFI or OpenTelemetry-py | ❌ Missing  |
| archimedes-router      | archimedes-py/server.rs (basic)  | ⚠️ Partial  |

#### Implementation Summary

Created `archimedes-py` crate with comprehensive Python bindings:

- **Core Classes**: `PyApp`, `PyConfig`, `PyRequestContext`, `PyIdentity`, `PyResponse`
- **Server Module**: `PyServer` with hyper/tokio, graceful shutdown, health endpoints
- **Handler System**: `HandlerRegistry` with decorator-based registration
- **Configuration**: YAML/JSON config loading, environment variable support
- **Error Handling**: Custom `ArchimedesError` Python exception
- **Type Stubs**: Complete `.pyi` files for IDE autocomplete support
- **Build System**: maturin-based build with pyproject.toml
- **Test Coverage**: 69 tests covering context, handlers, config, server

#### Week 51-52: Core Python Module ✅ COMPLETE

- [x] Create `archimedes-py` crate using PyO3 v0.24
- [x] Python-native API design:

  ```python
  from archimedes import App, Config, Response

  config = Config.from_file("archimedes.yaml")
  app = App(config)

  @app.operation("listUsers")
  def list_users(ctx):
      # ctx.identity provides caller information
      # Request already validated by middleware
      return Response.json({"users": []})
  ```

- [x] Type stubs (`.pyi` files) for IDE support
- [x] Request/Response types with builder pattern
- [x] Configuration system (from_file, from_env)

#### Week 53-54: Python HTTP Server ✅ BASIC COMPLETE

- [x] Create `archimedes` Python package structure (pyproject.toml)
- [x] Module naming: `_archimedes` native + `archimedes` Python wrapper
- [x] HTTP server with hyper (basic routing)
- [x] Handler invocation from Python
- [x] Example: `python-native` with 6 handlers + test.sh

#### Week 55-56: Middleware Integration 📋 PLANNED

- [ ] Request ID middleware (generate UUID, add to context)
- [ ] Tracing middleware (OpenTelemetry spans)
- [ ] Identity extraction middleware (from headers/JWT)
- [ ] Error normalization middleware
- [ ] Telemetry collection middleware

#### Week 57-58: Authorization & Validation 📋 PLANNED

- [ ] Authorization middleware (OPA/Eunomia integration)
- [ ] Request validation middleware (JSON Schema)
- [ ] Response validation middleware
- [ ] Contract-based routing (Sentinel integration)
- [ ] pytest plugin for testing handlers
- [ ] Migration guide: FastAPI → Archimedes
- [ ] Benchmark: archimedes-py vs FastAPI

### Phase A13.3: TypeScript/Node.js Bindings (Weeks 59-62)

> **Goal**: `npm install @archimedes/node` - Node.js developers use Archimedes directly
> **Technology**: napi-rs (Rust-Node.js bindings)

#### Week 59-60: Core Node Module

- [ ] Create `archimedes-node` crate using napi-rs
- [ ] TypeScript-first API design:

  ```typescript
  import { Archimedes, Request, Response } from "@archimedes/node";

  const app = new Archimedes({ contract: "contract.json" });

  app.operation("listUsers", async (request: Request): Promise<Response> => {
    // request.callerIdentity is typed!
    // request.body is already validated!
    const users = await db.getUsers();
    return Response.json({ users });
  });

  app.listen(8080);
  ```

- [ ] Full TypeScript types (no `any`)
- [ ] Native Promise support
- [ ] Streaming support for SSE/WebSocket

#### Week 61-62: Node Ecosystem Integration

- [ ] Publish to npm as `@archimedes/node`
- [ ] Jest/Vitest testing utilities
- [ ] OpenTelemetry JS integration
- [ ] Migration guide: Express/Fastify → Archimedes
- [ ] Benchmark: @archimedes/node vs Fastify (target: 1.5x throughput)
- [ ] Full middleware parity with Rust

### Phase A13.4: C++ Bindings (Weeks 63-65)

> **Goal**: `#include <archimedes/archimedes.hpp>` - C++ developers use Archimedes directly
> **Technology**: Direct C ABI with C++ wrapper

#### Week 63-64: C++ Headers

- [ ] Create `libarchimedes` with C++ wrapper headers
- [ ] Modern C++ API (C++17+):

  ```cpp
  #include <archimedes/archimedes.hpp>

  int main() {
      archimedes::App app{"contract.json"};

      app.operation("listUsers", [](const archimedes::Request& req) {
          // req.caller_identity() is typed!
          // req.body() is already validated!
          auto users = db.get_users();
          return archimedes::Response::json({{"users", users}});
      });

      app.run(8080);
  }
  ```

- [ ] RAII for resource management
- [ ] CMake integration
- [ ] vcpkg/conan package
- [ ] Header-only option for simple cases

#### Week 65: C++ Ecosystem Integration

- [ ] Full middleware parity with Rust
- [ ] Documentation and examples
- [ ] Benchmark: libarchimedes vs cpp-httplib

### Phase A13.5: Go Bindings (Weeks 66-69)

> **Goal**: `go get github.com/themis-platform/archimedes-go` - Go developers use Archimedes directly
> **Technology**: cgo (C bindings for Go)

#### Week 66-67: Core Go Module

- [ ] Create `archimedes-go` module using cgo
- [ ] Go-idiomatic API design:

  ```go
  package main

  import "github.com/themis-platform/archimedes-go"

  func main() {
      app := archimedes.New(archimedes.Config{
          Contract: "contract.json",
      })

      app.Operation("listUsers", func(ctx *archimedes.Context) error {
          // ctx.CallerIdentity is typed!
          // ctx.Body is already validated!
          users, err := db.GetUsers()
          if err != nil {
              return err
          }
          return ctx.JSON(200, map[string]any{"users": users})
      })

      app.Run(":8080")
  }
  ```

- [ ] Context with typed request access
- [ ] Error handling with Go idioms
- [ ] Static linking option (no cgo dependency in prod)

#### Week 68-69: Go Ecosystem Integration

- [ ] Create Go module with proper versioning
- [ ] Testing utilities
- [ ] OpenTelemetry Go integration
- [ ] Migration guide: Gin/Chi → Archimedes
- [ ] Benchmark: archimedes-go vs Gin (target: 1.5x throughput)
- [ ] Full middleware parity with Rust

### A13 Deliverables

| Deliverable            | Language   | Package Name           | Phase | Status         |
| ---------------------- | ---------- | ---------------------- | ----- | -------------- |
| Core FFI Layer         | C          | libarchimedes.so       | A13.1 | ✅ Complete    |
| Python Bindings        | Python     | archimedes (PyPI)      | A13.2 | 🔄 In Progress |
| TypeScript Bindings    | TypeScript | @archimedes/node (npm) | A13.3 | 📋 Planned     |
| C++ Bindings           | C++        | libarchimedes (vcpkg)  | A13.4 | 📋 Planned     |
| Go Bindings            | Go         | archimedes-go (module) | A13.5 | 📋 Planned     |
| Migration Guides       | All        | docs/migration/        | -     | 📋 Planned     |
| Performance Benchmarks | All        | benchmarks/            | -     | 📋 Planned     |

### Performance Targets

| Language   | Metric                  | Target            |
| ---------- | ----------------------- | ----------------- |
| Python     | Requests/sec vs FastAPI | ≥2x improvement   |
| TypeScript | Requests/sec vs Fastify | ≥1.5x improvement |
| C++        | FFI overhead per call   | <100ns            |
| Go         | Requests/sec vs Gin     | ≥1.5x improvement |
| All        | Memory per connection   | <10KB baseline    |

---

## Phase A13.6: Performance Benchmarking (Weeks 69-70) 🔥 PRIORITY #1

> **Goal**: Establish performance baselines and prove Archimedes is faster than competing frameworks
> **Status**: 📋 PLANNED
> **Priority**: **#1** - This validates our claim that Archimedes is 5-20x faster

### Why Benchmarking is Critical

Archimedes is built on Rust/Tokio/Hyper which should deliver:

| Comparison              | Expected Improvement | Rationale                                  |
| ----------------------- | -------------------- | ------------------------------------------ |
| Archimedes vs FastAPI   | 10-30x faster        | No Python GIL, no interpreter overhead     |
| Archimedes vs Flask     | 20-100x faster       | Flask is synchronous + Python overhead     |
| Archimedes vs Express   | 5-15x faster         | Node.js event loop has inherent overhead   |
| Archimedes vs Gin       | 2-5x faster          | Go's GC pauses, Rust has zero-cost allocs  |
| Archimedes vs Axum      | ~1x (parity)         | Same tech stack, validation adds overhead  |

### Benchmarking Deliverables

#### Week 69: Benchmark Infrastructure

- [ ] Create `benches/` directory with Criterion benchmarks
- [ ] Set up benchmark CI (run on every PR, store results)
- [ ] Create standard benchmark scenarios:
  - [ ] Hello World (minimal overhead)
  - [ ] JSON serialization (medium payload)
  - [ ] Large response (1MB payload)
  - [ ] Concurrent requests (10k, 100k connections)
  - [ ] Contract validation overhead
  - [ ] OPA policy evaluation overhead
- [ ] Integrate with TechEmpower benchmark suite format
- [ ] Create `BENCHMARKS.md` documenting methodology

#### Week 70: Cross-Framework Comparison

- [ ] Benchmark Archimedes (Rust native) vs:
  - [ ] FastAPI (Python) - same endpoints
  - [ ] Flask (Python) - same endpoints
  - [ ] Axum (Rust) - same endpoints
  - [ ] Actix-web (Rust) - same endpoints
  - [ ] Express (Node.js) - same endpoints
  - [ ] Gin (Go) - same endpoints
- [ ] Measure metrics:
  - [ ] Requests per second (RPS)
  - [ ] Latency percentiles (p50, p95, p99)
  - [ ] Memory usage under load
  - [ ] CPU utilization
  - [ ] Startup time
- [ ] Document results with graphs
- [ ] Publish results to README and docs

### Benchmark Tools

| Tool     | Purpose                          |
| -------- | -------------------------------- |
| wrk      | HTTP benchmarking (RPS, latency) |
| hey      | Simple load testing              |
| k6       | Complex scenario testing         |
| Criterion| Rust micro-benchmarks            |
| perf     | CPU profiling                    |
| heaptrack| Memory profiling                 |

### Expected Results (Hypothesis)

```
Benchmark: GET /users (JSON response, 10 users)
================================================================
Framework        | RPS       | p50 (ms) | p99 (ms) | Memory
-----------------+-----------+----------+----------+---------
Archimedes       | 150,000   | 0.2      | 1.5      | 45 MB
Axum             | 145,000   | 0.2      | 1.5      | 42 MB
Actix-web        | 148,000   | 0.2      | 1.4      | 40 MB
Gin              | 80,000    | 0.5      | 3.0      | 60 MB
Express          | 25,000    | 2.0      | 15.0     | 120 MB
FastAPI          | 12,000    | 4.0      | 25.0     | 180 MB
Flask            | 3,000     | 15.0     | 80.0     | 200 MB
================================================================
```

### A13.6 Milestone

**Criteria**: Documented benchmarks proving Archimedes is ≥2x faster than FastAPI

> 📊 Status: 📋 PLANNED
>
> - Benchmark infrastructure
> - Cross-framework comparison
> - Published results with methodology

---

## Phase A14: Framework Parity (Weeks 71-78) 📋 PLANNED

> **Goal**: Achieve feature parity with FastAPI and Axum to enable seamless migrations
> **Status**: 📋 PLANNED
> **Rationale**: Services already written in FastAPI/Axum/Express need a migration path

### Why Framework Parity?

Archimedes needs these features to replace existing services:

| Category                | FastAPI/Axum Has | Archimedes Status | Migration Blocker? |
| ----------------------- | ---------------- | ----------------- | ------------------ |
| CORS middleware         | ✅               | ❌ Missing        | **YES - P0**       |
| Test client             | ✅               | ❌ Missing        | **YES - P0**       |
| Startup/shutdown hooks  | ✅               | ❌ Missing        | **YES - P0**       |
| File uploads            | ✅               | ❌ Missing        | **YES - P1**       |
| Rate limiting           | ✅               | ❌ Missing        | **YES - P1**       |
| Cookie extraction       | ✅               | ❌ Missing        | P1                 |
| File download response  | ✅               | ❌ Missing        | P1                 |
| Static file serving     | ✅               | ❌ Missing        | P1                 |
| Sub-router nesting      | ✅               | ❌ Missing        | P2                 |
| Route prefixes          | ✅               | ❌ Missing        | P2                 |
| Compression middleware  | ✅               | ❌ Missing        | P2                 |
| Streaming responses     | ✅               | ⚠️ SSE only       | P2                 |
| Response header helpers | ✅               | ❌ Missing        | P2                 |

### Phase A14.1: Critical Missing Features (Weeks 71-73) 📋 P0

> **Goal**: Remove migration blockers for any browser-facing API

#### CORS Middleware

- [ ] Create `CorsMiddleware` with configurable origins, methods, headers
- [ ] Support `Access-Control-Allow-Origin`, `Access-Control-Allow-Methods`
- [ ] Support `Access-Control-Allow-Headers`, `Access-Control-Max-Age`
- [ ] Support credentials mode and preflight requests
- [ ] Add to middleware pipeline (before request ID)

```rust
// Target API
let cors = CorsConfig::builder()
    .allow_origins(["https://app.example.com"])
    .allow_methods([Method::GET, Method::POST])
    .allow_headers(["Content-Type", "Authorization"])
    .max_age(Duration::from_secs(3600))
    .build();
```

#### Test Client

- [ ] Create `TestClient` for in-memory HTTP testing
- [ ] Support all HTTP methods with builder pattern
- [ ] JSON body helpers with automatic serialization
- [ ] Response assertions (status, headers, body)
- [ ] No real network/port binding required

```rust
// Target API
let client = TestClient::new(app);

let response = client
    .get("/users/123")
    .header("Authorization", "Bearer token")
    .send()
    .await;

assert_eq!(response.status(), 200);
assert_eq!(response.json::<User>().await.id, "123");
```

#### Lifecycle Hooks

- [ ] Add `on_startup` callback registration
- [ ] Add `on_shutdown` callback registration
- [ ] Support async callbacks
- [ ] Support lifespan context manager pattern
- [ ] Ensure hooks run in order (startup) and reverse order (shutdown)

```rust
// Target API
app.on_startup(|container| async move {
    let db = Database::connect(&config.db_url).await?;
    container.register(db);
    Ok(())
});

app.on_shutdown(|container| async move {
    let db = container.get::<Database>()?;
    db.close().await;
    Ok(())
});
```

### Phase A14.2: File Handling (Weeks 74-75) 📋 P1

> **Goal**: Support file uploads and downloads

#### Multipart File Uploads

- [ ] Create `Multipart` extractor for form-data
- [ ] Create `File` type with filename, content_type, data
- [ ] Support streaming file uploads (don't buffer entire file)
- [ ] Support multiple files in single request
- [ ] Size limits and validation

```rust
// Target API
#[handler(operation = "uploadDocument")]
async fn upload(mut multipart: Multipart) -> Result<Response, ThemisError> {
    while let Some(field) = multipart.next().await? {
        let filename = field.filename().unwrap_or("unnamed");
        let data = field.bytes().await?;
        storage.save(filename, data).await?;
    }
    Ok(Response::no_content())
}
```

#### File Download Response

- [ ] Create `FileResponse` builder
- [ ] Support `Content-Disposition: attachment`
- [ ] Support `Content-Type` detection from extension
- [ ] Support streaming large files
- [ ] Support range requests (partial content)

```rust
// Target API
Response::file("/path/to/document.pdf")
    .filename("report.pdf")
    .content_type("application/pdf")
    .build()
```

#### Cookie Extractor

- [ ] Create `Cookie` extractor for reading cookies
- [ ] Create `SetCookie` response helper
- [ ] Support SameSite, Secure, HttpOnly flags
- [ ] Support signed/encrypted cookies (optional)

```rust
// Target API
#[handler(operation = "getSession")]
async fn get_session(cookies: Cookies) -> Result<Response, ThemisError> {
    let session_id = cookies.get("session_id")?;
    // ...
}
```

### Phase A14.3: Security & Performance (Weeks 76-77) 📋 P1

> **Goal**: Production security requirements

#### Rate Limiting Middleware

- [ ] Create `RateLimitMiddleware` with configurable limits
- [ ] Support per-IP, per-user, per-API-key limits
- [ ] Support sliding window algorithm
- [ ] Return `429 Too Many Requests` with `Retry-After` header
- [ ] Wire `RateLimitError` that already exists

```rust
// Target API
let rate_limit = RateLimitConfig::builder()
    .requests_per_second(100)
    .burst_size(200)
    .key_extractor(|ctx| ctx.identity.user_id().unwrap_or(ctx.client_ip))
    .build();
```

#### Compression Middleware

- [ ] Create `CompressionMiddleware` with gzip/brotli support
- [ ] Respect `Accept-Encoding` header
- [ ] Configurable compression level
- [ ] Skip compression for small responses
- [ ] Skip compression for already-compressed content types

```rust
// Target API
let compression = CompressionConfig::builder()
    .algorithms([Algorithm::Gzip, Algorithm::Brotli])
    .min_size(1024)  // Don't compress < 1KB
    .level(CompressionLevel::Default)
    .build();
```

#### Static File Serving

- [ ] Create `StaticFiles` handler for directory serving
- [ ] Support `index.html` fallback
- [ ] Support cache headers (ETag, Last-Modified)
- [ ] Support range requests for large files
- [ ] Security: prevent directory traversal

```rust
// Target API
app.mount("/static", StaticFiles::new("./public")
    .index("index.html")
    .cache_control("max-age=3600"));
```

### Phase A14.4: Router Enhancements (Week 78) 📋 P2

> **Goal**: Better code organization for large applications

#### Sub-Router Nesting

- [ ] Add `nest()` method for router composition
- [ ] Support path prefix for nested routers
- [ ] Merge middleware and handlers correctly

```rust
// Target API
let users_router = Router::new()
    .operation("listUsers", list_users)
    .operation("getUser", get_user);

let api_router = Router::new()
    .nest("/users", users_router)
    .nest("/orders", orders_router);

app.nest("/api/v1", api_router);
```

#### Route Prefixes & Tags

- [ ] Add `prefix()` method for path prefixes
- [ ] Add `tag()` method for OpenAPI grouping
- [ ] Support prefix on entire router

```rust
// Target API
let router = Router::new()
    .prefix("/api/v1")
    .tag("users")
    .operation("listUsers", list_users);
```

### A14 Deliverables

| Feature                | Crate                 | Priority | Status     |
| ---------------------- | --------------------- | -------- | ---------- |
| CORS middleware        | archimedes-middleware | P0       | 📋 Planned |
| Test client            | archimedes-test       | P0       | 📋 Planned |
| Lifecycle hooks        | archimedes-server     | P0       | 📋 Planned |
| Multipart/file uploads | archimedes-extract    | P1       | 📋 Planned |
| File download response | archimedes-extract    | P1       | 📋 Planned |
| Cookie extractor       | archimedes-extract    | P1       | 📋 Planned |
| Rate limiting          | archimedes-middleware | P1       | 📋 Planned |
| Compression middleware | archimedes-middleware | P2       | 📋 Planned |
| Static file serving    | archimedes-server     | P1       | 📋 Planned |
| Sub-router nesting     | archimedes-router     | P2       | 📋 Planned |
| Route prefixes/tags    | archimedes-router     | P2       | 📋 Planned |
| Streaming responses    | archimedes-extract    | P2       | 📋 Planned |

---

## Framework Feature Comparison

### Archimedes vs FastAPI vs Axum

| Category                 | FastAPI     | Axum      | Archimedes  | Notes                   |
| ------------------------ | ----------- | --------- | ----------- | ----------------------- |
| **Routing**              | ✅          | ✅        | ✅          | Radix tree router       |
| **Path parameters**      | ✅          | ✅        | ✅          | Contract-style `{id}`   |
| **Sub-routers**          | ✅          | ✅        | ❌          | Phase A14.4             |
| **JSON body**            | ✅          | ✅        | ✅          | Contract-validated      |
| **Form data**            | ✅          | ✅        | ✅          | `Form<T>` extractor     |
| **File uploads**         | ✅          | ✅        | ❌          | Phase A14.2             |
| **Cookies**              | ✅          | ⚠️        | ❌          | Phase A14.2             |
| **Request validation**   | ✅ Pydantic | Manual    | ✅ Contract | Auto from Themis        |
| **Response validation**  | ✅          | Manual    | ✅ Contract | Auto from Themis        |
| **Background tasks**     | ✅          | Via tokio | ✅ Superior | Built-in scheduler      |
| **Scheduled jobs**       | External    | External  | ✅ Built-in | Cron expressions        |
| **Startup hooks**        | ✅          | ✅        | ❌          | Phase A14.1             |
| **Shutdown hooks**       | ✅          | ✅        | ⚠️          | Graceful shutdown only  |
| **Middleware**           | ✅          | ✅ Tower  | ✅ Fixed    | Contract-enforced order |
| **CORS**                 | ✅          | ✅        | ❌          | Phase A14.1             |
| **Rate limiting**        | External    | External  | ❌          | Phase A14.3             |
| **Compression**          | ✅          | ✅        | ❌          | Phase A14.3             |
| **Static files**         | ✅          | ✅        | ❌          | Phase A14.3             |
| **WebSocket**            | ✅          | ✅        | ✅          | Full support            |
| **SSE**                  | External    | External  | ✅          | Built-in                |
| **OpenAPI docs**         | ✅ Auto     | External  | ✅ Contract | From Themis             |
| **Swagger UI**           | ✅          | External  | ✅          | Built-in                |
| **Test client**          | ✅          | ✅        | ❌          | Phase A14.1             |
| **OPA authorization**    | External    | External  | ✅ Built-in | Unique feature          |
| **Contract enforcement** | ❌          | ❌        | ✅ Built-in | Unique feature          |
| **Multi-language**       | Python only | Rust only | ✅ 5 langs  | Unique feature          |

### Extended Comparison: Flask, Sanic, Boost.Beast

| Category                 | Flask       | Sanic      | Boost.Beast | Archimedes  | Notes                      |
| ------------------------ | ----------- | ---------- | ----------- | ----------- | -------------------------- |
| **Language**             | Python      | Python     | C++         | Rust + FFI  |                            |
| **Async support**        | ⚠️ Limited  | ✅ Native  | ✅ Boost.Asio | ✅ Tokio   | Flask needs async wrapper  |
| **Performance**          | Slow        | Fast       | Very Fast   | Very Fast   | Rust/C++ > Python          |
| **Routing**              | ✅          | ✅         | Manual      | ✅          | Boost needs manual routing |
| **Path parameters**      | ✅ `<id>`   | ✅ `<id>`  | Manual      | ✅ `{id}`   |                            |
| **Blueprints/routers**   | ✅ Blueprint| ✅ Blueprint| ❌         | ❌          | Phase A14.4                |
| **JSON body**            | ✅ Manual   | ✅ Auto    | Manual      | ✅ Contract |                            |
| **Form data**            | ✅          | ✅         | Manual      | ✅          |                            |
| **File uploads**         | ✅          | ✅         | Manual      | ❌          | Phase A14.2                |
| **Cookies**              | ✅          | ✅         | Manual      | ❌          | Phase A14.2                |
| **Sessions**             | ✅ Built-in | ✅ External| ❌          | ❌          | Not planned (stateless)    |
| **Request validation**   | ❌ External | ❌ External| ❌          | ✅ Contract | Archimedes unique          |
| **Response validation**  | ❌          | ❌         | ❌          | ✅ Contract | Archimedes unique          |
| **Background tasks**     | ❌ Celery   | ✅ add_task| ❌          | ✅ Superior | Built-in scheduler         |
| **Scheduled jobs**       | ❌ Celery   | ❌ External| ❌          | ✅ Built-in | Cron expressions           |
| **Startup hooks**        | ✅ before_first_request | ✅ @before_server_start | ❌ | ❌ | Phase A14.1    |
| **Shutdown hooks**       | ✅ atexit   | ✅ @after_server_stop | ❌ | ⚠️       | Graceful shutdown only     |
| **Middleware**           | ✅ WSGI     | ✅ Middleware | Manual   | ✅ Fixed    | Contract-enforced order    |
| **CORS**                 | ✅ Flask-CORS | ✅ Built-in | Manual   | ❌          | Phase A14.1                |
| **Rate limiting**        | ✅ Flask-Limiter | ❌ External | ❌    | ❌          | Phase A14.3                |
| **Compression**          | ❌ External | ✅ Built-in | Manual    | ❌          | Phase A14.3                |
| **Static files**         | ✅ Built-in | ✅ Built-in | Manual    | ❌          | Phase A14.3                |
| **Templates (Jinja2)**   | ✅ Built-in | ✅ Jinja2  | ❌          | ❌          | Not planned (API-only)     |
| **WebSocket**            | ❌ Flask-SocketIO | ✅ Built-in | ✅ | ✅          | Full support               |
| **SSE**                  | ❌ External | ❌ Manual  | Manual      | ✅          | Built-in                   |
| **OpenAPI docs**         | ❌ Flask-RESTx | ✅ External | ❌      | ✅ Contract | From Themis                |
| **Swagger UI**           | ❌ External | ❌ External | ❌         | ✅          | Built-in                   |
| **Test client**          | ✅ Built-in | ✅ Built-in | ❌        | ❌          | Phase A14.1                |
| **OPA authorization**    | ❌          | ❌         | ❌          | ✅ Built-in | Unique feature             |
| **Contract enforcement** | ❌          | ❌         | ❌          | ✅ Built-in | Unique feature             |
| **Hot reload**           | ✅ Debug mode | ✅ Auto-reload | ❌   | ⚠️ Planned  | Phase A9                   |

### Framework Summary by Use Case

| Use Case | Best Choice | Why |
| -------- | ----------- | --- |
| **Rapid prototyping (Python)** | Flask | Simple, lots of extensions, huge ecosystem |
| **High-performance Python** | Sanic or FastAPI | Async, fast, modern Python |
| **Maximum performance** | Boost.Beast or Archimedes | C++/Rust, zero-overhead |
| **Contract-first APIs** | **Archimedes** | Only framework with built-in contract validation |
| **Multi-language platform** | **Archimedes** | Same behavior across Python, Go, TS, C++ |
| **Microservices with OPA** | **Archimedes** | Built-in authorization, no boilerplate |
| **Legacy migration** | Flask/Sanic → Archimedes-py | Use sidecar for gradual migration |

### Flask-Specific Features Missing in Archimedes

| Flask Feature | Description | Archimedes Status | Priority |
| ------------- | ----------- | ----------------- | -------- |
| **Blueprints** | Modular route organization | ❌ → Sub-routers | P2 (A14.4) |
| **Application factory** | Create app instances dynamically | ⚠️ Builder pattern | Low |
| **Flask-Login** | Session-based authentication | ❌ Not planned | N/A (JWT/SPIFFE) |
| **Flask-SQLAlchemy** | ORM integration | ⚠️ DI container | Low |
| **Flask-Migrate** | Database migrations | ❌ Out of scope | N/A |
| **Flask-WTF** | Form validation with CSRF | ❌ Contract validation | N/A |
| **Flask-RESTful** | REST API helpers | ✅ Contract-based | Done |
| **Flask-CORS** | CORS handling | ❌ → Middleware | P0 (A14.1) |
| **Flask-Limiter** | Rate limiting | ❌ → Middleware | P1 (A14.3) |
| **Debug toolbar** | Development debugging | ❌ Not planned | Low |
| **Error handlers** | Custom error pages | ✅ Error normalization | Done |
| **Context locals** | Request/app context | ✅ RequestContext | Done |
| **Signals (blinker)** | Event system | ❌ Not planned | Low |

### Sanic-Specific Features Missing in Archimedes

| Sanic Feature | Description | Archimedes Status | Priority |
| ------------- | ----------- | ----------------- | -------- |
| **Blueprints** | Route grouping | ❌ → Sub-routers | P2 (A14.4) |
| **Blueprint groups** | Nested blueprints | ❌ | P2 |
| **Middleware (request/response)** | Pre/post processing | ✅ Fixed pipeline | Done |
| **Listeners** | Startup/shutdown events | ❌ → Lifecycle hooks | P0 (A14.1) |
| **Background tasks** | `app.add_task()` | ✅ Superior | Done |
| **Streaming** | Request/response streaming | ⚠️ SSE only | P2 |
| **WebSocket** | Native support | ✅ | Done |
| **Named routes** | URL building | ❌ Not needed | N/A (contracts) |
| **Versioning** | API versioning | ❌ → Route prefixes | P2 (A14.4) |
| **Auto-reload** | Development hot reload | ⚠️ Planned | Low (A9) |
| **SSL/TLS** | Built-in HTTPS | ✅ Via config | Done |
| **Unix sockets** | Socket-based serving | ❌ | Low |
| **Inspector** | Runtime inspection | ❌ | Low |

### Boost.Beast-Specific Features Missing in Archimedes

| Boost.Beast Feature | Description | Archimedes Status | Priority |
| ------------------- | ----------- | ----------------- | -------- |
| **HTTP/1.1 parser** | Low-level HTTP | ✅ Via hyper | Done |
| **HTTP/2 support** | HTTP/2 protocol | ✅ Via hyper | Done |
| **WebSocket** | RFC 6455 support | ✅ | Done |
| **SSL/TLS** | Boost.Asio SSL | ✅ Via rustls | Done |
| **Custom allocators** | Memory control | ❌ | Low |
| **Zero-copy parsing** | Performance | ⚠️ Via hyper | Partial |
| **Coroutines** | C++20 coroutines | ✅ async/await | Done |
| **io_uring support** | Linux async I/O | ⚠️ Via tokio | Partial |
| **Header-only** | No linking | ❌ | N/A |
| **CMake integration** | Build system | ✅ Cargo | Done |

### Rust Frameworks: Actix-web, Rocket, Warp

| Category                 | Actix-web    | Rocket       | Warp         | Axum        | Archimedes  |
| ------------------------ | ------------ | ------------ | ------------ | ----------- | ----------- |
| **Architecture**         | Actor model  | Macro-based  | Filter-based | Tower-based | Middleware  |
| **Performance**          | Very Fast    | Fast         | Fast         | Fast        | Fast        |
| **TechEmpower ranking**  | #1-3 Rust    | Lower        | Mid          | Top 10      | Not tested  |
| **Learning curve**       | Medium       | Easy         | Hard         | Medium      | Medium      |
| **Type safety**          | ✅           | ✅ Excellent | ✅           | ✅          | ✅ Contract |
| **Routing**              | ✅ Macros    | ✅ Macros    | ✅ Filters   | ✅ Router   | ✅ Radix    |
| **Path parameters**      | ✅ `{id}`    | ✅ `<id>`    | ✅ Filters   | ✅ `/:id`   | ✅ `{id}`   |
| **Nested routers**       | ✅ scope()   | ✅ mount()   | ✅           | ✅ nest()   | ❌ A14.4    |
| **JSON body**            | ✅           | ✅           | ✅           | ✅          | ✅ Contract |
| **Form data**            | ✅           | ✅           | ✅           | ✅          | ✅          |
| **File uploads**         | ✅ Multipart | ✅           | ✅           | ✅          | ❌ A14.2    |
| **Cookies**              | ✅           | ✅ Private   | ✅           | ⚠️          | ❌ A14.2    |
| **Request guards**       | ✅           | ✅ Excellent | ✅ Filters   | ✅ Extract  | ✅ Contract |
| **Validation**           | External     | External     | External     | External    | ✅ Contract |
| **Middleware**           | ✅           | ✅ Fairings  | ✅ Filters   | ✅ Tower    | ✅ Fixed    |
| **CORS**                 | ✅           | ❌ External  | ✅           | ✅          | ❌ A14.1    |
| **Rate limiting**        | External     | External     | External     | External    | ❌ A14.3    |
| **Compression**          | ✅           | ✅           | ✅           | ✅          | ❌ A14.3    |
| **Static files**         | ✅           | ✅           | ✅           | ✅          | ❌ A14.3    |
| **WebSocket**            | ✅           | ❌           | ✅           | ✅          | ✅          |
| **SSE**                  | ✅           | ❌           | ✅           | External    | ✅          |
| **Background tasks**     | ✅ Arbiter   | External     | Via tokio    | Via tokio   | ✅ Superior |
| **Scheduled jobs**       | External     | External     | External     | External    | ✅ Built-in |
| **Startup hooks**        | ✅           | ✅           | ✅           | ✅          | ❌ A14.1    |
| **Database integration** | ✅ sqlx      | ✅ diesel    | External     | External    | ⚠️ DI       |
| **Test client**          | ✅           | ✅           | ✅           | ✅          | ❌ A14.1    |
| **OpenAPI**              | ❌ External  | ❌ External  | ❌ External  | ❌ External | ✅ Contract |
| **Hot reload**           | External     | External     | External     | External    | ⚠️ A9       |
| **OPA authorization**    | ❌           | ❌           | ❌           | ❌          | ✅ Built-in |
| **Contract enforcement** | ❌           | ❌           | ❌           | ❌          | ✅ Built-in |

#### Actix-web Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Actor system** | Message-passing concurrency | ❌ Not needed (async) |
| **Web sockets actors** | WS via actor messages | ✅ Direct async |
| **Connection pooling** | Built-in DB pools | ⚠️ Via DI container |
| **Multipart streaming** | Stream file uploads | ❌ A14.2 |
| **HTTP/2 push** | Server push | ❌ |
| **Payload limits** | Per-resource limits | ✅ Config |
| **Resource guards** | Type-safe auth | ✅ Contract + OPA |

#### Rocket Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Type-safe routing** | Compile-time route checking | ✅ Contract validation |
| **Request guards** | FromRequest trait | ✅ Extractors |
| **Responders** | Custom response types | ✅ Response builders |
| **Fairings** | Lifecycle callbacks | ❌ A14.1 |
| **Managed state** | Type-safe app state | ✅ DI container |
| **Private cookies** | Encrypted cookies | ❌ A14.2 |
| **Forms with validation** | Form FromForm | ✅ Contract |
| **Templating** | Built-in templates | ❌ (API-only) |

#### Warp Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Filter combinators** | Composable filters | ✅ Middleware |
| **Rejection handling** | Type-safe errors | ✅ ThemisError |
| **Reply trait** | Custom responses | ✅ Response |
| **Path composition** | and/or/map filters | ✅ Router |
| **TLS built-in** | Native TLS | ✅ Via rustls |

### Go Frameworks: Gin, Chi, Echo, Fiber

| Category                 | Gin          | Chi          | Echo         | Fiber        | Archimedes  |
| ------------------------ | ------------ | ------------ | ------------ | ------------ | ----------- |
| **Philosophy**           | Fast + simple| Minimal      | High perf    | Express-like | Contract    |
| **Performance**          | Fast         | Fast         | Very Fast    | Very Fast    | Very Fast   |
| **Router**               | Radix tree   | Radix tree   | Radix tree   | Radix tree   | Radix tree  |
| **Stdlib compatible**    | ✅           | ✅ Excellent | ✅           | ❌ Fasthttp  | N/A         |
| **Path parameters**      | ✅ `:id`     | ✅ `{id}`    | ✅ `:id`     | ✅ `:id`     | ✅ `{id}`   |
| **Route groups**         | ✅           | ✅           | ✅           | ✅           | ❌ A14.4    |
| **JSON binding**         | ✅           | Manual       | ✅           | ✅           | ✅ Contract |
| **Form binding**         | ✅           | Manual       | ✅           | ✅           | ✅          |
| **File uploads**         | ✅           | Manual       | ✅           | ✅           | ❌ A14.2    |
| **Validation**           | ✅ go-validator | External  | ✅ validator | ✅ validator | ✅ Contract |
| **Middleware**           | ✅           | ✅ Excellent | ✅           | ✅           | ✅ Fixed    |
| **CORS**                 | ✅ cors      | ✅ cors      | ✅           | ✅           | ❌ A14.1    |
| **Rate limiting**        | External     | External     | External     | ✅ Limiter   | ❌ A14.3    |
| **Compression**          | ✅           | ✅           | ✅           | ✅           | ❌ A14.3    |
| **Static files**         | ✅           | ✅           | ✅           | ✅           | ❌ A14.3    |
| **WebSocket**            | ❌ External  | ❌ External  | ✅           | ✅           | ✅          |
| **SSE**                  | External     | External     | External     | External     | ✅          |
| **Graceful shutdown**    | ✅           | ✅           | ✅           | ✅           | ✅          |
| **Test utilities**       | ✅           | ✅ Stdlib    | ✅           | ✅           | ❌ A14.1    |
| **OpenAPI/Swagger**      | ✅ swag      | External     | ✅ swag      | ✅ swagger   | ✅ Contract |
| **OPA authorization**    | ❌           | ❌           | ❌           | ❌           | ✅ Built-in |
| **Contract enforcement** | ❌           | ❌           | ❌           | ❌           | ✅ Built-in |

#### Gin Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Binding tags** | Struct tag validation | ✅ Contract schemas |
| **Custom validators** | Extensible validation | ✅ JSON Schema |
| **Render interface** | Multiple response formats | ✅ Response builders |
| **Recovery middleware** | Panic recovery | ✅ Error normalization |
| **Logger middleware** | Request logging | ✅ Telemetry |
| **BasicAuth** | HTTP Basic Auth | ⚠️ Via identity |
| **SecureJSON** | XSSI protection | ⚠️ Not needed |

#### Chi Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **100% net/http** | Stdlib compatible | N/A |
| **Context-based** | Request scoped values | ✅ RequestContext |
| **Middleware stack** | Composable middleware | ✅ Fixed pipeline |
| **URL patterns** | Named + regexp | ✅ Contract paths |
| **Subresources** | Nested routing | ❌ A14.4 |
| **Mount points** | Subrouter mounting | ❌ A14.4 |

#### Echo Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Auto TLS** | Let's Encrypt | ❌ External |
| **HTTP/2 support** | Built-in | ✅ Via hyper |
| **Data binding** | Multiple sources | ✅ Extractors |
| **Rendering** | Templates + JSON | ✅ JSON only |
| **Streaming response** | io.Writer | ⚠️ SSE only |
| **JWT middleware** | Built-in | ⚠️ Via identity |
| **CSRF middleware** | Built-in | ❌ (API-only) |

#### Fiber Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Express-like API** | Familiar syntax | Similar |
| **Fasthttp based** | Not stdlib | N/A (hyper) |
| **Zero allocation** | High performance | ⚠️ Via hyper |
| **Prefork mode** | Multi-process | ❌ (K8s replicas) |
| **Built-in limiter** | Rate limiting | ❌ A14.3 |
| **Built-in cache** | Response caching | ❌ |
| **Built-in monitor** | Metrics dashboard | ⚠️ Prometheus |
| **Helmet** | Security headers | ❌ |

### TypeScript/Node.js Frameworks: Express, Fastify, NestJS, Koa, Hono

| Category                 | Express      | Fastify      | NestJS       | Koa          | Hono         | Archimedes  |
| ------------------------ | ------------ | ------------ | ------------ | ------------ | ------------ | ----------- |
| **Architecture**         | Minimalist   | Performance  | Enterprise   | Middleware   | Edge-first   | Contract    |
| **Performance**          | Slow         | Fast         | Medium       | Medium       | Very Fast    | Very Fast   |
| **TypeScript**           | ⚠️ Types     | ✅           | ✅ Native    | ⚠️ Types     | ✅ Native    | ✅ Types    |
| **Learning curve**       | Easy         | Medium       | Steep        | Easy         | Easy         | Medium      |
| **Routing**              | ✅           | ✅           | ✅ Decorators| ✅           | ✅           | ✅ Contract |
| **Path parameters**      | ✅ `:id`     | ✅ `:id`     | ✅ `:id`     | ✅ `:id`     | ✅ `:id`     | ✅ `{id}`   |
| **Nested routers**       | ✅           | ✅           | ✅ Modules   | ✅           | ✅           | ❌ A14.4    |
| **JSON body**            | ✅ body-parser| ✅ Built-in | ✅           | ✅ koa-body  | ✅           | ✅ Contract |
| **Validation**           | External     | ✅ JSON Schema| ✅ class-validator | External | ✅ Valibot  | ✅ Contract |
| **Middleware**           | ✅           | ✅ Hooks     | ✅ Interceptors | ✅ Excellent | ✅         | ✅ Fixed    |
| **CORS**                 | ✅ cors      | ✅           | ✅           | ✅           | ✅           | ❌ A14.1    |
| **Rate limiting**        | External     | External     | ✅           | External     | External     | ❌ A14.3    |
| **Static files**         | ✅ static    | ✅           | ✅           | ✅           | ❌           | ❌ A14.3    |
| **WebSocket**            | ❌ ws        | ✅           | ✅           | External     | ✅           | ✅          |
| **SSE**                  | Manual       | Manual       | ✅           | Manual       | ✅           | ✅          |
| **GraphQL**              | ✅ apollo    | ✅           | ✅           | ✅           | ✅           | ❌          |
| **Test utilities**       | ✅ supertest | ✅           | ✅           | ✅           | ✅           | ❌ A14.1    |
| **OpenAPI**              | External     | ✅ Native    | ✅ Native    | External     | ✅           | ✅ Contract |
| **DI container**         | ❌           | ❌           | ✅ Native    | ❌           | ❌           | ✅          |
| **OPA authorization**    | ❌           | ❌           | ❌           | ❌           | ❌           | ✅ Built-in |
| **Contract enforcement** | ❌           | ❌           | ❌           | ❌           | ❌           | ✅ Built-in |

#### NestJS Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Modules** | Modular architecture | ⚠️ Crate structure |
| **Controllers** | Decorator-based routing | ✅ Handler macros |
| **Providers** | Dependency injection | ✅ DI container |
| **Pipes** | Validation/transform | ✅ Contract validation |
| **Guards** | Route guards | ✅ OPA authorization |
| **Interceptors** | AOP-style hooks | ⚠️ Middleware |
| **Exception filters** | Error handling | ✅ Error normalization |
| **Microservices** | Multiple transports | ⚠️ HTTP only V1 |
| **CQRS** | Command/Query separation | ❌ Out of scope |
| **Event sourcing** | Event-driven | ❌ Out of scope |

#### Koa Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Context object** | ctx with req/res | ✅ RequestContext |
| **Cascading middleware** | await next() | ✅ Fixed pipeline |
| **No bundled middleware** | BYO middleware | ✅ Bundled essential |
| **Error handling** | try/catch flow | ✅ ThemisError |
| **Body parsing** | Via koa-body | ✅ Extractors |

#### Hono Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Multi-runtime** | Node, Deno, Bun, CF | ⚠️ Rust FFI |
| **Edge-first** | Cloudflare Workers | ❌ Server-based |
| **Tiny bundle** | 12KB | ⚠️ Rust binary |
| **JSX support** | Server-side JSX | ❌ (API-only) |
| **Validator** | Built-in Valibot | ✅ Contract |
| **RPC mode** | Type-safe client | ⚠️ Codegen |

### Python Frameworks: Starlette, Tornado, Falcon, aiohttp

| Category                 | Starlette    | Tornado      | Falcon       | aiohttp      | Archimedes  |
| ------------------------ | ------------ | ------------ | ------------ | ------------ | ----------- |
| **Type**                 | ASGI toolkit | Full async   | REST API     | HTTP client/server | Contract |
| **Performance**          | Fast         | Medium       | Very Fast    | Fast         | Very Fast   |
| **Async native**         | ✅           | ✅           | ⚠️ ASGI adapter | ✅        | ✅          |
| **Routing**              | ✅           | ✅           | ✅           | ✅           | ✅ Contract |
| **Path parameters**      | ✅ `{id}`    | ✅ regex     | ✅ `{id}`    | ✅ `{id}`    | ✅ `{id}`   |
| **Route mounting**       | ✅           | ✅           | ✅           | ✅           | ❌ A14.4    |
| **JSON body**            | ✅           | ✅           | ✅ via media | ✅           | ✅ Contract |
| **Form data**            | ✅           | ✅           | ✅           | ✅           | ✅          |
| **File uploads**         | ✅           | ✅           | ✅           | ✅           | ❌ A14.2    |
| **Middleware**           | ✅ ASGI      | ✅           | ✅           | ✅           | ✅ Fixed    |
| **CORS**                 | ✅           | Manual       | ✅           | ✅           | ❌ A14.1    |
| **WebSocket**            | ✅           | ✅           | ❌           | ✅           | ✅          |
| **SSE**                  | ✅           | ✅           | ❌           | ✅           | ✅          |
| **Background tasks**     | ✅           | ✅ IOLoop    | ❌           | ✅           | ✅ Superior |
| **Test client**          | ✅           | ✅           | ✅           | ✅           | ❌ A14.1    |
| **OpenAPI**              | External     | ❌           | External     | External     | ✅ Contract |
| **OPA authorization**    | ❌           | ❌           | ❌           | ❌           | ✅ Built-in |
| **Contract enforcement** | ❌           | ❌           | ❌           | ❌           | ✅ Built-in |

#### Starlette Specific Features (FastAPI base)

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **ASGI standard** | Framework-agnostic | ⚠️ Custom |
| **Request/Response** | Starlette classes | ✅ Custom types |
| **Lifespan events** | Startup/shutdown | ❌ A14.1 |
| **Sessions** | Cookie sessions | ❌ A14.2 |
| **Static files** | Serve directories | ❌ A14.3 |
| **Templates** | Jinja2 support | ❌ (API-only) |
| **GraphQL** | Built-in support | ❌ |
| **Test client** | httpx-based | ❌ A14.1 |

#### Tornado Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **IOLoop** | Event loop | ✅ Tokio runtime |
| **Long polling** | Real-time updates | ✅ SSE/WebSocket |
| **Coroutines** | Native async | ✅ async/await |
| **Secure cookies** | Signed cookies | ❌ A14.2 |
| **XSRF protection** | Built-in | ❌ (API-only) |
| **User authentication** | Built-in | ✅ OPA |
| **HTTP client** | Async client | ⚠️ Via reqwest |
| **Process utilities** | Multi-process | ❌ (K8s replicas) |

#### Falcon Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **REST-focused** | Resource-based | ✅ Contract-based |
| **Minimalist** | No magic | ✅ Explicit |
| **Request/Response** | Efficient classes | ✅ Custom types |
| **Media handlers** | Pluggable serialization | ✅ JSON/form |
| **URI templates** | RFC 6570 | ✅ Contract paths |
| **Hooks** | Before/after | ✅ Middleware |
| **Cython support** | Performance boost | ⚠️ Rust native |

#### aiohttp Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **HTTP client + server** | Both in one | ⚠️ Server only |
| **Persistent sessions** | Client sessions | ⚠️ Via middleware |
| **Web sockets** | Full duplex | ✅ |
| **Multipart** | Streaming multipart | ❌ A14.2 |
| **Signals** | Lifecycle hooks | ❌ A14.1 |
| **Middlewares** | Composable | ✅ Fixed |
| **Pluggable routers** | Custom routing | ✅ Radix tree |

### C++ Frameworks: Drogon, oat++, cpp-httplib, Crow

| Category                 | Drogon       | oat++        | cpp-httplib  | Crow         | Archimedes  |
| ------------------------ | ------------ | ------------ | ------------ | ------------ | ----------- |
| **Async model**          | Coroutines   | Async I/O    | Sync (threads) | Async      | Tokio async |
| **Performance**          | Very Fast    | Very Fast    | Fast         | Fast         | Very Fast   |
| **TechEmpower ranking**  | Top 5        | Top 20       | Not ranked   | Not ranked   | Not tested  |
| **Ease of use**          | Medium       | Easy         | Very Easy    | Easy         | Medium      |
| **C++ standard**         | C++17/20     | C++11        | C++11        | C++14        | N/A (Rust)  |
| **Header-only**          | ❌           | ✅           | ✅           | ✅           | ❌ Binary   |
| **Routing**              | ✅ Attribute | ✅ Endpoint  | ✅ Lambda    | ✅ Lambda    | ✅ Contract |
| **Path parameters**      | ✅           | ✅           | ✅           | ✅           | ✅ `{id}`   |
| **JSON body**            | ✅ jsoncpp   | ✅ DTO       | ✅ nlohmann  | ✅           | ✅ Contract |
| **Form data**            | ✅           | ✅           | ✅           | ✅           | ✅          |
| **File uploads**         | ✅           | ✅           | ✅           | ✅           | ❌ A14.2    |
| **Validation**           | ❌           | ✅ DTO       | ❌           | ❌           | ✅ Contract |
| **Middleware/Filters**   | ✅ Filters   | ✅ Interceptors | Manual   | ✅ Middleware | ✅ Fixed   |
| **CORS**                 | ✅           | ✅           | Manual       | ✅           | ❌ A14.1    |
| **WebSocket**            | ✅           | ✅           | ❌           | ✅           | ✅          |
| **Database ORM**         | ✅ Drogon ORM| ✅ ORM       | ❌           | ❌           | ❌          |
| **Test utilities**       | ✅           | ✅           | ❌           | ❌           | ❌ A14.1    |
| **OpenAPI**              | ❌           | ✅ Swagger   | ❌           | ❌           | ✅ Contract |
| **OPA authorization**    | ❌           | ❌           | ❌           | ❌           | ✅ Built-in |
| **Contract enforcement** | ❌           | ❌           | ❌           | ❌           | ✅ Built-in |

#### Drogon Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **AOP support** | Aspect-oriented | ⚠️ Middleware |
| **HttpController** | MVC pattern | ✅ Handler macros |
| **Views** | CSP templates | ❌ (API-only) |
| **Sessions** | Server-side sessions | ❌ |
| **Plugins** | Extension system | ⚠️ Crate features |
| **Drogon ORM** | Async database | ❌ Out of scope |
| **Redis client** | Built-in | ⚠️ Via DI |
| **Coroutines** | C++20 co_await | ✅ async/await |

#### oat++ Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Object mapping** | DTO macros | ✅ Serde |
| **API Client** | Code generation | ⚠️ Themis codegen |
| **Swagger UI** | Built-in | ✅ |
| **Zero-copy** | Buffer management | ⚠️ Via hyper |
| **Modules** | Pluggable components | ✅ Crates |
| **Cross-platform** | Windows, Linux, Mac | ✅ |

#### cpp-httplib Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Single header** | Easy integration | ❌ |
| **Sync model** | Thread-per-request | ✅ Async |
| **SSL support** | OpenSSL | ✅ rustls |
| **Multipart** | File uploads | ❌ A14.2 |
| **Minimal deps** | Just OpenSSL | ⚠️ Multiple crates |
| **Simple API** | Beginner friendly | ⚠️ More complex |

#### Crow Specific Features

| Feature | Description | Archimedes Status |
| ------- | ----------- | ----------------- |
| **Flask-like** | Familiar API | Similar |
| **Mustache** | Template engine | ❌ (API-only) |
| **JSON** | Built-in JSON | ✅ Serde |
| **Compression** | Built-in | ❌ A14.3 |
| **Blueprints** | Route organization | ❌ A14.4 |
| **Multi-threaded** | Thread pool | ✅ Tokio workers |

### Unique Archimedes Features (Not in ANY Framework Above)

| Feature                       | Description                                         | Benefit                   |
| ----------------------------- | --------------------------------------------------- | ------------------------- |
| **Contract-first validation** | Request/response validated against Themis contracts | No validation code needed |
| **OPA authorization**         | Built-in policy evaluation with Eunomia bundles     | No auth boilerplate       |
| **Fixed middleware order**    | Cannot be reordered or disabled                     | Security by design        |
| **Multi-language bindings**   | Python, TypeScript, C++, Go from one codebase       | Consistent behavior       |
| **Sidecar mode**              | Proxy for gradual migration                         | Easy adoption             |

### Framework Selection Guide

| If you need... | Choose | Reason |
| -------------- | ------ | ------ |
| **Maximum Python perf** | FastAPI or Sanic | Async Python, well-maintained |
| **Simple Python API** | Flask | Huge ecosystem, easy learning |
| **Maximum Rust perf** | Actix-web | TechEmpower benchmarks |
| **Type-safe Rust** | Rocket | Compile-time guarantees |
| **Tower ecosystem** | Axum | Tower middleware reuse |
| **Fast Go API** | Fiber or Echo | Performance + ease of use |
| **Stdlib Go** | Chi | net/http compatible |
| **Enterprise Node.js** | NestJS | Modules, DI, enterprise patterns |
| **Fast Node.js** | Fastify or Hono | Performance-focused |
| **Simple Node.js** | Express or Koa | Easy to learn, huge ecosystem |
| **Max C++ perf** | Drogon | TechEmpower top 5 |
| **Simple C++** | cpp-httplib | Header-only, beginner friendly |
| **Contract-first** | **Archimedes** | **Only option** |
| **Built-in OPA** | **Archimedes** | **Only option** |
| **Multi-language platform** | **Archimedes** | **Only option** |

---

## Milestones Summary

| Milestone             | Target  | Criteria                          | Dependencies            |
| --------------------- | ------- | --------------------------------- | ----------------------- |
| **MVP Release**       |         |                                   |                         |
| A0: Shared Types      | Week 1  | Using `themis-platform-types`     | Themis creates crate    |
| A1: Foundation        | Week 4  | Core types, mock contracts        | `themis-platform-types` |
| A2: Server            | Week 8  | HTTP server, routing, handlers    | None                    |
| A3: Middleware        | Week 12 | Full pipeline with mocks          | None                    |
| A4: Observability     | Week 16 | Metrics, traces, logs, config     | None                    |
| A5: Integrated        | Week 20 | Themis + Eunomia integration      | Themis, Eunomia         |
| **Framework Release** |         |                                   |                         |
| A6: Core Framework    | Week 24 | Router, extractors (Axum parity)  | MVP                     |
| A7: FastAPI Parity    | Week 28 | Handler macros, auto-docs         | A6                      |
| A8: Extended Features | Week 32 | WebSocket, SSE, background tasks  | A7                      |
| A9: Developer Exp     | Week 36 | CLI, hot reload, templates        | A8                      |
| **Multi-Language**    |         |                                   |                         |
| A10: Sidecar          | Week 39 | Sidecar for non-Rust services     | A9                      |
| A11: Type Generation  | Week 42 | Python, Go, TypeScript, C++       | Themis codegen          |
| A12: Integration      | Week 46 | Multi-language E2E tests          | A10, A11                |
| **Native Bindings**   |         |                                   |                         |
| A13.1: Core FFI       | Week 50 | Stable C ABI for Archimedes       | A12                     |
| A13.2: Python         | Week 58 | archimedes PyPI (full parity)     | A13.1                   |
| A13.3: TypeScript     | Week 62 | @archimedes/node npm package      | A13.1                   |
| A13.4: C++            | Week 65 | libarchimedes headers             | A13.1                   |
| A13.5: Go             | Week 69 | archimedes-go module              | A13.1                   |
| **Framework Parity**  |         |                                   |                         |
| A14.1: Critical       | Week 73 | CORS, TestClient, lifecycle hooks | A13                     |
| A14.2: File Handling  | Week 75 | Uploads, downloads, cookies       | A14.1                   |
| A14.3: Security       | Week 77 | Rate limit, compression, static   | A14.2                   |
| A14.4: Router         | Week 78 | Sub-routers, prefixes, tags       | A14.3                   |
| **V1.0 Release**      | Week 78 | All features, production ready    | A14.4                   |

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

### Sidecar (Transitional)

- `archimedes-sidecar` - HTTP proxy for non-Rust services (Phase A10)
  > **Note**: The sidecar remains useful for:
  >
  > - Gradual migration from existing frameworks
  > - Polyglot environments where native bindings aren't ready
  > - Edge cases (WASM, exotic platforms)

### Native Language Bindings (Phase A13) 🆕

- `archimedes-ffi` - C ABI layer for cross-language FFI
- `archimedes-python` - PyO3-based Python bindings → **`archimedes` (PyPI)**
- `archimedes-go` - cgo-based Go bindings → **`github.com/themis-platform/archimedes-go`**
- `archimedes-node` - napi-rs Node.js bindings → **`@archimedes/node` (npm)**
- `libarchimedes` - C++ headers with C ABI → **vcpkg/conan package**

### Tools

- `archimedes-cli` - Command-line scaffolding tool
- `archimedes-dev` - Development server with hot reload

### Code Generators (Themis-Owned)

- `themis-codegen-rust` - Rust client/server generation
- `themis-codegen-python` - Python type generation
- `themis-codegen-typescript` - TypeScript type generation
- `themis-codegen-go` - Go type generation
- `themis-codegen-cpp` - C++ type generation

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
- **Native Python bindings (replaces FastAPI/Flask)** 🆕
- **Native Go bindings (replaces Gin/Chi)** 🆕
- **Native TypeScript bindings (replaces Express/Fastify)** 🆕
- **Native C++ bindings (replaces cpp-httplib/Crow)** 🆕

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
