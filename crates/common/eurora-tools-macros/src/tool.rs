//! The `#[tool]` attribute.
//!
//! In a correctly-written program, `#[tool]` is consumed and stripped by
//! the enclosing `#[adapter]` macro before it ever expands. The macro
//! itself only fires when the user forgot to put `#[adapter]` on the
//! trait — and in that case we emit a `compile_error!` that walks them
//! to the right shape.

use proc_macro2::TokenStream;
use quote::quote;

pub(crate) fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let error = syn::Error::new(
        proc_macro2::Span::call_site(),
        "#[tool] is only valid inside an `#[adapter]` trait body; \
         attach `#[adapter(namespace = \"…\")]` to the enclosing trait",
    )
    .into_compile_error();
    quote! {
        #error
        #item
    }
}
