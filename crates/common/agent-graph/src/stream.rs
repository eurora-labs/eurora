//! Stream module for LangGraph workflows.
//!
//! This module provides streaming functionality for workflow execution.

use serde::{Deserialize, Serialize};

/// How the stream method should emit outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StreamMode {
    /// Emit all values in the state after each step, including interrupts.
    /// When used with functional API, values are emitted once at the end of the workflow.
    Values,
    /// Emit only the node or task names and updates returned by the nodes or tasks after each step.
    /// If multiple updates are made in the same step (e.g. multiple nodes are run) then those updates are emitted separately.
    #[default]
    Updates,
    /// Emit custom data using from inside nodes or tasks using `StreamWriter`.
    Custom,
    /// Emit LLM messages token-by-token together with metadata for any LLM invocations inside nodes or tasks.
    Messages,
    /// Emit an event when a checkpoint is created, in the same format as returned by `get_state()`.
    Checkpoints,
    /// Emit events when tasks start and finish, including their results and errors.
    Tasks,
    /// Emit `"checkpoints"` and `"tasks"` events for debugging purposes.
    Debug,
}

/// A chunk of data emitted by the stream.
#[derive(Debug, Clone)]
pub struct StreamChunk<T> {
    /// The name of the node or task that produced this chunk.
    pub node: String,
    /// The data produced by the node or task.
    pub data: T,
}

impl<T> StreamChunk<T> {
    /// Create a new stream chunk.
    pub fn new(node: impl Into<String>, data: T) -> Self {
        Self {
            node: node.into(),
            data,
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Display for StreamChunk<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StreamChunk({}: {:?})", self.node, self.data)
    }
}
