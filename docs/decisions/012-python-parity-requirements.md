# ADR-012: Python Bindings Full Rust Parity Requirements

**Status**: Accepted  
**Date**: 2026-01-08  
**Decision Makers**: CTO, Platform Team  
**Related**: ADR-011 (Native Language Bindings)

## Context

Archimedes is transitioning from a sidecar-only model to native language bindings (ADR-011). Python bindings (`archimedes-py`) are the first non-Rust implementation.

The question is: What level of feature parity should Python bindings have with Rust native?

### Options Considered

1. **Minimal Parity**: Basic HTTP server, handlers, and responses only
2. **Middleware Parity**: Include all 8 middleware stages
3. **Full Parity**: Exact same behavior as Rust native, including all middleware, validation, authorization, and telemetry

## Decision

**Option 3: Full Parity** - Python bindings MUST have exact feature parity with Rust native Archimedes.

### Rationale

1. **Developer Experience**: Python developers should get the same security guarantees (validation, authorization) without thinking about it
2. **Consistency**: A handler registered in Python behaves identically to one registered in Rust
3. **Migration Path**: Services can be migrated from Rust to Python (or vice versa) without behavior changes
4. **No Surprises**: Contract validation and OPA authorization work the same everywhere
5. **Single Source of Truth**: The Rust implementation is authoritative; Python wraps it

### Parity Requirements

Every Python binding MUST support these features before the language is considered "complete":

| Category        | Feature                   | Required |
| --------------- | ------------------------- | -------- |
| **Server**      | HTTP/1.1 server           | ✅       |
| **Server**      | Graceful shutdown         | ✅       |
| **Server**      | Health/ready endpoints    | ✅       |
| **Handlers**    | Handler registration      | ✅       |
| **Handlers**    | Request context           | ✅       |
| **Handlers**    | Response builder          | ✅       |
| **Middleware**  | Request ID generation     | ✅       |
| **Middleware**  | Trace context propagation | ✅       |
| **Middleware**  | Identity extraction       | ✅       |
| **Middleware**  | Authorization (OPA)       | ✅       |
| **Middleware**  | Request validation        | ✅       |
| **Middleware**  | Response validation       | ✅       |
| **Middleware**  | Error normalization       | ✅       |
| **Middleware**  | Telemetry collection      | ✅       |
| **Routing**     | Contract-based routing    | ✅       |
| **Routing**     | Path parameter extraction | ✅       |
| **Routing**     | Query parameter parsing   | ✅       |
| **Config**      | YAML/JSON/TOML loading    | ✅       |
| **Config**      | Environment overrides     | ✅       |
| **Testing**     | Test utilities            | ✅       |
| **Performance** | ≥2x faster than FastAPI   | ✅       |

### Implementation Strategy

1. **Wrap Rust Code**: Use PyO3 to call into existing Rust implementations where possible
2. **Reuse Middleware**: Call into `archimedes-middleware` via FFI for middleware stages
3. **Reuse Validation**: Call into `archimedes-sentinel` for contract validation
4. **Reuse Authorization**: Call into `archimedes-authz` for OPA evaluation
5. **Python-Specific**: Only implement Python-specific wrapper code (decorators, type stubs)

### Test Requirements

Python bindings must have tests covering:

1. **Unit Tests**: Every public API surface
2. **Integration Tests**: Full request flow with all middleware
3. **Parity Tests**: Same inputs produce same outputs as Rust
4. **Performance Tests**: Benchmark against FastAPI to prove ≥2x improvement

## Consequences

### Positive

- Python developers get full Archimedes capabilities
- Contract validation and authorization work without opt-in
- Migration between languages is straightforward
- Consistent observability across all services

### Negative

- Longer development time for Python bindings
- More complex build (requires Rust toolchain for maturin)
- Larger binary size (includes Rust runtime)

### Neutral

- Python bindings will be ~2-10x faster than pure Python frameworks
- Developers cannot "opt out" of middleware (by design)

## Implementation Checklist

### Phase 1: Core (DONE ✅)

- [x] PyO3 bindings crate (`archimedes-py`)
- [x] `PyApp`, `PyConfig`, `PyRequestContext`, `PyIdentity`, `PyResponse`
- [x] Handler registration with `@app.handler` decorator
- [x] HTTP server with hyper
- [x] Health and ready endpoints
- [x] 69 unit tests

### Phase 2: Middleware (IN PROGRESS 🔄)

- [ ] Request ID middleware
- [ ] Trace context middleware
- [ ] Identity extraction middleware
- [ ] Error normalization middleware
- [ ] Telemetry collection middleware

### Phase 3: Authorization & Validation (PLANNED 📋)

- [ ] Wire `archimedes-sentinel` for request/response validation
- [ ] Wire `archimedes-authz` for OPA authorization
- [ ] Contract-based routing via `archimedes-router`

### Phase 4: Ecosystem (PLANNED 📋)

- [ ] pytest plugin for handler testing
- [ ] Type stubs for IDE support
- [ ] Migration guide from FastAPI
- [ ] Performance benchmarks

## Related Documents

- [ADR-011: Native Language Bindings](011-native-language-bindings.md)
- [docs/roadmap.md - Phase A13.2](../roadmap.md#phase-a132-python-bindings---full-rust-parity-weeks-51-58--in-progress)
- [docs/design.md - Phase A13](../design.md#-phase-a13-native-language-bindings-in-progress)
