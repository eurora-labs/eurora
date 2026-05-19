//! YouTube adapter for Eurora's unified tool-execution architecture.
//!
//! This crate declares the [`YoutubeAdapter`] trait — the client-side
//! contract for tools that operate on the YouTube tab the user is
//! currently watching — together with the argument and return types
//! every method exchanges with the agent loop.
//!
//! The `#[adapter]` macro in [`adapter`](self::adapter) generates a
//! static [`YOUTUBE_DESCRIPTORS`] table and a [`YoutubeDispatcher<T>`]
//! around any user-supplied [`YoutubeAdapter`] implementation. The
//! server never instantiates the trait; it consumes
//! `YOUTUBE_DESCRIPTORS` (via
//! [`ToolDescriptor::to_wire`](eurora_tools::ToolDescriptor::to_wire))
//! and routes calls through [`eurora_tools::RemoteToolBus`]. The
//! client-side bridge implementation lands in `euro-thread`'s
//! `YoutubeBridgeImpl` (Phase 8 of the plan).

mod adapter;
mod types;

pub use adapter::{YOUTUBE_DESCRIPTORS, YoutubeAdapter, YoutubeAdapterLocal, YoutubeDispatcher};
pub use types::{CapturedFrame, CurrentTimestamp, Transcript, TranscriptEntry};
