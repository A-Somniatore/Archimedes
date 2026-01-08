# Archimedes

**Async HTTP Server Framework for the Themis Platform**

[![Tests](https://img.shields.io/badge/tests-969%20passing-brightgreen)](docs/roadmap.md)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange)](Cargo.toml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)

Archimedes is an opinionated Rust-based server framework that provides:

- 🔒 **Contract-First Enforcement** – Validate all requests/responses against Themis contracts
- 🛡️ **Built-in Authorization** – Embedded OPA evaluator for Eunomia policies
- 📊 **First-Class Observability** – OpenTelemetry traces, metrics, and structured logs
- ⚡ **High Performance** – Async Rust with zero-cost abstractions
- 🔗 **Mandatory Middleware** – Core middleware cannot be disabled or reordered
- 🌐 **Multi-Language Support** – Sidecar proxy for Python, Go, TypeScript, C++ services

---

## Quick Links

- [Design Document](docs/design.md) – Architecture and implementation details
- [Specification](docs/spec.md) – Technical requirements
- [Roadmap](docs/roadmap.md) – Development progress and plans
- [Contributing](CONTRIBUTING.md) – Development guidelines
- [ADR-009](docs/decisions/009-archimedes-sidecar-multi-language.md) – Sidecar pattern for multi-language support

---

## Current Status

**Phase A10 Complete** – 969 tests passing across 17 crates

| Crate                   | Tests | Description                                     |
| ----------------------- | ----- | ----------------------------------------------- |
| `archimedes-core`       | 80    | Core types, DI, handler traits                  |
| `archimedes-server`     | 90    | HTTP server, routing, graceful shutdown         |
| `archimedes-middleware` | 104   | 8-stage fixed middleware pipeline               |
| `archimedes-router`     | 57    | High-performance radix tree router              |
| `archimedes-extract`    | 109   | Request extractors (Path, Query, Json, Headers) |
| `archimedes-config`     | 52    | TOML/JSON configuration with env overrides      |
| `archimedes-telemetry`  | 25    | OpenTelemetry traces, Prometheus metrics        |
| `archimedes-sentinel`   | 38    | Themis contract validation                      |
| `archimedes-authz`      | 26    | OPA policy evaluation (regorus)                 |
| `archimedes-docs`       | 29    | OpenAPI generation, Swagger UI, ReDoc           |
| `archimedes-macros`     | 14    | `#[handler]` proc macro                         |
| `archimedes-ws`         | 52    | WebSocket support                               |
| `archimedes-sse`        | 38    | Server-Sent Events                              |
| `archimedes-tasks`      | 41    | Background tasks and scheduled jobs             |
| `archimedes-sidecar`    | 39    | Multi-language sidecar proxy                    |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Archimedes Server                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Transport Layer                        │   │
│  │              HTTP/1.1 (hyper)  │  HTTP/2                  │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Middleware Pipeline (Fixed Order)            │   │
│  │                                                           │   │
│  │  Request ID → Tracing → Identity → AuthZ → Validation    │   │
│  │                              │                            │   │
│  │                              ▼                            │   │
│  │                         HANDLER                           │   │
│  │                              │                            │   │
│  │  Response Validation → Telemetry → Error Normalization   │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                   Supporting Systems                      │   │
│  │  Themis Sentinel │ OPA Evaluator │ Config │ Health       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Key Features

### Contract-First Enforcement

All requests and responses are validated against Themis contracts automatically:

```rust
use archimedes::prelude::*;

// Handler is validated against contract at startup
#[handler(operation = "createUser")]
async fn create_user(
    ctx: RequestContext,
    body: Json<CreateUserRequest>,
) -> Result<Json<User>, ThemisError> {
    // Request body already validated against contract schema
    let user = User::create(body.name.clone(), body.email.clone()).await?;
    Ok(Json(user))
}
```

### Mandatory Middleware (Cannot Be Disabled)

1. **Request ID** – UUID v7 for every request
2. **Tracing** – OpenTelemetry span initialization
3. **Identity** – SPIFFE/JWT/API key extraction
4. **Authorization** – OPA policy evaluation
5. **Validation** – Contract schema validation
6. **Response Validation** – Output schema enforcement
7. **Telemetry** – Metrics and logging
8. **Error Normalization** – Standard error envelopes

### Multi-Language Support (Sidecar)

Services in Python, Go, TypeScript, and C++ can use Archimedes via the sidecar proxy:

```yaml
# docker-compose.yml
services:
  app:
    image: my-python-service:latest
    ports:
      - "8081:8081"

  archimedes:
    image: ghcr.io/themis-platform/archimedes-sidecar:latest
    ports:
      - "8080:8080"
    environment:
      ARCHIMEDES_UPSTREAM: http://app:8081
      ARCHIMEDES_CONTRACT_PATH: /etc/archimedes/contract.json
```

The sidecar handles all middleware concerns – your service just implements business logic.

### Built-in Observability

Zero-config observability with automatic metrics per operation:

- `archimedes_requests_total{operation_id, status}`
- `archimedes_request_duration_seconds{operation_id}`
- `archimedes_authorization_decisions_total{operation_id, decision}`

---

## Project Structure

```
archimedes/
├── crates/
│   ├── archimedes/              # Main facade (re-exports)
│   ├── archimedes-core/         # Core types, DI, handlers
│   ├── archimedes-server/       # HTTP server, routing
│   ├── archimedes-middleware/   # 8-stage pipeline
│   ├── archimedes-router/       # Radix tree router
│   ├── archimedes-extract/      # Request extractors
│   ├── archimedes-config/       # Configuration
│   ├── archimedes-telemetry/    # OpenTelemetry
│   ├── archimedes-sentinel/     # Themis validation
│   ├── archimedes-authz/        # OPA authorization
│   ├── archimedes-docs/         # OpenAPI, Swagger, ReDoc
│   ├── archimedes-macros/       # Proc macros
│   ├── archimedes-ws/           # WebSocket
│   ├── archimedes-sse/          # Server-Sent Events
│   ├── archimedes-tasks/        # Background tasks
│   └── archimedes-sidecar/      # Multi-language proxy
├── docs/
│   ├── design.md                # Implementation design
│   ├── spec.md                  # Specification
│   ├── roadmap.md               # Development roadmap
│   └── decisions/               # Architecture Decision Records
├── README.md
└── CONTRIBUTING.md
```

---

## Usage

### Native Rust Service

```rust
use archimedes::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load contract and policy
    let contract = Contract::load("./contract.json")?;
    let policy = PolicyBundle::load("./policy.tar.gz")?;

    // Build the server
    let server = Archimedes::builder()
        .contract(contract)
        .policy(policy)
        .register_handler("getUser", get_user)
        .register_handler("createUser", create_user)
        .build()?;

    // Run with graceful shutdown
    server.serve("0.0.0.0:8080").await?;
    Ok(())
}

#[handler(operation = "getUser")]
async fn get_user(
    ctx: RequestContext,
    path: Path<UserId>,
    db: Inject<Database>,
) -> Result<Json<User>, ThemisError> {
    let user = db.find_user(path.user_id).await?;
    Ok(Json(user))
}
```

### Non-Rust Service (Python with Sidecar)

```python
# app.py - Your Python service just handles business logic
from fastapi import FastAPI, Header

app = FastAPI()

@app.get("/users/{user_id}")
async def get_user(
    user_id: str,
    x_request_id: str = Header(...),      # Provided by sidecar
    x_caller_identity: str = Header(...)  # Provided by sidecar
):
    # No validation needed - sidecar already validated
    # No auth needed - sidecar already authorized
    return {"id": user_id, "name": "Alice", "email": "alice@example.com"}
```

Deploy with:

```bash
docker run -d \
  -e ARCHIMEDES_UPSTREAM=http://localhost:8081 \
  -v ./contract.json:/etc/archimedes/contract.json \
  -p 8080:8080 \
  ghcr.io/themis-platform/archimedes-sidecar:latest
```

---

## Extractors

| Extractor   | Description             | Example                   |
| ----------- | ----------------------- | ------------------------- |
| `Path<T>`   | URL path parameters     | `Path<UserId>`            |
| `Query<T>`  | Query string parameters | `Query<Pagination>`       |
| `Json<T>`   | JSON request body       | `Json<CreateUserRequest>` |
| `Headers`   | Request headers         | `Headers`                 |
| `Inject<T>` | DI container service    | `Inject<Database>`        |

---

## Configuration

```toml
# archimedes.toml

[server]
address = "0.0.0.0:8080"
graceful_shutdown_timeout = "30s"

[contract]
path = "./contract.json"
validation_mode = "enforce"  # or "monitor"

[policy]
bundle_path = "./policy.tar.gz"
cache_ttl = "60s"

[telemetry]
otlp_endpoint = "http://otel-collector:4317"
service_name = "my-service"
```

---

## Development

### Prerequisites

- Rust 1.85+

### Commands

```bash
cargo build --workspace          # Build all crates
cargo test --workspace           # Run all tests (969 tests)
cargo clippy --workspace -- -D warnings  # Lint
cargo doc --workspace --no-deps  # Generate docs
```

---

## Related Projects

| Project                                                                        | Description                             |
| ------------------------------------------------------------------------------ | --------------------------------------- |
| [Themis](../themis/)                                                           | Contract validation and code generation |
| [Eunomia](../eunomia/)                                                         | Authorization policy platform           |
| [themis-platform-types](https://github.com/A-Somniatore/themis-platform-types) | Shared platform types                   |

---

## License

Apache License 2.0 – See [LICENSE](LICENSE) for details.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
