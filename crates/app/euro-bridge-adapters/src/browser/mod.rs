//! Bridge-backed implementations of the browser adapter traits declared
//! in [`eurora_tools_browser`].
//!
//! Each per-namespace submodule:
//!
//! - Defines the `*_*` bridge action constants the browser extension
//!   routes on (Phase 1 keeps the legacy `YOUTUBE_GET_TRANSCRIPT`-style
//!   strings; Phase 2 will flip them to the dotted action format).
//! - Implements the corresponding `*Adapter` trait against a shared
//!   [`crate::BridgeClient`], delegating every method to
//!   [`crate::BridgeClient::call_action`].
//!
//! New site adapters add a sibling submodule plus a `pub mod` line
//! below — no other wiring required.

pub mod web;
pub mod youtube;

pub use web::{
    WEB_GET_ACCESSIBILITY_TREE, WEB_GET_PAGE_METADATA, WEB_GET_READABILITY_ARTICLE,
    WEB_GET_SELECTED_TEXT, WEB_INSERT_TEXT, WEB_LIST_FORM_INPUTS, WEB_LIST_LINKS,
    WEB_QUERY_SELECTOR, WebBridgeImpl,
};
pub use youtube::{
    YOUTUBE_GET_CURRENT_FRAME, YOUTUBE_GET_CURRENT_TIMESTAMP, YOUTUBE_GET_TRANSCRIPT,
    YoutubeBridgeImpl,
};
