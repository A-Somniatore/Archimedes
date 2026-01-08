"""
Python Native Example Service using Archimedes

This service demonstrates how to build a Python service using native Archimedes
bindings (PyO3), replacing the need for FastAPI + sidecar pattern.

All middleware (validation, authorization, telemetry) is handled automatically
by Archimedes - the developer only writes business logic.
"""

import uuid
from datetime import datetime, timezone
from typing import Optional

# Import from archimedes native bindings
from archimedes import App, Config, Response, RequestContext


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
    # Run the Archimedes server
    # All middleware is automatically applied:
    # - Request ID generation
    # - Trace context propagation
    # - Identity extraction
    # - Authorization (OPA)
    # - Request/response validation
    # - Telemetry emission
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
