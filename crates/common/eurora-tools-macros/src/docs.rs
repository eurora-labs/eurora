//! Rustdoc extraction for trait methods inside `#[adapter]`.
//!
//! The first paragraph of the rustdoc on each tool method becomes the
//! LLM-facing `ToolDescriptor.description`. We extract it here so the
//! parsing rules are in one place and easy to test.

use syn::{Attribute, Expr, ExprLit, Lit, Meta};

/// Extract the first paragraph of a rustdoc-style attribute list.
///
/// "First paragraph" means: every `#[doc = "…"]` line from the start of
/// the attribute list until the first empty doc line (or the end of the
/// list). Leading single-space prefixes (from `///` rendering through
/// `rustdoc`) are stripped, the lines are joined with a single space, and
/// the result is trimmed.
///
/// Returns `None` when there is no doc content at all or when the
/// extracted text is empty after trimming. Callers turn that into a
/// `compile_error!` because the LLM-facing description is mandatory.
pub(crate) fn first_paragraph(attrs: &[Attribute]) -> Option<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut started = false;

    for attr in attrs.iter().filter(|a| a.path().is_ident("doc")) {
        let Meta::NameValue(nv) = &attr.meta else {
            continue;
        };
        let Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) = &nv.value
        else {
            continue;
        };
        let raw = s.value();
        let line = raw.strip_prefix(' ').unwrap_or(&raw).to_owned();
        let is_blank = line.trim().is_empty();

        if is_blank {
            if started {
                break;
            }
            // Skip leading blank doc lines without breaking the paragraph.
            continue;
        }
        started = true;
        lines.push(line);
    }

    let combined = lines.join(" ").trim().to_owned();
    if combined.is_empty() {
        None
    } else {
        Some(combined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    fn attrs(src: &str) -> Vec<Attribute> {
        let item: syn::TraitItemFn = syn::parse_str(&format!("{src}\nfn x(&self);")).unwrap();
        item.attrs
    }

    #[test]
    fn extracts_single_line_paragraph() {
        let a = attrs("/// hello world");
        assert_eq!(first_paragraph(&a).as_deref(), Some("hello world"));
    }

    #[test]
    fn joins_continuation_lines_with_single_space() {
        let a = attrs(
            "/// first line\n\
             /// second line",
        );
        assert_eq!(
            first_paragraph(&a).as_deref(),
            Some("first line second line")
        );
    }

    #[test]
    fn stops_at_first_blank_doc_line() {
        let a = attrs(
            "/// first paragraph\n\
             ///\n\
             /// second paragraph",
        );
        assert_eq!(first_paragraph(&a).as_deref(), Some("first paragraph"));
    }

    #[test]
    fn ignores_leading_blank_doc_lines() {
        let a = attrs(
            "///\n\
             /// real content",
        );
        assert_eq!(first_paragraph(&a).as_deref(), Some("real content"));
    }

    #[test]
    fn returns_none_for_no_doc_attrs() {
        let attrs: Vec<Attribute> = vec![parse_quote!(#[inline])];
        assert!(first_paragraph(&attrs).is_none());
    }

    #[test]
    fn returns_none_for_empty_doc() {
        let a = attrs("/// ");
        assert!(first_paragraph(&a).is_none());
    }

    #[test]
    fn strips_only_single_leading_space() {
        // rustdoc renders `///foo` (no space) verbatim; `/// foo` strips
        // exactly one space. Anything beyond that is significant indentation
        // and should be preserved.
        let a = attrs(
            "///  indented\n\
             ///also",
        );
        assert_eq!(first_paragraph(&a).as_deref(), Some("indented also"));
    }
}
