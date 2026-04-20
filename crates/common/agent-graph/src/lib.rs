//! Rust port of LangGraph — Pregel-based stateful workflows.
//!
//! See `plan.md` in the repository root for the implementation roadmap.
//! Most modules are placeholders at this stage; only [`constants`] and
//! [`errors`] are populated.

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

pub use constants::{END, START, TAG_HIDDEN, TAG_NOSTREAM};
pub use errors::{Error, ErrorCode, Result};
