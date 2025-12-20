//! Constants for LangGraph workflows.
//!
//! This module provides constants used throughout the graph execution.

/// The first (virtual) node in a graph.
/// Used to indicate where the graph execution should start.
pub const START: &str = "__start__";

/// The last (virtual) node in a graph.
/// Used to indicate where the graph execution should end.
pub const END: &str = "__end__";

/// Tag to disable streaming for a node.
pub const TAG_NOSTREAM: &str = "nostream";

/// Tag to hide a node/edge from certain tracing/streaming environments.
pub const TAG_HIDDEN: &str = "langsmith:hidden";
