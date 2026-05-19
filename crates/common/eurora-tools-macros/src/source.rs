//! Parse and reason about a `#[tool(source = "…")]` declaration.
//!
//! The `source` attribute drives both the wire-side `ToolSource` value
//! emitted into the descriptor and the static-type-check the macro
//! performs on the trait method's signature. Both concerns live here so
//! we can reuse the parsed form across modules.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{LitStr, Result};

/// Parsed and validated `source = "…"` value.
///
/// The string form is intentionally tiny so users can write it inline in
/// the macro attribute. Anything more structured (per-app config, etc.)
/// can layer on top of these variants later without breaking the wire
/// shape.
#[derive(Debug, Clone)]
pub(crate) enum SourceKind {
    /// `bridge(<app_kind>)` — routes via `euro-bridge` to a registered
    /// app. The inner `String` is the `app_kind` (e.g. `"browser"`).
    Bridge(String),
    /// `client_local` — in-process on the client.
    ClientLocal,
    /// `server_local` — in-process on the backend.
    ServerLocal,
    /// `acp` — piped through an ACP session.
    Acp,
}

/// What target parameter (if any) a tool method must declare.
///
/// The `&'static str` carries the expected reference-type rendering
/// (e.g. `"&BrowserOrigin"`) for inclusion in compile-error messages.
/// The variant name (`"Browser"`, …) drives the runtime `Origin` match
/// in the dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetKind {
    Browser,
    Focused,
    Acp,
    None,
}

impl TargetKind {
    /// Diagnostic rendering of the expected `&Type`.
    pub(crate) fn expected_type_repr(self) -> &'static str {
        match self {
            TargetKind::Browser => "&::eurora_tools::BrowserOrigin",
            TargetKind::Focused => "&::eurora_tools::FocusedOrigin",
            TargetKind::Acp => "&::eurora_tools::AcpOrigin",
            TargetKind::None => "(no target parameter)",
        }
    }

    /// The `Origin` variant ident the dispatcher destructures against.
    pub(crate) fn origin_variant(self) -> Option<&'static str> {
        match self {
            TargetKind::Browser => Some("Browser"),
            TargetKind::Focused => Some("Focused"),
            TargetKind::Acp => Some("Acp"),
            TargetKind::None => None,
        }
    }

    /// The last segment of the target type the user must write (without
    /// the leading `&`). Used by the signature checker to compare against
    /// the user's declared type.
    pub(crate) fn expected_type_ident(self) -> Option<&'static str> {
        match self {
            TargetKind::Browser => Some("BrowserOrigin"),
            TargetKind::Focused => Some("FocusedOrigin"),
            TargetKind::Acp => Some("AcpOrigin"),
            TargetKind::None => None,
        }
    }
}

impl SourceKind {
    /// Parse the string passed as `source = "…"`.
    ///
    /// `lit` is preserved as the diagnostic span so unknown values point
    /// directly at the user's string literal.
    pub(crate) fn parse(lit: &LitStr) -> Result<Self> {
        let raw = lit.value();
        let trimmed = raw.trim();

        if let Some(inner) = trimmed
            .strip_prefix("bridge(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let app_kind = inner.trim();
            if app_kind.is_empty() {
                return Err(syn::Error::new(
                    lit.span(),
                    "`bridge(...)` requires a non-empty app kind, e.g. `bridge(browser)`",
                ));
            }
            // Reject nested parentheses or whitespace — they indicate a
            // typo, not a meaningful kind.
            if app_kind.contains(['(', ')', ' ', '\t']) {
                return Err(syn::Error::new(
                    lit.span(),
                    format!("invalid app kind `{app_kind}` inside `bridge(...)`"),
                ));
            }
            return Ok(SourceKind::Bridge(app_kind.to_owned()));
        }

        match trimmed {
            "client_local" => Ok(SourceKind::ClientLocal),
            "server_local" => Ok(SourceKind::ServerLocal),
            "acp" => Ok(SourceKind::Acp),
            other => Err(syn::Error::new(
                lit.span(),
                format!(
                    "unknown `source` kind `{other}`; expected one of \
                     `bridge(<kind>)`, `client_local`, `server_local`, `acp`"
                ),
            )),
        }
    }

    /// The target type a method using this source must declare.
    pub(crate) fn target_kind(&self) -> TargetKind {
        match self {
            SourceKind::Bridge(app_kind) if app_kind == "browser" => TargetKind::Browser,
            SourceKind::Bridge(_) => TargetKind::Focused,
            SourceKind::Acp => TargetKind::Acp,
            SourceKind::ClientLocal | SourceKind::ServerLocal => TargetKind::None,
        }
    }

    /// Emit the `ToolSource` literal embedded in the descriptor table.
    ///
    /// `ToolSource::Bridge` carries `String`, which isn't `const`, so the
    /// macro always wraps the descriptor table in a `LazyLock`; this
    /// emission relies on that.
    pub(crate) fn to_tool_source_expr(&self, source_span: proc_macro2::Span) -> TokenStream {
        match self {
            SourceKind::Bridge(app_kind) => {
                let app_kind_lit = LitStr::new(app_kind, source_span);
                quote! {
                    ::eurora_tools::ToolSource::Bridge {
                        app_kind: ::std::string::String::from(#app_kind_lit),
                    }
                }
            }
            SourceKind::ClientLocal => {
                quote! { ::eurora_tools::ToolSource::ClientLocal }
            }
            SourceKind::ServerLocal => {
                quote! { ::eurora_tools::ToolSource::ServerLocal }
            }
            SourceKind::Acp => {
                quote! { ::eurora_tools::ToolSource::Acp }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn lit(s: &str) -> LitStr {
        parse_quote!(#s)
    }

    #[test]
    fn parses_bridge_browser() {
        let s = SourceKind::parse(&lit("bridge(browser)")).unwrap();
        match s {
            SourceKind::Bridge(k) => assert_eq!(k, "browser"),
            other => panic!("expected Bridge, got {other:?}"),
        }
    }

    #[test]
    fn parses_bridge_other_kind() {
        let s = SourceKind::parse(&lit("bridge(focused)")).unwrap();
        match s {
            SourceKind::Bridge(k) => assert_eq!(k, "focused"),
            other => panic!("expected Bridge, got {other:?}"),
        }
    }

    #[test]
    fn parses_unit_kinds() {
        assert!(matches!(
            SourceKind::parse(&lit("client_local")).unwrap(),
            SourceKind::ClientLocal
        ));
        assert!(matches!(
            SourceKind::parse(&lit("server_local")).unwrap(),
            SourceKind::ServerLocal
        ));
        assert!(matches!(
            SourceKind::parse(&lit("acp")).unwrap(),
            SourceKind::Acp
        ));
    }

    #[test]
    fn rejects_unknown_kind() {
        let err = SourceKind::parse(&lit("rocketship")).unwrap_err();
        assert!(err.to_string().contains("unknown `source` kind"));
    }

    #[test]
    fn rejects_empty_bridge() {
        let err = SourceKind::parse(&lit("bridge()")).unwrap_err();
        assert!(err.to_string().contains("non-empty app kind"));
    }

    #[test]
    fn target_kind_dispatch() {
        assert_eq!(
            SourceKind::Bridge("browser".into()).target_kind(),
            TargetKind::Browser
        );
        assert_eq!(
            SourceKind::Bridge("focused".into()).target_kind(),
            TargetKind::Focused
        );
        assert_eq!(SourceKind::Acp.target_kind(), TargetKind::Acp);
        assert_eq!(SourceKind::ClientLocal.target_kind(), TargetKind::None);
        assert_eq!(SourceKind::ServerLocal.target_kind(), TargetKind::None);
    }
}
