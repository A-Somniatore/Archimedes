# ADR-011: Native Language Bindings for Archimedes

**Status**: Accepted  
**Date**: 2026-01-10  
**Decision Makers**: Platform Architecture Team

## Context

Archimedes currently provides two patterns for multi-language support:

1. **Native Rust** - Direct use of Archimedes crates in Rust services
2. **Sidecar Proxy (ADR-009)** - HTTP proxy for non-Rust services (Python, Go, TypeScript, C++)

The sidecar pattern works but has limitations:

- Extra network hop adds 2-4ms latency per request
- Services still use language-specific frameworks (FastAPI, Express, Gin)
- Two deployment artifacts (sidecar + service)
- Framework-specific code means inconsistent behavior across languages
- Header parsing logic must be duplicated in each language

We need to standardize how services are built across all languages while maintaining the performance and safety guarantees of Archimedes.

## Decision

**Archimedes will provide native bindings for Python, Go, TypeScript, and C++ via FFI (Foreign Function Interface).**

This means:

1. **One Framework** - Archimedes becomes THE web framework for all internal services
2. **No More FastAPI/Express/Gin** - These frameworks will be replaced internally
3. **Single Codebase** - Rust core with language-specific bindings
4. **Consistent Behavior** - Same middleware, validation, auth, telemetry across all languages

### Binding Technologies

| Language   | Binding Technology | Package                           |
| ---------- | ------------------ | --------------------------------- |
| Python     | PyO3               | `archimedes` (PyPI)               |
| Go         | cgo                | `github.com/themis/archimedes-go` |
| TypeScript | napi-rs            | `@archimedes/node` (npm)          |
| C++        | C ABI              | `libarchimedes` (vcpkg/conan)     |

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    ARCHIMEDES CORE (Rust)                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ HTTP Server │ Router │ Middleware │ Validation │ AuthZ    │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│                   ┌──────────┴──────────┐                       │
│                   ▼                     ▼                       │
│            ┌─────────────┐       ┌─────────────┐               │
│            │ archimedes- │       │ C ABI Layer │               │
│            │    ffi      │       │  (stable)   │               │
│            └─────────────┘       └─────────────┘               │
└─────────────────────────────────────────────────────────────────┘
                   │                     │
      ┌────────────┼────────────┬────────┴────────┐
      ▼            ▼            ▼                 ▼
┌──────────┐ ┌──────────┐ ┌──────────────┐ ┌──────────┐
│  PyO3    │ │   cgo    │ │   napi-rs    │ │ C++ HDR  │
│ (Python) │ │  (Go)    │ │ (TypeScript) │ │  (C++)   │
└──────────┘ └──────────┘ └──────────────┘ └──────────┘
```

## Consequences

### Positive

1. **Performance**: In-process calls instead of HTTP proxy (~100ns vs ~4ms)
2. **Consistency**: Same middleware behavior across all languages
3. **Single Deployment**: One binary/artifact per service
4. **Type Safety**: Language-native types generated from contracts
5. **Observability**: Unified telemetry without header parsing
6. **Maintenance**: One codebase for all languages (Rust core)

### Negative

1. **Development Effort**: Significant work to create bindings for 4 languages
2. **Complexity**: FFI introduces memory safety concerns at boundaries
3. **Build Complexity**: Each language has different build systems
4. **Migration**: Existing services must be migrated from FastAPI/Express/etc.
5. **Learning Curve**: Teams must learn new Archimedes API

### Neutral

1. **Sidecar Still Useful**: Remains available for gradual migration, edge cases
2. **Timeline Extension**: Adds ~18 weeks to roadmap (Weeks 47-64)

## Implementation Plan

### Phase A13.1: Core FFI Layer (4 weeks)

- Create `archimedes-ffi` crate with stable C ABI
- Define memory-safe types (`#[repr(C)]`)
- Implement callback-based handler registration
- Create `archimedes.h` header via cbindgen
- Test FFI overhead (<100ns target)

### Phase A13.2: Python Bindings (4 weeks)

- Create `archimedes-python` crate using PyO3
- Python-native async support via `pyo3-asyncio`
- Type stubs (`.pyi`) for IDE support
- pytest plugin for testing
- Publish to PyPI as `archimedes`

### Phase A13.3: Go Bindings (3 weeks)

- Create `archimedes-go` module using cgo
- Go-idiomatic API with context
- Static linking option
- Publish as Go module

### Phase A13.4: TypeScript Bindings (3 weeks)

- Create `archimedes-node` crate using napi-rs
- TypeScript-first with full types
- Native Promise support
- Publish to npm as `@archimedes/node`

### Phase A13.5: C++ Bindings (2 weeks)

- Create C++ wrapper headers over C ABI
- Modern C++17 API
- CMake integration
- Publish via vcpkg

## Frameworks Being Replaced

| Language   | Being Replaced              | Reason for Replacement                     |
| ---------- | --------------------------- | ------------------------------------------ |
| Python     | FastAPI, Flask, Django REST | Inconsistent validation, no contract-first |
| Go         | Gin, Chi, Echo              | No built-in auth, manual validation        |
| TypeScript | Express, Fastify, NestJS    | No contract enforcement, varied patterns   |
| C++        | cpp-httplib, Crow, Drogon   | No middleware standardization              |

## Migration Strategy

1. **Phase 1**: New services use native bindings (green field)
2. **Phase 2**: Critical services migrated (high-traffic)
3. **Phase 3**: All internal services migrated (brownfield)
4. **Sidecar Deprecation**: After 100% migration (6+ months post-release)

## Performance Targets

| Metric                    | Sidecar (Current) | Native (Target) |
| ------------------------- | ----------------- | --------------- |
| Request overhead          | ~4ms              | <0.1ms          |
| Memory per connection     | ~20KB (two procs) | <10KB           |
| Requests/sec (Python)     | 10K (FastAPI)     | 20K+ (2x)       |
| Requests/sec (Go)         | 50K (Gin)         | 75K+ (1.5x)     |
| Requests/sec (TypeScript) | 30K (Fastify)     | 45K+ (1.5x)     |

## Alternatives Considered

### 1. Keep Sidecar Only

**Rejected** - Latency overhead and framework inconsistency are unacceptable for platform standardization.

### 2. Generate Code Per Language

**Rejected** - Would require maintaining 5 codebases (Rust + 4 generated). FFI provides single source of truth.

### 3. gRPC for Internal Services

**Rejected** - gRPC is post-MVP (ADR-006). HTTP with FFI is simpler and works today.

### 4. WebAssembly (WASM)

**Considered for Future** - WASM could provide another option for sandboxed execution, but current WASI limitations make FFI more practical.

## Related Decisions

- [ADR-009](009-archimedes-sidecar-multi-language.md): Sidecar pattern (transitional)
- [ADR-006](006-grpc-post-mvp.md): gRPC deferred to post-MVP
- [ADR-008](008-archimedes-full-framework.md): Archimedes as internal framework

## References

- [PyO3 User Guide](https://pyo3.rs/)
- [cgo Documentation](https://pkg.go.dev/cmd/cgo)
- [napi-rs Documentation](https://napi.rs/)
- [cbindgen](https://github.com/eyre-rs/cbindgen)
- [Rust FFI Guide](https://doc.rust-lang.org/nomicon/ffi.html)
