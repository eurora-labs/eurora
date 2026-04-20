//! Base interfaces for agent-graph checkpoint savers.
//!
//! Rust rewrite of the `langgraph-checkpoint` Python library. Phase 2 lands
//! the serialization layer (`codec`) and [`CheckpointMetadata`]. The
//! `BaseCheckpointSaver` async trait, full [`Checkpoint`] struct,
//! `CheckpointTuple`, and `InMemorySaver` arrive in Phase 3.

pub mod codec;
pub mod errors;
pub mod metadata;

pub use codec::{JsonSerializer, MsgpackSerializer, Serializer, dumps, loads};
pub use errors::{Error, Result};
pub use metadata::{CheckpointMetadata, CheckpointSource};
