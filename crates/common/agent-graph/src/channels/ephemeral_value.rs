//! Mirrors `langgraph/channels/ephemeral_value.py`.

use crate::channels::{BaseChannel, Value};
use crate::errors::{Error, Result};

/// Holds the value written in the immediately preceding super-step, then
/// clears it when an empty update arrives.
///
/// `guard = true` (the default) rejects multi-value updates.
#[derive(Debug, Default, Clone)]
pub struct EphemeralValue {
    key: String,
    value: Option<Value>,
    guard: bool,
}

impl EphemeralValue {
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

impl BaseChannel for EphemeralValue {
    fn key(&self) -> &str {
        &self.key
    }

    fn set_key(&mut self, key: String) {
        self.key = key;
    }

    fn update(&mut self, values: Vec<Value>) -> Result<bool> {
        if values.is_empty() {
            if self.value.is_none() {
                return Ok(false);
            }
            self.value = None;
            return Ok(true);
        }
        if values.len() > 1 && self.guard {
            return Err(Error::InvalidUpdate(format!(
                "At key '{}': EphemeralValue(guard=true) can receive only one value per step. Use EphemeralValue::unguarded() if you want to store any one of multiple values.",
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
        self.value.clone()
    }

    fn from_checkpoint(&self, checkpoint: Option<Value>) -> Result<Box<dyn BaseChannel>> {
        Ok(Box::new(Self {
            key: self.key.clone(),
            value: checkpoint,
            guard: self.guard,
        }))
    }

    fn clone_channel(&self) -> Box<dyn BaseChannel> {
        Box::new(self.clone())
    }
}
