//! Agent Graph - A Rust implementation of LangGraph-style workflows.
//!
//! This crate provides a framework for building stateful, multi-actor agents
//! similar to Python's LangGraph library.
//!
//! # Overview
//!
//! The library supports two main APIs:
//!
//! 1. **Graph API** - Build workflows using `StateGraph` with nodes and edges
//! 2. **Functional API** - Build workflows using `#[task]` and `#[entrypoint]` decorators
//!
//! # Example: Graph API
//!
//! ```ignore
//! use agent_graph::graph::{StateGraph, START, END};
//!
//! #[derive(Clone)]
//! struct State {
//!     text: String,
//! }
//!
//! let mut graph = StateGraph::<State>::new();
//!
//! graph.add_node("node_a", |mut state| async move {
//!     state.text.push_str("a");
//!     state
//! });
//!
//! graph.add_node("node_b", |mut state| async move {
//!     state.text.push_str("b");
//!     state
//! });
//!
//! graph.add_edge(START, "node_a");
//! graph.add_edge("node_a", "node_b");
//! graph.add_edge("node_b", END);
//!
//! let compiled = graph.compile();
//! let result = compiled.invoke(State { text: String::new() }).await;
//! // result.text == "ab"
//! ```
//!
//! # Example: Functional API
//!
//! ```ignore
//! use agent_graph::func::{task, entrypoint};
//! use agent_graph::stream::StreamMode;
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

pub mod checkpoint;
pub mod constants;
pub mod func;
pub mod graph;
pub mod stream;
pub mod types;

pub use checkpoint::InMemorySaver;
pub use constants::{END, START};
pub use func::{
    Entrypoint, EntrypointBuilder, Final, RunConfig, Task, TaskError, TaskFuture,
    create_entrypoint, create_task, entrypoint, task,
};
pub use graph::{CompiledGraph, MessagesState, StateGraph, add_messages};
pub use stream::{StreamChunk, StreamMode};
pub use types::{CachePolicy, Command, Interrupt, RetryPolicy, Send};
