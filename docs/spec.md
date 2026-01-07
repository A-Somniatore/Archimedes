# Archimedes – Async Runtime Specification (V1)

## Purpose

Archimedes is the primary execution runtime for Themis-native services. It is an async-first HTTP/gRPC server that owns the transport layer, middleware, enforcement, observability, and integration with contracts (Themis) and authorization policies (Eunomia).

This document is developer-ready and intended to be used directly to implement Archimedes.

---

## 1. Responsibilities

Archimedes is responsible for:

- Owning the HTTP and gRPC server implementation
- Async request handling and concurrency
- Mandatory core middleware execution
- Contract validation enforcement (Themis Sentinel)
- Authorization enforcement (OPA/Rego via Eunomia bundles)
- Structured logging, metrics, and tracing
- Typed request/response dispatch based on contracts

Archimedes is explicitly **not** responsible for:

- Defining API schemas (Themis)
- Defining authorization policy logic (Eunomia)
- Traffic exposure or routing decisions (Kratos / ingress)

---

## 2. Language & Runtime

- Language: **Rust**
- Async runtime: **Tokio**
- Execution model: async/await

### 2.1 Protocol Support

- HTTP/1.1
- HTTP/2
- gRPC (over HTTP/2)

HTTP/3 is explicitly out of scope for V1.

---

## 3. Concurrency Model

- Single async process per Archimedes instance
- No internal worker forking
- Horizontal scalability achieved via:
  - Kubernetes replicas
  - External process managers (non-core)

Each instance:

- Runs one Tokio runtime
- Handles many concurrent requests via non-blocking I/O

---

## 4. Developer Programming Model

### 4.1 Application Initialization

Conceptual API:

- Developers instantiate an Archimedes application
- Handlers are registered by `operationId`

Handlers:

- Are async functions
- Accept strongly typed request objects
- Return strongly typed responses or Themis error types

### 4.2 Handler Registration Rules

- Every handler must map to exactly one `operationId`
- `operationId` must exist in the published contract
- Duplicate or missing registrations are startup errors

---

## 5. Typed Request & Response Handling

- Request and response types are generated from Themis contracts
- Raw JSON handling is forbidden by default
- Deserialization occurs before handler invocation
- Serialization occurs after handler completion

Any mismatch results in a validation error.

---

## 6. Middleware Architecture

### 6.1 Core Middleware (Mandatory & Immutable)

Core middleware is always enabled, cannot be removed, reordered, or overridden.

Execution order:

1. Request ID generation and propagation
2. Trace and span context initialization (OpenTelemetry)
3. Identity extraction
   - SPIFFE identity for internal calls
   - Public identity context for external calls
4. Authorization enforcement (OPA evaluation)
5. Contract validation (request)
6. Handler invocation
7. Contract validation (response, configurable)
8. Telemetry emission
9. Error normalization

### 6.2 Extension Points

Archimedes exposes limited extension points:

- `pre_handler`: after identity, before authorization
- `post_handler`: after handler, before serialization

Restrictions:

- Extensions cannot mutate core context
- Extensions cannot suppress logging, tracing, or metrics
- Extensions cannot override auth decisions

---

## 7. Contract Enforcement (Themis Sentinel)

- Embedded in Archimedes for Themis-native services
- Validates requests and responses against contracts
- Produces standard Themis error envelopes on failure

### 7.1 Modes

- Enforced mode: validation failures block requests
- Monitor-only mode: validation failures logged only

Mode is configurable per service.

---

## 8. Authorization Enforcement

### 8.1 Policy Source

- Policies authored in OPA/Rego
- Compiled and distributed by Eunomia

### 8.2 Evaluation Model

- OPA evaluator embedded in Archimedes
- Policies evaluated locally per request
- Evaluation input includes:
  - Caller identity (SPIFFE ID, User, ApiKey, or Anonymous)
  - Target service name
  - `operationId`
  - HTTP method and path
  - Filtered request headers
  - Timestamp and environment
  - Additional context (extensible)

> **Note**: The `PolicyInput` schema is defined authoritatively in the integration specification.
> Both Archimedes and Eunomia MUST use identical schemas.

### 8.3 Control Plane Endpoint

- Archimedes exposes a private control endpoint
- Used by Eunomia to push policy bundles
- Protected via mTLS + SPIFFE allowlist

Policy updates are applied atomically with rollback support.

### 8.4 Failure Behavior

- Unauthorized requests are denied
- HTTP 403 or equivalent gRPC status returned
- Structured audit log emitted
- Authorization metrics incremented

---

## 9. Error Handling

### 9.1 Standard Error Envelope

- All errors conform to the Themis error schema
- Errors are typed and declared in contracts

### 9.2 Error Categories

- Validation errors
- Authorization errors
- Authentication context errors
- Internal server errors

Unhandled errors are converted to internal error envelopes.

---

## 10. Configuration

- Strict, typed configuration schema
- File-based configuration
- Environment variable overrides
- Unknown fields are fatal errors
- Missing required fields fail startup

---

## 11. Observability

### 11.1 Standards

- OpenTelemetry-first
- Structured JSON logs

### 11.2 Metrics

Emitted per `operationId`:

- Request count
- Latency histograms (p50, p95, p99)
- Status code distribution
- Error types
- Request/response sizes
- In-flight requests

### 11.3 Logs

- Emitted to stdout/stderr
- Include:
  - request_id
  - trace_id
  - span_id
  - service
  - operationId

---

## 12. CI & Deployment Integration

- Startup fails if:
  - Contract artifact is missing
  - Handler registrations do not match contract
  - Configuration is invalid
  - Authorization policy bundle is invalid

Archimedes binaries are not deployable unless fully compliant.

---

## 13. Testing Strategy

### 13.1 Unit Tests

- Middleware ordering
- Handler dispatch
- Error normalization

### 13.2 Integration Tests

- Contract validation
- Authorization enforcement
- Policy update behavior

### 13.3 Load & Concurrency Tests

- Async performance
- Backpressure behavior
- Latency under load

---

## 14. Real-Time Communication (V1.1)

> **Note**: These features are part of Phase A8, extending V1 capabilities.

### 14.1 WebSocket Support

Archimedes provides WebSocket support with contract-based message validation:

- **Protocol**: RFC 6455 WebSocket over HTTP/1.1 upgrade
- **Library**: `tokio-tungstenite` for async WebSocket handling
- **Middleware**: Connection-level middleware (identity, authorization) applied on upgrade
- **Validation**: Message schemas defined in Themis contracts

#### WebSocket Lifecycle

1. Client sends HTTP upgrade request
2. Identity middleware extracts caller identity
3. Authorization middleware validates connection permission
4. Upgrade completes if authorized
5. Messages validated against contract schemas
6. Heartbeat/ping-pong for connection health
7. Graceful close with proper close frames

#### WebSocket Contract Integration

```yaml
# In Themis contract
websocket:
  chat:
    path: /ws/chat
    authRequired: true
    messages:
      clientMessage:
        type: object
        properties:
          type: { enum: [message, typing, presence] }
          content: { type: string }
      serverMessage:
        type: object
        properties:
          type: { enum: [message, error, presence] }
          content: { type: string }
          timestamp: { type: string, format: date-time }
```

### 14.2 Server-Sent Events (SSE)

Archimedes provides SSE for server-to-client streaming:

- **Protocol**: HTTP/1.1 or HTTP/2 with `text/event-stream` content type
- **Backpressure**: Configurable buffer size with drop-oldest policy
- **Reconnection**: Client-side `Last-Event-ID` header support
- **Heartbeat**: Configurable keepalive comments

#### SSE Event Format

```
id: <event-id>
event: <event-type>
data: <JSON payload validated against contract>
retry: <reconnection delay in ms>
```

### 14.3 Connection Management

- **Connection tracking**: All active connections tracked for graceful shutdown
- **Idle timeout**: Configurable idle connection termination
- **Max connections**: Per-client and global connection limits
- **Metrics**: Connection count, duration, message throughput

---

## 15. Background Processing (V1.1)

> **Note**: These features are part of Phase A8.

### 15.1 Task Spawning

Archimedes provides a managed task system for background work:

- **Spawning**: `TaskSpawner::spawn()` for fire-and-forget tasks
- **Tracking**: Optional task handles for cancellation and result retrieval
- **Panic handling**: Panics logged and contained, don't crash server
- **Shutdown**: Graceful task completion on server shutdown

#### Task Constraints

- Tasks MUST be `Send + 'static`
- Tasks inherit the current span for tracing
- Tasks have access to DI container services
- Tasks are NOT subject to request middleware (no HTTP context)

### 15.2 Scheduled Jobs

Archimedes supports cron-based scheduled execution:

- **Syntax**: Standard cron expressions (5-field or 6-field with seconds)
- **Library**: `cron` crate for parsing, custom scheduler
- **Overlap**: Configurable overlap policy (skip, queue, concurrent)
- **Timezone**: UTC by default, configurable per job

#### Scheduled Job Definition

```rust
#[archimedes::scheduled(cron = "0 0 * * *", overlap = "skip")]
async fn daily_cleanup(db: Inject<Database>) -> Result<(), TaskError> {
    db.delete_expired_sessions().await
}
```

### 15.3 Task Queues (Future)

> **Note**: Full task queue support deferred to V1.2

Basic retry support in V1.1:

- Fixed delay retry
- Exponential backoff
- Max retry count

---

## 16. Sidecar Proxy (V1.0 - Multi-Language Support)

> **Note**: The sidecar is CRITICAL for enabling non-Rust services to use Archimedes.

### 16.1 Overview

The Archimedes sidecar is a standalone binary that provides all Archimedes middleware
functionality to services written in any language. It acts as a reverse proxy between
the ingress and the application service.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           Kubernetes Pod                                 │
│                                                                          │
│  ┌──────────────────────┐          ┌──────────────────────────────────┐ │
│  │  Archimedes Sidecar  │          │      Application Service         │ │
│  │                      │          │   (Python/Go/TypeScript/C++)     │ │
│  │  ┌────────────────┐  │   HTTP   │                                  │ │
│  │  │ Request ID     │  │ ───────► │  - Business logic only           │ │
│  │  │ Identity       │  │          │  - No middleware concerns        │ │
│  │  │ Authorization  │  │ ◄─────── │  - No contract validation        │ │
│  │  │ Validation     │  │          │  - No telemetry setup            │ │
│  │  │ Telemetry      │  │          │                                  │ │
│  │  └────────────────┘  │          └──────────────────────────────────┘ │
│  └──────────────────────┘                                                │
│           ▲                                                              │
│           │ HTTPS/mTLS                                                   │
└───────────┼──────────────────────────────────────────────────────────────┘
            │
    ┌───────┴───────┐
    │   Ingress     │
    └───────────────┘
```

### 16.2 Request Flow

1. Ingress routes request to sidecar (port 8080)
2. Sidecar executes middleware pipeline:
   - Request ID generation
   - Trace context propagation
   - Identity extraction (from mTLS, JWT, API key)
   - Authorization (OPA policy evaluation)
   - Request validation (Themis contract)
3. Sidecar forwards request to application (localhost:3000)
4. Application processes request, returns response
5. Sidecar validates response (optional)
6. Sidecar emits telemetry
7. Sidecar returns response to ingress

### 16.3 Configuration

The sidecar uses the same configuration format as native Archimedes:

```toml
# archimedes-sidecar.toml

[sidecar]
# Port the sidecar listens on (external traffic)
listen_port = 8080

# Application service URL
upstream_url = "http://localhost:3000"

# Request timeout (forwarding to application)
upstream_timeout = "30s"

# Health check path on upstream
upstream_health_path = "/health"

[contract]
# Path to Themis contract artifact
path = "/etc/archimedes/contract.json"

# Watch for changes (hot reload)
watch = true

[policy]
# Path to OPA policy bundle
bundle_path = "/etc/archimedes/policy.tar.gz"

# Watch for changes (hot reload)
watch = true

[telemetry]
# OTLP endpoint for traces
otlp_endpoint = "http://otel-collector:4317"

# Prometheus metrics port
metrics_port = 9090

[identity]
# mTLS certificate paths (optional)
mtls_cert = "/etc/certs/cert.pem"
mtls_key = "/etc/certs/key.pem"
mtls_ca = "/etc/certs/ca.pem"
```

### 16.4 Header Propagation

The sidecar propagates specific headers to the application:

| Header | Description | Source |
|--------|-------------|--------|
| `X-Request-Id` | Request correlation ID | Generated by sidecar |
| `X-Trace-Id` | Distributed trace ID | From incoming request or generated |
| `X-Span-Id` | Current span ID | Generated for this request |
| `X-Caller-Identity` | JSON-encoded caller identity | Extracted from mTLS/JWT/API key |
| `X-Operation-Id` | Matched operation from contract | Resolved by sidecar |

The application can use these headers for logging and context, but does NOT need to
validate them - the sidecar has already done validation.

### 16.5 Application Responsibilities

Applications running behind the sidecar have minimal responsibilities:

**MUST**:
- Expose HTTP endpoint on configured port
- Return appropriate HTTP status codes
- Implement `/health` and `/ready` endpoints

**MAY**:
- Read `X-Caller-Identity` header for authorization decisions
- Read `X-Request-Id` for logging correlation
- Propagate trace headers to downstream services

**MUST NOT**:
- Validate requests against contracts (sidecar does this)
- Evaluate authorization policies (sidecar does this)
- Generate request IDs (sidecar does this)
- Set up telemetry exporters (sidecar handles this)

### 16.6 Deployment Patterns

#### Kubernetes Sidecar Container

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: my-service
spec:
  containers:
    # Application container
    - name: app
      image: my-service:latest
      ports:
        - containerPort: 3000
      env:
        - name: PORT
          value: "3000"
    
    # Archimedes sidecar
    - name: archimedes
      image: archimedes-sidecar:latest
      ports:
        - containerPort: 8080  # External traffic
        - containerPort: 9090  # Metrics
      volumeMounts:
        - name: config
          mountPath: /etc/archimedes
        - name: certs
          mountPath: /etc/certs
          readOnly: true
  
  volumes:
    - name: config
      configMap:
        name: my-service-archimedes
    - name: certs
      secret:
        secretName: my-service-certs
```

#### Docker Compose (Local Development)

```yaml
version: '3.8'
services:
  app:
    build: .
    environment:
      PORT: 3000
    # No port exposed - only sidecar is accessible
  
  sidecar:
    image: archimedes-sidecar:latest
    ports:
      - "8080:8080"   # External traffic
      - "9090:9090"   # Metrics
    volumes:
      - ./archimedes.toml:/etc/archimedes/config.toml
      - ./contract.json:/etc/archimedes/contract.json
    environment:
      ARCHIMEDES_SIDECAR_UPSTREAM_URL: http://app:3000
```

### 16.7 Metrics

The sidecar exposes Prometheus metrics:

| Metric | Type | Description |
|--------|------|-------------|
| `archimedes_sidecar_requests_total` | Counter | Total requests processed |
| `archimedes_sidecar_request_duration_seconds` | Histogram | Request latency (sidecar + upstream) |
| `archimedes_sidecar_upstream_duration_seconds` | Histogram | Upstream (application) latency only |
| `archimedes_sidecar_validation_errors_total` | Counter | Request/response validation failures |
| `archimedes_sidecar_auth_decisions_total` | Counter | Authorization decisions by result |
| `archimedes_sidecar_active_connections` | Gauge | Current active connections |

### 16.8 Health Checks

The sidecar provides health endpoints:

- `/_archimedes/health` - Sidecar liveness (always returns 200 if running)
- `/_archimedes/ready` - Sidecar readiness (checks config loaded, upstream reachable)
- `/_archimedes/metrics` - Prometheus metrics endpoint

---

## 17. Non-Goals (V1)

- Plugin-based middleware systems
- Runtime policy authoring
- Dynamic handler registration
- HTTP/3 support
- Full distributed task queues (V1.2)
