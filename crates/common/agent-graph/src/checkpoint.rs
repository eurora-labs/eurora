//! Checkpoint module for persisting agent state across invocations.
//!
//! This module provides checkpointing capabilities similar to LangGraph's
//! checkpointer system, allowing agents to maintain thread history
//! and resume from previous states.

pub mod memory;

pub use memory::InMemorySaver;
