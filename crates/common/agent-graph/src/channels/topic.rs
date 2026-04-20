//! Mirrors `langgraph/channels/topic.py`.

use crate::channels::{BaseChannel, Value};
use crate::errors::{Error, Result};

/// A configurable PubSub topic.
///
/// Each update either appends a single value or flattens an array of values
/// into the topic. With `accumulate = false` the topic is cleared at the
/// start of each update; with `accumulate = true` values persist across
/// super-steps.
#[derive(Debug, Default, Clone)]
pub struct Topic {
    key: String,
    values: Vec<Value>,
    accumulate: bool,
}

impl Topic {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accumulating() -> Self {
        Self {
            accumulate: true,
            ..Default::default()
        }
    }
}

fn flatten(values: Vec<Value>) -> Vec<Value> {
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        match value {
            Value::Array(items) => out.extend(items),
            other => out.push(other),
        }
    }
    out
}

impl BaseChannel for Topic {
    fn key(&self) -> &str {
        &self.key
    }

    fn set_key(&mut self, key: String) {
        self.key = key;
    }

    fn update(&mut self, values: Vec<Value>) -> Result<bool> {
        let mut updated = false;
        if !self.accumulate {
            if !self.values.is_empty() {
                updated = true;
            }
            self.values.clear();
        }
        let flat = flatten(values);
        if !flat.is_empty() {
            updated = true;
            self.values.extend(flat);
        }
        Ok(updated)
    }

    fn get(&self) -> Result<Value> {
        if self.values.is_empty() {
            Err(Error::EmptyChannel(self.key.clone()))
        } else {
            Ok(Value::Array(self.values.clone()))
        }
    }

    fn is_available(&self) -> bool {
        !self.values.is_empty()
    }

    fn checkpoint(&self) -> Option<Value> {
        if self.values.is_empty() {
            None
        } else {
            Some(Value::Array(self.values.clone()))
        }
    }

    fn from_checkpoint(&self, checkpoint: Option<Value>) -> Result<Box<dyn BaseChannel>> {
        let values = match checkpoint {
            None => Vec::new(),
            Some(Value::Array(items)) => items,
            Some(other) => {
                return Err(Error::Other(format!(
                    "Topic checkpoint at '{}' must be an array, got {other}",
                    self.key
                )));
            }
        };
        Ok(Box::new(Self {
            key: self.key.clone(),
            values,
            accumulate: self.accumulate,
        }))
    }

    fn clone_channel(&self) -> Box<dyn BaseChannel> {
        Box::new(self.clone())
    }
}
