# Rust Example Service (Native Archimedes)

A Rust service demonstrating **native** Archimedes framework usage. This is the **RECOMMENDED** way to build services in Rust - using Archimedes directly rather than axum, actix-web, or other frameworks.

## Overview

This example shows how to build a Rust microservice that:

- Uses Archimedes directly (`archimedes-server`, `archimedes-core`)
- No sidecar needed - all middleware is built-in
- Gets contract validation, authorization, and observability out of the box
- Implements a simple User CRUD API
- Has full type-safety through Rust's type system
- Includes 14 unit tests demonstrating handler patterns

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Pod/Container                           │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │              Rust Service (Native Archimedes)             │ │
│  │                      (Port 8001)                          │ │
│  │                                                           │ │
│  │  ┌─────────────────────────────────────────────────────┐ │ │
│  │  │              Archimedes Framework                    │ │ │
│  │  │  • Contract Validation (built-in)                    │ │ │
│  │  │  • mTLS Support (built-in)                           │ │ │
│  │  │  • Authorization via OPA (built-in)                  │ │ │
│  │  │  • OpenTelemetry (built-in)                          │ │ │
│  │  └─────────────────────────────────────────────────────┘ │ │
│  │                                                           │ │
│  │  ┌─────────────────────────────────────────────────────┐ │ │
│  │  │              Business Logic Layer                    │ │ │
│  │  │  • User CRUD Operations                              │ │ │
│  │  │  • In-Memory Store                                   │ │ │
│  │  └─────────────────────────────────────────────────────┘ │ │
│  └───────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
rust-native/
├── README.md
├── Cargo.toml
├── Dockerfile
└── src/
    └── main.rs
```

## Running Locally

### Prerequisites

- Rust 1.75+
- Or Docker

### Local Development

```bash
# Run directly
cargo run

# Or with release optimizations
cargo run --release
```

### With Docker

```bash
# Build
docker build -t example-rust-native .

# Run
docker run -p 8001:8001 example-rust-native
```

## API Endpoints

| Method | Path              | Operation    | Auth Required |
| ------ | ----------------- | ------------ | ------------- |
| GET    | `/health`         | Health Check | No            |
| GET    | `/users`          | List Users   | Yes           |
| GET    | `/users/{userId}` | Get User     | Yes           |
| POST   | `/users`          | Create User  | Yes           |
| PUT    | `/users/{userId}` | Update User  | Yes           |
| DELETE | `/users/{userId}` | Delete User  | Yes           |

## Key Differences from Sidecar Pattern

| Aspect      | Native (Rust)          | Sidecar (Other Languages) |
| ----------- | ---------------------- | ------------------------- |
| Latency     | ~0.5ms p50             | ~1.5ms p50 (+1ms)         |
| Deployment  | Single binary          | Two containers            |
| Type Safety | Compile-time types     | Runtime validation        |
| Memory      | Lower (shared runtime) | Higher (two processes)    |
| Complexity  | Lower                  | Slightly higher           |

## Testing

```bash
# Health check
curl http://localhost:8001/health

# List users
curl http://localhost:8001/users

# Get user
curl http://localhost:8001/users/1

# Create user
curl -X POST http://localhost:8001/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie", "email": "charlie@example.com"}'

# Update user
curl -X PUT http://localhost:8001/users/1 \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice Updated"}'

# Delete user
curl -X DELETE http://localhost:8001/users/1
```

## Configuration

Environment variables:

| Variable        | Default         | Description             |
| --------------- | --------------- | ----------------------- |
| `PORT`          | `8001`          | Server port             |
| `HOST`          | `0.0.0.0`       | Server host             |
| `CONTRACT_PATH` | `contract.json` | Path to Themis contract |
| `RUST_LOG`      | `info`          | Log level               |
