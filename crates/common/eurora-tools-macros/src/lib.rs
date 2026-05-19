//! Procedural macros backing the `eurora-tools` adapter framework.
//!
//! This crate is an implementation detail. Adapter authors should
//! import `adapter` and `tool` from [`eurora_tools`], which re-exports
//! both — that way the macro emissions and the runtime they reference
//! version together.
//!
//! [`eurora_tools`]: https://docs.rs/eurora-tools

mod adapter;
mod attrs;
mod docs;
mod signature;
mod source;
mod tool;

use proc_macro::TokenStream;

/// Mark a trait as a tool adapter.
///
/// The macro consumes the trait declaration and emits:
///
/// 1. The trait itself, rewritten so the public name is the
///    `Send`-bounded variant produced by `trait_variant::make`. The
///    original trait body becomes the `…Local` source variant; the
///    blanket impl `impl<T: <Trait>> <Trait>Local for T` makes the
///    Send variant satisfy either bound.
/// 2. A `pub struct <Trait>Dispatcher<T: <Trait> + 'static>` with `new`
///    and `from_arc` constructors, plus an `impl Dispatcher` whose
///    `dispatch` method matches descriptor names, verifies the
///    [`eurora_tools::Origin`] variant, decodes the JSON arguments,
///    awaits the user-written adapter method, and encodes the result.
/// 3. A `pub static <UPPER>_DESCRIPTORS: LazyLock<[ToolDescriptor; N]>`
///    table populated from `#[tool(...)]` metadata on each method. The
///    symbol name is derived from the trait identifier with a trailing
///    `Adapter` stripped, then converted to `SHOUTY_SNAKE_CASE`
///    (`YoutubeAdapter` → `YOUTUBE_DESCRIPTORS`).
/// 4. A `const _: () = { ... };` block per method that asserts the
///    argument and return types implement
///    `Serialize + DeserializeOwned + JsonSchema`. Span anchored on the
///    user's types so errors point at the offending declaration.
///
/// # Adapter attribute syntax
///
/// ```ignore
/// #[adapter(namespace = "browser::youtube", version = 1)]
/// pub trait YoutubeAdapter: Send + Sync { … }
/// ```
///
/// - `namespace` — required; the tool name for each method becomes
///   `"{namespace}::{method_ident}"`.
/// - `version` — optional `u32`; parsed and currently ignored
///   (reserved for future cache-busting).
///
/// # Tool attribute syntax
///
/// Inside the trait, each method that should become a tool carries a
/// `#[tool(...)]` attribute:
///
/// ```ignore
/// /// First paragraph is taken verbatim as the LLM-facing description.
/// ///
/// /// Subsequent paragraphs are ignored by the macro.
/// #[tool(
///     timeout_ms = 2_000,
///     source = "bridge(browser)",
///     requires_context = "youtube::watch_page",
///     requires_user_approval = false,
/// )]
/// async fn get_current_timestamp(
///     &self,
///     target: &BrowserOrigin,
///     args: Empty,
/// ) -> Result<CurrentTimestamp, ToolError>;
/// ```
///
/// - `timeout_ms` — required `u64` literal in milliseconds.
/// - `source` — required string. One of `"bridge(<kind>)"`,
///   `"client_local"`, `"server_local"`, `"acp"`.
/// - `requires_context` — optional. A single `"key"` string or a
///   `["key", …]` array; repeatable. Aggregated into the descriptor's
///   `required_contexts` slice in declaration order, deduplicated.
/// - `requires_user_approval` — optional `bool`, defaults to `false`.
///
/// The `source` attribute determines the method's required target type:
///
/// | `source`            | Target parameter type |
/// | ------------------- | --------------------- |
/// | `"bridge(browser)"` | `&BrowserOrigin`      |
/// | `"bridge(<other>)"` | `&FocusedOrigin`      |
/// | `"acp"`             | `&AcpOrigin`          |
/// | `"client_local"`    | *(no target)*         |
/// | `"server_local"`    | *(no target)*         |
///
/// A mismatch between the declared `source` and the method's actual
/// signature is a compile error, with the diagnostic spanned on the
/// offending token.
#[proc_macro_attribute]
pub fn adapter(attr: TokenStream, item: TokenStream) -> TokenStream {
    adapter::expand(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Marks a trait method inside an `#[adapter]` trait as a tool.
///
/// This attribute is consumed (and stripped) by `#[adapter]` before it
/// ever expands. If the macro fires standalone — e.g. the user forgot
/// to put `#[adapter]` on the enclosing trait — it emits a
/// `compile_error!` guiding them toward the correct usage.
///
/// See the [`macro@adapter`] documentation for the full attribute
/// schema.
#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    tool::expand(attr.into(), item.into()).into()
}
