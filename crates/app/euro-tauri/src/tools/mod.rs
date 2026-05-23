//! Client-side tool adapter wiring.
//!
//! Adapter traits (the `*Adapter` declarations consumed by the macro)
//! live in `eurora-tools-browser`; their bridge-backed implementations
//! live in `euro-bridge-adapters`. This module is the desktop's entry
//! point for handing each implementation off to the framework's
//! [`Catalog`](eurora_tools::Catalog) — see `main.rs` for the
//! registration call site.
//!
//! New transports follow the same shape: declare the trait in a
//! `crates/common/tools/eurora-tools-*` crate, ship the bridge-backed
//! impl in `euro-bridge-adapters` (or its successor for non-bridge
//! transports), and add a re-export here.

pub use euro_bridge_adapters::browser::{WebBridgeImpl, YoutubeBridgeImpl};
