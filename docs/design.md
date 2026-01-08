# Archimedes – Implementation Design Document

> **Version**: 3.0.0
> **Status**: Implementation Phase (Phase A13 Planned)
> **Last Updated**: 2026-01-10
> **Component**: archimedes

---

## Implementation Status

| Crate                   | Status      | Tests | Description                                                                                               |
| ----------------------- | ----------- | ----- | --------------------------------------------------------------------------------------------------------- |
| `archimedes`            | ✅ Complete | -     | Main facade crate (re-exports all crates)                                                                 |
| `archimedes-core`       | ✅ Complete | 80    | Core types: RequestContext, Handler, ThemisError, CallerIdentity, Contract, DI, InvocationContext, Binder |
| `archimedes-server`     | ✅ Complete | 90    | HTTP server, routing (radix tree), handler registry, graceful shutdown                                    |
| `archimedes-middleware` | ✅ Complete | 104   | All 8 middleware stages + pipeline                                                                        |
| `archimedes-telemetry`  | ✅ Complete | 25    | Prometheus metrics, OpenTelemetry tracing, structured logging                                             |
| `archimedes-config`     | ✅ Complete | 52    | Typed configuration with TOML/JSON, env overrides                                                         |
| `archimedes-router`     | ✅ Complete | 57    | High-performance radix tree router with method merging                                                    |
| `archimedes-extract`    | ✅ Complete | 109   | Request extractors, response builders, DI injection                                                       |
| `archimedes-macros`     | ✅ Complete | 14    | Handler macros for FastAPI-style definition (wiring complete)                                             |
| `archimedes-sentinel`   | ✅ Complete | 38    | Themis contract integration                                                                               |
| `archimedes-authz`      | ✅ Complete | 26    | Eunomia/OPA integration                                                                                   |
| `archimedes-docs`       | ✅ Complete | 29    | OpenAPI generation, Swagger UI, ReDoc                                                                     |
| `archimedes-ws`         | ✅ Complete | 52    | WebSocket support with connection management                                                              |
| `archimedes-sse`        | ✅ Complete | 38    | Server-Sent Events with backpressure handling                                                             |
| `archimedes-tasks`      | ✅ Complete | 41    | Background task spawner and job scheduler                                                                 |
| `archimedes-sidecar`    | ✅ Complete | 39    | Multi-language sidecar proxy (Phase A10)                                                                  |
| `archimedes-ffi`        | 📋 Planned  | -     | C ABI for cross-language FFI (Phase A13.1)                                                                |
| `archimedes-python`     | 📋 Planned  | -     | Python bindings via PyO3 (Phase A13.2)                                                                    |
| `archimedes-go`         | 📋 Planned  | -     | Go bindings via cgo (Phase A13.3)                                                                         |
| `archimedes-node`       | 📋 Planned  | -     | Node.js bindings via napi-rs (Phase A13.4)                                                                |
| `libarchimedes`         | 📋 Planned  | -     | C++ headers with C ABI (Phase A13.5)                                                                      |

**Total Tests**: 1019 passing across all crates

---

## 🚀 Phase A13: Native Language Bindings (PLANNED)

### Vision: One Framework, All Languages

Archimedes will provide **native bindings** for Python, Go, TypeScript, and C++. This means:

- **No more FastAPI, Flask, Express, Gin** for internal services
- **Archimedes IS the framework** - same behavior across all languages
- **Single codebase** - Rust core with FFI bindings

### Why Native Bindings Over Sidecar?

| Sidecar Pattern (A10)                 | Native Bindings (A13)              |
| ------------------------------------- | ---------------------------------- |
| Extra network hop (~2-4ms latency)    | In-process calls (~100ns overhead) |
| Separate process (memory, deployment) | Single binary deployment           |
| Header parsing in each language       | Direct struct access               |
| Still using FastAPI/Express/etc.      | Unified Archimedes API             |
| Two things to deploy                  | One artifact                       |

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           ARCHIMEDES CORE (Rust)                             │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │ HTTP Server │ Router │ Middleware │ Validation │ AuthZ │ Telemetry  │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                      │                                       │
│                    ┌─────────────────┼─────────────────┐                    │
│                    ▼                 ▼                 ▼                    │
│             ┌───────────┐     ┌───────────┐     ┌───────────┐              │
│             │ C ABI     │     │ C ABI     │     │ C ABI     │              │
│             │ (stable)  │     │ (stable)  │     │ (stable)  │              │
│             └───────────┘     └───────────┘     └───────────┘              │
└─────────────────────────────────────────────────────────────────────────────┘
                    │                 │                 │
       ┌────────────┘                 │                 └────────────┐
       ▼                              ▼                              ▼
┌─────────────────┐          ┌─────────────────┐          ┌─────────────────┐
│  archimedes-py  │          │  archimedes-go  │          │ @archimedes/node│
│    (PyO3)       │          │    (cgo)        │          │   (napi-rs)     │
└─────────────────┘          └─────────────────┘          └─────────────────┘
       │                              │                              │
       ▼                              ▼                              ▼
┌─────────────────┐          ┌─────────────────┐          ┌─────────────────┐
│  Python App     │          │    Go App       │          │  Node.js App    │
│                 │          │                 │          │                 │
│ @app.operation( │          │ app.Operation(  │          │ app.operation(  │
│   "listUsers")  │          │   "listUsers",  │          │   "listUsers",  │
│ async def       │          │   handler)      │          │   async (req)=> │
│   handler():    │          │                 │          │     {})         │
└─────────────────┘          └─────────────────┘          └─────────────────┘
```

### API Preview

**Python** (replaces FastAPI/Flask):

```python
from archimedes import Archimedes, Request, Response

app = Archimedes(contract="contract.json")

@app.operation("listUsers")
async def list_users(request: Request) -> Response:
    users = await db.get_users()
    return Response.json({"users": users})

app.run(port=8080)
```

**Go** (replaces Gin/Chi):

```go
app := archimedes.New(archimedes.Config{Contract: "contract.json"})

app.Operation("listUsers", func(ctx *archimedes.Context) error {
    users, _ := db.GetUsers()
    return ctx.JSON(200, map[string]any{"users": users})
})

app.Run(":8080")
```

**TypeScript** (replaces Express/Fastify):

```typescript
const app = new Archimedes({ contract: "contract.json" });

app.operation("listUsers", async (request: Request): Promise<Response> => {
  const users = await db.getUsers();
  return Response.json({ users });
});

app.listen(8080);
```

**C++** (replaces cpp-httplib/Crow):

```cpp
archimedes::App app{"contract.json"};

app.operation("listUsers", [](const archimedes::Request& req) {
    auto users = db.get_users();
    return archimedes::Response::json({{"users", users}});
});

app.run(8080);
```

### Frameworks Being Replaced

| Language   | Current (Being Replaced)    | Future (Archimedes)    |
| ---------- | --------------------------- | ---------------------- |
| Rust       | -                           | archimedes (native)    |
| Python     | FastAPI, Flask, Django REST | archimedes (PyPI)      |
| Go         | Gin, Chi, Echo, net/http    | archimedes-go (module) |
| TypeScript | Express, Fastify, NestJS    | @archimedes/node (npm) |
| C++        | cpp-httplib, Crow, Drogon   | libarchimedes          |

---

## Recent Updates (Phase A10 Complete)

### Sidecar Proxy (v2.14.0) - COMPLETE

> **Note**: The sidecar pattern remains useful for:
>
> - Gradual migration from existing frameworks
> - Polyglot environments during transition
> - Edge cases (WASM, exotic platforms)

- **archimedes-sidecar**: Multi-language support crate (39 tests)
  - `SidecarServer`: HTTP proxy using `hyper`
  - `ProxyClient`: HTTP client for upstream forwarding (reqwest)
  - `HealthChecker`: Liveness and readiness endpoints
  - `PropagatedHeaders`: W3C Trace Context propagation
  - `SidecarConfig`: TOML/JSON configuration with env overrides
  - `MiddlewarePipeline`: Sentinel and Authz integration
  - Header filtering (hop-by-hop, security-sensitive)
  - Internal endpoints (`/_archimedes/health`, `ready`, `metrics`)
  - Dockerfile for containerized deployment
  - Kubernetes manifests and Docker Compose examples
  - ADR-009: Sidecar pattern documentation

### Extended Features (v2.12.0) - COMPLETE

- **archimedes-ws**: WebSocket support crate (52 tests)

  - RFC 6455 compliant via `tokio-tungstenite`
  - `WebSocket` for bidirectional communication
  - `ConnectionManager` with global/per-client limits
  - Automatic ping/pong for connection health
  - Graceful shutdown with connection notification
  - JSON message serialization support

- **archimedes-sse**: Server-Sent Events crate (38 tests)

  - `SseStream` for server-to-client streaming
  - `SseEvent` with id, event type, data, and retry
  - `SseSender` for async event publishing
  - Backpressure handling with configurable buffer
  - Keep-alive with comment-based heartbeats

- **archimedes-tasks**: Background processing crate (41 tests)
  - `Spawner` for async task execution
  - `TaskHandle` for cancellation and result retrieval
  - `Scheduler` for cron-based job scheduling
  - Task timeout and concurrent limit support
  - `SharedSpawner` for DI integration

### Automatic Documentation (v2.10.0)

- **archimedes-docs**: New crate for API documentation generation
- `OpenApiGenerator`: Converts Themis artifacts to OpenAPI 3.1 specs
- `SwaggerUi`: Generates interactive Swagger UI pages (CDN-loaded)
- `ReDoc`: Generates beautiful ReDoc documentation pages
- Full OpenAPI type system (Info, Server, PathItem, Operation, Parameter, Schema)
- Schema conversion from Themis Schema to OpenAPI Schema
- Path parameter extraction from URL templates
- 29 tests covering all functionality

### Contract Binding (v2.9.0)

- **archimedes-core**: Added `HandlerBinder` for contract binding validation
- Validates handlers against contract operations at startup
- Ensures all required operations have handlers (no missing handlers)
- Prevents duplicate handler registration
- Prevents registration of unknown operations
- 6 unit tests covering all validation cases

### InvocationContext (v2.8.0)

- **archimedes-core**: Added `InvocationContext` to bridge handler invocation with extraction system
- Aggregates HTTP request details (method, URI, headers, body)
- Includes path parameters from router matching
- Carries middleware `RequestContext` (identity, request ID, trace info)
- Optional DI container via `Arc<Container>`
- `BoxedHandler` signature updated to use `InvocationContext`

### Macro Wiring (v2.8.0)

- **archimedes-extract**: Added `ExtractionContext::from_invocation()` bridge method
- **archimedes-macros**: Handler codegen now works end-to-end with extractors
- Integration tests verify full extraction pipeline (JSON, Path, Query, Headers, Inject)

### Handler Macros (v2.7.0)

- **archimedes-macros**: New proc-macro crate for FastAPI-style handler definitions
- `#[handler(operation = "...")]` attribute macro for handler functions
- Parsing utilities for handler attributes, parameters, and function signatures
- Code generation for handler registration functions

### Dependency Injection (v2.7.0)

- **archimedes-core**: Added DI container with TypeId-based service registry
- `Container`: Thread-safe service container with Arc-wrapped services
- `Inject<T>`: Type-safe wrapper for injected services
- `InjectionError`: Error type for missing service dependencies

### Inject Extractor (v2.7.0)

- **archimedes-extract**: Added `Inject<T>` extractor for DI in handlers
- `ExtractionContext` now supports optional DI container
- Seamless integration with existing extractor pattern

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Goals & Non-Goals](#2-goals--non-goals)
3. [Architecture Overview](#3-architecture-overview)
4. [Core Components](#4-core-components)
5. [Crate Structure](#5-crate-structure)
6. [API Design](#6-api-design)
7. [Middleware Pipeline](#7-middleware-pipeline)
8. [Contract Integration (Themis)](#8-contract-integration-themis)
9. [Authorization Integration (Eunomia)](#9-authorization-integration-eunomia)
10. [Configuration System](#10-configuration-system)
11. [Observability](#11-observability)
12. [Error Handling](#12-error-handling)
13. [Testing Strategy](#13-testing-strategy)

---

## 1. Executive Summary

Archimedes is a **Rust-based async HTTP/gRPC server framework** that provides:

- **Contract-first enforcement** via Themis integration
- **Mandatory middleware** that cannot be disabled or reordered
- **Built-in authorization** via Eunomia/OPA policy bundles
- **First-class observability** with OpenTelemetry
- **Typed request/response handling** generated from contracts

Unlike general-purpose frameworks (Axum, Actix, FastAPI), Archimedes is **opinionated by design**. Services built with Archimedes automatically comply with platform standards.

### Why Rust?

| Concern     | Rust Advantage                                             |
| ----------- | ---------------------------------------------------------- |
| Performance | Zero-cost abstractions, no GC pauses                       |
| Safety      | Memory safety without runtime overhead                     |
| Concurrency | Fearless concurrency via ownership model                   |
| Deployment  | Single binary, minimal container images                    |
| Type System | Strong typing enforces contract compliance at compile time |

---

## 2. Goals & Non-Goals

### Goals

- ✅ Provide a standardized runtime for all Themis-native services
- ✅ Enforce contract validation at request/response boundaries
- ✅ Embed authorization evaluation (OPA) in the request path
- ✅ Emit consistent telemetry (logs, metrics, traces) automatically
- ✅ Support HTTP/1.1, HTTP/2, and gRPC
- ✅ Enable code generation from Themis contracts
- ✅ Make non-compliance a compile-time or startup error

### Non-Goals (V1 MVP)

- ❌ Plugin-based middleware systems
- ❌ Runtime policy authoring or hot-reload of business logic
- ❌ HTTP/3 / QUIC support
- ❌ WebSocket support (planned for Phase A8 post-MVP)
- ❌ Acting as a general-purpose web framework

---

## 3. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              ARCHIMEDES                                      │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                         Transport Layer                                 │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────────┐ │ │
│  │  │  HTTP/1.1   │  │   HTTP/2    │  │     gRPC (tonic)                │ │ │
│  │  │   (hyper)   │  │   (hyper)   │  │                                 │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                    │                                         │
│                                    ▼                                         │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                      Middleware Pipeline (Fixed Order)                  │ │
│  │                                                                         │ │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐ │ │
│  │  │ Request  │→│ Tracing  │→│ Identity │→│  AuthZ   │→│   Contract   │ │ │
│  │  │   ID     │ │  Init    │ │ Extract  │ │  (OPA)   │ │  Validation  │ │ │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────────┘ │ │
│  │                                                             │          │ │
│  │                              ┌──────────────────────────────┘          │ │
│  │                              ▼                                         │ │
│  │                    ┌──────────────────┐                                │ │
│  │                    │  pre_handler()   │  (Extension Point)             │ │
│  │                    └────────┬─────────┘                                │ │
│  │                             │                                          │ │
│  │                             ▼                                          │ │
│  │                    ┌──────────────────┐                                │ │
│  │                    │     HANDLER      │  (User Business Logic)         │ │
│  │                    └────────┬─────────┘                                │ │
│  │                             │                                          │ │
│  │                             ▼                                          │ │
│  │                    ┌──────────────────┐                                │ │
│  │                    │  post_handler()  │  (Extension Point)             │ │
│  │                    └────────┬─────────┘                                │ │
│  │                             │                                          │ │
│  │  ┌──────────────────────────┘                                          │ │
│  │  ▼                                                                     │ │
│  │  ┌──────────────┐ ┌──────────────┐ ┌────────────────────────────────┐ │ │
│  │  │  Response    │→│  Telemetry   │→│     Error Normalization        │ │ │
│  │  │  Validation  │ │   Emit       │ │                                │ │ │
│  │  └──────────────┘ └──────────────┘ └────────────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                         Supporting Systems                              │ │
│  │                                                                         │ │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐ │ │
│  │  │ Themis Sentinel │  │  OPA Evaluator  │  │   Config Manager        │ │ │
│  │  │ (Contract Val.) │  │  (Eunomia)      │  │                         │ │ │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────────────┘ │ │
│  │                                                                         │ │
│  │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐ │ │
│  │  │ OpenTelemetry   │  │  Health/Ready   │  │   Graceful Shutdown     │ │ │
│  │  │ Exporters       │  │  Probes         │  │                         │ │ │
│  │  └─────────────────┘  └─────────────────┘  └─────────────────────────┘ │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Core Components

### 4.0 Shared Platform Types

Archimedes integrates with `themis-platform-types` to ensure type compatibility across the Themis Platform. This was a CTO-mandated requirement to avoid type definition duplication.

**Imported Types (from `themis-platform-types`)**:

| Type                  | Purpose                                 | Usage in Archimedes                     |
| --------------------- | --------------------------------------- | --------------------------------------- |
| `CallerIdentity`      | Enum representing authenticated caller  | Used in `RequestContext`, authorization |
| `RequestId`           | UUID v7 wrapper for request correlation | Generated in Request ID middleware      |
| `SpiffeIdentity`      | SPIFFE identity with service URI        | Service-to-service auth                 |
| `UserIdentity`        | Human user with roles/tenant            | User authentication                     |
| `ApiKeyIdentity`      | API key with scopes                     | Machine-to-machine auth                 |
| `PolicyInput`         | OPA evaluation input                    | Phase A5: Authorization                 |
| `PolicyDecision`      | OPA evaluation result                   | Phase A5: Authorization                 |
| `ThemisErrorEnvelope` | Standard error response                 | Phase A5: Themis integration            |

**Extension Traits**:

Archimedes provides extension traits for Archimedes-specific functionality:

```rust
/// Extension trait adding Archimedes-specific methods to CallerIdentity
pub trait CallerIdentityExt {
    /// Returns a log-safe identifier (no secrets)
    fn log_id(&self) -> String;

    /// Returns roles for authorization
    fn roles(&self) -> Vec<&str>;
}

impl CallerIdentityExt for CallerIdentity {
    fn log_id(&self) -> String {
        match self {
            CallerIdentity::Spiffe(s) => s.spiffe_id.clone(),
            CallerIdentity::User(u) => format!("user:{}", u.user_id),
            CallerIdentity::ApiKey(a) => format!("apikey:{}", a.key_id),
            CallerIdentity::Anonymous => "anonymous".to_string(),
        }
    }
    // ...
}
```

**Re-exports**:

For convenience, shared types are re-exported through `archimedes-core`:

```rust
// In archimedes-core/src/lib.rs
pub use themis_platform_types::{
    CallerIdentity, RequestId,
    SpiffeIdentity, UserIdentity, ApiKeyIdentity,
};
pub use crate::identity::CallerIdentityExt;
```

### 4.1 Transport Layer

**HTTP Server**: Built on `hyper` with `tokio` runtime.

```rust
// Internal - users don't interact with this directly
struct ArchimedesServer {
    http_listener: TcpListener,
    grpc_listener: Option<TcpListener>,
    router: Router,
    middleware_stack: MiddlewareStack,
    config: ArchimedesConfig,
}
```

**Supported Protocols**:

- HTTP/1.1 (default)
- HTTP/2 (negotiated via ALPN or h2c)
- gRPC (via `tonic`, HTTP/2 only)

### 4.2 Router

The router maps `operationId` → handler. It does NOT use path-based routing directly; paths are resolved via the contract.

```rust
struct Router {
    /// Maps operationId to handler function
    handlers: HashMap<OperationId, BoxedHandler>,

    /// Contract artifact (loaded at startup)
    contract: ThemisContract,

    /// Path → OperationId resolution (derived from contract)
    path_resolver: PathResolver,
}
```

### 4.3 Request Context

Every request carries an immutable context through the pipeline:

```rust
#[derive(Clone)]
pub struct RequestContext {
    /// Unique request identifier (UUID v7)
    pub request_id: RequestId,

    /// OpenTelemetry trace context
    pub trace_context: TraceContext,

    /// Caller identity (SPIFFE or public)
    pub identity: CallerIdentity,

    /// Resolved operation from contract
    pub operation: ResolvedOperation,

    /// Request arrival timestamp
    pub received_at: Instant,
}
```

### 4.4 Invocation Context

When invoking a handler, the server creates an `InvocationContext` that bridges the HTTP layer with the extraction system:

```rust
pub struct InvocationContext {
    /// HTTP method (GET, POST, etc.)
    method: Method,

    /// Request URI with path and query
    uri: Uri,

    /// HTTP headers
    headers: HeaderMap,

    /// Request body (buffered)
    body: Bytes,

    /// Path parameters extracted by router (e.g., {userId} → "123")
    path_params: Params,

    /// Middleware context (identity, request ID, trace info)
    request_context: RequestContext,

    /// Optional DI container for service injection
    container: Option<Arc<Container>>,
}
```

**Purpose**: `InvocationContext` aggregates all information needed to invoke a handler:

- HTTP request details for extractors (Path, Query, Json, Headers)
- Middleware context for request correlation and identity
- DI container for `Inject<T>` extractor

**Conversion to ExtractionContext**:

```rust
// In handler macro-generated code:
let extraction_ctx = ExtractionContext::from_invocation(&ctx);

// Extractors use ExtractionContext to access request data
let user_id: Path<UserId> = Path::from_request(&extraction_ctx)?;
let body: Json<CreateUserRequest> = Json::from_request(&extraction_ctx)?;
```

**Handler Type**:

```rust
/// Type-erased handler function signature
pub type BoxedHandler = Box<
    dyn Fn(InvocationContext) -> BoxFuture<'static, Result<Response<Body>, ThemisError>>
        + Send
        + Sync,
>;
```

### 4.5 Handler Trait

Handlers implement a standard trait with typed request/response:

```rust
#[async_trait]
pub trait Handler<Req, Res>: Send + Sync + 'static
where
    Req: DeserializeOwned + Validate,
    Res: Serialize,
{
    async fn handle(
        &self,
        ctx: &RequestContext,
        request: Req,
    ) -> Result<Res, ThemisError>;
}
```

---

## 5. Crate Structure

The Archimedes repository is organized as a Cargo workspace:

> **Note**: Implemented crates are marked with ✅, planned crates with 🔜

```
archimedes/
├── Cargo.toml                    # Workspace root
├── README.md
├── LICENSE
│
├── crates/
│   ├── archimedes/               # ✅ Main library crate (facade)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Re-exports public API
│   │       └── prelude.rs        # Common imports
│   │
│   ├── archimedes-core/          # ✅ Core types and traits
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── context.rs        # RequestContext, RequestId
│   │       ├── handler.rs        # Handler trait
│   │       ├── error.rs          # ThemisError, ErrorEnvelope
│   │       ├── identity.rs       # CallerIdentity
│   │       ├── contract.rs       # Mock Contract, Operation, MockSchema
│   │       └── fixtures.rs       # Test fixtures
│   │
│   ├── archimedes-server/        # ✅ HTTP server implementation
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs         # Main server struct
│   │       ├── router.rs         # Request routing
│   │       ├── handler.rs        # Handler registry
│   │       ├── config.rs         # Server configuration
│   │       ├── health.rs         # Health/readiness endpoints
│   │       └── shutdown.rs       # Graceful shutdown
│   │
│   ├── archimedes-middleware/    # ✅ Middleware pipeline
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── middleware.rs     # Middleware trait
│   │       ├── context.rs        # MiddlewareContext
│   │       ├── pipeline.rs       # Fixed middleware chain
│   │       └── stages/
│   │           ├── request_id.rs       # Stage 1: UUID v7 generation
│   │           ├── tracing.rs          # Stage 2: W3C Trace Context
│   │           ├── identity.rs         # Stage 3: SPIFFE/JWT/ApiKey
│   │           ├── authorization.rs    # Stage 4: RBAC authorization
│   │           ├── validation.rs       # Stage 5: Request validation
│   │           ├── response_validation.rs  # Stage 6
│   │           ├── telemetry.rs        # Stage 7: Metrics/logs
│   │           └── error_normalization.rs  # Stage 8
│   │
│   ├── archimedes-sentinel/      # 🔜 Themis contract validation (Phase A5)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── loader.rs         # Contract artifact loading
│   │       ├── validator.rs      # Request/response validation
│   │       ├── resolver.rs       # Path → OperationId
│   │       └── schema.rs         # JSON Schema validation
│   │
│   ├── archimedes-authz/         # Eunomia/OPA integration
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── evaluator.rs      # OPA evaluator
│   │       ├── bundle.rs         # Policy bundle management
│   │       ├── input.rs          # Policy evaluation input
│   │       └── control.rs        # Control plane endpoint
│   │
│   ├── archimedes-telemetry/     # Observability
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── metrics.rs        # Prometheus metrics
│   │       ├── tracing.rs        # OpenTelemetry tracing
│   │       └── logging.rs        # Structured logging
│   │
│   └── archimedes-config/        # Configuration management
│       └── src/
│           ├── lib.rs
│           ├── schema.rs         # Config schema
│           ├── loader.rs         # File + env loading
│           └── validate.rs       # Strict validation
│
├── examples/
│   ├── hello-world/              # Minimal example
│   ├── users-service/            # Full example with auth
│   └── grpc-service/             # gRPC example
│
└── tests/
    ├── integration/
    └── e2e/
```

---

## 6. API Design

### 6.1 Application Bootstrap

```rust
use archimedes::prelude::*;

#[tokio::main]
async fn main() -> Result<(), ArchimedesError> {
    // Load configuration (file + env)
    let config = ArchimedesConfig::load()?;

    // Create application
    let app = Archimedes::builder()
        .config(config)
        .contract_artifact("./contracts/v1/service.artifact.json")
        .register_handler("getUser", GetUserHandler)
        .register_handler("createUser", CreateUserHandler)
        .register_handler("deleteUser", DeleteUserHandler)
        .build()
        .await?;

    // Run server (blocks until shutdown signal)
    app.run().await
}
```

### 6.2 Handler Implementation

```rust
use archimedes::prelude::*;

// Types generated from contract (via archimedes-codegen)
use crate::generated::{GetUserRequest, GetUserResponse, UserNotFoundError};

pub struct GetUserHandler {
    user_repo: Arc<dyn UserRepository>,
}

#[async_trait]
impl Handler<GetUserRequest, GetUserResponse> for GetUserHandler {
    async fn handle(
        &self,
        ctx: &RequestContext,
        request: GetUserRequest,
    ) -> Result<GetUserResponse, ThemisError> {
        // Business logic only - validation, auth, logging handled by framework
        let user = self.user_repo
            .find_by_id(&request.user_id)
            .await?
            .ok_or_else(|| UserNotFoundError {
                user_id: request.user_id.clone(),
            })?;

        Ok(GetUserResponse {
            id: user.id,
            name: user.name,
            email: user.email,
        })
    }
}
```

### 6.3 Extension Points

```rust
// Optional pre/post handler hooks
let app = Archimedes::builder()
    .config(config)
    .contract_artifact("./contracts/v1/service.artifact.json")
    .pre_handler(|ctx, req| async move {
        // Custom logic after identity extraction, before authz
        // Cannot modify ctx or suppress middleware
        tracing::info!(custom_field = "value");
        Ok(())
    })
    .post_handler(|ctx, res| async move {
        // Custom logic after handler, before serialization
        Ok(())
    })
    .register_handler("getUser", GetUserHandler)
    .build()
    .await?;
```

---

## 7. Middleware Pipeline

### 7.1 Fixed Execution Order

The middleware pipeline is **immutable**. Services cannot:

- Disable any middleware
- Reorder middleware
- Insert middleware between core stages

```rust
// Internal implementation - not configurable by users
pub(crate) struct MiddlewarePipeline {
    stages: [MiddlewareStage; 8],
}

impl MiddlewarePipeline {
    pub fn new() -> Self {
        Self {
            stages: [
                MiddlewareStage::RequestId(RequestIdMiddleware::new()),
                MiddlewareStage::Tracing(TracingMiddleware::new()),
                MiddlewareStage::Identity(IdentityMiddleware::new()),
                MiddlewareStage::Authorization(AuthorizationMiddleware::new()),
                MiddlewareStage::RequestValidation(RequestValidationMiddleware::new()),
                // --- Handler invocation happens here (not a middleware stage) ---
                MiddlewareStage::ResponseValidation(ResponseValidationMiddleware::new()),
                MiddlewareStage::Telemetry(TelemetryMiddleware::new()),
                MiddlewareStage::ErrorNormalization(ErrorNormalizationMiddleware::new()),
            ],
        }
    }
}
```

### 7.2 Middleware Stages

| Stage | Middleware          | Purpose                                 |
| ----- | ------------------- | --------------------------------------- |
| 1     | Request ID          | Generate/propagate request ID (UUID v7) |
| 2     | Tracing             | Initialize OpenTelemetry span           |
| 3     | Identity            | Extract caller identity (SPIFFE/JWT)    |
| 4     | Authorization       | OPA policy evaluation                   |
| 5     | Request Validation  | Validate against contract schema        |
| 6     | Response Validation | Validate response (configurable)        |
| 7     | Telemetry           | Emit metrics and structured logs        |
| 8     | Error Normalization | Convert errors to standard envelope     |

---

## 8. Contract Integration (Themis)

### 8.1 Contract Artifact Loading

At startup, Archimedes loads the compiled contract artifact:

```rust
pub struct ThemisSentinel {
    contract: CompiledContract,
    schemas: HashMap<OperationId, OperationSchemas>,
    path_index: PathIndex,
}

impl ThemisSentinel {
    pub async fn load(path: &Path) -> Result<Self, ContractError> {
        let artifact = fs::read(path).await?;
        let contract: CompiledContract = serde_json::from_slice(&artifact)?;

        // Validate artifact integrity
        contract.verify_checksum()?;

        // Build schema index for fast lookup
        let schemas = contract.operations.iter()
            .map(|op| (op.id.clone(), op.schemas.clone()))
            .collect();

        // Build path index for routing
        let path_index = PathIndex::from_contract(&contract);

        Ok(Self { contract, schemas, path_index })
    }
}
```

### 8.2 Validation Modes

```rust
pub enum ValidationMode {
    /// Validation failures block requests (production default)
    Enforced,

    /// Validation failures logged but requests proceed (migration aid)
    MonitorOnly,

    /// No validation (testing only, requires explicit opt-in)
    Disabled,
}
```

---

## 9. Authorization Integration (Eunomia)

### 9.1 Embedded OPA Evaluator

Archimedes embeds an OPA evaluator for local policy evaluation:

```rust
pub struct OpaEvaluator {
    engine: opa::Engine,
    bundle: RwLock<PolicyBundle>,
    metrics: AuthzMetrics,
}

impl OpaEvaluator {
    pub async fn evaluate<I: Serialize>(
        &self,
        query: &str,
        input: &I,
    ) -> Result<PolicyDecision, OpaError> {
        let bundle = self.bundle.read().await;
        let input_json = serde_json::to_value(input)?;

        let start = Instant::now();
        let result = self.engine.eval(query, &input_json, &bundle)?;
        let duration = start.elapsed();

        // Record metrics
        self.metrics.evaluation_duration.record(duration);
        self.metrics.evaluation_count.increment();

        Ok(PolicyDecision::from_opa_result(result))
    }
}
```

### 9.2 Policy Input Schema

> **Note**: This schema is defined authoritatively in [integration-spec.md](../../../docs/integration/integration-spec.md).
> Both Archimedes and Eunomia MUST use identical schemas.

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInput {
    /// Caller identity
    pub caller: CallerIdentity,

    /// Target service name
    pub service: String,

    /// Target operation ID (from contract)
    pub operation_id: String,

    /// HTTP method (GET, POST, PUT, DELETE, PATCH)
    pub method: String,

    /// Request path
    pub path: String,

    /// Filtered request headers (authorization headers stripped)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,

    /// Request timestamp (ISO 8601)
    pub timestamp: String,

    /// Environment (production, staging, development)
    pub environment: String,

    /// Additional context (extensible)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub context: HashMap<String, serde_json::Value>,
}
```

---

## 10. Configuration System

### 10.1 Configuration Schema

```rust
#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)] // Reject unknown fields
pub struct ArchimedesConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Contract configuration
    pub contract: ContractConfig,

    /// Authorization configuration
    pub authorization: AuthorizationConfig,

    /// Telemetry configuration
    pub telemetry: TelemetryConfig,

    /// TLS configuration
    pub tls: Option<TlsConfig>,
}
```

### 10.2 Example Configuration

```toml
# archimedes.toml

[server]
http_addr = "0.0.0.0:8080"
grpc_addr = "0.0.0.0:8081"
control_addr = "0.0.0.0:9090"
request_timeout = "30s"
shutdown_timeout = "10s"

[contract]
artifact_path = "./contracts/v1/service.artifact.json"
validation_mode = "enforced"  # "enforced" | "monitor_only"
validate_responses = true

[authorization]
policy_bundle_path = "./policies/bundle.tar.gz"
default_deny = true

[telemetry]
service_name = "users-service"
service_version = "1.0.0"
environment = "production"
```

---

## 11. Observability

### 11.1 Metrics (Prometheus)

```rust
/// Standard metrics emitted for every request
pub struct ArchimedesMetrics {
    /// Total requests by operation and status
    requests_total: CounterVec<[&'static str; 2]>,

    /// Request duration histogram
    request_duration_seconds: HistogramVec<[&'static str; 1]>,

    /// In-flight requests gauge
    in_flight_requests: Gauge,

    /// Request/response sizes
    request_size_bytes: HistogramVec<[&'static str; 1]>,
    response_size_bytes: HistogramVec<[&'static str; 1]>,
}
```

### 11.2 Tracing (OpenTelemetry)

Every request creates a span with standard attributes:

| Attribute          | Source                  |
| ------------------ | ----------------------- |
| `request_id`       | Generated or propagated |
| `trace_id`         | W3C Trace Context       |
| `span_id`          | Generated               |
| `service`          | Configuration           |
| `operation_id`     | Contract resolution     |
| `http.method`      | Request                 |
| `http.status_code` | Response                |

### 11.3 Logging (Structured JSON)

```json
{
  "timestamp": "2024-01-15T10:30:00.000Z",
  "level": "info",
  "message": "request completed",
  "request_id": "01HQVF8...",
  "trace_id": "abc123...",
  "operation_id": "getUser",
  "status": 200,
  "duration_ms": 45
}
```

---

## 12. Error Handling

### 12.1 Standard Error Envelope

All errors conform to the Themis error schema:

```rust
#[derive(Serialize)]
pub struct ThemisErrorEnvelope {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
    pub request_id: String,
    pub timestamp: DateTime<Utc>,
}
```

### 12.2 Error Categories

| Category       | HTTP Status | Code Pattern   |
| -------------- | ----------- | -------------- |
| Validation     | 400         | `VALIDATION_*` |
| Authentication | 401         | `AUTH_*`       |
| Authorization  | 403         | `AUTHZ_*`      |
| Not Found      | 404         | `NOT_FOUND_*`  |
| Internal       | 500         | `INTERNAL_*`   |

---

## 13. Testing Strategy

### 13.1 Unit Tests

- Core type serialization/deserialization
- Middleware ordering and context propagation
- Error normalization

### 13.2 Integration Tests

- Full request pipeline with mock handlers
- Contract validation enforcement
- Authorization decision caching

### 13.3 End-to-End Tests

- Complete service with real contracts
- Policy updates and hot-reload
- Graceful shutdown behavior

---

## 14. Real-Time Features (Phase A8)

> **Status**: 🔜 Planned for Weeks 29-32

### 14.1 WebSocket Architecture

**Crate**: `archimedes-ws`

```
┌─────────────────────────────────────────────────────────────────┐
│                     WebSocket Flow                               │
│                                                                  │
│  HTTP Upgrade    Identity    Authorization    WS Handler         │
│  Request ──────► Middleware ──────► Check ──────► Loop          │
│                      │                │              │           │
│                      ▼                ▼              ▼           │
│                  Extract          Validate      Message          │
│                  Caller           Permission    Validation       │
│                      │                │              │           │
│                      └────────────────┴──────────────┘           │
│                                  │                               │
│                                  ▼                               │
│                          Connection Manager                      │
│                    (tracking, limits, shutdown)                  │
└─────────────────────────────────────────────────────────────────┘
```

**Key Types**:

```rust
/// WebSocket connection with contract validation
pub struct WebSocket {
    inner: tokio_tungstenite::WebSocketStream<...>,
    inbound_schema: Option<Schema>,
    outbound_schema: Option<Schema>,
    connection_id: ConnectionId,
}

/// WebSocket upgrade extractor
pub struct WebSocketUpgrade {
    config: WebSocketConfig,
    on_upgrade: oneshot::Sender<WebSocket>,
}

/// WebSocket message types
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close(Option<CloseFrame>),
}
```

**Middleware Integration**:

- Identity middleware runs on HTTP upgrade request
- Authorization middleware validates WS connection permission
- Message validation happens per-message (optional, configurable)
- Telemetry tracks connection lifecycle and message counts

### 14.2 Server-Sent Events Architecture

**Crate**: `archimedes-sse`

```rust
/// SSE stream for server-to-client events
pub struct SseStream {
    sender: mpsc::Sender<SseEvent>,
    config: SseConfig,
}

/// Individual SSE event
pub struct SseEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: String,
    pub retry: Option<Duration>,
}

/// SSE response type for handlers
pub struct Sse {
    stream: SseStream,
    keep_alive: Duration,
}
```

**Handler Example**:

```rust
async fn event_stream(
    auth: Auth,
    sse: Sse,
) -> impl IntoResponse {
    // Spawn background task to push events
    tokio::spawn(async move {
        loop {
            sse.send(SseEvent {
                event: Some("update".into()),
                data: json!({"status": "ok"}).to_string(),
                ..Default::default()
            }).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    sse.into_response()
}
```

### 14.3 Connection Management

```rust
/// Global connection manager
pub struct ConnectionManager {
    connections: DashMap<ConnectionId, ConnectionInfo>,
    limits: ConnectionLimits,
    shutdown: ShutdownSignal,
}

pub struct ConnectionLimits {
    pub max_connections: usize,
    pub max_per_client: usize,
    pub idle_timeout: Duration,
}

pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub client_id: Option<String>,
    pub connected_at: Instant,
    pub last_activity: Instant,
    pub connection_type: ConnectionType,  // WebSocket | SSE
}
```

---

## 15. Background Processing (Phase A8)

> **Status**: 🔜 Planned for Weeks 31-32

### 15.1 Task System Architecture

**Crate**: `archimedes-tasks`

```
┌─────────────────────────────────────────────────────────────────┐
│                     Task System                                  │
│                                                                  │
│  ┌────────────────┐  ┌────────────────┐  ┌──────────────────┐  │
│  │  TaskSpawner   │  │   Scheduler    │  │  TaskRegistry    │  │
│  │  (fire-forget) │  │  (cron-based)  │  │  (tracking)      │  │
│  └───────┬────────┘  └───────┬────────┘  └────────┬─────────┘  │
│          │                   │                    │             │
│          └───────────────────┼────────────────────┘             │
│                              ▼                                  │
│                    ┌──────────────────┐                         │
│                    │  Tokio Runtime   │                         │
│                    │  (spawn_local or │                         │
│                    │   spawn)         │                         │
│                    └──────────────────┘                         │
└─────────────────────────────────────────────────────────────────┘
```

**Key Types**:

```rust
/// Task spawner with DI access
pub struct TaskSpawner {
    container: Arc<Container>,
    registry: Arc<TaskRegistry>,
    shutdown: ShutdownSignal,
}

impl TaskSpawner {
    /// Spawn a fire-and-forget task
    pub fn spawn<F>(&self, task: F) -> TaskHandle
    where
        F: Future<Output = ()> + Send + 'static;

    /// Spawn a task with result
    pub fn spawn_with_result<F, T>(&self, task: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static;
}

/// Handle for tracking spawned tasks
pub struct TaskHandle {
    id: TaskId,
    abort_handle: AbortHandle,
}

/// Scheduled job configuration
pub struct ScheduledJob {
    pub name: &'static str,
    pub cron: CronSchedule,
    pub overlap_policy: OverlapPolicy,
    pub timeout: Option<Duration>,
}

pub enum OverlapPolicy {
    Skip,      // Skip if previous run still running
    Queue,     // Queue for execution after current completes
    Concurrent, // Allow concurrent executions
}
```

### 15.2 Scheduler Implementation

```rust
/// Cron-based job scheduler
pub struct Scheduler {
    jobs: Vec<RegisteredJob>,
    runtime: Handle,
    shutdown: ShutdownSignal,
}

impl Scheduler {
    pub fn register<F, Fut>(&mut self, job: ScheduledJob, handler: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), TaskError>> + Send;

    /// Start the scheduler loop
    pub async fn run(&self) {
        loop {
            let next_job = self.find_next_due_job();
            tokio::select! {
                _ = tokio::time::sleep_until(next_job.next_run) => {
                    self.execute_job(next_job).await;
                }
                _ = self.shutdown.recv() => {
                    break;
                }
            }
        }
    }
}
```

### 15.3 Task Telemetry

```rust
// Metrics emitted by task system
task_spawned_total{task_type="ad-hoc|scheduled"}
task_completed_total{task_type, status="success|error|cancelled"}
task_duration_seconds{task_type}
scheduled_job_runs_total{job_name, status}
scheduled_job_last_run_timestamp{job_name}
```
