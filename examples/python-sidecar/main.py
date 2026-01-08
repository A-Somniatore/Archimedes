"""
Python Example Service with Archimedes Sidecar

This service demonstrates how to build a Python service that works with
the Archimedes sidecar for contract validation, authorization, and observability.
"""

import json
import logging
import uuid
from datetime import datetime
from typing import Optional

from fastapi import FastAPI, HTTPException, Header, Request
from pydantic import BaseModel, EmailStr

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
)
logger = logging.getLogger("example-service")

app = FastAPI(
    title="Example Python Service",
    description="A Python service demonstrating Archimedes sidecar integration",
    version="1.0.0",
)


# =============================================================================
# Models
# =============================================================================


class CallerIdentity(BaseModel):
    """Caller identity extracted from X-Caller-Identity header."""

    type: str  # "spiffe", "user", "api_key", "anonymous"
    id: Optional[str] = None
    # SPIFFE-specific
    trust_domain: Optional[str] = None
    path: Optional[str] = None
    # User-specific
    user_id: Optional[str] = None
    roles: Optional[list[str]] = None
    # API Key-specific
    key_id: Optional[str] = None


class CreateUserRequest(BaseModel):
    """Request body for creating a user."""

    name: str
    email: EmailStr


class UpdateUserRequest(BaseModel):
    """Request body for updating a user."""

    name: Optional[str] = None
    email: Optional[EmailStr] = None


class User(BaseModel):
    """User model."""

    id: str
    name: str
    email: str
    created_at: str


class HealthResponse(BaseModel):
    """Health check response."""

    status: str
    service: str
    timestamp: str


class UsersResponse(BaseModel):
    """List users response."""

    users: list[User]
    total: int


class ErrorResponse(BaseModel):
    """Error response."""

    code: str
    message: str
    request_id: Optional[str] = None


# =============================================================================
# In-Memory Database
# =============================================================================


users_db: dict[str, User] = {
    "1": User(
        id="1",
        name="Alice Smith",
        email="alice@example.com",
        created_at="2026-01-01T00:00:00Z",
    ),
    "2": User(
        id="2",
        name="Bob Johnson",
        email="bob@example.com",
        created_at="2026-01-02T00:00:00Z",
    ),
}


# =============================================================================
# Helper Functions
# =============================================================================


def parse_caller_identity(header_value: Optional[str]) -> Optional[CallerIdentity]:
    """Parse the X-Caller-Identity header from the sidecar."""
    if not header_value:
        return None
    try:
        data = json.loads(header_value)
        return CallerIdentity(**data)
    except (json.JSONDecodeError, ValueError) as e:
        logger.warning(f"Failed to parse caller identity: {e}")
        return None


def get_request_context(
    request: Request,
    x_request_id: Optional[str] = Header(None),
    x_caller_identity: Optional[str] = Header(None),
    x_operation_id: Optional[str] = Header(None),
) -> dict:
    """Extract request context from sidecar-provided headers."""
    caller = parse_caller_identity(x_caller_identity)
    return {
        "request_id": x_request_id or str(uuid.uuid4()),
        "caller": caller,
        "operation_id": x_operation_id,
        "path": request.url.path,
        "method": request.method,
    }


# =============================================================================
# API Endpoints
# =============================================================================


@app.get("/health", response_model=HealthResponse)
async def health_check():
    """Health check endpoint - no authentication required."""
    return HealthResponse(
        status="healthy",
        service="example-python",
        timestamp=datetime.utcnow().isoformat() + "Z",
    )


@app.get("/users", response_model=UsersResponse)
async def list_users(
    request: Request,
    x_request_id: Optional[str] = Header(None),
    x_caller_identity: Optional[str] = Header(None),
    x_operation_id: Optional[str] = Header(None),
):
    """List all users."""
    ctx = get_request_context(request, x_request_id, x_caller_identity, x_operation_id)
    logger.info(f"[{ctx['request_id']}] Listing users, caller: {ctx['caller']}")

    return UsersResponse(
        users=list(users_db.values()),
        total=len(users_db),
    )


@app.get("/users/{user_id}", response_model=User)
async def get_user(
    user_id: str,
    request: Request,
    x_request_id: Optional[str] = Header(None),
    x_caller_identity: Optional[str] = Header(None),
    x_operation_id: Optional[str] = Header(None),
):
    """Get a user by ID."""
    ctx = get_request_context(request, x_request_id, x_caller_identity, x_operation_id)
    logger.info(f"[{ctx['request_id']}] Getting user {user_id}, caller: {ctx['caller']}")

    user = users_db.get(user_id)
    if not user:
        raise HTTPException(
            status_code=404,
            detail=ErrorResponse(
                code="USER_NOT_FOUND",
                message=f"User with ID '{user_id}' not found",
                request_id=ctx["request_id"],
            ).model_dump(),
        )
    return user


@app.post("/users", response_model=User, status_code=201)
async def create_user(
    body: CreateUserRequest,
    request: Request,
    x_request_id: Optional[str] = Header(None),
    x_caller_identity: Optional[str] = Header(None),
    x_operation_id: Optional[str] = Header(None),
):
    """Create a new user."""
    ctx = get_request_context(request, x_request_id, x_caller_identity, x_operation_id)
    logger.info(f"[{ctx['request_id']}] Creating user, caller: {ctx['caller']}")

    # Check for duplicate email
    for user in users_db.values():
        if user.email == body.email:
            raise HTTPException(
                status_code=400,
                detail=ErrorResponse(
                    code="EMAIL_EXISTS",
                    message=f"User with email '{body.email}' already exists",
                    request_id=ctx["request_id"],
                ).model_dump(),
            )

    user_id = str(uuid.uuid4())
    user = User(
        id=user_id,
        name=body.name,
        email=body.email,
        created_at=datetime.utcnow().isoformat() + "Z",
    )
    users_db[user_id] = user

    logger.info(f"[{ctx['request_id']}] Created user {user_id}")
    return user


@app.put("/users/{user_id}", response_model=User)
async def update_user(
    user_id: str,
    body: UpdateUserRequest,
    request: Request,
    x_request_id: Optional[str] = Header(None),
    x_caller_identity: Optional[str] = Header(None),
    x_operation_id: Optional[str] = Header(None),
):
    """Update a user."""
    ctx = get_request_context(request, x_request_id, x_caller_identity, x_operation_id)
    logger.info(f"[{ctx['request_id']}] Updating user {user_id}, caller: {ctx['caller']}")

    user = users_db.get(user_id)
    if not user:
        raise HTTPException(
            status_code=404,
            detail=ErrorResponse(
                code="USER_NOT_FOUND",
                message=f"User with ID '{user_id}' not found",
                request_id=ctx["request_id"],
            ).model_dump(),
        )

    # Update fields
    if body.name is not None:
        user = User(
            id=user.id,
            name=body.name,
            email=user.email,
            created_at=user.created_at,
        )
    if body.email is not None:
        user = User(
            id=user.id,
            name=user.name,
            email=body.email,
            created_at=user.created_at,
        )
    users_db[user_id] = user

    logger.info(f"[{ctx['request_id']}] Updated user {user_id}")
    return user


@app.delete("/users/{user_id}", status_code=204)
async def delete_user(
    user_id: str,
    request: Request,
    x_request_id: Optional[str] = Header(None),
    x_caller_identity: Optional[str] = Header(None),
    x_operation_id: Optional[str] = Header(None),
):
    """Delete a user."""
    ctx = get_request_context(request, x_request_id, x_caller_identity, x_operation_id)
    logger.info(f"[{ctx['request_id']}] Deleting user {user_id}, caller: {ctx['caller']}")

    if user_id not in users_db:
        raise HTTPException(
            status_code=404,
            detail=ErrorResponse(
                code="USER_NOT_FOUND",
                message=f"User with ID '{user_id}' not found",
                request_id=ctx["request_id"],
            ).model_dump(),
        )

    del users_db[user_id]
    logger.info(f"[{ctx['request_id']}] Deleted user {user_id}")


# =============================================================================
# Main Entry Point
# =============================================================================


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(app, host="0.0.0.0", port=3000)
