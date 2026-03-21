# Users API Authorization Policy
# 
# This policy controls access to the Users API based on:
# - Caller identity type (user, service, api_key)
# - User roles (admin, user)
# - Operation being performed
#
# Policy Input Structure (from themis_platform_types::PolicyInput):
# {
#   "caller": {
#     "type": "user" | "spiffe" | "api_key" | "anonymous",
#     "user_id": "...",
#     "roles": ["admin", "user"],
#     ...
#   },
#   "service": "users-api",
#   "operation_id": "listUsers" | "getUser" | "createUser" | ...,
#   "method": "GET" | "POST" | ...,
#   "path": "/users" | "/users/{userId}",
#   "request_id": "...",
#   "timestamp": "...",
#   "context": {...}
# }

package authz.users

import rego.v1

# Default deny
default allow := false

# Allow if the request passes authorization
allow if {
    some rule in rules
    rule
}

# ============================================================================
# RBAC Rules
# ============================================================================

# Rule 1: Admins can do anything
rules contains true if {
    is_admin
}

# Rule 2: Regular users can list users
rules contains true if {
    is_authenticated_user
    input.operation_id == "listUsers"
}

# Rule 3: Regular users can get their own user details
rules contains true if {
    is_authenticated_user
    input.operation_id == "getUser"
    # For self-access, check if the userId in path matches caller
    is_own_resource
}

# Rule 4: Regular users can update their own profile
rules contains true if {
    is_authenticated_user
    input.operation_id == "updateUser"
    is_own_resource
}

# Rule 5: Services can perform any read operation
rules contains true if {
    is_service
    input.method == "GET"
}

# Rule 6: API keys with "users:read" scope can read
rules contains true if {
    is_api_key
    has_scope("users:read")
    input.method == "GET"
}

# Rule 7: API keys with "users:write" scope can write
rules contains true if {
    is_api_key
    has_scope("users:write")
    input.method in ["POST", "PUT", "DELETE"]
}

# ============================================================================
# Helper Functions
# ============================================================================

# Check if caller is authenticated (not anonymous)
is_authenticated if {
    input.caller.type != "anonymous"
}

# Check if caller is a user
is_authenticated_user if {
    input.caller.type == "user"
}

# Check if caller is an admin
is_admin if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

# Check if caller is a service (SPIFFE)
is_service if {
    input.caller.type == "spiffe"
}

# Check if caller is using an API key
is_api_key if {
    input.caller.type == "api_key"
}

# Check if the API key has a specific scope
has_scope(scope) if {
    input.caller.type == "api_key"
    scope in input.caller.scopes
}

# Check if the resource being accessed belongs to the caller
# This looks for user_id in context (set by the server from path params)
is_own_resource if {
    input.caller.type == "user"
    input.context.user_id == input.caller.user_id
}

# ============================================================================
# Denial Reasons (for audit/logging)
# ============================================================================

# Collect denial reasons
reasons contains "anonymous access not allowed" if {
    input.caller.type == "anonymous"
}

reasons contains "insufficient permissions" if {
    is_authenticated
    not allow
}

reasons contains msg if {
    is_authenticated_user
    not is_admin
    input.operation_id in ["createUser", "deleteUser"]
    msg := sprintf("operation '%s' requires admin role", [input.operation_id])
}

# Return the first denial reason
denial_reason := reasons[_] if {
    not allow
    count(reasons) > 0
}

denial_reason := "access denied" if {
    not allow
    count(reasons) == 0
}
