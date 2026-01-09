"""
Python Native Example Service using Archimedes

This service demonstrates how to build a Python service using native Archimedes
bindings (PyO3), replacing the need for FastAPI + sidecar pattern.

All middleware (validation, authorization, telemetry) is handled automatically
by Archimedes - the developer only writes business logic.

Features demonstrated:
- Handler registration with decorators
- Sub-routers with Router.prefix() and Router.tag()
- Lifecycle hooks with @app.on_startup and @app.on_shutdown
- Request context and path parameters
- Response helpers (ok, created, not_found, etc.)
"""

import uuid
from datetime import datetime, timezone
from typing import Optional

# Import from archimedes native bindings
from archimedes import App, Config, Response, RequestContext, Router


# =============================================================================
# In-Memory Database
# =============================================================================

users_db: dict[str, dict] = {
    "1": {
        "id": "1",
        "name": "Alice Smith",
        "email": "alice@example.com",
        "created_at": "2026-01-01T00:00:00Z",
    },
    "2": {
        "id": "2",
        "name": "Bob Johnson",
        "email": "bob@example.com",
        "created_at": "2026-01-02T00:00:00Z",
    },
}


# =============================================================================
# Application Setup
# =============================================================================

# Create configuration from file or environment
# In development, you can also create config directly:
#   config = Config(contract_path="../contract.json", listen_port=8002)
config = Config.from_file("archimedes.yaml")
app = App(config)


# =============================================================================
# Lifecycle Hooks (NEW in Phase A15.1)
# =============================================================================


@app.on_startup("database_connect")
async def connect_database():
    """Initialize database connection on startup.
    
    Startup hooks run before the server accepts connections.
    They run in registration order.
    """
    print("[startup] Connecting to database...")
    # In a real app: await db.connect()
    print("[startup] Database connected!")


@app.on_startup("cache_warmup")
async def warmup_cache():
    """Warm up caches on startup."""
    print("[startup] Warming up caches...")
    # In a real app: await cache.warmup()
    print("[startup] Caches warmed!")


@app.on_shutdown("database_disconnect")
async def disconnect_database():
    """Close database connection on shutdown.
    
    Shutdown hooks run after server stops accepting connections.
    They run in reverse order (LIFO).
    """
    print("[shutdown] Disconnecting from database...")
    # In a real app: await db.close()
    print("[shutdown] Database disconnected!")


@app.on_shutdown("flush_metrics")
async def flush_metrics():
    """Flush metrics before shutdown."""
    print("[shutdown] Flushing metrics...")
    # In a real app: await metrics.flush()
    print("[shutdown] Metrics flushed!")


# =============================================================================
# Sub-Router Example (NEW in Phase A15.1)
# =============================================================================

# Create a users router with prefix and tags
users_router = Router().prefix("/users").tag("users").tag("api")


# =============================================================================
# Handlers
# =============================================================================


@app.handler("healthCheck")
def health_check(ctx: RequestContext) -> Response:
    """Health check endpoint.
    
    No authentication required - this is handled by contract configuration.
    """
    return Response.ok({
        "status": "healthy",
        "service": "example-python-native",
        "timestamp": datetime.now(timezone.utc).isoformat(),
    })


@app.handler("listUsers")
def list_users(ctx: RequestContext) -> Response:
    """List all users.
    
    Authorization is handled automatically by Archimedes middleware.
    The handler only contains business logic.
    """
    # ctx.identity provides caller information (already validated)
    # ctx.trace_id for distributed tracing correlation
    
    return Response.ok({
        "users": list(users_db.values()),
        "total": len(users_db),
    })


@app.handler("getUser")
def get_user(ctx: RequestContext) -> Response:
    """Get a user by ID.
    
    Path parameters are extracted automatically and validated
    against the contract schema.
    """
    user_id = ctx.path_params["userId"]
    
    user = users_db.get(user_id)
    if not user:
        return Response.not_found(f"User with ID '{user_id}' not found")
    
    return Response.ok(user)


@app.handler("createUser")
def create_user(ctx: RequestContext, body: dict) -> Response:
    """Create a new user.
    
    Request body is already validated against the contract schema.
    No need for Pydantic or manual validation.
    """
    # Check for duplicate email
    for user in users_db.values():
        if user["email"] == body["email"]:
            return Response.bad_request(f"User with email '{body['email']}' already exists")
    
    user_id = str(uuid.uuid4())
    user = {
        "id": user_id,
        "name": body["name"],
        "email": body["email"],
        "created_at": datetime.now(timezone.utc).isoformat(),
    }
    users_db[user_id] = user
    
    return Response.created(user)


@app.handler("updateUser")
def update_user(ctx: RequestContext, body: dict) -> Response:
    """Update a user.
    
    Partial updates supported - only provided fields are updated.
    """
    user_id = ctx.path_params["userId"]
    
    user = users_db.get(user_id)
    if not user:
        return Response.not_found(f"User with ID '{user_id}' not found")
    
    # Update only provided fields
    if "name" in body and body["name"] is not None:
        user["name"] = body["name"]
    if "email" in body and body["email"] is not None:
        user["email"] = body["email"]
    
    users_db[user_id] = user
    return Response.ok(user)


@app.handler("deleteUser")
def delete_user(ctx: RequestContext) -> Response:
    """Delete a user.
    
    Returns 204 No Content on success.
    """
    user_id = ctx.path_params["userId"]
    
    if user_id not in users_db:
        return Response.not_found(f"User with ID '{user_id}' not found")
    
    del users_db[user_id]
    return Response.no_content()


# =============================================================================
# Main Entry Point
# =============================================================================

if __name__ == "__main__":
    # Demonstrate router composition (NEW in Phase A15.1)
    # The users_router handlers are merged into the app
    app.merge(users_router)
    
    # Run the Archimedes server
    # All middleware is automatically applied:
    # - Request ID generation
    # - Trace context propagation
    # - Identity extraction
    # - Authorization (OPA)
    # - Request/response validation
    # - Telemetry emission
    #
    # Lifecycle hooks will run:
    # - on_startup hooks run before server accepts connections
    # - on_shutdown hooks run after server stops
    print(f"Starting Archimedes Python server...")
    print(f"Registered operations: {app.operation_ids()}")
    print(f"Config: {config}")
    
    # NOTE: The HTTP server integration is currently a placeholder.
    # Once archimedes-py is wired to archimedes-server, this will
    # start an actual HTTP server on the configured port.
    #
    # For now, this demonstrates the API structure and handler registration.
    # The handlers ARE registered and can be invoked programmatically.
    
    try:
        app.run()
    except Exception as e:
        print(f"Server stopped: {e}")
