//! Parsing utilities for handler macros.
//!
//! This module provides parsing for handler attributes and function signatures.

use proc_macro2::Span;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, ExprLit, FnArg, Ident, ItemFn, Lit, Meta, Pat, PatIdent, PatType, Path, Token, Type,
};

/// Parsed handler attributes.
///
/// Contains all the configuration for a handler from its attribute macro.
#[derive(Debug)]
pub struct HandlerAttrs {
    /// The operation ID from the contract.
    pub operation: String,
    /// Optional HTTP method override.
    pub method: Option<String>,
    /// Optional path override.
    pub path: Option<String>,
}

impl Parse for HandlerAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut operation = None;
        let mut method = None;
        let mut path = None;

        let meta_list: Punctuated<Meta, Token![,]> = Punctuated::parse_terminated(input)?;

        for meta in meta_list {
            match meta {
                Meta::NameValue(nv) => {
                    let ident = nv
                        .path
                        .get_ident()
                        .ok_or_else(|| syn::Error::new(nv.path.span(), "expected identifier"))?
                        .to_string();

                    let value = match &nv.value {
                        Expr::Lit(ExprLit {
                            lit: Lit::Str(s), ..
                        }) => s.value(),
                        _ => {
                            return Err(syn::Error::new(
                                nv.value.span(),
                                "expected string literal",
                            ))
                        }
                    };

                    match ident.as_str() {
                        "operation" => operation = Some(value),
                        "method" => method = Some(value),
                        "path" => path = Some(value),
                        _ => {
                            return Err(syn::Error::new(
                                nv.path.span(),
                                format!("unknown attribute: {ident}"),
                            ))
                        }
                    }
                }
                _ => return Err(syn::Error::new(meta.span(), "expected name = value")),
            }
        }

        let operation = operation.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "missing required attribute: operation")
        })?;

        Ok(Self {
            operation,
            method,
            path,
        })
    }
}

/// A parsed handler parameter.
#[derive(Debug)]
pub struct HandlerParam {
    /// The parameter name.
    pub name: Ident,
    /// The parameter type.
    pub ty: Type,
    /// Whether this is an injection parameter (Inject<T>).
    pub is_inject: bool,
    /// The pattern for destructuring (e.g., `Json(body)` vs `body`).
    pub pattern: Pat,
}

impl HandlerParam {
    /// Parses a function argument into a handler parameter.
    pub fn from_fn_arg(arg: &FnArg) -> syn::Result<Self> {
        match arg {
            FnArg::Typed(PatType { pat, ty, .. }) => {
                let (name, pattern) = Self::extract_name_and_pattern(pat)?;
                let is_inject = Self::is_inject_type(ty);

                Ok(Self {
                    name,
                    ty: (**ty).clone(),
                    is_inject,
                    pattern: (**pat).clone(),
                })
            }
            FnArg::Receiver(_) => {
                Err(syn::Error::new(arg.span(), "handlers cannot have self parameter"))
            }
        }
    }

    /// Extracts the parameter name and pattern from a pattern.
    fn extract_name_and_pattern(pat: &Pat) -> syn::Result<(Ident, Pat)> {
        match pat {
            Pat::Ident(PatIdent { ident, .. }) => Ok((ident.clone(), pat.clone())),
            Pat::TupleStruct(ts) => {
                // Pattern like `Json(body)` - extract the inner ident
                if let Some(Pat::Ident(PatIdent { ident, .. })) = ts.elems.first() {
                    // Use the inner name (e.g., `body`) but keep the full pattern
                    Ok((ident.clone(), pat.clone()))
                } else {
                    Err(syn::Error::new(
                        pat.span(),
                        "expected identifier in tuple struct pattern",
                    ))
                }
            }
            Pat::Struct(ps) => {
                // For struct patterns, generate a name from the type
                let type_name = ps
                    .path
                    .segments
                    .last()
                    .map(|s| s.ident.to_string().to_lowercase())
                    .unwrap_or_else(|| "param".to_string());
                let name = Ident::new(&format!("__{type_name}"), pat.span());
                Ok((name, pat.clone()))
            }
            _ => Err(syn::Error::new(pat.span(), "unsupported parameter pattern")),
        }
    }

    /// Checks if a type is an `Inject<T>`.
    fn is_inject_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                return segment.ident == "Inject";
            }
        }
        false
    }
}

/// Parsed handler function information.
#[derive(Debug)]
pub struct HandlerFn {
    /// The function name.
    pub name: Ident,
    /// The function parameters.
    pub params: Vec<HandlerParam>,
    /// The return type.
    pub return_type: Type,
    /// Whether the function is async.
    pub is_async: bool,
    /// The original function item (for re-emission).
    pub item: ItemFn,
}

impl HandlerFn {
    /// Parses an `ItemFn` into a `HandlerFn`.
    pub fn parse(item: ItemFn) -> syn::Result<Self> {
        let name = item.sig.ident.clone();
        let is_async = item.sig.asyncness.is_some();

        if !is_async {
            return Err(syn::Error::new(
                item.sig.fn_token.span,
                "handlers must be async functions",
            ));
        }

        let params = item
            .sig
            .inputs
            .iter()
            .map(HandlerParam::from_fn_arg)
            .collect::<syn::Result<Vec<_>>>()?;

        let return_type = match &item.sig.output {
            syn::ReturnType::Default => {
                return Err(syn::Error::new(
                    item.sig.fn_token.span,
                    "handlers must have a return type",
                ))
            }
            syn::ReturnType::Type(_, ty) => (**ty).clone(),
        };

        Ok(Self {
            name,
            params,
            return_type,
            is_async,
            item,
        })
    }

    /// Returns the inner return type if wrapped in Result.
    pub fn unwrap_result_type(&self) -> Option<(&Type, &Type)> {
        if let Type::Path(type_path) = &self.return_type {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Result" {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        let mut iter = args.args.iter();
                        if let (
                            Some(syn::GenericArgument::Type(ok)),
                            Some(syn::GenericArgument::Type(err)),
                        ) = (iter.next(), iter.next())
                        {
                            return Some((ok, err));
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_handler_attrs() {
        let attrs: HandlerAttrs = syn::parse_quote!(operation = "getUser");
        assert_eq!(attrs.operation, "getUser");
        assert!(attrs.method.is_none());
        assert!(attrs.path.is_none());
    }

    #[test]
    fn test_parse_handler_attrs_with_all() {
        let attrs: HandlerAttrs =
            syn::parse_quote!(operation = "createUser", method = "POST", path = "/users");
        assert_eq!(attrs.operation, "createUser");
        assert_eq!(attrs.method, Some("POST".to_string()));
        assert_eq!(attrs.path, Some("/users".to_string()));
    }

    #[test]
    fn test_parse_handler_fn() {
        let item: ItemFn = parse_quote! {
            async fn get_user(path: Path<UserId>) -> Result<Json<User>, AppError> {
                todo!()
            }
        };
        let handler = HandlerFn::parse(item).unwrap();
        assert_eq!(handler.name.to_string(), "get_user");
        assert!(handler.is_async);
        assert_eq!(handler.params.len(), 1);
    }

    #[test]
    fn test_parse_handler_fn_with_destructuring() {
        let item: ItemFn = parse_quote! {
            async fn create_user(Json(body): Json<CreateUserRequest>) -> Result<Json<User>, AppError> {
                todo!()
            }
        };
        let handler = HandlerFn::parse(item).unwrap();
        assert_eq!(handler.params.len(), 1);
        assert_eq!(handler.params[0].name.to_string(), "body");
    }

    #[test]
    fn test_non_async_handler_rejected() {
        let item: ItemFn = parse_quote! {
            fn sync_handler() -> Result<(), Error> {
                todo!()
            }
        };
        assert!(HandlerFn::parse(item).is_err());
    }
}
