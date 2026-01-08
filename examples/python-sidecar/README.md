# Python Example Service with Archimedes Sidecar

A simple FastAPI service demonstrating the Archimedes sidecar pattern.

## Running Locally (Development)

```bash
# Create virtual environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate

# Install dependencies
pip install -r requirements.txt

# Run the service
uvicorn main:app --host 0.0.0.0 --port 3000
```

## Running with Docker

```bash
docker build -t example-python .
docker run -p 3000:3000 example-python
```

## Running with Sidecar (Production)

See the parent `docker-compose.yml` for the full sidecar configuration.

## Headers from Sidecar

When running behind the Archimedes sidecar, your service receives these headers:

- `X-Request-Id`: Unique correlation ID for the request
- `X-Caller-Identity`: JSON-encoded caller identity
- `traceparent`: W3C Trace Context parent
- `X-Operation-Id`: Matched operation from contract

## API Endpoints

- `GET /health` - Health check
- `GET /users` - List all users
- `GET /users/{id}` - Get user by ID
- `POST /users` - Create a new user
- `PUT /users/{id}` - Update a user
- `DELETE /users/{id}` - Delete a user
