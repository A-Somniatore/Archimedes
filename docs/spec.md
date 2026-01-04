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

## 14. Non-Goals (V1)

- Plugin-based middleware systems
- Runtime policy authoring
- Dynamic handler registration
- HTTP/3 support
