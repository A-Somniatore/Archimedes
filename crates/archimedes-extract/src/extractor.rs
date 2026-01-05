//! Core extractor trait.
//!
//! The [`FromRequest`] trait is the foundation for all extractors.

use crate::{ExtractionContext, ExtractionError};

/// Trait for types that can be extracted from an HTTP request.
///
/// This is the core trait that all extractors implement. It provides
/// a synchronous extraction method that operates on an [`ExtractionContext`].
///
/// # Implementing `FromRequest`
///
/// ```rust
/// use archimedes_extract::{FromRequest, ExtractionContext, ExtractionError, ExtractionSource};
///
/// // Custom extractor for a specific header
/// struct ApiVersion(u32);
///
/// impl FromRequest for ApiVersion {
///     fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
///         let version = ctx.header("x-api-version")
///             .ok_or_else(|| ExtractionError::missing(ExtractionSource::Header, "x-api-version"))?;
///         
///         let version: u32 = version.parse()
///             .map_err(|_| ExtractionError::invalid_type(
///                 ExtractionSource::Header,
///                 "x-api-version",
///                 "expected integer version",
///             ))?;
///         
///         Ok(ApiVersion(version))
///     }
/// }
/// ```
///
/// # Tuple Extractors
///
/// Multiple extractors can be combined using tuples:
///
/// ```rust
/// use archimedes_extract::{FromRequest, ExtractionContext, ExtractionError};
///
/// // Tuples of extractors are automatically extractors
/// // (Path<UserId>, Query<Params>) implements FromRequest
/// ```
pub trait FromRequest: Sized {
    /// Extracts this type from the request context.
    ///
    /// # Errors
    ///
    /// Returns an [`ExtractionError`] if extraction fails.
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError>;
}

// Implement FromRequest for Option<T> where T: FromRequest
// This makes extraction optional (None if it fails)
impl<T: FromRequest> FromRequest for Option<T> {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        Ok(T::from_request(ctx).ok())
    }
}

// Implement FromRequest for Result<T, ExtractionError> where T: FromRequest
// This allows handling extraction errors inline
impl<T: FromRequest> FromRequest for Result<T, ExtractionError> {
    fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        Ok(T::from_request(ctx))
    }
}

// Implement FromRequest for tuples to allow combining extractors
macro_rules! impl_from_request_for_tuple {
    ($($T:ident),*) => {
        impl<$($T: FromRequest),*> FromRequest for ($($T,)*) {
            fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
                Ok(($($T::from_request(ctx)?,)*))
            }
        }
    };
}

impl_from_request_for_tuple!(T1);
impl_from_request_for_tuple!(T1, T2);
impl_from_request_for_tuple!(T1, T2, T3);
impl_from_request_for_tuple!(T1, T2, T3, T4);
impl_from_request_for_tuple!(T1, T2, T3, T4, T5);
impl_from_request_for_tuple!(T1, T2, T3, T4, T5, T6);

// Implement FromRequest for () (unit type) - always succeeds
impl FromRequest for () {
    fn from_request(_ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ExtractionContextBuilder;
    use http::{Method, Uri};

    // A simple test extractor
    struct TestExtractor(String);

    impl FromRequest for TestExtractor {
        fn from_request(ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
            Ok(TestExtractor(ctx.path().to_string()))
        }
    }

    // An extractor that always fails
    struct FailingExtractor;

    impl FromRequest for FailingExtractor {
        fn from_request(_ctx: &ExtractionContext) -> Result<Self, ExtractionError> {
            Err(ExtractionError::missing(
                crate::ExtractionSource::Path,
                "required_field",
            ))
        }
    }

    #[test]
    fn test_basic_extraction() {
        let ctx = ExtractionContextBuilder::new()
            .method(Method::GET)
            .uri(Uri::from_static("/test/path"))
            .build();

        let result = TestExtractor::from_request(&ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, "/test/path");
    }

    #[test]
    fn test_option_extraction_success() {
        let ctx = ExtractionContextBuilder::new()
            .method(Method::GET)
            .uri(Uri::from_static("/test"))
            .build();

        let result = <Option<TestExtractor>>::from_request(&ctx);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_option_extraction_failure() {
        let ctx = ExtractionContextBuilder::new()
            .method(Method::GET)
            .uri(Uri::from_static("/test"))
            .build();

        let result = <Option<FailingExtractor>>::from_request(&ctx);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_result_extraction() {
        let ctx = ExtractionContextBuilder::new()
            .method(Method::GET)
            .uri(Uri::from_static("/test"))
            .build();

        let result = <Result<TestExtractor, ExtractionError>>::from_request(&ctx);
        assert!(result.is_ok());
        let inner = result.unwrap();
        assert!(inner.is_ok());
    }

    #[test]
    fn test_tuple_extraction() {
        let ctx = ExtractionContextBuilder::new()
            .method(Method::GET)
            .uri(Uri::from_static("/test"))
            .build();

        let result = <(TestExtractor, TestExtractor)>::from_request(&ctx);
        assert!(result.is_ok());
        let (a, b) = result.unwrap();
        assert_eq!(a.0, "/test");
        assert_eq!(b.0, "/test");
    }

    #[test]
    fn test_unit_extraction() {
        let ctx = ExtractionContextBuilder::new()
            .method(Method::GET)
            .uri(Uri::from_static("/test"))
            .build();

        let result = <()>::from_request(&ctx);
        assert!(result.is_ok());
    }
}
