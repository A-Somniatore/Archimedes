# TypeScript Example Service with Archimedes Sidecar

A TypeScript/Express service demonstrating integration with the Archimedes sidecar for contract validation, authorization, and observability.

## Overview

This example shows how to build a TypeScript microservice that:
- Implements a simple User CRUD API with Express
- Receives validated requests through the Archimedes sidecar
- Uses caller identity for authorization decisions
- Propagates trace context for distributed tracing

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Pod/Container                           │
│  ┌─────────────────────────────┐  ┌──────────────────────────┐ │
│  │     Archimedes Sidecar      │  │   TypeScript Service     │ │
│  │         (Port 8004)         │  │      (Port 3000)         │ │
│  │                             │  │                          │ │
│  │  • Contract Validation      │  │  • Express App           │ │
│  │  • mTLS Termination         │──│  • Business Logic        │ │
│  │  • Authorization (OPA)      │  │  • In-Memory Store       │ │
│  │  • Observability            │  │  • User CRUD Operations  │ │
│  └─────────────────────────────┘  └──────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
typescript-sidecar/
├── README.md
├── package.json
├── tsconfig.json
├── Dockerfile
└── src/
    └── index.ts
```

## Running Locally

### Prerequisites

- Node.js 20+
- npm or yarn
- Or Docker

### Local Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run dev

# Build and run in production mode
npm run build
npm start
```

### With Docker

```bash
# Build
docker build -t example-typescript-sidecar .

# Run (standalone)
docker run -p 3000:3000 example-typescript-sidecar
```

## API Endpoints

| Method | Path              | Operation       | Auth Required |
|--------|-------------------|-----------------|---------------|
| GET    | `/health`         | Health Check    | No            |
| GET    | `/users`          | List Users      | Yes           |
| GET    | `/users/:userId`  | Get User        | Yes           |
| POST   | `/users`          | Create User     | Yes           |
| PUT    | `/users/:userId`  | Update User     | Yes           |
| DELETE | `/users/:userId`  | Delete User     | Yes           |

## Headers from Sidecar

The sidecar injects these headers into every request:

| Header              | Description                        |
|---------------------|------------------------------------|
| `X-Request-Id`      | Unique request identifier          |
| `X-Caller-Identity` | JSON-encoded caller identity       |
| `X-Operation-Id`    | Matched operation from contract    |
| `traceparent`       | W3C Trace Context parent           |
| `tracestate`        | W3C Trace Context state            |

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

| Variable | Default   | Description       |
|----------|-----------|-------------------|
| `PORT`   | `3000`    | Server port       |
| `HOST`   | `0.0.0.0` | Server host       |
