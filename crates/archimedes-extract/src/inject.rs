//! Dependency injection extractor.
//!
//! The [`Inject<T>`] extractor retrieves services from the DI container.
//!
//! # Example
//!
//! ```rust,ignore
//! use archimedes::prelude::*;
//!
//! struct Database { /* ... */ }
//!
//! #[archimedes::handler(operation = "getUser")]
//! async fn get_user(
//!     db: Inject<Database>,
//!     Path(user_id): Path<UserId>,
//! ) -> Result<Json<User>, AppError> {
//!     let user = db.get_user(user_id).await?;
//!     Ok(Json(user))
//! }
//! ```

use crate::{ExtractionContext, ExtractionError, ExtractionSource, FromRequest};
use archimedes_core::di::InjectionError;
use std::fmt;
use std::sync::Arc;

/// Extractor for dependency-injected services.
///
/// `Inject<T>` extracts a service of type `T` from the DI container.
/// The service must have been registered at application startup.
///
/// # Example
///
/// ```rust,ignore
/// use archimedes::prelude::*;
///
/// #[archimedes::handler(operation = "getUsers")]
/// async fn list_users(db: Inject<Database>) -> Result<Json<Vec<User>>, AppError> {
///     let users = db.list_users().await?;
///     Ok(Json(users))
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

    /// Consumes the wrapper and returns the inner Arc.
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

impl<T: Send + Sync + 'static> FromRequest for Inject<T> {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        let container = ctx.container().ok_or_else(|| {
            ExtractionError::custom(
                ExtractionSource::Other,
                "inject",
                "No DI container available",
            )
        })?;

        container
            .resolve::<T>()
            .map(Inject)
            .ok_or_else(|| {
                ExtractionError::custom(
                    ExtractionSource::Other,
                    std::any::type_name::<T>(),
                    format!("Service '{}' not registered in DI container", std::any::type_name::<T>()),
                )
            })
    }
}

/// Extension trait for converting injection errors.
///
/// This trait provides a convenient way to convert `InjectionError` into
/// `ExtractionError` for use in the extraction pipeline.
#[allow(dead_code)]
pub trait InjectExt {
    /// Converts to an extraction error.
    fn into_extraction_error(self) -> ExtractionError;
}

impl InjectExt for InjectionError {
    fn into_extraction_error(self) -> ExtractionError {
        ExtractionError::custom(
            ExtractionSource::Other,
            self.type_name,
            self.reason,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archimedes_core::di::Container;
    use archimedes_router::Params;
    use bytes::Bytes;
    use http::{HeaderMap, Method, Uri};

    #[derive(Debug, Clone)]
    struct TestService {
        value: String,
    }

    impl TestService {
        fn new(value: &str) -> Self {
            Self { value: value.to_string() }
        }
    }

    fn create_context_with_container(container: Arc<Container>) -> ExtractionContext {
        ExtractionContext::with_container(
            Method::GET,
            Uri::from_static("/test"),
            HeaderMap::new(),
            Bytes::new(),
            Params::new(),
            container,
        )
    }

    #[test]
    fn test_inject_from_request() {
        let mut container = Container::new();
        container.register(Arc::new(TestService::new("hello")));
        let ctx = create_context_with_container(Arc::new(container));

        let inject: Inject<TestService> = Inject::from_request(&ctx).unwrap();
        assert_eq!(inject.value, "hello");
    }

    #[test]
    fn test_inject_missing_service() {
        let container = Container::new();
        let ctx = create_context_with_container(Arc::new(container));

        let result: Result<Inject<TestService>, _> = Inject::from_request(&ctx);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("not registered"));
    }

    #[test]
    fn test_inject_no_container() {
        let ctx = ExtractionContext::new(
            Method::GET,
            Uri::from_static("/test"),
            HeaderMap::new(),
            Bytes::new(),
            Params::new(),
        );

        let result: Result<Inject<TestService>, _> = Inject::from_request(&ctx);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.to_string().contains("No DI container"));
    }

    #[test]
    fn test_inject_deref() {
        let mut container = Container::new();
        container.register(Arc::new(TestService::new("deref test")));
        let ctx = create_context_with_container(Arc::new(container));

        let inject: Inject<TestService> = Inject::from_request(&ctx).unwrap();
        assert_eq!(inject.value, "deref test");
    }

    #[test]
    fn test_inject_clone() {
        let mut container = Container::new();
        container.register(Arc::new(TestService::new("clone test")));
        let ctx = create_context_with_container(Arc::new(container));

        let inject: Inject<TestService> = Inject::from_request(&ctx).unwrap();
        let cloned = inject.clone();
        assert_eq!(cloned.value, "clone test");
    }
}
