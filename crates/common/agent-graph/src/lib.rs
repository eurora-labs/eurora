//! Rust port of LangGraph — Pregel-based stateful workflows.
//!
//! See `plan.md` in the repository root for the implementation roadmap.
//! Phase 0 populated [`constants`] + [`errors`]; Phase 1 shipped
//! [`channels`]; Phase 2 covers [`types`] and [`config`]. Later phases
//! replace the remaining placeholders.

pub mod channels;
pub mod config;
pub mod constants;
pub mod errors;
pub mod func;
pub mod graph;
pub mod managed;
pub mod pregel;
pub mod runtime;
pub mod types;

pub use config::RunnableConfig;
pub use constants::{END, START, TAG_HIDDEN, TAG_NOSTREAM};
pub use errors::{Error, ErrorCode, Result};
pub use types::{
    CachePolicy, Command, Durability, GoTo, Interrupt, Overwrite, PARENT, PregelTask, RetryPolicy,
    RetryPredicate, Send, StateSnapshot, StreamMode, TaskPath, TaskState,
};
