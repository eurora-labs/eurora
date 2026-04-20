//! Core types (`Interrupt`, `Command`, `Send`, ...).
//!
//! Most types land in Phase 2. For now this module only defines
//! [`Overwrite`], which [`crate::channels::BinaryOperatorAggregate`]
//! consumes to bypass its reducer.

use serde::{Deserialize, Serialize};

use crate::channels::Value;
use crate::constants::internal::OVERWRITE;

/// Bypass a reducer and write the wrapped value directly to a
/// [`crate::channels::BinaryOperatorAggregate`] channel.
///
/// Mirrors `langgraph.types.Overwrite`. Receiving multiple `Overwrite`
/// values for the same channel in a single super-step raises
/// [`crate::errors::Error::InvalidUpdate`]. Serialises to
/// `{"__overwrite__": value}` so Rust- and Python-produced payloads are
/// on-the-wire compatible.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Overwrite {
    #[serde(rename = "__overwrite__")]
    pub value: Value,
}

impl Overwrite {
    pub fn new(value: Value) -> Self {
        Self { value }
    }

    /// Detect the dict-form overwrite marker used on the Python side.
    ///
    /// Python encodes an overwrite either with an `Overwrite` instance or a
    /// dict `{"__overwrite__": value}`. Rust callers can construct an
    /// [`Overwrite`] directly, but incoming JSON values may carry the dict
    /// form (e.g. from a Python-written checkpoint), so the channel layer
    /// must recognise it.
    pub(crate) fn from_value(value: &Value) -> Option<Value> {
        let object = value.as_object()?;
        if object.len() == 1
            && let Some(inner) = object.get(OVERWRITE)
        {
            return Some(inner.clone());
        }
        None
    }
}
