//! Attribute parsing for `#[adapter(...)]` and `#[tool(...)]`.
//!
//! Both attributes use `meta = value` syntax via `syn`'s
//! `parse_nested_meta` helper, so they integrate cleanly with the
//! existing attribute-syntax machinery and surface span-anchored errors
//! automatically.

use proc_macro2::{Span, TokenStream};
use syn::meta::ParseNestedMeta;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{Expr, ExprArray, ExprLit, Lit, LitBool, LitInt, LitStr, Result, Token};

use crate::source::SourceKind;

/// Parsed `#[adapter(...)]` attribute arguments.
#[derive(Debug)]
pub(crate) struct AdapterAttrs {
    /// `namespace = "browser::youtube"` — required.
    pub(crate) namespace: LitStr,
    /// `version = 1` — optional. Parsed for forward compatibility but
    /// currently unused.
    #[allow(dead_code)]
    pub(crate) version: Option<LitInt>,
}

impl AdapterAttrs {
    pub(crate) fn parse(attr: TokenStream) -> Result<Self> {
        let mut namespace: Option<LitStr> = None;
        let mut version: Option<LitInt> = None;

        let parser = syn::meta::parser(|meta| {
            if meta.path.is_ident("namespace") {
                namespace = Some(meta.value()?.parse::<LitStr>()?);
            } else if meta.path.is_ident("version") {
                version = Some(meta.value()?.parse::<LitInt>()?);
            } else {
                return Err(
                    meta.error("unknown #[adapter] argument; expected `namespace` or `version`")
                );
            }
            Ok(())
        });

        parser.parse2(attr)?;

        let namespace = namespace.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "#[adapter] requires `namespace = \"…\"`")
        })?;

        // Reject empty / pathological namespaces early so the emitted
        // tool name (`{namespace}::{method}`) is always well-formed.
        let ns_value = namespace.value();
        if ns_value.trim().is_empty() {
            return Err(syn::Error::new(
                namespace.span(),
                "#[adapter] `namespace` must be non-empty",
            ));
        }
        if ns_value.contains(char::is_whitespace) {
            return Err(syn::Error::new(
                namespace.span(),
                "#[adapter] `namespace` must not contain whitespace",
            ));
        }

        Ok(Self { namespace, version })
    }
}

/// Parsed `#[tool(...)]` attribute arguments attached to a trait
/// method.
#[derive(Debug)]
pub(crate) struct ToolAttrs {
    pub(crate) timeout_ms: u64,
    pub(crate) source_lit: LitStr,
    pub(crate) source: SourceKind,
    pub(crate) required_contexts: Vec<LitStr>,
    pub(crate) requires_user_approval: bool,
}

impl ToolAttrs {
    /// Parse the `Punctuated<Meta, ,>` payload of `#[tool(...)]`.
    ///
    /// Repeated `requires_context = "…"` entries accumulate; array form
    /// (`requires_context = ["a", "b"]`) is also accepted for ergonomics.
    /// Duplicate keys (apart from `requires_context`) are an error.
    pub(crate) fn parse(attr: TokenStream) -> Result<Self> {
        let mut timeout_ms_lit: Option<LitInt> = None;
        let mut source_lit: Option<LitStr> = None;
        let mut required_contexts: Vec<LitStr> = Vec::new();
        let mut requires_user_approval: Option<LitBool> = None;

        let parser = syn::meta::parser(|meta: ParseNestedMeta<'_>| {
            if meta.path.is_ident("timeout_ms") {
                if timeout_ms_lit.is_some() {
                    return Err(meta.error("`timeout_ms` specified more than once"));
                }
                timeout_ms_lit = Some(meta.value()?.parse::<LitInt>()?);
            } else if meta.path.is_ident("source") {
                if source_lit.is_some() {
                    return Err(meta.error("`source` specified more than once"));
                }
                source_lit = Some(meta.value()?.parse::<LitStr>()?);
            } else if meta.path.is_ident("requires_context") {
                parse_context_value(meta, &mut required_contexts)?;
            } else if meta.path.is_ident("requires_user_approval") {
                if requires_user_approval.is_some() {
                    return Err(meta.error("`requires_user_approval` specified more than once"));
                }
                requires_user_approval = Some(meta.value()?.parse::<LitBool>()?);
            } else {
                return Err(meta.error(
                    "unknown #[tool] argument; expected one of \
                     `timeout_ms`, `source`, `requires_context`, \
                     `requires_user_approval`",
                ));
            }
            Ok(())
        });

        parser.parse2(attr)?;

        let timeout_ms = timeout_ms_lit
            .ok_or_else(|| {
                syn::Error::new(Span::call_site(), "#[tool] requires `timeout_ms = <ms>`")
            })?
            .base10_parse::<u64>()?;
        let source_lit = source_lit.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "#[tool] requires `source = \"…\"`")
        })?;

        let source = SourceKind::parse(&source_lit)?;

        // Dedupe required_contexts while preserving declaration order.
        let mut seen = std::collections::BTreeSet::new();
        let required_contexts = required_contexts
            .into_iter()
            .filter(|lit| seen.insert(lit.value()))
            .collect();

        Ok(Self {
            timeout_ms,
            source_lit,
            source,
            required_contexts,
            requires_user_approval: requires_user_approval.map(|b| b.value()).unwrap_or(false),
        })
    }
}

fn parse_context_value(meta: ParseNestedMeta<'_>, out: &mut Vec<LitStr>) -> Result<()> {
    let value = meta.value()?;
    // Peek at the lookahead to decide between scalar and array forms
    // without consuming tokens we can't put back.
    let expr: Expr = value.parse()?;
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => {
            push_context(out, s)?;
        }
        Expr::Array(ExprArray { elems, .. }) => {
            if elems.is_empty() {
                return Err(syn::Error::new_spanned(
                    elems,
                    "`requires_context` array must contain at least one key",
                ));
            }
            for elem in elems {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = elem
                {
                    push_context(out, s)?;
                } else {
                    return Err(syn::Error::new_spanned(
                        elem,
                        "`requires_context` array entries must be string literals",
                    ));
                }
            }
        }
        other => {
            return Err(syn::Error::new_spanned(
                other,
                "`requires_context` must be a string literal or `[\"…\", …]` array",
            ));
        }
    }
    Ok(())
}

fn push_context(out: &mut Vec<LitStr>, lit: LitStr) -> Result<()> {
    let value = lit.value();
    if value.trim().is_empty() {
        return Err(syn::Error::new(lit.span(), "context key must be non-empty"));
    }
    if value.contains(char::is_whitespace) {
        return Err(syn::Error::new(
            lit.span(),
            "context key must not contain whitespace",
        ));
    }
    out.push(lit);
    Ok(())
}

/// Convenience parser used by the `Parse` impl on a one-shot helper in
/// `tool.rs` — keeps the surface API uniform between the two macros.
#[allow(dead_code)]
pub(crate) struct CommaSeparated<T: Parse>(pub Vec<T>);

impl<T: Parse> Parse for CommaSeparated<T> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let punct: Punctuated<T, Token![,]> = Punctuated::parse_terminated(input)?;
        Ok(Self(punct.into_iter().collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn adapter_parses_namespace_and_version() {
        let attr = quote! { namespace = "browser::youtube", version = 1 };
        let parsed = AdapterAttrs::parse(attr).unwrap();
        assert_eq!(parsed.namespace.value(), "browser::youtube");
        assert!(parsed.version.is_some());
    }

    #[test]
    fn adapter_requires_namespace() {
        let attr = quote! { version = 2 };
        let err = AdapterAttrs::parse(attr).unwrap_err();
        assert!(err.to_string().contains("requires `namespace"));
    }

    #[test]
    fn adapter_rejects_whitespace_namespace() {
        let attr = quote! { namespace = "browser youtube" };
        let err = AdapterAttrs::parse(attr).unwrap_err();
        assert!(err.to_string().contains("whitespace"));
    }

    #[test]
    fn tool_parses_full_form() {
        let attr = quote! {
            timeout_ms = 2_000,
            source = "bridge(browser)",
            requires_context = "youtube::watch_page",
            requires_user_approval = false,
        };
        let parsed = ToolAttrs::parse(attr).unwrap();
        assert_eq!(parsed.timeout_ms, 2000);
        assert_eq!(parsed.source_lit.value(), "bridge(browser)");
        assert_eq!(parsed.required_contexts.len(), 1);
        assert!(!parsed.requires_user_approval);
    }

    #[test]
    fn tool_supports_array_contexts() {
        let attr = quote! {
            timeout_ms = 1,
            source = "client_local",
            requires_context = ["a::b", "c::d"],
        };
        let parsed = ToolAttrs::parse(attr).unwrap();
        let names: Vec<String> = parsed.required_contexts.iter().map(LitStr::value).collect();
        assert_eq!(names, vec!["a::b".to_string(), "c::d".to_string()]);
    }

    #[test]
    fn tool_dedupes_repeated_contexts() {
        let attr = quote! {
            timeout_ms = 1,
            source = "client_local",
            requires_context = "a",
            requires_context = "a",
            requires_context = ["b", "a"],
        };
        let parsed = ToolAttrs::parse(attr).unwrap();
        let names: Vec<String> = parsed.required_contexts.iter().map(LitStr::value).collect();
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn tool_rejects_unknown_arg() {
        let attr = quote! {
            timeout_ms = 1,
            source = "client_local",
            wibble = true,
        };
        let err = ToolAttrs::parse(attr).unwrap_err();
        assert!(err.to_string().contains("unknown #[tool] argument"));
    }

    #[test]
    fn tool_requires_timeout_and_source() {
        let attr_no_timeout = quote! { source = "client_local" };
        assert!(
            ToolAttrs::parse(attr_no_timeout)
                .unwrap_err()
                .to_string()
                .contains("`timeout_ms")
        );
        let attr_no_source = quote! { timeout_ms = 1 };
        assert!(
            ToolAttrs::parse(attr_no_source)
                .unwrap_err()
                .to_string()
                .contains("`source")
        );
    }

    #[test]
    fn tool_rejects_empty_context_key() {
        let attr = quote! {
            timeout_ms = 1,
            source = "client_local",
            requires_context = "",
        };
        let err = ToolAttrs::parse(attr).unwrap_err();
        assert!(err.to_string().contains("non-empty"));
    }
}
