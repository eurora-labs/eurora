//! Client-side tool adapter wiring.
//!
//! The actual adapter implementations live in their respective
//! `eurora-tools-*` crates so non-Tauri consumers (e.g. `euro-activity`)
//! can use them too. This module is the desktop's entry point for
//! constructing each adapter against the shared [`BridgeService`] and
//! handing the resulting dispatcher to the framework's
//! [`Catalog`](eurora_tools::Catalog).
//!
//! New adapters add a re-export here, depend on the corresponding
//! adapter crate with its `"bridge"` feature, and register a dispatcher
//! in `main.rs`.

pub use eurora_tools_web::WebBridgeImpl;
pub use eurora_tools_youtube::YoutubeBridgeImpl;
