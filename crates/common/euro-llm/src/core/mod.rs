//! Core traits and types for the LLM library ecosystem.
//!
//! This crate provides the foundational abstractions that all LLM providers
//! implement, including traits for chat, completion, streaming, and tool calling,
//! as well as standardized request/response types and error handling.

pub mod config;
pub mod error;
pub mod traits;
pub mod types;
#[cfg(feature = "dynamic-image")]
mod util;

// Re-export core types for convenience
pub use config::*;
pub use error::*;
pub use traits::*;
pub use types::*;

// External dependencies
pub use async_trait::async_trait;
pub use chrono::{DateTime, Utc};
pub use futures::Stream;
pub use serde::{Deserialize, Serialize};
pub use serde_json::Value;
pub use std::collections::HashMap;
pub use std::error::Error;
pub use std::time::Duration;
