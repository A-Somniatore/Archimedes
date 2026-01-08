# Go Example Service with Archimedes Sidecar

A Go service demonstrating integration with the Archimedes sidecar for contract validation, authorization, and observability.

## Overview

This example shows how to build a Go microservice that:

- Implements a simple User CRUD API
- Receives validated requests through the Archimedes sidecar
- Uses caller identity for authorization decisions
- Propagates trace context for distributed tracing

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Pod/Container                           │
│  ┌─────────────────────────────┐  ┌──────────────────────────┐ │
│  │     Archimedes Sidecar      │  │      Go Service          │ │
│  │         (Port 8003)         │  │      (Port 3000)         │ │
│  │                             │  │                          │ │
│  │  • Contract Validation      │  │  • Business Logic        │ │
│  │  • mTLS Termination         │──│  • In-Memory Store       │ │
│  │  • Authorization (OPA)      │  │  • User CRUD Operations  │ │
│  │  • Observability            │  │                          │ │
│  └─────────────────────────────┘  └──────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
go-sidecar/
├── README.md
├── go.mod
├── go.sum
├── main.go
└── Dockerfile
```

## Running Locally

### Prerequisites

- Go 1.21+
- Or Docker

### Local Development

```bash
# Run directly
go run main.go

# Or build and run
go build -o example-go .
./example-go
```

### With Docker

```bash
# Build
docker build -t example-go-sidecar .

# Run (standalone)
docker run -p 3000:3000 example-go-sidecar
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

## Headers from Sidecar

The sidecar injects these headers into every request:

| Header              | Description                     |
| ------------------- | ------------------------------- |
| `X-Request-Id`      | Unique request identifier       |
| `X-Caller-Identity` | JSON-encoded caller identity    |
| `X-Operation-Id`    | Matched operation from contract |
| `traceparent`       | W3C Trace Context parent        |
| `tracestate`        | W3C Trace Context state         |

## Testing

```bash
# Health check
curl http://localhost:3000/health

# List users
curl http://localhost:3000/users

# Get user
curl http://localhost:3000/users/1

# Create user
curl -X POST http://localhost:3000/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie", "email": "charlie@example.com"}'

# Update user
curl -X PUT http://localhost:3000/users/1 \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice Updated"}'

# Delete user
curl -X DELETE http://localhost:3000/users/1
```

## Configuration

Environment variables:

| Variable | Default   | Description |
| -------- | --------- | ----------- |
| `PORT`   | `3000`    | Server port |
| `HOST`   | `0.0.0.0` | Server host |
