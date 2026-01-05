# Archimedes – Implementation Design Document

> **Version**: 2.6.0  
> **Status**: Implementation Phase (Phase A4 Complete, A0 Migration Complete, A6 Complete + Integration)  
> **Last Updated**: 2025-01-06  
> **Component**: archimedes

---

## Implementation Status

| Crate                   | Status      | Tests | Description                                                                |
| ----------------------- | ----------- | ----- | -------------------------------------------------------------------------- |
| `archimedes`            | ✅ Complete | -     | Main facade crate (re-exports all crates)                                  |
| `archimedes-core`       | ✅ Complete | 55    | Core types: RequestContext, Handler, ThemisError, CallerIdentity, Contract |
| `archimedes-server`     | ✅ Complete | 90    | HTTP server, routing (radix tree), handler registry, graceful shutdown     |
| `archimedes-middleware` | ✅ Complete | 104   | All 8 middleware stages + pipeline                                         |
| `archimedes-telemetry`  | ✅ Complete | 25    | Prometheus metrics, OpenTelemetry tracing, structured logging              |
| `archimedes-config`     | ✅ Complete | 52    | Typed configuration with TOML/JSON, env overrides                          |
| `archimedes-router`     | ✅ Complete | 57    | High-performance radix tree router with method merging                     |
| `archimedes-extract`    | ✅ Complete | 104   | Request extractors and response builders                                   |
| `archimedes-sentinel`   | 🔜 Phase A5 | -     | Themis contract integration                                                |
| `archimedes-authz`      | 🔜 Phase A5 | -     | Eunomia/OPA integration                                                    |

**Total Tests**: 561 passing

---

## Recent Updates (Phase A6 Integration)

### Router Integration (v2.6.0)
- **archimedes-server** now uses **archimedes-router** internally
- Replaced O(n) linear Vec-based routing with O(k) radix tree matching
- Added `MethodRouter::merge()` for incremental route registration
- Full backward-compatible API for `Router`, `RouteMatch`

### Extract Integration (v2.6.0)
- Main `archimedes` crate now re-exports `archimedes-router` and `archimedes-extract`
- Prelude includes common extractors: `Json`, `Form`, `Query`, `Path`, `Headers`
- Prelude includes common responses: `JsonResponse`, `ErrorResponse`, `Redirect`

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

### 4.4 Handler Trait

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
