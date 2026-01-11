# ADR-013: Binding Parity Feature Priorities

> **Status**: Accepted
> **Date**: 2026-01-13
> **Decision Makers**: CTO, Staff Engineer

## Context

Phase A15 (Binding Parity) aims to ensure all language bindings (Python, TypeScript, C++, Go) have feature parity with Rust. With Phases A15.1-A15.3 complete, we need to decide the priority and scope for the remaining phases:

- **A15.4**: Real-time (WebSocket, SSE)
- **A15.5**: Background Tasks (TaskSpawner, JobScheduler)
- **A15.6**: TestClient

## Current Status (Post A15.3)

| Feature Category | Status | All Bindings |
|------------------|--------|--------------|
| Core Server | ✅ Complete | Python, TS, C++, Go |
| Router/Lifecycle | ✅ Complete | Python, TS, C++, Go |
| Extractors (JSON, Path, Query, Headers) | ✅ Complete | Python, TS, C++, Go |
| Extractors (Form, Cookie, Multipart) | ✅ Complete | Python, TS, C++, Go |
| Responses (JSON, File, Redirect, SetCookie) | ✅ Complete | Python, TS, C++, Go |
| Middleware Config (CORS, RateLimit, Compression, Static) | ✅ Complete | Python, TS, C++, Go |
| Real-time (WebSocket, SSE) | ❌ Not Started | None |
| Background Tasks | ❌ Not Started | None |
| TestClient | ❌ Not Started | None |

## Decision

### Priority 1: TestClient (A15.6) → Move to V1.0

**Rationale**: TestClient is essential for developers to write integration tests. Without it, developers must start actual HTTP servers for testing, which is slow and error-prone.

**Scope for V1.0**:
- `TestClient` class that can make requests without starting a real server
- Basic assertion helpers (`assert_status`, `assert_json`, `assert_header`)
- Support for JSON, form, and multipart request bodies
- Cookie jar support for session testing

### Priority 2: Real-time (A15.4) → Defer to V1.1

**Rationale**: WebSocket and SSE are more complex to expose through FFI due to:
- Stateful connection management
- Async streaming patterns differ significantly across languages
- Python asyncio, Node.js event loop, Go goroutines, C++ callbacks all have different models

**V1.0 Workaround**: Use the sidecar pattern for services that need real-time features. The Rust sidecar handles WebSocket/SSE, proxying to the application.

### Priority 3: Background Tasks (A15.5) → Defer to V1.1

**Rationale**: Each language has its own task/async model:
- Python: `asyncio.create_task`, threading
- Node.js: Native async, worker threads
- Go: goroutines
- C++: `std::async`, thread pools

**V1.0 Workaround**: Use language-native task systems. The Archimedes middleware pipeline doesn't require custom task spawning.

## Implementation Plan

### Phase A15.6: TestClient (Week 84) - V1.0 SCOPE

#### Python
```python
from archimedes import TestClient

client = TestClient(app)
response = client.get("/users/123")
response.assert_status(200)
response.assert_json({"id": "123", "name": "Alice"})
```

#### TypeScript
```typescript
const client = new TestClient(app);
const response = await client.get('/users/123');
response.assertStatus(200);
response.assertJson({ id: '123', name: 'Alice' });
```

#### C++
```cpp
archimedes::TestClient client(app);
auto response = client.get("/users/123");
response.assert_status(200);
response.assert_json(R"({"id": "123", "name": "Alice"})");
```

#### Go
```go
client := archimedes.NewTestClient(app)
response := client.Get("/users/123")
response.AssertStatus(200)
response.AssertJSON(map[string]any{"id": "123", "name": "Alice"})
```

### Deferred to V1.1

- WebSocket bindings (all languages)
- SSE bindings (all languages)
- TaskSpawner bindings (all languages)
- JobScheduler bindings (all languages)

## Consequences

### Positive
1. V1.0 ships with comprehensive testing support
2. Developers can write proper integration tests in their language of choice
3. Complex real-time features get proper design time in V1.1
4. Language-native async patterns aren't forced into unnatural FFI shapes

### Negative
1. Services needing WebSocket/SSE in V1.0 must use sidecar pattern
2. Background task integration deferred (use language-native solutions)

### Neutral
1. Phase A15 scope reduced for V1.0, extended in V1.1
2. Documentation must clearly explain V1.0 vs V1.1 feature availability

## References

- [features.md](../features.md) - Feature parity matrix
- [roadmap.md](../roadmap.md) - Phase A15 details
- [ADR-011](011-native-language-bindings.md) - Native bindings architecture
