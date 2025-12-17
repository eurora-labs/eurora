//! Functional API module for LangGraph workflows.
//!
//! This module provides the `task` and `entrypoint` decorators
//! for defining LangGraph workflows using a functional approach.
//!
//! # Example
//!
//! ```ignore
//! use agent_graph::func::{task, entrypoint};
//!
//! #[task]
//! async fn process_data(input: String) -> String {
//!     input.to_uppercase()
//! }
//!
//! #[entrypoint]
//! async fn my_workflow(input: String) -> String {
//!     let result = process_data(input).await;
//!     result
//! }
//! ```

// Re-export the macros from agent-graph-macros
pub use agent_graph_macros::{entrypoint, task};
