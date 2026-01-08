# Archimedes Multi-Language Examples

This directory contains example services demonstrating how to use Archimedes with different programming languages.

## Overview

Archimedes supports two deployment patterns:

1. **Native (Rust)**: Use the Archimedes framework directly for maximum performance
2. **Sidecar (All Languages)**: Deploy Archimedes as a reverse proxy sidecar for any language

## Services

| Language   | Framework    | Pattern  | Port | Description                        |
| ---------- | ------------ | -------- | ---- | ---------------------------------- |
| Rust       | Archimedes   | Native   | 8001 | Direct framework usage             |
| Python     | FastAPI      | Sidecar  | 8002 | Python web service with sidecar    |
| Go         | net/http     | Sidecar  | 8003 | Go service with sidecar            |
| TypeScript | Express      | Sidecar  | 8004 | Node.js/TypeScript with sidecar    |
| C++        | cpp-httplib  | Sidecar  | 8005 | C++ service with sidecar           |

## Quick Start

### Run All Services with Docker Compose

```bash
cd examples
docker-compose up --build
```

This starts:
- All 5 example services
- An Archimedes sidecar for each non-Rust service
- A Jaeger instance for distributed tracing
- A Prometheus instance for metrics

### Test the Services

Each service exposes the same API:

```bash
# Health check
curl http://localhost:8001/health

# List users
curl http://localhost:8001/users

# Get user by ID
curl http://localhost:8001/users/123

# Create user
curl -X POST http://localhost:8001/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice", "email": "alice@example.com"}'
```

## Architecture

### Sidecar Pattern

```
                    ┌─────────────────────────────────────────┐
                    │              Pod / Container             │
                    │                                         │
    Request         │  ┌─────────────┐    ┌───────────────┐  │
   ─────────────────┼─▶│  Archimedes │───▶│   Your App    │  │
                    │  │   Sidecar   │    │  (Python/Go/  │  │
                    │  │  :8080      │    │   TS/C++)     │  │
                    │  └─────────────┘    └───────────────┘  │
                    │        │                    │          │
                    │        │ Propagated Headers │          │
                    │        │ - X-Request-Id     │          │
                    │        │ - X-Caller-Identity│          │
                    │        │ - traceparent      │          │
                    │        │ - tracestate       │          │
                    │        ▼                    ▼          │
                    │  ┌─────────────────────────────────────┐│
                    │  │        Observability Stack          ││
                    │  │   (Jaeger, Prometheus, etc.)        ││
                    │  └─────────────────────────────────────┘│
                    └─────────────────────────────────────────┘
```

### What the Sidecar Provides

1. **Contract Validation**: Request/response validation against Themis contracts
2. **Authorization**: OPA policy evaluation via Eunomia bundles
3. **Identity Propagation**: Caller identity extracted and forwarded
4. **Observability**: Automatic tracing, metrics, and structured logging
5. **Request ID**: Correlation ID generation and propagation

## Headers Your Service Receives

When using the sidecar, your service receives these headers:

| Header              | Description                                      | Example                                    |
| ------------------- | ------------------------------------------------ | ------------------------------------------ |
| `X-Request-Id`      | Unique request correlation ID                    | `01234567-89ab-cdef-0123-456789abcdef`     |
| `X-Caller-Identity` | JSON-encoded caller identity                     | `{"type":"spiffe","id":"spiffe://..."}` |
| `traceparent`       | W3C Trace Context parent                         | `00-abc123...-def456...-01`                |
| `tracestate`        | W3C Trace Context state                          | `archimedes=...`                           |
| `X-Operation-Id`    | Matched operation from contract (if matched)     | `getUser`                                  |

## Directory Structure

```
examples/
├── README.md              # This file
├── docker-compose.yml     # Run all services
├── contract.json          # Shared Themis contract
├── policy.tar.gz          # Shared OPA policy bundle
│
├── rust-native/           # Native Rust service
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/main.rs
│
├── python-sidecar/        # Python + Sidecar
│   ├── Dockerfile
│   ├── requirements.txt
│   └── main.py
│
├── go-sidecar/            # Go + Sidecar
│   ├── Dockerfile
│   ├── go.mod
│   └── main.go
│
├── typescript-sidecar/    # TypeScript + Sidecar
│   ├── Dockerfile
│   ├── package.json
│   ├── tsconfig.json
│   └── src/index.ts
│
└── cpp-sidecar/           # C++ + Sidecar
    ├── Dockerfile
    ├── CMakeLists.txt
    └── main.cpp
```

## Performance Expectations

| Metric         | Native Rust | With Sidecar |
| -------------- | ----------- | ------------ |
| Latency (p50)  | ~0.5ms      | ~1.5ms       |
| Latency (p99)  | ~2ms        | ~4ms         |
| Throughput     | ~50k rps    | ~30k rps     |

The sidecar adds approximately 1-2ms of latency for the benefits of:
- Language-agnostic deployment
- Zero changes to existing services
- Consistent observability across all services

## Next Steps

1. Choose a language and explore the example
2. Copy the pattern to your own service
3. Deploy with Kubernetes using the provided manifests
4. Monitor with Jaeger and Prometheus
