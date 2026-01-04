# Archimedes – Development Roadmap

> **Version**: 1.0.0  
> **Created**: 2026-01-04  
> **Target Completion**: Week 20 (MVP with integrations)

---

## Overview

Archimedes is the async HTTP/gRPC server framework for the Themis Platform. **Archimedes core can be developed in parallel** with Themis and Eunomia using mock contracts and policies.

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

| Phase                | Duration | Weeks | Description                       | Dependencies            |
| -------------------- | -------- | ----- | --------------------------------- | ----------------------- |
| A0: Shared Types     | 1 week   | 1     | Integrate `themis-platform-types` | Themis creates crate    |
| A1: Foundation       | 3 weeks  | 2-4   | Core types, server scaffold       | `themis-platform-types` |
| A2: Server & Routing | 4 weeks  | 5-8   | HTTP server, routing, handlers    | None (mock contracts)   |
| A3: Middleware       | 4 weeks  | 9-12  | Middleware pipeline, validation   | None (mock validation)  |
| A4: Observability    | 4 weeks  | 13-16 | Metrics, tracing, logging, config | None                    |
| A5: Integration      | 4 weeks  | 17-20 | Themis + Eunomia integration      | Themis, Eunomia         |

**Total**: 20 weeks (16 weeks parallel, 4 weeks integration)

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

- [ ] Implement handler registration API
- [ ] Add compile-time type checking
- [ ] Validate handler signatures
- [ ] Implement request deserialization
- [ ] Implement response serialization
- [ ] Test handler invocation

### Week 8: Handler Pipeline

- [ ] Wire handlers to router
- [ ] Add request body parsing
- [ ] Add response body serialization
- [ ] Implement timeout handling
- [ ] Add basic error responses
- [ ] Integration tests with mock handlers

### Phase A2 Milestone

**Criteria**: HTTP server runs, routes requests, invokes handlers

---

## Phase A3: Middleware Pipeline (Weeks 9-12) — PARALLEL

### Week 9: Middleware Architecture

- [ ] Create `archimedes-middleware` crate
- [ ] Design middleware trait
- [ ] Implement fixed-order pipeline
- [ ] Add middleware context passing
- [ ] Ensure middleware cannot be reordered
- [ ] Document middleware constraints

### Week 10: Core Middleware (Part 1)

- [ ] Implement Request ID middleware
- [ ] Implement Tracing middleware (span creation)
- [ ] Implement Identity extraction middleware
- [ ] Add SPIFFE identity parsing
- [ ] Add JWT identity parsing
- [ ] Test identity extraction

### Week 11: Core Middleware (Part 2)

- [ ] Implement mock Authorization middleware
- [ ] Implement mock Validation middleware
- [ ] Add extension points (`pre_handler`, `post_handler`)
- [ ] Test middleware ordering
- [ ] Test extension points

### Week 12: Response Pipeline

- [ ] Implement response validation middleware (mock)
- [ ] Implement telemetry emission middleware
- [ ] Implement error normalization middleware
- [ ] Wire complete request→response pipeline
- [ ] End-to-end pipeline tests

### Phase A3 Milestone

**Criteria**: Full middleware pipeline works with mock validation/auth

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

**Criteria**: Full integration with Themis and Eunomia, production-ready

---

## Milestones Summary

| Milestone         | Target  | Criteria                       | Dependencies            |
| ----------------- | ------- | ------------------------------ | ----------------------- |
| A0: Shared Types  | Week 1  | Using `themis-platform-types`  | Themis creates crate    |
| A1: Foundation    | Week 4  | Core types, mock contracts     | `themis-platform-types` |
| A2: Server        | Week 8  | HTTP server, routing, handlers | None                    |
| A3: Middleware    | Week 12 | Full pipeline with mocks       | None                    |
| A4: Observability | Week 16 | Metrics, traces, logs, config  | None                    |
| A5: Integrated    | Week 20 | Themis + Eunomia integration   | Themis, Eunomia         |

---

## Deliverables

### Crates

- `themis-platform-types` - **Shared types** (dependency, not owned)
- `archimedes` - Main facade crate (re-exports)
- `archimedes-core` - Core types and traits (depends on `themis-platform-types`)
- `archimedes-server` - HTTP/gRPC server
- `archimedes-middleware` - Middleware pipeline
- `archimedes-sentinel` - Themis contract validation
- `archimedes-authz` - OPA/Eunomia integration
- `archimedes-telemetry` - OpenTelemetry integration
- `archimedes-config` - Configuration management

### Features

- HTTP/1.1 and HTTP/2 support
- gRPC support (via Tonic)
- Fixed-order middleware pipeline
- Contract validation (enforced/monitor modes)
- OPA authorization
- OpenTelemetry observability
- Typed configuration
- Graceful shutdown

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
