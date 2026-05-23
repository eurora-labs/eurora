//! Browser-transport adapters for Eurora's unified tool-execution
//! architecture.
//!
//! Every tool the desktop routes through the browser bridge lives in
//! this crate, grouped by adapter namespace:
//!
//! - [`youtube`] — tools for the YouTube tab the user is currently
//!   watching ([`YoutubeAdapter`](youtube::YoutubeAdapter)).
//! - [`web`] — tools that operate on whichever `http(s)` tab the user
//!   has focused ([`WebAdapter`](web::WebAdapter)).
//!
//! Each module exports a `#[adapter]`-macro-generated descriptor table
//! (`YOUTUBE_DESCRIPTORS`, `WEB_DESCRIPTORS`) consumed by the
//! server-side agent loop via
//! [`ToolDescriptor::to_wire`](eurora_tools::ToolDescriptor::to_wire),
//! and a dispatcher (`YoutubeDispatcher<T>`, `WebDispatcher<T>`) the
//! desktop wraps around any concrete trait impl before registering it
//! with the framework's [`Catalog`](eurora_tools::Catalog).
//!
//! The crate is intentionally transport-agnostic: it carries the trait
//! declarations, argument/return types, and macro-emitted glue, but
//! never knows about the bridge wire protocol. The bridge-backed
//! implementations of these traits live in `euro-bridge-adapters` on
//! the app side — keeping the dependency graph strictly common → app,
//! never the other way around.

pub mod web;
pub mod youtube;
