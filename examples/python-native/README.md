# Python Native Example with Archimedes

This example demonstrates using **native Archimedes Python bindings** instead of FastAPI + sidecar.

## Key Differences from Sidecar Pattern

| Sidecar Pattern | Native Bindings |
|-----------------|-----------------|
| Requires separate sidecar container | Single process |
| FastAPI/Flask for business logic | Archimedes handlers directly |
| Network hop between sidecar ↔ app | In-process calls |
| Parse headers manually | Typed `RequestContext` |
| Two deployments | One deployment |

## Prerequisites

1. **Build the native module**:
   ```bash
   cd crates/archimedes-py
   pip install maturin
   maturin develop
   ```

2. **Or install from wheel**:
   ```bash
   pip install archimedes  # Once published to PyPI
   ```

## Running the Example

```bash
# From this directory
python main.py
```

The server will start on port 8002 with all middleware enabled:
- Request ID generation
- Distributed tracing (OpenTelemetry)
- Identity extraction (mTLS/JWT/API Key)
- Authorization (OPA/Rego policies)
- Request/response validation (Themis contracts)

## API Endpoints

All endpoints are validated against `../contract.json`:

| Operation | Method | Path | Description |
|-----------|--------|------|-------------|
| `healthCheck` | GET | `/health` | Health check |
| `listUsers` | GET | `/users` | List all users |
| `getUser` | GET | `/users/{userId}` | Get user by ID |
| `createUser` | POST | `/users` | Create a user |
| `updateUser` | PUT | `/users/{userId}` | Update a user |
| `deleteUser` | DELETE | `/users/{userId}` | Delete a user |

## Example Requests

```bash
# Health check
curl http://localhost:8002/health

# List users
curl http://localhost:8002/users

# Get user
curl http://localhost:8002/users/1

# Create user
curl -X POST http://localhost:8002/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie Brown", "email": "charlie@example.com"}'

# Update user
curl -X PUT http://localhost:8002/users/1 \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice Updated"}'

# Delete user
curl -X DELETE http://localhost:8002/users/1
```

## Code Comparison

### Before (FastAPI + Sidecar)

```python
from fastapi import FastAPI, Header
from pydantic import BaseModel

app = FastAPI()

class CreateUserRequest(BaseModel):
    name: str
    email: str

@app.post("/users")
async def create_user(
    body: CreateUserRequest,
    x_request_id: str = Header(None),
    x_caller_identity: str = Header(None),
):
    # Parse headers manually
    # No contract validation
    # Authorization already done by sidecar
    ...
```

### After (Native Archimedes)

```python
from archimedes import App, Config, Response

config = Config.from_file("archimedes.yaml")
app = App(config)

@app.handler("createUser")
def create_user(ctx, body):
    # body is already validated against contract
    # ctx.identity is typed
    # Authorization is automatic
    return Response.created({"id": "123", "name": body["name"]})
```

## Configuration

See `archimedes.yaml` for full configuration options:

- **Server**: Listen address and port
- **Contract**: Themis contract path and validation settings
- **Authorization**: OPA policy bundle and defaults
- **Telemetry**: Tracing, metrics, and logging
- **Identity**: mTLS, JWT, and API key settings

## Docker

```bash
# Build
docker build -t example-python-native .

# Run
docker run -p 8002:8002 example-python-native
```

## Testing

```bash
# Install test dependencies
pip install pytest pytest-asyncio

# Run tests
pytest tests/
```
