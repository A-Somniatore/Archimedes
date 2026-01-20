# Archimedes Multi-Language Examples

This directory contains example services demonstrating how to use Archimedes with different programming languages.

## Overview

Archimedes supports two deployment patterns:

1. **Native Bindings (Recommended)**: Use language-specific bindings for in-process middleware with full Rust performance
2. **Sidecar (Legacy)**: Deploy Archimedes as a reverse proxy sidecar for gradual migration

## Native Binding Services (Recommended)

All native bindings provide the same middleware pipeline as Rust, with no network hop overhead.

| Language       | Package           | Directory            | Port | Status      |
| -------------- | ----------------- | -------------------- | ---- | ----------- |
| **Rust**       | archimedes        | `rust-native/`       | 8001 | ✅ Complete |
| **Python**     | archimedes (PyPI) | `python-native/`     | 8002 | ✅ Complete |
| **Go**         | archimedes-go     | `go-native/`         | 8003 | ✅ Complete |
| **TypeScript** | @archimedes/node  | `typescript-native/` | 8004 | ✅ Complete |
| **C++**        | libarchimedes     | `cpp-native/`        | 8005 | ✅ Complete |

## Sidecar Services (Legacy/Migration)

The sidecar pattern is available for gradual migration from existing frameworks.

| Language       | Directory            | Port | Description                |
| -------------- | -------------------- | ---- | -------------------------- |
| Python         | `python-sidecar/`    | -    | FastAPI + sidecar          |
| Go             | `go-sidecar/`        | -    | net/http + sidecar         |
| TypeScript     | `typescript-sidecar/`| -    | Express + sidecar          |
| C++            | `cpp-sidecar/`       | -    | cpp-httplib + sidecar      |

## Feature Showcase

The `feature-showcase/` directory contains a comprehensive Rust example demonstrating ALL Archimedes features. Use this as the reference implementation.

## Quick Start

### Run All Native Services with Docker Compose

```bash
cd examples
docker-compose up --build
```

This starts:

- All 5 native example services (Rust, Python, Go, TypeScript, C++)
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

### Native Bindings (Recommended)

```
    Request         ┌─────────────────────────────────────────┐
   ─────────────────▶│          Your Application               │
                    │                                         │
                    │  ┌─────────────────────────────────────┐│
                    │  │      Archimedes Native Binding      ││
                    │  │                                     ││
                    │  │  • Request ID generation            ││
                    │  │  • Identity extraction              ││
                    │  │  • Authorization (OPA)              ││
                    │  │  • Contract validation              ││
                    │  │  • Telemetry (traces, metrics)      ││
                    │  │  • Error normalization              ││
                    │  └─────────────────────────────────────┘│
                    │                                         │
                    │  ┌─────────────────────────────────────┐│
                    │  │        Your Business Logic          ││
                    │  │     (handlers, services, etc.)      ││
                    │  └─────────────────────────────────────┘│
                    └─────────────────────────────────────────┘
```

### Sidecar Pattern (Legacy)

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

## What Archimedes Provides

All binding languages get the same features:

1. **Contract Validation**: Request/response validation against Themis contracts
2. **Authorization**: OPA policy evaluation via Eunomia bundles
3. **Identity Extraction**: Caller identity from mTLS/JWT/API keys
4. **Observability**: Automatic tracing, metrics, and structured logging
5. **Request ID**: Correlation ID generation and propagation
6. **CORS**: Cross-origin resource sharing middleware
7. **Rate Limiting**: Per-IP/user/key rate limiting
8. **Compression**: gzip, brotli, deflate, zstd
9. **Static Files**: Serve static assets with caching
10. **TestClient**: In-memory testing without network

## Directory Structure

```
examples/
├── README.md              # This file
├── docker-compose.yml     # Run all services
├── contract.json          # Shared Themis contract
│
├── feature-showcase/      # Reference implementation (all features)
│   ├── Cargo.toml
│   ├── README.md
│   └── src/main.rs
│
├── rust-native/           # Native Rust service
│   ├── Cargo.toml
│   ├── Dockerfile
│   └── src/main.rs
│
├── python-native/         # Python with Native Bindings
│   ├── Dockerfile
│   ├── requirements.txt
│   ├── archimedes.yaml
│   └── main.py
│
├── go-native/             # Go with Native Bindings
│   ├── Dockerfile
│   ├── go.mod
│   ├── main.go
│   └── archimedes/        # Go bindings package
│
├── typescript-native/     # TypeScript with Native Bindings
│   ├── Dockerfile
│   ├── package.json
│   ├── tsconfig.json
│   └── src/index.ts
│
├── cpp-native/            # C++ with Native Bindings
│   ├── Dockerfile
│   ├── CMakeLists.txt
│   └── src/main.cpp
│
└── *-sidecar/             # Legacy sidecar examples
```

## Performance Comparison

| Pattern         | Latency (p50) | Latency (p99) | Throughput |
| --------------- | ------------- | ------------- | ---------- |
| Native Rust     | ~0.5ms        | ~2ms          | ~50k rps   |
| Native Bindings | ~0.8ms        | ~3ms          | ~40k rps   |
| Sidecar         | ~1.5ms        | ~5ms          | ~30k rps   |

Native bindings add minimal overhead (~0.3ms) compared to pure Rust, while the sidecar adds ~1-2ms network latency.

## Migration Guide

### From FastAPI to archimedes-py

```python
# Before (FastAPI)
from fastapi import FastAPI
app = FastAPI()

@app.get("/users")
async def list_users():
    return {"users": []}

# After (Archimedes)
from archimedes import Archimedes
app = Archimedes(contract="contract.json")

@app.operation("listUsers")
async def list_users(request):
    return Response.json({"users": []})
```

### From Express to @archimedes/node

```typescript
// Before (Express)
import express from 'express';
const app = express();

app.get('/users', (req, res) => {
  res.json({ users: [] });
});

// After (Archimedes)
import { Archimedes } from '@archimedes/node';
const app = new Archimedes({ contract: 'contract.json' });

app.operation('listUsers', async (request) => {
  return Response.json({ users: [] });
});
```

## Next Steps

1. Choose a language and explore the native example
2. Copy the pattern to your own service
3. Deploy with Kubernetes using the provided manifests
4. Monitor with Jaeger and Prometheus
