# Archimedes Feature Showcase

> **⚠️ API REFERENCE DOCUMENT**: This example represents the **target API design** for
> Archimedes. Not all features shown here are fully implemented yet. This serves as
> the reference for what bindings should implement as features are added.

This example demonstrates **ALL** features available in Archimedes. It serves as:

1. **Reference Implementation** - Shows every feature in action
2. **Binding Parity Guide** - Other language bindings should implement the same features
3. **Testing Template** - Comprehensive test coverage patterns

## Features Demonstrated

### 1. Core Features

- [x] HTTP Server with HTTP/1.1 and HTTP/2
- [x] Graceful shutdown
- [x] Health probes (`/health`, `/ready`)
- [x] Operation-based routing
- [x] Sub-router nesting (`nest()`)
- [x] Route prefixes (`prefix()`)
- [x] Route merging (`merge()`)
- [x] OpenAPI tags (`tag()`)

### 2. Extractors

- [x] `Json<T>` - JSON body deserialization
- [x] `Form<T>` - URL-encoded form data
- [x] `Path<T>` - Path parameters
- [x] `Query<T>` - Query string parameters
- [x] `Headers` - HTTP headers access
- [x] `Cookies` - Cookie values
- [x] `Multipart` - File uploads
- [x] `Inject<T>` - DI container injection
- [x] `State<T>` - Shared application state

### 3. Response Builders

- [x] `Response::json()` - JSON responses
- [x] `Response::text()` - Plain text
- [x] `Response::html()` - HTML responses
- [x] `Response::redirect()` - HTTP redirects
- [x] `Response::no_content()` - 204 responses
- [x] `FileResponse` - File downloads
- [x] `SetCookie` - Cookie setting

### 4. Middleware

- [x] Fixed middleware pipeline (9 stages)
- [x] CORS middleware
- [x] Rate limiting
- [x] Compression (gzip/brotli)

### 5. Contract Integration

- [x] Contract loading from file
- [x] Request validation
- [x] Response validation
- [x] Monitor mode

### 6. Authorization

- [x] OPA policy evaluation
- [x] RBAC support
- [x] Decision caching

### 7. Telemetry

- [x] Prometheus metrics
- [x] OpenTelemetry tracing
- [x] Structured JSON logging

### 8. Real-Time Communication

- [x] WebSocket connections
- [x] Server-Sent Events (SSE)
- [x] Connection management
- [x] Broadcasting

### 9. Background Processing

- [x] Task spawning
- [x] Job scheduling (cron)
- [x] Graceful task shutdown

### 10. Server Features

- [x] Lifecycle hooks (on_startup/on_shutdown)
- [x] Static file serving
- [x] Configuration management
- [x] Hot reload

### 11. Testing

- [x] TestClient usage
- [x] Request builders
- [x] Response assertions

## Project Structure

```
feature-showcase/
├── Cargo.toml              # Dependencies
├── README.md               # This file
├── archimedes.toml         # Configuration
├── contract.json           # API contract
├── policies/               # OPA policies
│   └── authz.rego
├── static/                 # Static files
│   └── index.html
└── src/
    ├── main.rs             # Entry point
    ├── config.rs           # Configuration
    ├── routes/             # Route modules (sub-routers)
    │   ├── mod.rs
    │   ├── users.rs        # User CRUD operations
    │   ├── files.rs        # File upload/download
    │   ├── auth.rs         # Authentication examples
    │   └── realtime.rs     # WebSocket & SSE
    ├── middleware.rs       # Custom middleware examples
    ├── tasks.rs            # Background tasks
    └── tests.rs            # Test examples
```

## Running the Example

```bash
# From the archimedes root directory
cargo run -p feature-showcase

# With environment variables
ARCHIMEDES_LOG_LEVEL=debug cargo run -p feature-showcase
```

## Endpoints

| Method | Path                          | Description             |
| ------ | ----------------------------- | ----------------------- |
| GET    | `/health`                     | Health check            |
| GET    | `/ready`                      | Readiness check         |
| GET    | `/api/v1/users`               | List users              |
| POST   | `/api/v1/users`               | Create user (JSON body) |
| GET    | `/api/v1/users/{id}`          | Get user by ID          |
| PUT    | `/api/v1/users/{id}`          | Update user             |
| DELETE | `/api/v1/users/{id}`          | Delete user             |
| POST   | `/api/v1/files/upload`        | Upload file (multipart) |
| GET    | `/api/v1/files/{id}/download` | Download file           |
| POST   | `/api/v1/auth/login`          | Login (form data)       |
| GET    | `/api/v1/auth/session`        | Get session (cookies)   |
| GET    | `/api/v1/realtime/events`     | SSE event stream        |
| WS     | `/api/v1/realtime/ws`         | WebSocket connection    |
| GET    | `/docs`                       | Swagger UI              |
| GET    | `/redoc`                      | ReDoc documentation     |

## For Binding Implementers

When implementing bindings for Python, TypeScript, C++, or Go:

1. **Use this example as your reference** - Every feature shown here should work in your binding
2. **Match the API style** - Keep decorator/attribute patterns similar
3. **Test against this contract** - Use the same `contract.json` for validation testing
4. **Follow the same module structure** - Helps with documentation and onboarding

### Feature Checklist for Bindings

Copy this checklist to track your binding implementation:

```markdown
## Python/TypeScript/C++/Go Binding Parity

### Core

- [ ] HTTP Server
- [ ] Graceful shutdown
- [ ] Health probes
- [ ] Sub-routers/Blueprints

### Extractors

- [ ] JSON body
- [ ] Form data
- [ ] Path params
- [ ] Query params
- [ ] Headers
- [ ] Cookies
- [ ] Multipart uploads
- [ ] DI injection

### Responses

- [ ] JSON response
- [ ] Text response
- [ ] HTML response
- [ ] Redirect
- [ ] No content
- [ ] File download
- [ ] Set cookie

### Middleware

- [ ] Fixed pipeline
- [ ] CORS
- [ ] Rate limiting
- [ ] Compression

### Features

- [ ] Contract validation
- [ ] OPA authorization
- [ ] Telemetry
- [ ] WebSocket
- [ ] SSE
- [ ] Background tasks
- [ ] Job scheduler
- [ ] Static files
- [ ] Lifecycle hooks
- [ ] Hot reload config
- [ ] TestClient
```
