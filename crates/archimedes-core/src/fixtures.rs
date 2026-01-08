//! Test fixtures for Archimedes development and testing.
//!
//! This module provides pre-built mock contracts and operations that can be used
//! in tests across the Archimedes codebase.
//!
//! # Example
//!
//! ```
//! use archimedes_core::fixtures;
//!
//! // Get a pre-built user service contract
//! let contract = fixtures::user_service_contract();
//!
//! // Use it in tests
//! assert!(contract.get_operation("getUser").is_some());
//! ```

use crate::contract::{Contract, MockSchema, Operation};
use http::Method;

/// Creates a mock user service contract for testing.
///
/// This contract defines common user management operations:
/// - `getUser` - GET /users/{userId}
/// - `listUsers` - GET /users
/// - `createUser` - POST /users
/// - `updateUser` - PUT /users/{userId}
/// - `deleteUser` - DELETE /users/{userId}
///
/// # Example
///
/// ```
/// use archimedes_core::fixtures::user_service_contract;
///
/// let contract = user_service_contract();
/// assert_eq!(contract.name(), "user-service");
/// assert_eq!(contract.operations().len(), 5);
/// ```
#[must_use]
pub fn user_service_contract() -> Contract {
    Contract::builder("user-service")
        .version("1.0.0")
        .operation(
            Operation::builder("getUser")
                .method(Method::GET)
                .path("/users/{userId}")
                .description("Retrieves a user by ID")
                .tag("users")
                .response_schema(user_schema())
                .build(),
        )
        .operation(
            Operation::builder("listUsers")
                .method(Method::GET)
                .path("/users")
                .description("Lists all users with pagination")
                .tag("users")
                .response_schema(MockSchema::object(vec![
                    ("users", MockSchema::array(user_schema())),
                    ("total", MockSchema::integer()),
                    ("page", MockSchema::integer()),
                    ("pageSize", MockSchema::integer()),
                ]))
                .build(),
        )
        .operation(
            Operation::builder("createUser")
                .method(Method::POST)
                .path("/users")
                .description("Creates a new user")
                .tag("users")
                .request_schema(create_user_schema())
                .response_schema(user_schema())
                .build(),
        )
        .operation(
            Operation::builder("updateUser")
                .method(Method::PUT)
                .path("/users/{userId}")
                .description("Updates an existing user")
                .tag("users")
                .request_schema(update_user_schema())
                .response_schema(user_schema())
                .build(),
        )
        .operation(
            Operation::builder("deleteUser")
                .method(Method::DELETE)
                .path("/users/{userId}")
                .description("Deletes a user")
                .tag("users")
                .build(),
        )
        .build()
}

/// Creates a mock health check contract for testing.
///
/// This contract defines a simple health check endpoint that doesn't require authentication.
///
/// # Example
///
/// ```
/// use archimedes_core::fixtures::health_contract;
///
/// let contract = health_contract();
/// let op = contract.get_operation("healthCheck").unwrap();
/// assert!(!op.requires_auth());
/// ```
#[must_use]
pub fn health_contract() -> Contract {
    Contract::builder("health")
        .version("1.0.0")
        .operation(
            Operation::builder("healthCheck")
                .method(Method::GET)
                .path("/health")
                .description("Health check endpoint")
                .no_auth()
                .response_schema(MockSchema::object(vec![
                    ("status", MockSchema::string().required()),
                    ("version", MockSchema::string()),
                    ("uptime", MockSchema::integer()),
                ]))
                .build(),
        )
        .operation(
            Operation::builder("readiness")
                .method(Method::GET)
                .path("/ready")
                .description("Readiness probe endpoint")
                .no_auth()
                .response_schema(MockSchema::object(vec![
                    ("ready", MockSchema::boolean().required()),
                    ("checks", MockSchema::any()),
                ]))
                .build(),
        )
        .build()
}

/// Creates a mock order service contract for testing.
///
/// This contract defines order management operations with nested resources.
#[must_use]
pub fn order_service_contract() -> Contract {
    Contract::builder("order-service")
        .version("1.0.0")
        .operation(
            Operation::builder("getOrder")
                .method(Method::GET)
                .path("/orders/{orderId}")
                .description("Retrieves an order by ID")
                .tag("orders")
                .response_schema(order_schema())
                .build(),
        )
        .operation(
            Operation::builder("createOrder")
                .method(Method::POST)
                .path("/orders")
                .description("Creates a new order")
                .tag("orders")
                .request_schema(create_order_schema())
                .response_schema(order_schema())
                .build(),
        )
        .operation(
            Operation::builder("getOrderItems")
                .method(Method::GET)
                .path("/orders/{orderId}/items")
                .description("Lists items in an order")
                .tag("orders")
                .tag("items")
                .response_schema(MockSchema::array(order_item_schema()))
                .build(),
        )
        .operation(
            Operation::builder("addOrderItem")
                .method(Method::POST)
                .path("/orders/{orderId}/items")
                .description("Adds an item to an order")
                .tag("orders")
                .tag("items")
                .request_schema(MockSchema::object(vec![
                    ("productId", MockSchema::string().required()),
                    ("quantity", MockSchema::integer().required().minimum_int(1)),
                ]))
                .response_schema(order_item_schema())
                .build(),
        )
        .build()
}

/// Returns a user schema for testing.
#[must_use]
pub fn user_schema() -> MockSchema {
    MockSchema::object(vec![
        ("id", MockSchema::string().required()),
        ("email", MockSchema::string().required()),
        ("name", MockSchema::string().required()),
        ("createdAt", MockSchema::string()),
        ("updatedAt", MockSchema::string()),
    ])
}

/// Returns a create user request schema.
#[must_use]
pub fn create_user_schema() -> MockSchema {
    MockSchema::object(vec![
        ("email", MockSchema::string().required()),
        (
            "name",
            MockSchema::string()
                .required()
                .min_length(1)
                .max_length(100),
        ),
        ("password", MockSchema::string().required().min_length(8)),
    ])
}

/// Returns an update user request schema.
#[must_use]
pub fn update_user_schema() -> MockSchema {
    MockSchema::object(vec![
        ("email", MockSchema::string()),
        ("name", MockSchema::string().min_length(1).max_length(100)),
    ])
}

/// Returns an order schema for testing.
#[must_use]
pub fn order_schema() -> MockSchema {
    MockSchema::object(vec![
        ("id", MockSchema::string().required()),
        ("userId", MockSchema::string().required()),
        ("status", MockSchema::string().required()),
        ("items", MockSchema::array(order_item_schema())),
        ("total", MockSchema::number()),
        ("createdAt", MockSchema::string()),
    ])
}

/// Returns a create order request schema.
#[must_use]
pub fn create_order_schema() -> MockSchema {
    MockSchema::object(vec![
        (
            "items",
            MockSchema::array(MockSchema::object(vec![
                ("productId", MockSchema::string().required()),
                ("quantity", MockSchema::integer().required().minimum_int(1)),
            ]))
            .required()
            .min_items(1),
        ),
        ("shippingAddress", address_schema().required()),
    ])
}

/// Returns an order item schema.
#[must_use]
pub fn order_item_schema() -> MockSchema {
    MockSchema::object(vec![
        ("id", MockSchema::string().required()),
        ("productId", MockSchema::string().required()),
        ("productName", MockSchema::string()),
        ("quantity", MockSchema::integer().required()),
        ("unitPrice", MockSchema::number()),
        ("totalPrice", MockSchema::number()),
    ])
}

/// Returns an address schema.
#[must_use]
pub fn address_schema() -> MockSchema {
    MockSchema::object(vec![
        ("street", MockSchema::string().required()),
        ("city", MockSchema::string().required()),
        ("state", MockSchema::string()),
        ("postalCode", MockSchema::string().required()),
        ("country", MockSchema::string().required()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_user_service_contract_structure() {
        let contract = user_service_contract();

        assert_eq!(contract.name(), "user-service");
        assert_eq!(contract.version(), "1.0.0");
        assert_eq!(contract.operations().len(), 5);

        // All operations should require auth
        for op in contract.operations() {
            assert!(
                op.requires_auth(),
                "Operation {} should require auth",
                op.operation_id()
            );
        }
    }

    #[test]
    fn test_user_service_routing() {
        let contract = user_service_contract();

        // GET /users/123 -> getUser
        let (op, params) = contract
            .match_operation(&Method::GET, "/users/123")
            .unwrap();
        assert_eq!(op.operation_id(), "getUser");
        assert_eq!(params.get("userId"), Some(&"123".to_string()));

        // GET /users -> listUsers
        let (op, _) = contract.match_operation(&Method::GET, "/users").unwrap();
        assert_eq!(op.operation_id(), "listUsers");

        // POST /users -> createUser
        let (op, _) = contract.match_operation(&Method::POST, "/users").unwrap();
        assert_eq!(op.operation_id(), "createUser");

        // PUT /users/456 -> updateUser
        let (op, params) = contract
            .match_operation(&Method::PUT, "/users/456")
            .unwrap();
        assert_eq!(op.operation_id(), "updateUser");
        assert_eq!(params.get("userId"), Some(&"456".to_string()));

        // DELETE /users/789 -> deleteUser
        let (op, _) = contract
            .match_operation(&Method::DELETE, "/users/789")
            .unwrap();
        assert_eq!(op.operation_id(), "deleteUser");
    }

    #[test]
    fn test_health_contract_no_auth() {
        let contract = health_contract();

        let health_op = contract.get_operation("healthCheck").unwrap();
        assert!(!health_op.requires_auth());

        let ready_op = contract.get_operation("readiness").unwrap();
        assert!(!ready_op.requires_auth());
    }

    #[test]
    fn test_create_user_validation() {
        let schema = create_user_schema();

        // Valid request
        assert!(schema
            .validate(&json!({
                "email": "test@example.com",
                "name": "Test User",
                "password": "securepassword123"
            }))
            .is_ok());

        // Missing required field
        assert!(schema
            .validate(&json!({
                "email": "test@example.com",
                "name": "Test User"
            }))
            .is_err());

        // Password too short
        assert!(schema
            .validate(&json!({
                "email": "test@example.com",
                "name": "Test User",
                "password": "short"
            }))
            .is_err());
    }

    #[test]
    fn test_order_service_nested_routing() {
        let contract = order_service_contract();

        // GET /orders/ord-123 -> getOrder
        let (op, params) = contract
            .match_operation(&Method::GET, "/orders/ord-123")
            .unwrap();
        assert_eq!(op.operation_id(), "getOrder");
        assert_eq!(params.get("orderId"), Some(&"ord-123".to_string()));

        // GET /orders/ord-123/items -> getOrderItems
        let (op, params) = contract
            .match_operation(&Method::GET, "/orders/ord-123/items")
            .unwrap();
        assert_eq!(op.operation_id(), "getOrderItems");
        assert_eq!(params.get("orderId"), Some(&"ord-123".to_string()));

        // POST /orders/ord-123/items -> addOrderItem
        let (op, _) = contract
            .match_operation(&Method::POST, "/orders/ord-456/items")
            .unwrap();
        assert_eq!(op.operation_id(), "addOrderItem");
    }

    #[test]
    fn test_create_order_validation() {
        let schema = create_order_schema();

        // Valid request
        assert!(schema
            .validate(&json!({
                "items": [
                    {"productId": "prod-1", "quantity": 2}
                ],
                "shippingAddress": {
                    "street": "123 Main St",
                    "city": "Springfield",
                    "postalCode": "12345",
                    "country": "US"
                }
            }))
            .is_ok());

        // Empty items array
        assert!(schema
            .validate(&json!({
                "items": [],
                "shippingAddress": {
                    "street": "123 Main St",
                    "city": "Springfield",
                    "postalCode": "12345",
                    "country": "US"
                }
            }))
            .is_err());

        // Invalid quantity
        assert!(schema
            .validate(&json!({
                "items": [
                    {"productId": "prod-1", "quantity": 0}
                ],
                "shippingAddress": {
                    "street": "123 Main St",
                    "city": "Springfield",
                    "postalCode": "12345",
                    "country": "US"
                }
            }))
            .is_err());
    }
}
