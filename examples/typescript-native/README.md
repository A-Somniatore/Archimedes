# TypeScript Native Example

This example demonstrates using Archimedes native bindings (`@archimedes/node`) instead of Express or other Node.js frameworks.

## Features

- **Contract-first API**: Automatically validates requests/responses against Themis contract
- **Built-in middleware**: Request ID, tracing, identity extraction, authorization
- **Type-safe handlers**: Full TypeScript support with typed request/response
- **No Express/Fastify**: Uses Archimedes HTTP server directly

## Prerequisites

1. Build the Archimedes Node bindings:
   ```bash
   cd ../../crates/archimedes-node
   npm run build
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

## Running

### Development
```bash
npm run dev
```

### Production
```bash
npm run build
npm start
```

## API Endpoints

| Method | Path           | Operation    | Description       |
|--------|----------------|--------------|-------------------|
| GET    | /health        | healthCheck  | Health check      |
| GET    | /users         | listUsers    | List all users    |
| GET    | /users/:userId | getUser      | Get user by ID    |
| POST   | /users         | createUser   | Create new user   |
| PUT    | /users/:userId | updateUser   | Update user       |
| DELETE | /users/:userId | deleteUser   | Delete user       |

## Example Requests

### Health Check
```bash
curl http://localhost:8004/health
```

### List Users
```bash
curl http://localhost:8004/users
```

### Get User
```bash
curl http://localhost:8004/users/1
```

### Create User
```bash
curl -X POST http://localhost:8004/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie", "email": "charlie@example.com"}'
```

### Update User
```bash
curl -X PUT http://localhost:8004/users/1 \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice Updated"}'
```

### Delete User
```bash
curl -X DELETE http://localhost:8004/users/1
```

## Comparison: Express vs Archimedes Native

### Express (sidecar pattern)
```typescript
import express from 'express';

const app = express();

// No validation - must implement manually
app.get('/users/:userId', (req, res) => {
  // Parse sidecar headers manually
  const requestId = req.headers['x-request-id'];
  const caller = JSON.parse(req.headers['x-caller-identity'] || '{}');
  
  // Business logic
  const user = getUser(req.params.userId);
  res.json(user);
});

app.listen(3000);
```

### Archimedes Native
```typescript
import { Archimedes, Request, Response } from '@archimedes/node';

const app = new Archimedes({ contractPath: 'contract.json' });

// Validation automatic from contract
app.operation('getUser', async (req: Request): Promise<Response> => {
  // request.requestId, request.caller already available
  // request body already validated against contract schema
  const user = getUser(req.pathParams.userId);
  return Response.json(user);
});

app.listen(8004);
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    @archimedes/node                              │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    NAPI-RS Bridge                          │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │              Archimedes Core (Rust)                        │  │
│  │  • HTTP Server (Hyper)                                     │  │
│  │  • Router (Radix Tree)                                     │  │
│  │  • Middleware Pipeline                                     │  │
│  │  • Contract Validation (Sentinel)                          │  │
│  │  • Authorization (OPA/Eunomia)                             │  │
│  │  • Telemetry (OpenTelemetry)                               │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## Docker

```bash
docker build -t typescript-native-example .
docker run -p 8004:8004 typescript-native-example
```

## Tests

```bash
npm test
```

## License

Apache-2.0
