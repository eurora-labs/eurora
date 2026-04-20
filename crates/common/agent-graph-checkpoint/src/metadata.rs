//! Checkpoint metadata surfaced to callers of `get_state`/`stream`.
//!
//! The full [`crate::Checkpoint`] struct and the `BaseCheckpointSaver` trait
//! land in Phase 3; [`CheckpointMetadata`] is defined here because Phase 2's
//! `StateSnapshot` references it. Keeping only the metadata type in this crate
//! avoids dragging the rest of the saver machinery forward prematurely.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Provenance tag for a checkpoint.
///
/// Mirrors the `Literal["input", "loop", "update", "fork"]` on Python's
/// `CheckpointMetadata.source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckpointSource {
    /// Checkpoint taken immediately after `invoke`/`stream` ingested input.
    Input,
    /// Checkpoint taken at the end of a Pregel super-step.
    Loop,
    /// Checkpoint produced by an explicit `update_state` call.
    Update,
    /// Checkpoint created by forking a prior checkpoint into a new thread.
    Fork,
}

/// User-visible metadata attached to a checkpoint.
///
/// All fields are optional to match Python's `TypedDict(total=False)` —
/// checkpoints produced by older versions or by user code may omit any of
/// them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<CheckpointSource>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<i64>,

    /// Parent checkpoint ids keyed by checkpoint namespace.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub parents: HashMap<String, String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}
