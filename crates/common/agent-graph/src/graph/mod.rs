//! Graph module for LangGraph workflows.
//!
//! This module provides the core graph building blocks:
//! - `StateGraph` - A builder for creating stateful workflows
//! - `add_messages` - A reducer function for message lists
//! - `MessagesState` - A trait for states with messages

pub mod message;
pub mod state;

// Re-export commonly used types from submodules
pub use message::{HasId, MessagesState, add_messages};
pub use state::{CompiledGraph, GraphStructure, StateGraph};

// Re-export constants for convenience (matching langgraph's graph/__init__.py)
pub use crate::constants::{END, START};
