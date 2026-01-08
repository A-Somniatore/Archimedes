# Go Native Example

This example demonstrates using Archimedes native bindings for Go instead of net/http, Gin, Chi, or other Go web frameworks.

## Features

- **Contract-first API**: Automatically validates requests/responses against Themis contract
- **Built-in middleware**: Request ID, tracing, identity extraction, authorization
- **Familiar API**: Similar to Gin/Echo context-based handlers
- **No net/http boilerplate**: Uses Archimedes HTTP server directly via cgo

## Prerequisites

1. Build the Archimedes FFI library:
   ```bash
   cd ../..
   cargo build --release -p archimedes-ffi
   ```

2. The library will be at `target/release/libarchimedes_ffi.so` (Linux) or `.dylib` (macOS)

## Building

```bash
# Set library path for linking
export CGO_LDFLAGS="-L../../target/release"
export CGO_CFLAGS="-I../../target/include"

# Build
go build -o go-native-example .
```

## Running

```bash
# Set library path for runtime
export LD_LIBRARY_PATH=../../target/release:$LD_LIBRARY_PATH  # Linux
export DYLD_LIBRARY_PATH=../../target/release:$DYLD_LIBRARY_PATH  # macOS

./go-native-example
```

Or run directly:
```bash
go run .
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
curl http://localhost:8003/health
```

### List Users
```bash
curl http://localhost:8003/users
```

### Get User
```bash
curl http://localhost:8003/users/1
```

### Create User
```bash
curl -X POST http://localhost:8003/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie", "email": "charlie@example.com"}'
```

### Update User
```bash
curl -X PUT http://localhost:8003/users/1 \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice Updated"}'
```

### Delete User
```bash
curl -X DELETE http://localhost:8003/users/1
```

## Comparison: net/http vs Archimedes Native

### net/http (sidecar pattern)
```go
func main() {
    http.HandleFunc("/users/", func(w http.ResponseWriter, r *http.Request) {
        // Parse sidecar headers manually
        requestID := r.Header.Get("X-Request-Id")
        callerJSON := r.Header.Get("X-Caller-Identity")
        
        // Parse path manually
        userID := strings.TrimPrefix(r.URL.Path, "/users/")
        
        // Business logic
        user := getUser(userID)
        json.NewEncoder(w).Encode(user)
    })
    http.ListenAndServe(":3000", nil)
}
```

### Archimedes Native
```go
func main() {
    app, _ := archimedes.New(archimedes.Config{
        Contract: "contract.json",
    })

    // Validation automatic from contract
    app.Operation("getUser", func(ctx *archimedes.Context) error {
        // ctx.RequestID, ctx.Caller already available
        // ctx.Body() already validated against contract schema
        user := getUser(ctx.PathParam("userId"))
        return ctx.JSON(200, user)
    })

    app.Run(":8003")
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    archimedes-go (cgo)                           │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    CGO Bridge                              │  │
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
docker build -t go-native-example .
docker run -p 8003:8003 go-native-example
```

## Tests

```bash
go test ./...
```

## Static Linking (Optional)

For deployments without cgo runtime dependency:

```bash
# Build static binary (requires musl on Linux)
CGO_ENABLED=1 go build -ldflags="-linkmode external -extldflags -static" -o go-native-example .
```

## License

Apache-2.0
