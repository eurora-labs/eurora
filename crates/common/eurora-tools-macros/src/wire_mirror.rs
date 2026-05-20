//! `#[derive(WireMirror)]` — generate bidirectional `From` impls between a
//! framework error enum (`Cow<'static, str>` everywhere, `#[source]` causes)
//! and its wire-side counterpart (`String` everywhere, source dropped).
//!
//! The wire enum is user-maintained — the derive does NOT emit it. The
//! payoff is the 94-line manual conversion block that lived next to
//! [`eurora_tools::ToolError`] collapses into the derive, and adding a new
//! variant to either side surfaces as a compile error in the derive's
//! generated match arms.
//!
//! Container attribute:
//! ```ignore
//! #[derive(WireMirror)]
//! #[wire_mirror(
//!     target = "::path::to::WireErrorEnum",
//!     // The wire enum is `#[non_exhaustive]`; pick a variant that absorbs
//!     // any unknown wire variant on the reverse conversion. The catch-all
//!     // variant must take exactly one named field `message: String`.
//!     catch_all = "Adapter",
//!     catch_all_message = "unsupported tool error variant: {variant:?}",
//! )]
//! ```
//!
//! Variant rules:
//! - Named-field variants are mapped 1:1 by name. Fields with type
//!   `Cow<'static, str>` are converted via `.into_owned()` (forward) and
//!   `Cow::Owned(...)` (reverse). Fields tagged `#[wire_mirror(skip)]` are
//!   dropped in the forward conversion and reconstructed via
//!   `Default::default()` in the reverse conversion.
//! - Tuple variants are not supported; convert them to named-field variants
//!   first. This keeps the derive's surface narrow.
//! - Unit variants are mapped 1:1 with no field handling.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataEnum, DeriveInput, Error, Expr, ExprLit, Field, Fields, GenericArgument,
    Lit, LitStr, Path, PathArguments, Result, Token, Type, TypePath, Variant, parse2,
    punctuated::Punctuated,
};

pub fn expand(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = parse2(input)?;
    let source_ident = &input.ident;

    let container = ContainerAttrs::parse(&input.attrs)?;

    let data = match &input.data {
        Data::Enum(data) => data,
        Data::Struct(_) | Data::Union(_) => {
            return Err(Error::new(input.span(), "WireMirror only supports enums"));
        }
    };

    let forward = build_forward(source_ident, &container.target, data)?;
    let reverse = build_reverse(source_ident, &container, data)?;

    Ok(quote! {
        #forward
        #reverse
    })
}

struct ContainerAttrs {
    target: Path,
    catch_all: Option<CatchAll>,
}

struct CatchAll {
    variant: syn::Ident,
    message: String,
}

impl ContainerAttrs {
    fn parse(attrs: &[Attribute]) -> Result<Self> {
        let mut target: Option<Path> = None;
        let mut catch_all_variant: Option<syn::Ident> = None;
        let mut catch_all_message: Option<String> = None;

        for attr in attrs {
            if !attr.path().is_ident("wire_mirror") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("target") {
                    let value: LitStr = meta.value()?.parse()?;
                    let parsed: Path = syn::parse_str(&value.value())
                        .map_err(|err| meta.error(format!("invalid target path: {err}")))?;
                    target = Some(parsed);
                } else if meta.path.is_ident("catch_all") {
                    let value: LitStr = meta.value()?.parse()?;
                    catch_all_variant = Some(format_ident!("{}", value.value()));
                } else if meta.path.is_ident("catch_all_message") {
                    let value: LitStr = meta.value()?.parse()?;
                    catch_all_message = Some(value.value());
                } else {
                    return Err(meta.error(
                        "unknown wire_mirror attribute (expected one of `target`, `catch_all`, `catch_all_message`)",
                    ));
                }
                Ok(())
            })?;
        }

        let target = target.ok_or_else(|| {
            Error::new(
                Span::call_site(),
                "#[derive(WireMirror)] requires `#[wire_mirror(target = \"::path::to::Wire\")]`",
            )
        })?;

        let catch_all = match (catch_all_variant, catch_all_message) {
            (Some(variant), Some(message)) => Some(CatchAll { variant, message }),
            (None, None) => None,
            (Some(_), None) => {
                return Err(Error::new(
                    Span::call_site(),
                    "wire_mirror: `catch_all` requires `catch_all_message`",
                ));
            }
            (None, Some(_)) => {
                return Err(Error::new(
                    Span::call_site(),
                    "wire_mirror: `catch_all_message` requires `catch_all`",
                ));
            }
        };

        Ok(Self { target, catch_all })
    }
}

fn build_forward(source: &syn::Ident, target: &Path, data: &DataEnum) -> Result<TokenStream> {
    let arms = data
        .variants
        .iter()
        .map(|variant| forward_arm(source, target, variant))
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        #[automatically_derived]
        impl ::core::convert::From<#source> for #target {
            fn from(value: #source) -> Self {
                match value {
                    #(#arms)*
                }
            }
        }
    })
}

fn forward_arm(source: &syn::Ident, target: &Path, variant: &Variant) -> Result<TokenStream> {
    let v_ident = &variant.ident;
    match &variant.fields {
        Fields::Unit => Ok(quote_spanned! { variant.span() =>
            #source::#v_ident => #target::#v_ident,
        }),
        Fields::Named(named) => {
            let mut bindings = Vec::new();
            let mut assigns = Vec::new();
            let mut has_skipped = false;
            for field in &named.named {
                let ident = field
                    .ident
                    .as_ref()
                    .expect("named field must have an ident");
                if is_skip(&field.attrs)? {
                    has_skipped = true;
                    continue;
                }
                bindings.push(ident.clone());
                if is_cow_static_str(&field.ty) {
                    assigns.push(quote_spanned! { field.span() =>
                        #ident: #ident.into_owned()
                    });
                } else {
                    assigns.push(quote_spanned! { field.span() =>
                        #ident
                    });
                }
            }
            // Skipped fields drop into the `..` rest pattern so source-only
            // causes don't get spelled out in the bindings list.
            let pattern = if has_skipped {
                quote! { { #(#bindings,)* .. } }
            } else {
                quote! { { #(#bindings),* } }
            };
            Ok(quote_spanned! { variant.span() =>
                #source::#v_ident #pattern => #target::#v_ident { #(#assigns),* },
            })
        }
        Fields::Unnamed(_) => Err(Error::new(
            variant.span(),
            "WireMirror does not support tuple variants; convert to a named-field variant",
        )),
    }
}

fn build_reverse(
    source: &syn::Ident,
    container: &ContainerAttrs,
    data: &DataEnum,
) -> Result<TokenStream> {
    let target = &container.target;
    let arms = data
        .variants
        .iter()
        .map(|variant| reverse_arm(source, target, variant))
        .collect::<Result<Vec<_>>>()?;

    let catch_all = match container.catch_all.as_ref() {
        Some(catch) => build_catch_all(source, data, catch)?,
        None => quote! {},
    };

    Ok(quote! {
        #[automatically_derived]
        impl ::core::convert::From<#target> for #source {
            fn from(value: #target) -> Self {
                match value {
                    #(#arms)*
                    #catch_all
                }
            }
        }
    })
}

/// Build the wildcard arm that absorbs unknown wire variants (only
/// meaningful when the target is `#[non_exhaustive]`).
///
/// The catch-all variant must exist on the source side; we inspect its
/// fields and populate each one with a sensible default:
/// - A field literally named `message` (with a `Cow<'static, str>` type)
///   is populated from the rendered `catch_all_message` format string.
/// - Every other field is `Default::default()`. This typically means
///   `#[wire_mirror(skip)]` source-cause fields land as `None`, but
///   nothing in the macro hardcodes that — the variant's declaration is
///   the source of truth.
fn build_catch_all(source: &syn::Ident, data: &DataEnum, catch: &CatchAll) -> Result<TokenStream> {
    let variant = data
        .variants
        .iter()
        .find(|v| v.ident == catch.variant)
        .ok_or_else(|| {
            Error::new(
                catch.variant.span(),
                format!(
                    "wire_mirror: catch_all variant `{}` does not exist on the source enum",
                    catch.variant
                ),
            )
        })?;
    let variant_ident = &catch.variant;
    let format_lit = LitStr::new(&catch.message, Span::call_site());

    match &variant.fields {
        Fields::Unit => Ok(quote! {
            other => {
                // The catch-all is a unit variant; drop the rendered
                // message — surfacing the dropped diagnostic via tracing
                // would impose a runtime dep we don't want to take here.
                let _ = ::std::format!(#format_lit, variant = other);
                #source::#variant_ident
            },
        }),
        Fields::Named(named) => {
            let assigns = named.named.iter().map(|field| {
                let ident = field
                    .ident
                    .as_ref()
                    .expect("named field must have an ident");
                if ident == "message" && is_cow_static_str(&field.ty) {
                    quote_spanned! { field.span() =>
                        #ident: ::std::borrow::Cow::Owned(
                            ::std::format!(#format_lit, variant = other),
                        )
                    }
                } else {
                    quote_spanned! { field.span() =>
                        #ident: ::core::default::Default::default()
                    }
                }
            });
            Ok(quote! {
                other => #source::#variant_ident { #(#assigns),* },
            })
        }
        Fields::Unnamed(_) => Err(Error::new(
            variant.span(),
            "wire_mirror: catch_all variant cannot be a tuple variant",
        )),
    }
}

fn reverse_arm(source: &syn::Ident, target: &Path, variant: &Variant) -> Result<TokenStream> {
    let v_ident = &variant.ident;
    match &variant.fields {
        Fields::Unit => Ok(quote_spanned! { variant.span() =>
            #target::#v_ident => #source::#v_ident,
        }),
        Fields::Named(named) => {
            let mut bindings = Vec::new();
            let mut assigns = Vec::new();
            for field in &named.named {
                let ident = field
                    .ident
                    .as_ref()
                    .expect("named field must have an ident");
                if is_skip(&field.attrs)? {
                    // Skipped fields don't exist on the wire side and are
                    // rebuilt with their type's `Default` impl.
                    assigns.push(quote_spanned! { field.span() =>
                        #ident: ::core::default::Default::default()
                    });
                    continue;
                }
                bindings.push(ident.clone());
                if is_cow_static_str(&field.ty) {
                    assigns.push(quote_spanned! { field.span() =>
                        #ident: ::std::borrow::Cow::Owned(#ident)
                    });
                } else {
                    assigns.push(quote_spanned! { field.span() =>
                        #ident
                    });
                }
            }
            Ok(quote_spanned! { variant.span() =>
                #target::#v_ident { #(#bindings),* } => #source::#v_ident { #(#assigns),* },
            })
        }
        Fields::Unnamed(_) => Err(Error::new(
            variant.span(),
            "WireMirror does not support tuple variants; convert to a named-field variant",
        )),
    }
}

fn is_skip(attrs: &[Attribute]) -> Result<bool> {
    for attr in attrs {
        if !attr.path().is_ident("wire_mirror") {
            continue;
        }
        let mut skip = false;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
                Ok(())
            } else {
                Err(meta.error("unknown wire_mirror field attribute (expected `skip`)"))
            }
        })?;
        if skip {
            return Ok(true);
        }
    }
    Ok(false)
}

/// True if `ty` is syntactically `Cow<'static, str>` (with or without the
/// `std::borrow::` prefix). The check is purely syntactic — sufficient for
/// the macro's narrow target audience.
fn is_cow_static_str(ty: &Type) -> bool {
    let Type::Path(TypePath { qself: None, path }) = ty else {
        return false;
    };
    let last = match path.segments.last() {
        Some(seg) => seg,
        None => return false,
    };
    if last.ident != "Cow" {
        return false;
    }
    let PathArguments::AngleBracketed(args) = &last.arguments else {
        return false;
    };
    let mut has_static = false;
    let mut has_str = false;
    for arg in &args.args {
        match arg {
            GenericArgument::Lifetime(lt) if lt.ident == "static" => has_static = true,
            GenericArgument::Type(Type::Path(p)) if p.path.is_ident("str") => {
                has_str = true;
            }
            _ => {}
        }
    }
    has_static && has_str
}

// `Punctuated`/`Token` are referenced indirectly through `Fields::Named.named`
// — keep the `use` so the file stays clean under `rustc --explain` style
// hover; the compiler doesn't warn on these even when only used in types.
const _: fn() = || {
    let _: Punctuated<Field, Token![,]> = Punctuated::new();
    let _: Option<Expr> = None;
    let _: Option<ExprLit> = None;
    let _: Option<Lit> = None;
};
