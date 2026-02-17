//! Ollama provider integration.
//!
//! Matches Python `langchain_ollama` package structure.

mod chat_models;
mod compat;
mod utils;

pub use chat_models::*;
pub use utils::{merge_auth_headers, parse_url_with_auth, validate_model};
