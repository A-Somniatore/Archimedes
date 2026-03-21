# Archimedes Feature Reference

> **Version**: 1.5.0
> **Last Updated**: 2026-01-22
> **Purpose**: Comprehensive feature checklist for testing and language binding parity

This document lists all features available in Archimedes. It serves as:

1. **Testing Checklist** - Ensure all features are tested
2. **Language Binding Parity** - Ensure Python, TypeScript, C++, Go bindings implement all features
3. **Migration Guide** - Help teams migrating from FastAPI, Axum, Express, etc.

---

## Quick Reference

| Category              | Features | Rust | Python | TypeScript | C++ | Go  | V1.0 Status    |
| --------------------- | -------- | ---- | ------ | ---------- | --- | --- | -------------- |
| **Core**              | 12       | ✅   | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| **Extractors**        | 10       | ✅   | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| **Response Builders** | 6        | ✅   | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| **Middleware**        | 10       | ✅   | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| **Real-Time**         | 2        | ✅   | ⏸️     | ⏸️         | ⏸️  | ⏸️  | ⏸️ V1.1        |
| **Background Tasks**  | 2        | ✅   | ⏸️     | ⏸️         | ⏸️  | ⏸️  | ⏸️ V1.1        |
| **Documentation**     | 3        | ✅   | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| **Testing**           | 3        | ✅   | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| **Server**            | 14       | ✅   | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |

Legend: ✅ Complete | ⏸️ Deferred to V1.1 | 🔄 Partial | ❌ Not Started

**Test Summary**: 1,300+ tests across all Rust crates (1,279 unit tests + doc tests)

---

## 1. Core Features

### 1.1 HTTP Server

| Feature               | Description                      | Rust Crate        | Tests | Binding Priority |
| --------------------- | -------------------------------- | ----------------- | ----- | ---------------- |
| **HTTP/1.1 Support**  | Standard HTTP/1.1 protocol       | archimedes-server | ✅    | P0               |
| **HTTP/2 Support**    | HTTP/2 with multiplexing         | archimedes-server | ✅    | P0               |
| **Graceful Shutdown** | Drain connections on SIGTERM     | archimedes-server | ✅ 14 | P0               |
| **Health Probes**     | `/health` and `/ready` endpoints | archimedes-server | ✅ 8  | P0               |
| **TLS/HTTPS**         | Via rustls configuration         | archimedes-server | ✅    | P1               |

### 1.2 Routing

| Feature                     | Description                     | Rust Crate        | Tests | Binding Priority |
| --------------------------- | ------------------------------- | ----------------- | ----- | ---------------- |
| **Radix Tree Router**       | High-performance routing        | archimedes-router | ✅ 74 | P0               |
| **Path Parameters**         | `{id}` style extraction         | archimedes-router | ✅    | P0               |
| **Wildcard Routes**         | `{*path}` catch-all             | archimedes-router | ✅    | P1               |
| **Method Merging**          | Multiple methods per route      | archimedes-router | ✅    | P0               |
| **Operation-based Routing** | Routes by `operationId`         | archimedes-server | ✅    | P0               |
| **Sub-Router Nesting**      | `nest()` for composition        | archimedes-router | ✅ 6  | P2               |
| **Route Prefixes**          | `prefix()` for path prefixes    | archimedes-router | ✅ 4  | P2               |
| **Route Merge**             | `merge()` for combining routers | archimedes-router | ✅    | P2               |
| **OpenAPI Tags**            | `tag()` for route grouping      | archimedes-router | ✅ 2  | P2               |

### 1.3 Request Context

| Feature               | Description                      | Rust Crate      | Tests | Binding Priority |
| --------------------- | -------------------------------- | --------------- | ----- | ---------------- |
| **RequestContext**    | Request ID, identity, trace info | archimedes-core | ✅ 80 | P0               |
| **InvocationContext** | Full request + DI access         | archimedes-core | ✅    | P0               |
| **Caller Identity**   | SPIFFE, User, ApiKey, Anonymous  | archimedes-core | ✅    | P0               |

---

## 2. Extractors

All extractors implement the `FromRequest` trait and can be used as handler parameters.

### 2.1 Body Extractors

| Extractor     | Description               | Rust Crate         | Tests | Binding Priority |
| ------------- | ------------------------- | ------------------ | ----- | ---------------- |
| **Json\<T\>** | JSON body deserialization | archimedes-extract | ✅ 20 | P0               |
| **Form\<T\>** | URL-encoded form data     | archimedes-extract | ✅ 15 | P0               |
| **Bytes**     | Raw request body          | archimedes-extract | ✅ 8  | P1               |
| **Text**      | UTF-8 text body           | archimedes-extract | ✅ 8  | P1               |
| **Multipart** | Multipart form data       | archimedes-extract | ✅ 14 | P1               |

### 2.2 Parameter Extractors

| Extractor      | Description              | Rust Crate         | Tests | Binding Priority |
| -------------- | ------------------------ | ------------------ | ----- | ---------------- |
| **Path\<T\>**  | Path parameters (`{id}`) | archimedes-extract | ✅ 18 | P0               |
| **Query\<T\>** | Query string parameters  | archimedes-extract | ✅ 16 | P0               |
| **Headers**    | HTTP headers access      | archimedes-extract | ✅ 12 | P0               |
| **Cookies**    | Cookie values            | archimedes-extract | ✅ 16 | P1               |

### 2.3 Context Extractors

| Extractor       | Description              | Rust Crate         | Tests | Binding Priority |
| --------------- | ------------------------ | ------------------ | ----- | ---------------- |
| **Inject\<T\>** | DI container injection   | archimedes-extract | ✅ 10 | P1               |
| **State\<T\>**  | Shared application state | archimedes-extract | ✅ 8  | P1               |

---

## 3. Response Builders

### 3.1 Standard Responses

| Builder                    | Description                     | Rust Crate        | Tests | Binding Priority |
| -------------------------- | ------------------------------- | ----------------- | ----- | ---------------- |
| **Response::json()**       | JSON response with Content-Type | archimedes-server | ✅    | P0               |
| **Response::text()**       | Plain text response             | archimedes-server | ✅    | P0               |
| **Response::html()**       | HTML response                   | archimedes-server | ✅    | P1               |
| **Response::no_content()** | 204 No Content                  | archimedes-server | ✅    | P0               |
| **Response::redirect()**   | HTTP redirects (301, 302, 307)  | archimedes-server | ✅    | P1               |

### 3.2 File Responses

| Builder                        | Description                            | Rust Crate         | Tests | Binding Priority |
| ------------------------------ | -------------------------------------- | ------------------ | ----- | ---------------- |
| **FileResponse**               | File download with Content-Disposition | archimedes-extract | ✅ 13 | P1               |
| **FileResponse::attachment()** | Force download                         | archimedes-extract | ✅    | P1               |
| **FileResponse::inline()**     | Display in browser                     | archimedes-extract | ✅    | P1               |

### 3.3 Cookie Responses

| Builder                    | Description               | Rust Crate         | Tests | Binding Priority |
| -------------------------- | ------------------------- | ------------------ | ----- | ---------------- |
| **SetCookie**              | Set-Cookie header builder | archimedes-extract | ✅ 16 | P1               |
| **SetCookie::secure()**    | Secure flag               | archimedes-extract | ✅    | P1               |
| **SetCookie::http_only()** | HttpOnly flag             | archimedes-extract | ✅    | P1               |
| **SetCookie::same_site()** | SameSite attribute        | archimedes-extract | ✅    | P1               |

---

## 4. Middleware Pipeline

### 4.1 Fixed Middleware (Cannot be disabled)

| Stage | Middleware              | Description                     | Rust Crate            | Tests |
| ----- | ----------------------- | ------------------------------- | --------------------- | ----- |
| 1     | **Request ID**          | Generate/propagate X-Request-Id | archimedes-middleware | ✅ 8  |
| 2     | **Tracing**             | OpenTelemetry span creation     | archimedes-middleware | ✅ 12 |
| 3     | **Identity**            | Extract caller identity         | archimedes-middleware | ✅ 15 |
| 4     | **Authorization**       | OPA policy evaluation           | archimedes-middleware | ✅ 10 |
| 5     | **Request Validation**  | Contract schema validation      | archimedes-middleware | ✅ 18 |
| 6     | **Handler**             | User handler invocation         | archimedes-middleware | -     |
| 7     | **Response Validation** | Response schema validation      | archimedes-middleware | ✅ 12 |
| 8     | **Telemetry**           | Metrics and logging             | archimedes-middleware | ✅ 8  |
| 9     | **Error Normalization** | Standard error format           | archimedes-middleware | ✅ 10 |

### 4.2 Optional Middleware

| Middleware        | Description                   | Rust Crate            | Tests | Binding Priority |
| ----------------- | ----------------------------- | --------------------- | ----- | ---------------- |
| **CORS**          | Cross-Origin Resource Sharing | archimedes-middleware | ✅ 19 | P0               |
| **Rate Limiting** | Per-IP/user/key limits        | archimedes-middleware | ✅ 27 | P1               |
| **Compression**   | gzip/brotli/deflate support   | archimedes-middleware | ✅ 39 | P2               |

---

## 5. Contract Integration (Themis Sentinel)

| Feature                          | Description                            | Rust Crate          | Tests | Binding Priority |
| -------------------------------- | -------------------------------------- | ------------------- | ----- | ---------------- |
| **ArtifactLoader**               | Load contracts from file/JSON/registry | archimedes-sentinel | ✅ 12 | P0               |
| **OperationResolver**            | Match requests to operations           | archimedes-sentinel | ✅ 10 | P0               |
| **SchemaValidator**              | JSON Schema validation                 | archimedes-sentinel | ✅ 16 | P0               |
| **ValidationMiddleware**         | Request validation middleware          | archimedes-sentinel | ✅    | P0               |
| **ResponseValidationMiddleware** | Response validation                    | archimedes-sentinel | ✅    | P1               |
| **Monitor Mode**                 | Log-only validation                    | archimedes-sentinel | ✅ 7  | P1               |

---

## 6. Authorization (Eunomia/OPA)

| Feature                     | Description                    | Rust Crate       | Tests | Binding Priority |
| --------------------------- | ------------------------------ | ---------------- | ----- | ---------------- |
| **PolicyEvaluator**         | OPA/Rego evaluation (regorus)  | archimedes-authz | ✅ 26 | P0               |
| **BundleLoader**            | Load OPA tar.gz bundles        | archimedes-authz | ✅ 11 | P0               |
| **DecisionCache**           | TTL-based decision caching     | archimedes-authz | ✅ 8  | P1               |
| **EvaluatorConfig**         | Production/development presets | archimedes-authz | ✅ 5  | P1               |
| **AuthorizationMiddleware** | Pipeline integration           | archimedes-authz | ✅    | P0               |

---

## 7. Telemetry

### 7.1 Metrics (Prometheus)

| Metric                            | Type      | Description                     | Rust Crate           | Tests |
| --------------------------------- | --------- | ------------------------------- | -------------------- | ----- |
| **http_requests_total**           | Counter   | Total requests by operation     | archimedes-telemetry | ✅    |
| **http_request_duration_seconds** | Histogram | Request latency (p50, p95, p99) | archimedes-telemetry | ✅    |
| **http_request_size_bytes**       | Histogram | Request body size               | archimedes-telemetry | ✅    |
| **http_response_size_bytes**      | Histogram | Response body size              | archimedes-telemetry | ✅    |
| **http_requests_in_flight**       | Gauge     | Current active requests         | archimedes-telemetry | ✅    |
| **authz_decisions_total**         | Counter   | Authorization decisions         | archimedes-authz     | ✅    |
| **validation_errors_total**       | Counter   | Validation failures             | archimedes-sentinel  | ✅    |

### 7.2 Tracing (OpenTelemetry)

| Feature               | Description                      | Rust Crate           | Tests |
| --------------------- | -------------------------------- | -------------------- | ----- |
| **Span Creation**     | Create spans per request         | archimedes-telemetry | ✅ 10 |
| **W3C Trace Context** | Propagate traceparent/tracestate | archimedes-telemetry | ✅    |
| **Span Attributes**   | request_id, operation_id, etc.   | archimedes-telemetry | ✅    |
| **OTLP Export**       | Export to OTLP collectors        | archimedes-telemetry | ✅    |

### 7.3 Logging

| Feature             | Description            | Rust Crate           | Tests |
| ------------------- | ---------------------- | -------------------- | ----- |
| **Structured JSON** | JSON log format        | archimedes-telemetry | ✅    |
| **Request Logging** | Log request/response   | archimedes-telemetry | ✅    |
| **Correlation**     | request_id in all logs | archimedes-telemetry | ✅    |

---

## 8. Real-Time Communication

### 8.1 WebSocket

| Feature                | Description                     | Rust Crate    | Tests | Binding Priority |
| ---------------------- | ------------------------------- | ------------- | ----- | ---------------- |
| **Connection Upgrade** | HTTP → WebSocket upgrade        | archimedes-ws | ✅ 52 | P1               |
| **Message Types**      | Text, Binary, Ping, Pong, Close | archimedes-ws | ✅    | P1               |
| **Connection Manager** | Track active connections        | archimedes-ws | ✅    | P1               |
| **Broadcast**          | Send to all connections         | archimedes-ws | ✅    | P1               |
| **JSON Messages**      | Serde JSON serialization        | archimedes-ws | ✅    | P1               |

### 8.2 Server-Sent Events (SSE)

| Feature          | Description                          | Rust Crate     | Tests | Binding Priority |
| ---------------- | ------------------------------------ | -------------- | ----- | ---------------- |
| **Event Stream** | text/event-stream responses          | archimedes-sse | ✅ 38 | P1               |
| **Event Types**  | Named event types                    | archimedes-sse | ✅    | P1               |
| **Event ID**     | Last-Event-ID support                | archimedes-sse | ✅    | P1               |
| **Retry Hint**   | Client reconnection delay            | archimedes-sse | ✅    | P1               |
| **Keepalive**    | Comment-based heartbeats             | archimedes-sse | ✅    | P1               |
| **Backpressure** | Configurable buffer with drop policy | archimedes-sse | ✅    | P2               |

---

## 9. Background Processing

### 9.1 Task Spawning

| Feature               | Description                 | Rust Crate       | Tests | Binding Priority |
| --------------------- | --------------------------- | ---------------- | ----- | ---------------- |
| **Spawner**           | Fire-and-forget async tasks | archimedes-tasks | ✅ 41 | P1               |
| **Task Handles**      | Cancel and track tasks      | archimedes-tasks | ✅    | P2               |
| **Panic Recovery**    | Contain panics, log errors  | archimedes-tasks | ✅    | P1               |
| **Graceful Shutdown** | Wait for tasks on shutdown  | archimedes-tasks | ✅    | P1               |
| **SharedSpawner**     | DI container integration    | archimedes-tasks | ✅    | P1               |

### 9.2 Job Scheduler

| Feature              | Description                | Rust Crate       | Tests | Binding Priority |
| -------------------- | -------------------------- | ---------------- | ----- | ---------------- |
| **Cron Expressions** | Standard cron syntax       | archimedes-tasks | ✅    | P1               |
| **Job Registration** | Schedule recurring jobs    | archimedes-tasks | ✅    | P1               |
| **Overlap Policy**   | Skip, queue, or concurrent | archimedes-tasks | ✅    | P2               |
| **Job Status**       | Query job status           | archimedes-tasks | ✅    | P2               |
| **Manual Trigger**   | Run jobs on demand         | archimedes-tasks | ✅    | P2               |

---

## 10. API Documentation

| Feature               | Description          | Rust Crate      | Tests | Binding Priority |
| --------------------- | -------------------- | --------------- | ----- | ---------------- |
| **OpenAPI Generator** | Themis → OpenAPI 3.1 | archimedes-docs | ✅ 29 | P1               |
| **Swagger UI**        | Interactive API docs | archimedes-docs | ✅    | P1               |
| **ReDoc**             | Beautiful API docs   | archimedes-docs | ✅    | P1               |

---

## 11. Testing Utilities

| Feature                 | Description           | Rust Crate      | Tests | Binding Priority |
| ----------------------- | --------------------- | --------------- | ----- | ---------------- |
| **TestClient**          | In-memory HTTP client | archimedes-test | ✅ 30 | P0               |
| **TestRequest**         | Request builder       | archimedes-test | ✅    | P0               |
| **TestResponse**        | Response assertions   | archimedes-test | ✅    | P0               |
| **assert_status()**     | Status code assertion | archimedes-test | ✅    | P0               |
| **assert_json_field()** | JSON field assertion  | archimedes-test | ✅    | P0               |
| **assert_header()**     | Header assertion      | archimedes-test | ✅    | P0               |

---

## 12. Server Features

### 12.1 Lifecycle

| Feature             | Description                       | Rust Crate        | Tests | Binding Priority |
| ------------------- | --------------------------------- | ----------------- | ----- | ---------------- |
| **Lifecycle Hooks** | on_startup / on_shutdown          | archimedes-server | ✅ 11 | P0               |
| **Named Hooks**     | Debug-friendly hook names         | archimedes-server | ✅    | P1               |
| **Async Callbacks** | Async hook functions              | archimedes-server | ✅    | P0               |
| **Error Handling**  | Startup stops, shutdown continues | archimedes-server | ✅    | P0               |

### 12.2 Static Files

| Feature              | Description                        | Rust Crate        | Tests | Binding Priority |
| -------------------- | ---------------------------------- | ----------------- | ----- | ---------------- |
| **StaticFiles**      | Serve directory contents           | archimedes-server | ✅ 31 | P1               |
| **Index Fallback**   | index.html for directories         | archimedes-server | ✅    | P1               |
| **Cache Headers**    | ETag, Last-Modified, Cache-Control | archimedes-server | ✅    | P1               |
| **Range Requests**   | Partial content (206)              | archimedes-server | ✅    | P2               |
| **Precompressed**    | Serve .gz and .br variants         | archimedes-server | ✅    | P2               |
| **Security**         | Directory traversal prevention     | archimedes-server | ✅    | P0               |
| **MIME Types**       | 40+ content types                  | archimedes-server | ✅    | P1               |
| **304 Not Modified** | If-None-Match, If-Modified-Since   | archimedes-server | ✅    | P1               |

### 12.3 Configuration

| Feature           | Description                      | Rust Crate        | Tests | Binding Priority |
| ----------------- | -------------------------------- | ----------------- | ----- | ---------------- |
| **Typed Config**  | TOML/JSON configuration          | archimedes-config | ✅ 52 | P0               |
| **Env Overrides** | Environment variable overrides   | archimedes-config | ✅    | P0               |
| **Hot Reload**    | File watching for config changes | archimedes-config | ✅ 15 | P1               |
| **FileWatcher**   | Cross-platform file monitoring   | archimedes-config | ✅    | P1               |
| **Debouncing**    | Prevent reload storms            | archimedes-config | ✅    | P1               |

### 12.4 Server Middleware Integration

Native server middleware configuration for identity extraction and authorization without the sidecar.

| Feature                        | Description                              | Rust Crate        | Tests | Binding Priority |
| ------------------------------ | ---------------------------------------- | ----------------- | ----- | ---------------- |
| **MiddlewareConfig**           | Middleware pipeline configuration        | archimedes-server | ✅ 5  | P0               |
| **MiddlewareConfigBuilder**    | Builder pattern for middleware config    | archimedes-server | ✅    | P0               |
| **ServerBuilder::middleware()** | Configure middleware on server           | archimedes-server | ✅    | P0               |
| **Identity Extraction**        | Extract identity from HTTP headers       | archimedes-server | ✅    | P0               |
| **JWT Token Support**          | Authorization: Bearer token parsing      | archimedes-server | ✅    | P1               |
| **X-Caller-Identity**          | JSON identity header from proxies        | archimedes-server | ✅    | P0               |
| **X-User-Id/Roles Headers**    | Simple user identity headers             | archimedes-server | ✅    | P0               |
| **Anonymous Fallback**         | Default anonymous identity when no auth  | archimedes-server | ✅    | P0               |

**Supported Identity Headers** (checked in order):
1. Trusted Identity Header (configurable, e.g., `X-Archimedes-Identity`)
2. `X-Caller-Identity` - JSON-encoded identity object from sidecar/proxy
3. `Authorization: Bearer <jwt>` - JWT token (extracts claims without crypto verification)
4. `X-User-Id` + `X-User-Roles` - Simple identity headers

**Usage Example:**
```rust
use archimedes_server::{Server, MiddlewareConfig};

let server = Server::builder()
    .http_addr("0.0.0.0:8080")
    .middleware(MiddlewareConfig::builder()
        .enable_identity()           // Extract identity from headers
        .enable_authorization()      // Enable OPA policy evaluation
        .service_name("my-service")
        .build())
    .build();
```

---

## 13. Dependency Injection

| Feature         | Description                 | Rust Crate         | Tests | Binding Priority |
| --------------- | --------------------------- | ------------------ | ----- | ---------------- |
| **Container**   | Type-safe DI container      | archimedes-core    | ✅ 15 | P0               |
| **Inject\<T\>** | Handler parameter injection | archimedes-extract | ✅    | P0               |
| **Singleton**   | Single instance services    | archimedes-core    | ✅    | P0               |
| **Scoped**      | Request-scoped services     | archimedes-core    | ✅    | P1               |

---

## 14. Handler Macros

| Feature                  | Description                   | Rust Crate        | Tests | Binding Priority |
| ------------------------ | ----------------------------- | ----------------- | ----- | ---------------- |
| **#[handler]**           | Handler attribute macro       | archimedes-macros | ✅ 14 | Rust only        |
| **Operation binding**    | `operation = "operationId"`   | archimedes-macros | ✅    | Rust only        |
| **Parameter extraction** | Auto-extract from request     | archimedes-macros | ✅    | Rust only        |
| **HandlerBinder**        | Validate handlers vs contract | archimedes-core   | ✅ 6  | P1               |

---

## 15. Error Handling

| Feature                 | Description                      | Rust Crate            | Tests | Binding Priority |
| ----------------------- | -------------------------------- | --------------------- | ----- | ---------------- |
| **ThemisError**         | Standard error type              | archimedes-core       | ✅    | P0               |
| **ErrorCategory**       | Validation, Auth, Internal, etc. | archimedes-core       | ✅    | P0               |
| **Error Envelope**      | Structured JSON errors           | archimedes-core       | ✅    | P0               |
| **Error Normalization** | Consistent error format          | archimedes-middleware | ✅ 10 | P0               |

---

## 16. Sidecar Proxy (Multi-Language)

| Feature                | Description                     | Rust Crate         | Tests | Binding Priority |
| ---------------------- | ------------------------------- | ------------------ | ----- | ---------------- |
| **SidecarServer**      | Reverse proxy server            | archimedes-sidecar | ✅ 39 | N/A              |
| **ProxyClient**        | HTTP forwarding                 | archimedes-sidecar | ✅    | N/A              |
| **Header Propagation** | X-Request-Id, X-Caller-Identity | archimedes-sidecar | ✅    | N/A              |
| **Health Endpoints**   | /\_archimedes/health, /ready    | archimedes-sidecar | ✅    | N/A              |
| **SidecarConfig**      | TOML/JSON configuration         | archimedes-sidecar | ✅    | N/A              |

---

## Language Binding Parity Matrix

### P0 Features (Must Have for V1.0)

These features MUST be implemented in all language bindings before release:

| Feature                  | Python | TypeScript | C++ | Go  |
| ------------------------ | ------ | ---------- | --- | --- |
| HTTP Server              | ✅     | ✅         | ✅  | ✅  |
| Handler Registration     | ✅     | ✅         | ✅  | ✅  |
| Request Context          | ✅     | ✅         | ✅  | ✅  |
| Response Builder         | ✅     | ✅         | ✅  | ✅  |
| JSON Extractor           | ✅     | ✅         | ✅  | ✅  |
| Path Extractor           | ✅     | ✅         | ✅  | ✅  |
| Query Extractor          | ✅     | ✅         | ✅  | ✅  |
| Headers Extractor        | ✅     | ✅         | ✅  | ✅  |
| Request ID Middleware    | ✅     | ✅         | ✅  | ✅  |
| Tracing Middleware       | ✅     | ✅         | ✅  | ✅  |
| Identity Middleware      | ✅     | ✅         | ✅  | ✅  |
| Authorization Middleware | ✅     | ✅         | ✅  | ✅  |
| Request Validation       | ✅     | ✅         | ✅  | ✅  |
| Response Validation      | ✅     | ✅         | ✅  | ✅  |
| Error Normalization      | ✅     | ✅         | ✅  | ✅  |
| Telemetry                | ✅     | ✅         | ✅  | ✅  |
| Contract Loading         | ✅     | ✅         | ✅  | ✅  |
| Graceful Shutdown        | ✅     | ✅         | ✅  | ✅  |
| DI Container             | ✅     | ✅         | ✅  | ✅  |
| Lifecycle Hooks          | ✅     | ✅         | ✅  | ✅  |
| CORS Middleware          | ✅     | ✅         | ✅  | ✅  |
| TestClient               | 🔄     | 🔄         | 🔄  | 🔄  |

### P1 Features (Should Have)

| Feature              | Python | TypeScript | C++ | Go  | V1.0 Status    |
| -------------------- | ------ | ---------- | --- | --- | -------------- |
| Form Extractor       | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| Cookie Extractor     | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| Multipart Uploads    | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| FileResponse         | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| SetCookie            | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| Rate Limiting Config | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| Static Files Config  | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| TestClient           | ✅     | ✅         | ✅  | ✅  | ✅ Complete    |
| WebSocket            | ❌     | ❌         | ❌  | ❌  | ⏸️ V1.1        |
| SSE                  | ❌     | ❌         | ❌  | ❌  | ⏸️ V1.1        |
| Task Spawner         | ❌     | ❌         | ❌  | ❌  | ⏸️ V1.1        |
| Job Scheduler        | ❌     | ❌         | ❌  | ❌  | ⏸️ V1.1        |
| OpenAPI Docs         | 🔄     | 🔄         | 🔄  | 🔄  | 🔄 Partial     |
| Config Hot Reload    | 🔄     | 🔄         | 🔄  | 🔄  | 🔄 Partial     |

### P2 Features (Nice to Have)

| Feature             | Python | TypeScript | C++ | Go  |
| ------------------- | ------ | ---------- | --- | --- |
| Compression Config  | ✅     | ✅         | ✅  | ✅  |
| Sub-routers         | ❌     | ❌         | ❌  | ❌  |
| Route Prefixes      | ❌     | ❌         | ❌  | ❌  |
| Range Requests      | 🔄     | 🔄         | 🔄  | 🔄  |
| Precompressed Files | 🔄     | 🔄         | 🔄  | 🔄  |

---

## Test Count Summary

| Crate                 | Unit Tests | Doc Tests | E2E Tests | Total    |
| --------------------- | ---------- | --------- | --------- | -------- |
| archimedes-core       | 80         | -         | -         | 80       |
| archimedes-server     | 141        | 53        | -         | 194      |
| archimedes-middleware | 131        | -         | 26        | 157      |
| archimedes-extract    | 152        | 36        | -         | 188      |
| archimedes-router     | 57         | -         | -         | 57       |
| archimedes-telemetry  | 25         | -         | -         | 25       |
| archimedes-config     | 67         | -         | -         | 67       |
| archimedes-sentinel   | 38         | -         | -         | 38       |
| archimedes-authz      | 26         | -         | -         | 26       |
| archimedes-docs       | 29         | -         | -         | 29       |
| archimedes-ws         | 52         | -         | -         | 52       |
| archimedes-sse        | 38         | -         | -         | 38       |
| archimedes-tasks      | 41         | -         | -         | 41       |
| archimedes-sidecar    | 39         | -         | -         | 39       |
| archimedes-macros     | 14         | -         | -         | 14       |
| archimedes-ffi        | 44         | -         | -         | 44       |
| archimedes-py         | 111        | -         | -         | 111      |
| archimedes-node       | 95         | -         | -         | 95       |
| archimedes-test       | 30         | -         | -         | 30       |
| examples/rust-native  | 14         | -         | -         | 14       |
| examples/go-native    | 9          | -         | -         | 9        |
| **TOTAL**             | **1254**   | **89**    | **26**    | **1369** |

---

## Migration Checklists

### From FastAPI

- [ ] Replace `@app.get()` with `@app.operation()`
- [ ] Replace Pydantic models with JSON Schema (contract)
- [ ] Replace `Depends()` with `Inject<T>`
- [ ] Replace `BackgroundTasks` with `Spawner`
- [ ] Replace `CORSMiddleware` with `CorsConfig`
- [ ] Remove manual request validation (automatic from contract)
- [ ] Remove manual response validation (automatic from contract)
- [ ] Configure OPA policies for authorization

### From Axum

- [ ] Replace `Router::new().route()` with operation-based handlers
- [ ] Replace tower middleware with fixed pipeline
- [ ] Replace extractors with Archimedes extractors
- [ ] Configure contract for validation
- [ ] Configure OPA policies for authorization

### From Express

- [ ] Replace `app.get()` with `app.operation()`
- [ ] Replace body-parser with automatic JSON extraction
- [ ] Replace express-validator with contract validation
- [ ] Replace passport with OPA authorization
- [ ] Replace cors() with CorsConfig

---

## Appendix: Feature Flags

| Feature Flag | Crate                 | Description                            |
| ------------ | --------------------- | -------------------------------------- |
| `sentinel`   | archimedes-middleware | Enable Themis contract validation      |
| `opa`        | archimedes-middleware | Enable OPA authorization               |
| `authz`      | archimedes-server     | Enable OPA authorization in server     |
| `sentinel`   | archimedes-server     | Enable contract validation in server   |
| `full`       | archimedes-server     | Enable all server middleware features  |
| `full`       | archimedes            | Enable all features                    |
| `ws`         | archimedes            | Enable WebSocket support               |
| `sse`        | archimedes            | Enable SSE support                     |
| `tasks`      | archimedes            | Enable background tasks                |
| `docs`       | archimedes            | Enable API documentation               |
