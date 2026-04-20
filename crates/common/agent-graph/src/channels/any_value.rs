//! Mirrors `langgraph/channels/any_value.py`.

use crate::channels::{BaseChannel, Value};
use crate::errors::{Error, Result};

/// Stores the last value received; assumes all values seen in a single step
/// are equivalent (the channel itself does not enforce equality).
#[derive(Debug, Default, Clone)]
pub struct AnyValue {
    key: String,
    value: Option<Value>,
}

impl AnyValue {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BaseChannel for AnyValue {
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
        }))
    }

    fn clone_channel(&self) -> Box<dyn BaseChannel> {
        Box::new(self.clone())
    }
}
