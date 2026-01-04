# Archimedes

**Async HTTP/gRPC/GraphQL Server Framework for the Themis Platform**

Archimedes is an opinionated Rust-based server framework that provides:

- 🔒 **Contract-First Enforcement** – Validate all requests/responses against Themis contracts
- 🛡️ **Built-in Authorization** – Embedded OPA evaluator for Eunomia policies
- 📊 **First-Class Observability** – OpenTelemetry traces, metrics, and structured logs
- ⚡ **High Performance** – Async Rust with zero-cost abstractions
- 🔗 **Mandatory Middleware** – Core middleware cannot be disabled or reordered

## Quick Links

- [Design Document](docs/design.md)
- [Specification](docs/spec.md)
- [Roadmap](docs/roadmap.md)
- [Contributing](CONTRIBUTING.md)
- [Integration Specification](../docs/integration/integration-spec.md) – Shared schemas with Themis/Eunomia
- [Themis Platform](../)

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Archimedes Server                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Transport Layer                        │   │
│  │   HTTP/1.1 (hyper)  │  HTTP/2  │  gRPC (tonic)           │   │
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

## Key Features

### Contract-First Enforcement

- All requests validated against Themis contracts
- All responses validated (configurable)
- Type-safe handlers generated from contracts

### Mandatory Middleware

- **Request ID** – UUID v7 for every request
- **Tracing** – OpenTelemetry span initialization
- **Identity** – SPIFFE/JWT extraction
- **Authorization** – OPA policy evaluation
- **Validation** – Contract schema validation

### Observability

- OpenTelemetry traces with context propagation
- Prometheus metrics per operation
- Structured JSON logging
- Request/response timing

### Multi-Protocol Support

- HTTP/1.1 and HTTP/2
- gRPC via Tonic
- GraphQL support (planned)

## Project Structure (Planned)

```
archimedes/
├── .github/
│   └── copilot-instructions.md
├── docs/
│   ├── design.md                 # Implementation design
│   ├── spec.md                   # Specification
│   └── roadmap.md                # Development roadmap
├── crates/                       # (when code is added)
│   ├── archimedes/               # Main facade crate
│   ├── archimedes-core/          # Core types and traits
│   ├── archimedes-server/        # HTTP/gRPC server
│   ├── archimedes-middleware/    # Middleware pipeline
│   ├── archimedes-sentinel/      # Themis contract validation
│   ├── archimedes-authz/         # OPA/Eunomia integration
│   ├── archimedes-telemetry/     # OpenTelemetry integration
│   └── archimedes-config/        # Configuration management
├── tests/                        # Integration tests
├── examples/                     # Example services
├── README.md
└── CONTRIBUTING.md
```

## Usage Example (Planned API)

```rust
use archimedes::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load contract artifact
    let contract = Contract::load("./contract-artifact.json")?;

    // Build the server
    let server = Archimedes::builder()
        .contract(contract)
        .register_handler("getUser", get_user_handler)
        .register_handler("createUser", create_user_handler)
        .build()?;

    // Run the server
    server.serve("0.0.0.0:8080").await?;

    Ok(())
}

async fn get_user_handler(
    ctx: &RequestContext,
    req: GetUserRequest,
) -> Result<User, ThemisError> {
    // Your business logic here
    Ok(User { id: req.user_id, name: "Alice".to_string() })
}
```

## Related Projects

- **[Themis](../themis/)** – Contract validation and code generation
- **[Eunomia](../eunomia/)** – Authorization policy platform
- **[Stoa](../docs/components/stoa-design.md)** – Web UI for service governance

## License

[License to be determined]

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
