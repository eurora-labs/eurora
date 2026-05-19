//! Validate the signature of a tool method against its declared
//! `source` and extract the types the dispatcher needs to wire up.

use syn::spanned::Spanned;
use syn::{FnArg, GenericArgument, PathArguments, Receiver, ReturnType, Type, TypeReference};

use crate::source::TargetKind;

/// Result of validating a `#[tool]` method's signature.
#[cfg_attr(test, derive(Debug))]
pub(crate) struct SignatureInfo {
    /// The target parameter (`target: &BrowserOrigin`), or `None` when
    /// the source is `client_local`/`server_local`.
    pub(crate) target: Option<TargetParam>,
    /// The arguments parameter (`args: Empty`).
    pub(crate) args: ArgsParam,
    /// The `Ok` type from the method's return type (`Result<Ok, …>`).
    pub(crate) return_ty: Type,
}

/// The `target: &OriginType` parameter on a tool method.
#[cfg_attr(test, derive(Debug))]
pub(crate) struct TargetParam {
    /// The user-visible parameter name (used in the dispatcher's
    /// generated call site).
    pub(crate) name: syn::Ident,
}

/// The `args: ArgType` parameter on a tool method.
#[cfg_attr(test, derive(Debug))]
pub(crate) struct ArgsParam {
    pub(crate) name: syn::Ident,
    pub(crate) ty: Type,
}

/// Inspect `method` against the expected `target` kind. Returns the
/// parts the dispatcher needs, or a compile-error spanned on the
/// offending element of the signature.
pub(crate) fn analyze(method: &syn::TraitItemFn, target: TargetKind) -> syn::Result<SignatureInfo> {
    let sig = &method.sig;

    if sig.asyncness.is_none() {
        return Err(syn::Error::new(
            sig.fn_token.span(),
            "tool methods must be `async fn` so the dispatcher can await them",
        ));
    }

    let mut inputs = sig.inputs.iter();

    let receiver = inputs.next().ok_or_else(|| {
        syn::Error::new(
            sig.paren_token.span.span(),
            "tool methods must take `&self`",
        )
    })?;
    expect_ref_self(receiver)?;

    let target_param = if target == TargetKind::None {
        None
    } else {
        let next = inputs.next().ok_or_else(|| {
            syn::Error::new(
                sig.ident.span(),
                format!(
                    "tool method missing target parameter; expected `target: {}` before the args parameter",
                    target.expected_type_repr()
                ),
            )
        })?;
        Some(parse_target(next, target)?)
    };

    let args_arg = inputs.next().ok_or_else(|| {
        syn::Error::new(
            sig.ident.span(),
            "tool method missing args parameter; expected `args: <ArgsType>`",
        )
    })?;
    let args = parse_args(args_arg, target)?;

    if let Some(extra) = inputs.next() {
        return Err(syn::Error::new_spanned(
            extra,
            "tool method has too many parameters; expected `(&self, [target,] args)`",
        ));
    }

    let return_ty = parse_return(&sig.output)?;

    Ok(SignatureInfo {
        target: target_param,
        args,
        return_ty,
    })
}

fn expect_ref_self(arg: &FnArg) -> syn::Result<()> {
    match arg {
        FnArg::Receiver(Receiver {
            reference: Some(_),
            mutability: None,
            ..
        }) => Ok(()),
        FnArg::Receiver(receiver) => Err(syn::Error::new_spanned(
            receiver,
            "tool methods must take `&self` (not `self`, `&mut self`, or other receivers)",
        )),
        FnArg::Typed(typed) => Err(syn::Error::new_spanned(
            typed,
            "first parameter of a tool method must be `&self`",
        )),
    }
}

fn parse_target(arg: &FnArg, target: TargetKind) -> syn::Result<TargetParam> {
    let FnArg::Typed(pat_type) = arg else {
        return Err(syn::Error::new_spanned(
            arg,
            "tool method target parameter must be a typed binding",
        ));
    };

    let name = pat_ident(&pat_type.pat)?;
    let Type::Reference(TypeReference {
        mutability: None,
        elem,
        ..
    }) = pat_type.ty.as_ref()
    else {
        // The second positional arg isn't a reference: the user almost
        // certainly wrote `args: …` where the target should have been.
        return Err(syn::Error::new_spanned(
            &pat_type.ty,
            format!(
                "tool method missing target parameter; expected `target: {}` here \
                 before the args parameter",
                target.expected_type_repr()
            ),
        ));
    };

    let expected_ident = target
        .expected_type_ident()
        .expect("None target handled before parse_target");
    let last_seg = type_path_last_segment(elem.as_ref()).ok_or_else(|| {
        syn::Error::new_spanned(
            &pat_type.ty,
            format!(
                "target parameter must reference `{}`",
                target.expected_type_repr()
            ),
        )
    })?;
    if last_seg != expected_ident {
        return Err(syn::Error::new_spanned(
            &pat_type.ty,
            format!(
                "source kind expects `{}`, but the method declares `&{}`",
                target.expected_type_repr(),
                last_seg
            ),
        ));
    }

    Ok(TargetParam { name })
}

fn parse_args(arg: &FnArg, target: TargetKind) -> syn::Result<ArgsParam> {
    let FnArg::Typed(pat_type) = arg else {
        return Err(syn::Error::new_spanned(
            arg,
            "tool method args parameter must be a typed binding",
        ));
    };
    let name = pat_ident(&pat_type.pat)?;
    let ty = (*pat_type.ty).clone();

    // Reject reference / mutable bindings — the dispatcher decodes the
    // args from JSON, so it must own the value. For no-target sources
    // this almost always means the user declared an extra `target: &…`
    // parameter that doesn't belong; spell that out.
    if matches!(&ty, Type::Reference(_)) {
        if target == TargetKind::None {
            return Err(syn::Error::new_spanned(
                &pat_type.ty,
                "tool method has too many parameters; client_local/server_local sources \
                 must not declare a target — expected `(&self, args: <ArgsType>)`",
            ));
        }
        return Err(syn::Error::new_spanned(
            &pat_type.ty,
            "args parameter must be an owned type; the dispatcher decodes it from JSON",
        ));
    }

    Ok(ArgsParam { name, ty })
}

fn parse_return(output: &ReturnType) -> syn::Result<Type> {
    let ReturnType::Type(_, ty) = output else {
        return Err(syn::Error::new_spanned(
            output,
            "tool method must return `Result<_, eurora_tools::ToolError>`",
        ));
    };

    let last = type_path_last(ty.as_ref()).ok_or_else(|| {
        syn::Error::new_spanned(
            ty,
            "tool method return type must be a path ending in `Result<_, _>`",
        )
    })?;
    if last.ident != "Result" {
        return Err(syn::Error::new_spanned(
            &last.ident,
            format!(
                "tool method must return `Result<_, eurora_tools::ToolError>` (got `{}`)",
                last.ident
            ),
        ));
    }
    let PathArguments::AngleBracketed(generics) = &last.arguments else {
        return Err(syn::Error::new_spanned(
            &last.arguments,
            "`Result` must have `<Ok, Err>` generic arguments",
        ));
    };
    let mut tys = generics.args.iter().filter_map(|a| match a {
        GenericArgument::Type(t) => Some(t.clone()),
        _ => None,
    });
    let ok_ty = tys.next().ok_or_else(|| {
        syn::Error::new_spanned(
            &last.arguments,
            "`Result` is missing its `Ok` type parameter",
        )
    })?;
    // The Err parameter is required (this is a fully-spelled `Result<T, E>`,
    // not the std-prelude `Result<T>`-style aliases). Accept either form;
    // we only need the `Ok` type for serialization.
    Ok(ok_ty)
}

fn pat_ident(pat: &syn::Pat) -> syn::Result<syn::Ident> {
    if let syn::Pat::Ident(pat_ident) = pat {
        Ok(pat_ident.ident.clone())
    } else {
        Err(syn::Error::new_spanned(
            pat,
            "tool method parameters must use plain identifier bindings (e.g. `args: Foo`)",
        ))
    }
}

fn type_path_last_segment(ty: &Type) -> Option<String> {
    type_path_last(ty).map(|seg| seg.ident.to_string())
}

fn type_path_last(ty: &Type) -> Option<&syn::PathSegment> {
    if let Type::Path(path) = ty {
        path.path.segments.last()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn parse_method(src: proc_macro2::TokenStream) -> syn::TraitItemFn {
        syn::parse2(src).expect("method parses")
    }

    #[test]
    fn analyzes_full_browser_method() {
        let m = parse_method(quote::quote! {
            async fn fetch(
                &self,
                target: &BrowserOrigin,
                args: MyArgs,
            ) -> Result<MyRet, ToolError>;
        });
        let info = analyze(&m, TargetKind::Browser).unwrap();
        assert!(info.target.is_some());
        assert_eq!(info.args.name.to_string(), "args");
    }

    #[test]
    fn analyzes_client_local_no_target() {
        let m = parse_method(quote::quote! {
            async fn fetch(&self, args: MyArgs) -> Result<MyRet, ToolError>;
        });
        let info = analyze(&m, TargetKind::None).unwrap();
        assert!(info.target.is_none());
        assert_eq!(info.args.name.to_string(), "args");
    }

    #[test]
    fn rejects_non_async() {
        let m: syn::TraitItemFn = parse_quote! {
            fn fetch(&self, args: MyArgs) -> Result<MyRet, ToolError>;
        };
        let err = analyze(&m, TargetKind::None).unwrap_err();
        assert!(err.to_string().contains("`async fn`"));
    }

    #[test]
    fn rejects_origin_mismatch() {
        let m = parse_method(quote::quote! {
            async fn fetch(
                &self,
                target: &FocusedOrigin,
                args: MyArgs,
            ) -> Result<MyRet, ToolError>;
        });
        let err = analyze(&m, TargetKind::Browser).unwrap_err();
        assert!(err.to_string().contains("expects"));
    }

    #[test]
    fn rejects_missing_target() {
        let m = parse_method(quote::quote! {
            async fn fetch(&self, args: MyArgs) -> Result<MyRet, ToolError>;
        });
        let err = analyze(&m, TargetKind::Browser).unwrap_err();
        assert!(err.to_string().contains("missing target"));
    }

    #[test]
    fn rejects_extra_target() {
        let m = parse_method(quote::quote! {
            async fn fetch(
                &self,
                target: &BrowserOrigin,
                args: MyArgs,
            ) -> Result<MyRet, ToolError>;
        });
        let err = analyze(&m, TargetKind::None).unwrap_err();
        assert!(err.to_string().contains("too many parameters"));
    }

    #[test]
    fn rejects_bad_return() {
        let m = parse_method(quote::quote! {
            async fn fetch(&self, args: MyArgs) -> MyRet;
        });
        let err = analyze(&m, TargetKind::None).unwrap_err();
        assert!(err.to_string().contains("Result"));
    }
}
