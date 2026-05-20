//! Expansion of the `#[adapter]` attribute.
//!
//! The expansion is the single source of truth that ties together every
//! other module in the crate: it parses adapter-level args
//! ([`crate::attrs::AdapterAttrs`]), walks each method to pull the
//! `#[tool]` metadata ([`crate::attrs::ToolAttrs`]), runs the source +
//! signature validators ([`crate::source`], [`crate::signature`]), then
//! emits the descriptor table, the trait variant, the dispatcher
//! struct, and the per-method type-bound assertions.
//!
//! The emitted code uses fully-qualified paths everywhere
//! (`::eurora_tools::…`, `::serde_json::…`, `::std::sync::…`) so
//! adapter crates don't need to bring those names into scope at the
//! macro call site.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Attribute, ItemTrait, LitStr, TraitItem, TraitItemFn, Visibility};

use crate::attrs::{AdapterAttrs, ToolAttrs};
use crate::docs::first_paragraph;
use crate::signature::{ArgsParam, SignatureInfo, TargetParam, analyze};
use crate::source::TargetKind;

pub(crate) fn expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let adapter_attrs = AdapterAttrs::parse(attr)?;
    let mut input: ItemTrait = syn::parse2(item)?;
    let trait_span = input.ident.span();

    // Accumulate per-method analyses up front so a single bad method
    // doesn't abort emission for the others — combined errors give the
    // user the full picture in one compile cycle.
    let mut combined: Option<syn::Error> = None;
    let mut tools: Vec<ToolEmission> = Vec::with_capacity(input.items.len());

    for item in &mut input.items {
        let method = match item {
            TraitItem::Fn(f) => f,
            _ => continue,
        };
        match prepare_method(method, &adapter_attrs) {
            Ok(Some(tool)) => tools.push(tool),
            Ok(None) => {}
            Err(e) => match &mut combined {
                Some(acc) => acc.combine(e),
                None => combined = Some(e),
            },
        }
    }

    if let Some(err) = combined {
        return Err(err);
    }
    if tools.is_empty() {
        return Err(syn::Error::new(
            trait_span,
            "#[adapter] trait must declare at least one `#[tool]` method",
        ));
    }

    let local_trait = build_local_trait(input, &adapter_attrs);
    let trait_ident = local_trait.public_ident.clone();
    let trait_visibility = local_trait.visibility.clone();
    let trait_tokens = local_trait.tokens;

    let dispatcher_ident = dispatcher_ident(&trait_ident);
    let descriptors_ident = descriptor_table_ident(&trait_ident);
    let descriptor_count = tools.len();

    let assertions = tools.iter().map(|t| t.assertions.clone());
    let descriptor_entries = tools.iter().map(|t| t.descriptor.clone());
    let match_arms = tools.iter().map(|t| t.match_arm.clone());

    let expanded = quote! {
        #trait_tokens

        #(#assertions)*

        #[allow(non_upper_case_globals)]
        #trait_visibility static #descriptors_ident:
            ::std::sync::LazyLock<[::eurora_tools::ToolDescriptor; #descriptor_count]> =
            ::std::sync::LazyLock::new(|| [#(#descriptor_entries),*]);

        #trait_visibility struct #dispatcher_ident<T>
        where
            T: #trait_ident + 'static,
        {
            inner: ::std::sync::Arc<T>,
        }

        impl<T> #dispatcher_ident<T>
        where
            T: #trait_ident + 'static,
        {
            /// Build a dispatcher around an owned adapter implementation.
            pub fn new(inner: T) -> Self {
                Self { inner: ::std::sync::Arc::new(inner) }
            }

            /// Build a dispatcher around a shared adapter implementation.
            pub fn from_arc(inner: ::std::sync::Arc<T>) -> Self {
                Self { inner }
            }

            /// Borrow the inner adapter implementation.
            pub fn inner(&self) -> &::std::sync::Arc<T> {
                &self.inner
            }
        }

        impl<T> ::eurora_tools::Dispatcher for #dispatcher_ident<T>
        where
            T: #trait_ident + 'static,
        {
            fn descriptors(&self) -> &'static [::eurora_tools::ToolDescriptor] {
                &#descriptors_ident[..]
            }

            fn dispatch(
                &self,
                call: ::eurora_tools::IncomingCall,
            ) -> ::eurora_tools::__private::futures::future::BoxFuture<
                '_,
                ::core::result::Result<
                    ::eurora_tools::__private::serde_json::Value,
                    ::eurora_tools::ToolError,
                >,
            > {
                let inner = ::std::sync::Arc::clone(&self.inner);
                ::std::boxed::Box::pin(async move {
                    match call.descriptor_name {
                        #(#match_arms)*
                        other => ::core::result::Result::Err(
                            ::eurora_tools::ToolError::Remote {
                                code: 404,
                                message: ::std::format!("unknown tool {other}"),
                                details: ::core::option::Option::None,
                            },
                        ),
                    }
                })
            }
        }
    };

    Ok(expanded)
}

/// Per-method emission bundle. Held in `Vec`s and stitched together at
/// the end of `expand`.
struct ToolEmission {
    descriptor: TokenStream,
    match_arm: TokenStream,
    assertions: TokenStream,
}

fn prepare_method(
    method: &mut TraitItemFn,
    adapter_attrs: &AdapterAttrs,
) -> syn::Result<Option<ToolEmission>> {
    let tool_attr = take_tool_attribute(&mut method.attrs)?;
    let Some((tool_attr_span, tool_meta_tokens)) = tool_attr else {
        return Ok(None);
    };

    let tool_attrs = ToolAttrs::parse(tool_meta_tokens.clone()).map_err(|mut e| {
        e.combine(syn::Error::new(
            tool_attr_span,
            "while parsing this `#[tool]`",
        ));
        e
    })?;

    let description = first_paragraph(&method.attrs).ok_or_else(|| {
        syn::Error::new(
            method.sig.fn_token.span(),
            "tool methods require a non-empty rustdoc comment; the first paragraph \
             is sent to the LLM as the tool description",
        )
    })?;

    let target_kind = tool_attrs.source.target_kind();
    let signature = analyze(method, target_kind)?;

    let tool_name = format!("{}::{}", adapter_attrs.namespace.value(), method.sig.ident);
    let tool_name_lit = LitStr::new(&tool_name, method.sig.ident.span());

    let descriptor = build_descriptor(&tool_name_lit, &description, &tool_attrs, &signature);
    let match_arm = build_match_arm(&tool_name_lit, &method.sig.ident, target_kind, &signature);
    let assertions = build_assertions(&signature);

    Ok(Some(ToolEmission {
        descriptor,
        match_arm,
        assertions,
    }))
}

/// Find and remove the `#[tool(...)]` attribute from a method.
///
/// Returns the original attribute's span (for diagnostics) and the raw
/// meta tokens (for `ToolAttrs::parse`). An error is raised if multiple
/// `#[tool]` attributes are present.
fn take_tool_attribute(attrs: &mut Vec<Attribute>) -> syn::Result<Option<(Span, TokenStream)>> {
    let mut indices: Vec<usize> = attrs
        .iter()
        .enumerate()
        .filter_map(|(i, attr)| attr.path().is_ident("tool").then_some(i))
        .collect();
    if indices.len() > 1 {
        return Err(syn::Error::new_spanned(
            &attrs[indices[1]],
            "method has more than one `#[tool]` attribute",
        ));
    }
    let Some(idx) = indices.pop() else {
        return Ok(None);
    };
    let attr = attrs.remove(idx);
    let span = attr.span();
    let tokens = match attr.meta {
        syn::Meta::List(list) => list.tokens,
        syn::Meta::Path(_) => TokenStream::new(),
        syn::Meta::NameValue(nv) => {
            return Err(syn::Error::new_spanned(
                nv,
                "`#[tool]` does not accept `=`-style arguments; use `#[tool(...)]`",
            ));
        }
    };
    Ok(Some((span, tokens)))
}

fn build_descriptor(
    tool_name_lit: &LitStr,
    description: &str,
    tool_attrs: &ToolAttrs,
    signature: &SignatureInfo,
) -> TokenStream {
    let description_lit = LitStr::new(description, tool_name_lit.span());

    let timeout_ms = tool_attrs.timeout_ms;

    let source_expr = tool_attrs
        .source
        .to_tool_source_expr(tool_attrs.source_lit.span());

    let context_lits = tool_attrs.required_contexts.iter();
    let approval = tool_attrs.requires_user_approval;

    let args_ty = &signature.args.ty;
    let return_ty = &signature.return_ty;

    quote! {
        ::eurora_tools::ToolDescriptor {
            name: #tool_name_lit,
            description: #description_lit,
            input_schema: ::eurora_tools::schema_of::<#args_ty>,
            output_schema: ::eurora_tools::schema_of::<#return_ty>,
            timeout: ::core::time::Duration::from_millis(#timeout_ms),
            source: #source_expr,
            required_contexts: &[ #( #context_lits ),* ],
            requires_user_approval: #approval,
        }
    }
}

fn build_match_arm(
    tool_name_lit: &LitStr,
    method_ident: &syn::Ident,
    target_kind: TargetKind,
    signature: &SignatureInfo,
) -> TokenStream {
    let SignatureInfo { target, args, .. } = signature;
    let ArgsParam {
        name: args_name,
        ty: args_ty,
    } = args;

    let decode_args = quote! {
        let #args_name: #args_ty =
            ::eurora_tools::__private::serde_json::from_value(call.arguments)
                .map_err(::eurora_tools::ToolError::decode)?;
    };
    let encode_result = quote! {
        ::eurora_tools::__private::serde_json::to_value(__result)
            .map_err(::eurora_tools::ToolError::encode)
    };

    if let (Some(variant_name), Some(TargetParam { name: target_name })) =
        (target_kind.origin_variant(), target.as_ref())
    {
        let variant_ident = format_ident!("{}", variant_name);
        let variant_lit = LitStr::new(variant_name, Span::call_site());
        // `call.origin` is `Arc<Origin>`; the framework keeps it shared
        // across every dispatch in a turn so the per-turn snapshot avoids
        // deep-cloning string-heavy variants. `.as_ref()` borrows the
        // inner `Origin` for the pattern match.
        quote! {
            #tool_name_lit => {
                let ::eurora_tools::Origin::#variant_ident(#target_name) = call.origin.as_ref() else {
                    return ::core::result::Result::Err(
                        ::eurora_tools::ToolError::OriginMismatch {
                            tool: ::std::borrow::Cow::Borrowed(#tool_name_lit),
                            expected: ::std::borrow::Cow::Borrowed(#variant_lit),
                            got: ::std::borrow::Cow::Borrowed(call.origin.variant_name()),
                        },
                    );
                };
                #decode_args
                let __result = inner.#method_ident(#target_name, #args_name).await?;
                #encode_result
            }
        }
    } else {
        quote! {
            #tool_name_lit => {
                #decode_args
                let __result = inner.#method_ident(#args_name).await?;
                #encode_result
            }
        }
    }
}

fn build_assertions(signature: &SignatureInfo) -> TokenStream {
    let args_ty = &signature.args.ty;
    let return_ty = &signature.return_ty;
    let args_span = args_ty.span();
    let return_span = return_ty.span();

    let args_assert = quote_spanned! { args_span =>
        const _: () = {
            fn _eurora_tools_check<T>()
            where
                T: ::eurora_tools::__private::serde::Serialize
                    + ::eurora_tools::__private::serde::de::DeserializeOwned
                    + ::eurora_tools::__private::schemars::JsonSchema,
            {}
            let _ = _eurora_tools_check::<#args_ty>;
        };
    };
    let return_assert = quote_spanned! { return_span =>
        const _: () = {
            fn _eurora_tools_check<T>()
            where
                T: ::eurora_tools::__private::serde::Serialize
                    + ::eurora_tools::__private::serde::de::DeserializeOwned
                    + ::eurora_tools::__private::schemars::JsonSchema,
            {}
            let _ = _eurora_tools_check::<#return_ty>;
        };
    };

    quote! {
        #args_assert
        #return_assert
    }
}

/// Output of `build_local_trait`: the public-facing ident the user
/// writes `impl … for` against (the Send-bounded variant after
/// `trait_variant::make` expansion) and the rewritten trait declaration
/// with `#[trait_variant::make]` attached.
///
/// The non-Send `…Local` source trait is renamed in-place inside
/// `tokens`; consumers only ever bind against `public_ident`.
struct LocalTrait {
    public_ident: syn::Ident,
    visibility: Visibility,
    tokens: TokenStream,
}

/// Rewrite the user-written trait so its public name is the
/// `Send`-bounded variant emitted by `trait_variant::make`. The trait's
/// source body keeps the user's bounds — `: Send + Sync` etc. — and is
/// renamed to `<Trait>Local`.
fn build_local_trait(mut item: ItemTrait, _adapter_attrs: &AdapterAttrs) -> LocalTrait {
    let public_ident = item.ident.clone();
    let local_ident = format_ident!("{}Local", public_ident);
    let visibility = item.vis.clone();

    item.ident = local_ident;
    let trait_variant_attr: Attribute = syn::parse_quote! {
        #[::eurora_tools::__private::trait_variant::make(#public_ident: Send)]
    };
    item.attrs.insert(0, trait_variant_attr);

    let tokens = quote! { #item };

    LocalTrait {
        public_ident,
        visibility,
        tokens,
    }
}

/// Strip a trailing `Adapter` suffix from a trait identifier.
///
/// `YoutubeAdapter` → `Youtube`, `Math` → `Math`. The suffix strip is
/// case-sensitive on the canonical `Adapter` form; permissively
/// accepting `ADAPTER` / `adapter` made the snake-case derivation
/// noisier than it was worth.
fn trait_stem(trait_ident: &syn::Ident) -> String {
    let raw = trait_ident.to_string();
    let stem = raw.strip_suffix("Adapter").unwrap_or(&raw);
    if stem.is_empty() {
        raw
    } else {
        stem.to_owned()
    }
}

/// `YoutubeAdapter` → `YOUTUBE_DESCRIPTORS`.
fn descriptor_table_ident(trait_ident: &syn::Ident) -> syn::Ident {
    let snake = to_snake_case(&trait_stem(trait_ident)).to_uppercase();
    format_ident!("{}_DESCRIPTORS", snake, span = trait_ident.span())
}

/// `YoutubeAdapter` → `YoutubeDispatcher`.
fn dispatcher_ident(trait_ident: &syn::Ident) -> syn::Ident {
    format_ident!(
        "{}Dispatcher",
        trait_stem(trait_ident),
        span = trait_ident.span()
    )
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0
                && let Some(prev) = out.chars().last()
                && prev != '_'
            {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn descriptor_table_ident_strips_adapter_suffix() {
        let ident: syn::Ident = parse_quote!(YoutubeAdapter);
        assert_eq!(
            descriptor_table_ident(&ident).to_string(),
            "YOUTUBE_DESCRIPTORS"
        );
    }

    #[test]
    fn descriptor_table_ident_handles_no_suffix() {
        let ident: syn::Ident = parse_quote!(Youtube);
        assert_eq!(
            descriptor_table_ident(&ident).to_string(),
            "YOUTUBE_DESCRIPTORS"
        );
    }

    #[test]
    fn descriptor_table_ident_camel_case_to_snake() {
        let ident: syn::Ident = parse_quote!(YoutubeWatchAdapter);
        assert_eq!(
            descriptor_table_ident(&ident).to_string(),
            "YOUTUBE_WATCH_DESCRIPTORS"
        );
    }

    #[test]
    fn dispatcher_ident_strips_adapter_suffix() {
        let ident: syn::Ident = parse_quote!(YoutubeAdapter);
        assert_eq!(dispatcher_ident(&ident).to_string(), "YoutubeDispatcher");
    }

    #[test]
    fn dispatcher_ident_handles_no_suffix() {
        let ident: syn::Ident = parse_quote!(Math);
        assert_eq!(dispatcher_ident(&ident).to_string(), "MathDispatcher");
    }

    #[test]
    fn snake_case_basic() {
        assert_eq!(to_snake_case("YoutubeAdapter"), "youtube_adapter");
        assert_eq!(to_snake_case("ABCAdapter"), "a_b_c_adapter");
        assert_eq!(to_snake_case("foo"), "foo");
    }
}
