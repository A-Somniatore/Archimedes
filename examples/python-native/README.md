# Python Native Example with Archimedes

This example demonstrates using **native Archimedes Python bindings** instead of FastAPI + sidecar.

## Key Differences from Sidecar Pattern

| Sidecar Pattern                     | Native Bindings              |
| ----------------------------------- | ---------------------------- |
| Requires separate sidecar container | Single process               |
| FastAPI/Flask for business logic    | Archimedes handlers directly |
| Network hop between sidecar ↔ app   | In-process calls             |
| Parse headers manually              | Typed `RequestContext`       |
| Two deployments                     | One deployment               |

## Prerequisites

**Rust** (latest stable) and **Python 3.8+** are required.

### Quick Start

```bash
# From this directory
cd examples/python-native

# Create and activate virtual environment
python3 -m venv .venv
source .venv/bin/activate  # On Windows: .venv\Scripts\activate

# Install maturin
pip install maturin

# Build and install the archimedes native bindings
maturin develop --manifest-path ../../crates/archimedes-py/Cargo.toml

# Run the example
python main.py
```

### Alternative: Install from Wheel

```bash
# Once published to PyPI
pip install archimedes
```

## Running the Example

```bash
# From this directory (with venv activated)
python main.py
```

**Output:**

```
Starting Archimedes Python server...
Registered operations: ['healthCheck', 'listUsers', 'getUser', 'createUser', 'updateUser', 'deleteUser']
Config: Config(contract_path="../contract.json", listen_port=8002, listen_addr="0.0.0.0")
[archimedes] Binding to 0.0.0.0:8002...
[archimedes] Archimedes Python server listening on http://0.0.0.0:8002
```

The server starts on port 8002 with all middleware enabled:

- Request ID generation
- Distributed tracing (OpenTelemetry)
- Identity extraction (mTLS/JWT/API Key)
- Authorization (OPA/Rego policies)
- Request/response validation (Themis contracts)

## API Endpoints

All endpoints are validated against `../contract.json`:

| Operation     | Method | Path              | Description    |
| ------------- | ------ | ----------------- | -------------- |
| `healthCheck` | GET    | `/health`         | Health check   |
| `listUsers`   | GET    | `/users`          | List all users |
| `getUser`     | GET    | `/users/{userId}` | Get user by ID |
| `createUser`  | POST   | `/users`          | Create a user  |
| `updateUser`  | PUT    | `/users/{userId}` | Update a user  |
| `deleteUser`  | DELETE | `/users/{userId}` | Delete a user  |

## Example Requests

Test the server with `curl`:

```bash
# Health check
curl http://localhost:8002/health
# Response: {"service":"archimedes-python","status":"healthy"}

# List users (pre-seeded data)
curl http://localhost:8002/users | jq .
# Response:
# {
#   "users": [
#     {"id": "1", "name": "Alice Smith", "email": "alice@example.com", ...},
#     {"id": "2", "name": "Bob Johnson", "email": "bob@example.com", ...}
#   ],
#   "total": 2
# }

# Get user by ID
curl http://localhost:8002/users/1 | jq .
# Response: {"id": "1", "name": "Alice Smith", "email": "alice@example.com", ...}

# Create user
curl -X POST http://localhost:8002/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Charlie Brown", "email": "charlie@example.com"}' | jq .
# Response: {"id": "<uuid>", "name": "Charlie Brown", "email": "charlie@example.com", ...}

# Update user
curl -X PUT http://localhost:8002/users/1 \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice Updated"}' | jq .
# Response: {"id": "1", "name": "Alice Updated", "email": "alice@example.com", ...}

# Delete user
curl -X DELETE http://localhost:8002/users/1 -w "\nHTTP Status: %{http_code}\n"
# Response: HTTP Status: 204
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

### Quick Test Script

Run the included test script to validate all endpoints:

```bash
# Make sure server is running first
python main.py &

# Run tests
./test.sh
# Or: bash test.sh

# Stop server when done
pkill -f "python main.py"
```

### Manual Testing

```bash
# Install test dependencies
pip install pytest pytest-asyncio httpx

# Run tests
pytest tests/
```

### Expected Test Results

All 6 operations should work:

- ✅ `healthCheck` - Returns 200 with status "healthy"
- ✅ `listUsers` - Returns 200 with list of users
- ✅ `getUser` - Returns 200 with user data, 404 if not found
- ✅ `createUser` - Returns 201 with created user
- ✅ `updateUser` - Returns 200 with updated user
- ✅ `deleteUser` - Returns 204 on success
