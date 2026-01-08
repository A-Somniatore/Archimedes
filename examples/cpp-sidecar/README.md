# C++ Example Service with Archimedes Sidecar

A C++ service demonstrating integration with the Archimedes sidecar for contract validation, authorization, and observability.

## Overview

This example shows how to build a C++ microservice that:

- Implements a simple User CRUD API using cpp-httplib
- Receives validated requests through the Archimedes sidecar
- Uses caller identity for authorization decisions
- Propagates trace context for distributed tracing

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Pod/Container                           │
│  ┌─────────────────────────────┐  ┌──────────────────────────┐ │
│  │     Archimedes Sidecar      │  │      C++ Service         │ │
│  │         (Port 8005)         │  │      (Port 3000)         │ │
│  │                             │  │                          │ │
│  │  • Contract Validation      │  │  • cpp-httplib Server    │ │
│  │  • mTLS Termination         │──│  • Business Logic        │ │
│  │  • Authorization (OPA)      │  │  • In-Memory Store       │ │
│  │  • Observability            │  │  • User CRUD Operations  │ │
│  └─────────────────────────────┘  └──────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
cpp-sidecar/
├── README.md
├── CMakeLists.txt
├── Dockerfile
├── include/
│   └── json.hpp        # nlohmann/json header-only library
└── src/
    └── main.cpp
```

## Running Locally

### Prerequisites

- C++17 compiler (g++ 9+ or clang++ 10+)
- CMake 3.16+
- Or Docker

### Local Development

```bash
# Create build directory
mkdir build && cd build

# Configure and build
cmake ..
make

# Run
./example-cpp
```

### With Docker

```bash
# Build
docker build -t example-cpp-sidecar .

# Run (standalone)
docker run -p 3000:3000 example-cpp-sidecar
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

## Dependencies

This example uses header-only libraries for simplicity:

- **cpp-httplib**: HTTP server library (fetched via CMake)
- **nlohmann/json**: JSON parsing library (fetched via CMake)

## Configuration

Environment variables:

| Variable | Default   | Description |
| -------- | --------- | ----------- |
| `PORT`   | `3000`    | Server port |
| `HOST`   | `0.0.0.0` | Server host |
