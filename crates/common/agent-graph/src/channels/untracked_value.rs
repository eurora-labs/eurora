//! Mirrors `langgraph/channels/untracked_value.py`.

use crate::channels::{BaseChannel, Value};
use crate::errors::{Error, Result};

/// Stores the last value received; never checkpointed.
///
/// `guard = true` rejects multi-value updates (mirroring Python's default).
/// `guard = false` silently keeps the last one.
#[derive(Debug, Default, Clone)]
pub struct UntrackedValue {
    key: String,
    value: Option<Value>,
    guard: bool,
}

impl UntrackedValue {
    pub fn new() -> Self {
        Self {
            guard: true,
            ..Default::default()
        }
    }

    pub fn unguarded() -> Self {
        Self {
            guard: false,
            ..Default::default()
        }
    }
}

impl BaseChannel for UntrackedValue {
    fn key(&self) -> &str {
        &self.key
    }

    fn set_key(&mut self, key: String) {
        self.key = key;
    }

    fn update(&mut self, values: Vec<Value>) -> Result<bool> {
        if values.is_empty() {
            return Ok(false);
        }
        if values.len() > 1 && self.guard {
            return Err(Error::InvalidUpdate(format!(
                "At key '{}': UntrackedValue(guard=true) can receive only one value per step. Use UntrackedValue::unguarded() if you want to store any one of multiple values.",
                self.key
            )));
        }
        self.value = values.into_iter().last();
        Ok(true)
    }

    fn get(&self) -> Result<Value> {
        self.value
            .clone()
            .ok_or_else(|| Error::EmptyChannel(self.key.clone()))
    }

    fn is_available(&self) -> bool {
        self.value.is_some()
    }

    fn checkpoint(&self) -> Option<Value> {
        None
    }

    fn from_checkpoint(&self, _checkpoint: Option<Value>) -> Result<Box<dyn BaseChannel>> {
        Ok(Box::new(Self {
            key: self.key.clone(),
            value: None,
            guard: self.guard,
        }))
    }

    fn clone_channel(&self) -> Box<dyn BaseChannel> {
        Box::new(self.clone())
    }
}
