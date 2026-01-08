//! Dependency injection container.
//!
//! This module provides a simple dependency injection system for Archimedes handlers.
//! Services are registered at application startup and injected into handlers via `Inject<T>`.
//!
//! # Example
//!
//! ```rust
//! use archimedes_core::di::{Container, Inject};
//! use std::sync::Arc;
//!
//! // Define a service
//! struct Database {
//!     connection_string: String,
//! }
//!
//! impl Database {
//!     fn new(connection_string: &str) -> Self {
//!         Self {
//!             connection_string: connection_string.to_string(),
//!         }
//!     }
//! }
//!
//! // Register services at startup
//! let mut container = Container::new();
//! container.register(Arc::new(Database::new("postgres://localhost/db")));
//!
//! // Later, in a handler, resolve the service
//! let db: Arc<Database> = container.resolve().unwrap();
//! ```

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Error when a dependency cannot be resolved.
#[derive(Debug, Clone)]
pub struct InjectionError {
    /// The type name that could not be resolved.
    pub type_name: &'static str,
    /// The reason for the failure.
    pub reason: String,
}

impl fmt::Display for InjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to inject {}: {}", self.type_name, self.reason)
    }
}

impl std::error::Error for InjectionError {}

impl InjectionError {
    /// Creates a new injection error for a missing service.
    pub fn not_registered<T>() -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            reason: "service not registered".to_string(),
        }
    }

    /// Creates a new injection error with a custom reason.
    pub fn custom<T>(reason: impl Into<String>) -> Self {
        Self {
            type_name: std::any::type_name::<T>(),
            reason: reason.into(),
        }
    }
}

/// A dependency injection container.
///
/// The container stores Arc-wrapped services keyed by their type.
/// Services are registered once at startup and resolved by type in handlers.
///
/// # Thread Safety
///
/// The container is `Send + Sync` and can be safely shared across threads.
/// Services must be `Arc<T>` where `T: Send + Sync`.
#[derive(Default)]
pub struct Container {
    services: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl Container {
    /// Creates a new empty container.
    #[must_use]
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Registers a service in the container.
    ///
    /// # Arguments
    ///
    /// * `service` - An `Arc`-wrapped service instance
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_core::di::Container;
    /// use std::sync::Arc;
    ///
    /// struct MyService;
    ///
    /// let mut container = Container::new();
    /// container.register(Arc::new(MyService));
    /// ```
    pub fn register<T: Send + Sync + 'static>(&mut self, service: Arc<T>) {
        self.services.insert(TypeId::of::<T>(), service);
    }

    /// Resolves a service from the container.
    ///
    /// Returns `None` if the service is not registered.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_core::di::Container;
    /// use std::sync::Arc;
    ///
    /// struct MyService;
    ///
    /// let mut container = Container::new();
    /// container.register(Arc::new(MyService));
    ///
    /// let service: Option<Arc<MyService>> = container.resolve();
    /// assert!(service.is_some());
    /// ```
    #[must_use]
    pub fn resolve<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|s| s.clone().downcast::<T>().ok())
    }

    /// Resolves a service or returns an error.
    ///
    /// # Errors
    ///
    /// Returns `InjectionError` if the service is not registered.
    ///
    /// # Example
    ///
    /// ```rust
    /// use archimedes_core::di::Container;
    /// use std::sync::Arc;
    ///
    /// struct MyService;
    ///
    /// let container = Container::new();
    /// let result: Result<Arc<MyService>, _> = container.resolve_required();
    /// assert!(result.is_err()); // Not registered
    /// ```
    pub fn resolve_required<T: Send + Sync + 'static>(&self) -> Result<Arc<T>, InjectionError> {
        self.resolve()
            .ok_or_else(InjectionError::not_registered::<T>)
    }

    /// Checks if a service is registered.
    #[must_use]
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.services.contains_key(&TypeId::of::<T>())
    }

    /// Returns the number of registered services.
    #[must_use]
    pub fn len(&self) -> usize {
        self.services.len()
    }

    /// Returns `true` if no services are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }
}

impl fmt::Debug for Container {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Container")
            .field("service_count", &self.services.len())
            .finish()
    }
}

/// A wrapper for injected dependencies.
///
/// `Inject<T>` extracts a dependency from the DI container during handler execution.
/// The type `T` must be registered in the container at application startup.
///
/// # Example
///
/// ```rust,ignore
/// use archimedes::prelude::*;
///
/// struct Database { /* ... */ }
///
/// #[archimedes::handler(operation = "getUser")]
/// async fn get_user(
///     db: Inject<Database>,
///     Path(user_id): Path<UserId>,
/// ) -> Result<Json<User>, AppError> {
///     let user = db.get_user(user_id).await?;
///     Ok(Json(user))
/// }
/// ```
#[derive(Clone)]
pub struct Inject<T>(pub Arc<T>);

impl<T> Inject<T> {
    /// Creates a new `Inject` wrapper.
    pub fn new(inner: Arc<T>) -> Self {
        Self(inner)
    }

    /// Returns a reference to the inner service.
    pub fn inner(&self) -> &T {
        &self.0
    }

    /// Converts into the inner `Arc`.
    pub fn into_inner(self) -> Arc<T> {
        self.0
    }
}

impl<T> std::ops::Deref for Inject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for Inject<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Inject").field(&self.0).finish()
    }
}

impl<T: Send + Sync + 'static> Inject<T> {
    /// Extracts the service from a container.
    ///
    /// # Errors
    ///
    /// Returns `InjectionError` if the service is not registered.
    pub fn from_container(container: &Container) -> Result<Self, InjectionError> {
        container.resolve_required::<T>().map(Inject)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestService {
        value: String,
    }

    impl TestService {
        fn new(value: &str) -> Self {
            Self {
                value: value.to_string(),
            }
        }
    }

    #[test]
    fn test_container_new() {
        let container = Container::new();
        assert!(container.is_empty());
        assert_eq!(container.len(), 0);
    }

    #[test]
    fn test_container_register_and_resolve() {
        let mut container = Container::new();
        container.register(Arc::new(TestService::new("hello")));

        let service: Option<Arc<TestService>> = container.resolve();
        assert!(service.is_some());
        assert_eq!(service.unwrap().value, "hello");
    }

    #[test]
    fn test_container_resolve_missing() {
        let container = Container::new();
        let service: Option<Arc<TestService>> = container.resolve();
        assert!(service.is_none());
    }

    #[test]
    fn test_container_resolve_required() {
        let mut container = Container::new();
        container.register(Arc::new(TestService::new("test")));

        let result: Result<Arc<TestService>, _> = container.resolve_required();
        assert!(result.is_ok());
    }

    #[test]
    fn test_container_resolve_required_missing() {
        let container = Container::new();
        let result: Result<Arc<TestService>, _> = container.resolve_required();
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("TestService"));
        assert!(err.to_string().contains("not registered"));
    }

    #[test]
    fn test_container_contains() {
        let mut container = Container::new();
        assert!(!container.contains::<TestService>());

        container.register(Arc::new(TestService::new("test")));
        assert!(container.contains::<TestService>());
    }

    #[test]
    fn test_inject_deref() {
        let service = Arc::new(TestService::new("deref test"));
        let inject = Inject::new(service);

        assert_eq!(inject.value, "deref test");
    }

    #[test]
    fn test_inject_from_container() {
        let mut container = Container::new();
        container.register(Arc::new(TestService::new("inject")));

        let inject: Result<Inject<TestService>, _> = Inject::from_container(&container);
        assert!(inject.is_ok());
        assert_eq!(inject.unwrap().value, "inject");
    }

    #[test]
    fn test_inject_from_container_missing() {
        let container = Container::new();
        let inject: Result<Inject<TestService>, _> = Inject::from_container(&container);
        assert!(inject.is_err());
    }

    #[test]
    fn test_container_multiple_services() {
        struct ServiceA;
        struct ServiceB;

        let mut container = Container::new();
        container.register(Arc::new(ServiceA));
        container.register(Arc::new(ServiceB));

        assert_eq!(container.len(), 2);
        assert!(container.resolve::<ServiceA>().is_some());
        assert!(container.resolve::<ServiceB>().is_some());
    }

    #[test]
    fn test_container_debug() {
        let mut container = Container::new();
        container.register(Arc::new(TestService::new("debug")));

        let debug = format!("{:?}", container);
        assert!(debug.contains("Container"));
        assert!(debug.contains("service_count"));
    }

    #[test]
    fn test_injection_error_display() {
        let err = InjectionError::not_registered::<TestService>();
        let msg = err.to_string();
        assert!(msg.contains("TestService"));
        assert!(msg.contains("not registered"));
    }
}
