//! Handler macro implementation.
//!
//! This module contains the core logic for expanding `#[handler]` attributes.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemFn;

use crate::parse::{HandlerAttrs, HandlerFn, HandlerParam};

/// Expands the `#[handler]` attribute macro.
///
/// This function performs the main transformation:
/// 1. Parse the attributes and function
/// 2. Generate extraction code for each parameter
/// 3. Generate the wrapper function
/// 4. Generate the registration function
pub fn expand_handler(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    // Parse attributes
    let attrs: HandlerAttrs = syn::parse2(attr)?;

    // Parse the function
    let item_fn: ItemFn = syn::parse2(item)?;
    let handler = HandlerFn::parse(item_fn)?;

    // Generate the expanded code
    let expanded = generate_handler_code(&attrs, &handler)?;

    Ok(expanded)
}

/// Generates the complete handler code including:
/// - The original function (preserved)
/// - A registration function that creates the handler wrapper
fn generate_handler_code(attrs: &HandlerAttrs, handler: &HandlerFn) -> syn::Result<TokenStream> {
    let fn_name = &handler.name;
    let operation_id = &attrs.operation;
    let vis = &handler.item.vis;
    let original_fn = &handler.item;

    // Generate extraction code for each parameter
    let (extraction_bindings, call_args) = generate_extractions(&handler.params)?;

    // Generate the registration function name
    let registration_fn_name = format_ident!("__archimedes_register_{}", fn_name);

    // Generate the handler info struct name
    let handler_info_name = format_ident!("__ArchimedesHandler_{}", fn_name);

    // Generate method and path if provided
    let method_attr = attrs.method.as_ref().map(|m| {
        quote! { method: Some(#m), }
    }).unwrap_or_else(|| quote! { method: None, });

    let path_attr = attrs.path.as_ref().map(|p| {
        quote! { path: Some(#p), }
    }).unwrap_or_else(|| quote! { path: None, });

    let expanded = quote! {
        // Preserve the original function
        #original_fn

        /// Handler metadata for registration.
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        #vis struct #handler_info_name;

        impl #handler_info_name {
            /// Returns the operation ID for this handler.
            pub const fn operation_id() -> &'static str {
                #operation_id
            }

            /// Returns the HTTP method if overridden.
            pub const fn method() -> Option<&'static str> {
                #method_attr
                None
            }

            /// Returns the path if overridden.
            pub const fn path() -> Option<&'static str> {
                #path_attr
                None
            }
        }

        /// Registers this handler with a handler registry.
        ///
        /// The handler receives an [`InvocationContext`] containing all HTTP request
        /// details and middleware context. The macro generates extraction code for
        /// each parameter type.
        #[doc(hidden)]
        #vis fn #registration_fn_name<F>(mut register: F)
        where
            F: FnMut(&str, archimedes_core::handler::BoxedHandler),
        {
            use archimedes_extract::FromRequest;

            let handler = move |ctx: archimedes_core::InvocationContext| {
                Box::pin(async move {
                    // Create extraction context from the invocation context
                    let extraction_ctx = archimedes_extract::ExtractionContext::from_invocation(&ctx);

                    // Extract all parameters
                    #extraction_bindings

                    // Call the handler
                    let result = #fn_name(#call_args).await;

                    // Convert result to response
                    archimedes_core::handler::into_response(result)
                }) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<bytes::Bytes, archimedes_core::ThemisError>> + Send>>
            };

            register(
                #operation_id,
                Box::new(handler),
            );
        }
    };

    Ok(expanded)
}

/// Generates extraction code for handler parameters.
///
/// Returns a tuple of:
/// - Token stream for extraction bindings (let statements)
/// - Token stream for call arguments
fn generate_extractions(params: &[HandlerParam]) -> syn::Result<(TokenStream, TokenStream)> {
    let mut bindings = Vec::new();
    let mut call_args = Vec::new();

    for param in params {
        let name = &param.name;
        let ty = &param.ty;
        let pattern = &param.pattern;

        if param.is_inject {
            // For Inject<T>, use the DI container
            bindings.push(quote! {
                let #pattern: #ty = archimedes_core::di::Inject::from_container(&extraction_ctx)
                    .map_err(|e| archimedes_core::ThemisError::validation(e.to_string()))?;
            });
        } else {
            // For regular extractors, use FromRequest
            bindings.push(quote! {
                let #pattern: #ty = <#ty as archimedes_extract::FromRequest>::from_request(&extraction_ctx)
                    .map_err(|e| archimedes_core::ThemisError::validation(e.to_string()))?;
            });
        }

        // Add to call arguments (use the inner name for destructured patterns)
        call_args.push(quote! { #name });
    }

    let bindings_stream = quote! { #(#bindings)* };
    let call_args_stream = if call_args.is_empty() {
        quote! {}
    } else {
        quote! { #(#call_args),* }
    };

    Ok((bindings_stream, call_args_stream))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_basic_handler() {
        let attr: TokenStream = quote! { operation = "getUser" };
        let item: TokenStream = quote! {
            async fn get_user() -> Result<(), Error> {
                Ok(())
            }
        };

        let result = expand_handler(attr, item);
        assert!(result.is_ok(), "expansion failed: {:?}", result.err());
    }

    #[test]
    fn test_expand_handler_with_params() {
        let attr: TokenStream = quote! { operation = "createUser" };
        let item: TokenStream = quote! {
            async fn create_user(body: Json<CreateUserRequest>) -> Result<Json<User>, Error> {
                Ok(Json(User::default()))
            }
        };

        let result = expand_handler(attr, item);
        assert!(result.is_ok(), "expansion failed: {:?}", result.err());
    }

    #[test]
    fn test_expand_handler_missing_operation() {
        let attr: TokenStream = quote! {};
        let item: TokenStream = quote! {
            async fn handler() -> Result<(), Error> {
                Ok(())
            }
        };

        let result = expand_handler(attr, item);
        assert!(result.is_err());
    }
}
