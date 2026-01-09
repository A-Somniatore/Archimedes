# Archimedes Feature Showcase - OPA Authorization Policy
# ======================================================
# This policy demonstrates how to use OPA for authorization in Archimedes.

package archimedes.authz

import rego.v1

# Default deny all requests
default allow := false

# Allow health check endpoints without authentication
allow if {
    input.path == "/health"
}

allow if {
    input.path == "/ready"
}

# Allow static files without authentication
allow if {
    startswith(input.path, "/static/")
}

# Allow API documentation without authentication
allow if {
    input.path == "/docs"
}

allow if {
    input.path == "/openapi.json"
}

# Allow login endpoint without authentication
allow if {
    input.path == "/api/v1/auth/login"
    input.method == "POST"
}

# Authenticated users can access user endpoints
allow if {
    startswith(input.path, "/api/v1/users")
    is_authenticated
}

# Authenticated users can access file endpoints
allow if {
    startswith(input.path, "/api/v1/files")
    is_authenticated
}

# Authenticated users can access real-time endpoints
allow if {
    startswith(input.path, "/api/v1/realtime")
    is_authenticated
}

# Authenticated users can access auth management endpoints
allow if {
    startswith(input.path, "/api/v1/auth")
    is_authenticated
}

# Admin users can delete any user
allow if {
    input.path == concat("/", ["api", "v1", "users", _])
    input.method == "DELETE"
    is_admin
}

# Helper: Check if user is authenticated
is_authenticated if {
    input.identity != null
    input.identity.user_id != ""
}

# Helper: Check if user is an admin
is_admin if {
    is_authenticated
    input.identity.role == "admin"
}

# Helper: Check if user owns the resource
is_owner if {
    is_authenticated
    path_user_id := split(input.path, "/")[4]
    input.identity.user_id == path_user_id
}

# Rate limiting decision
rate_limit_exceeded if {
    input.rate_limit.current > input.rate_limit.limit
}

# CORS preflight always allowed
allow if {
    input.method == "OPTIONS"
}
