//! Mirrors `langgraph/channels/last_value.py`.

use crate::channels::{BaseChannel, Value};
use crate::errors::{Error, ErrorCode, Result, create_message};

/// Stores the last value received; can receive at most one value per step.
#[derive(Debug, Default, Clone)]
pub struct LastValue {
    key: String,
    value: Option<Value>,
}

impl LastValue {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BaseChannel for LastValue {
    fn key(&self) -> &str {
        &self.key
    }

    fn set_key(&mut self, key: String) {
        self.key = key;
    }

    fn update(&mut self, values: Vec<Value>) -> Result<bool> {
        match values.len() {
            0 => Ok(false),
            1 => {
                self.value = values.into_iter().next();
                Ok(true)
            }
            _ => Err(Error::InvalidUpdate(create_message(
                &format!(
                    "At key '{}': Can receive only one value per step. Use an Annotated key to handle multiple values.",
                    self.key
                ),
                ErrorCode::InvalidConcurrentGraphUpdate,
            ))),
        }
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

/// Stores the last value received, but only makes it available after
/// [`BaseChannel::finish`]. Once read, the value is cleared on the next
/// [`BaseChannel::consume`].
#[derive(Debug, Default, Clone)]
pub struct LastValueAfterFinish {
    key: String,
    value: Option<Value>,
    finished: bool,
}

impl LastValueAfterFinish {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BaseChannel for LastValueAfterFinish {
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
        self.finished = false;
        self.value = values.into_iter().last();
        Ok(true)
    }

    fn get(&self) -> Result<Value> {
        if !self.finished {
            return Err(Error::EmptyChannel(self.key.clone()));
        }
        self.value
            .clone()
            .ok_or_else(|| Error::EmptyChannel(self.key.clone()))
    }

    fn is_available(&self) -> bool {
        self.finished && self.value.is_some()
    }

    fn checkpoint(&self) -> Option<Value> {
        self.value.as_ref().map(|value| {
            serde_json::json!({
                "value": value,
                "finished": self.finished,
            })
        })
    }

    fn from_checkpoint(&self, checkpoint: Option<Value>) -> Result<Box<dyn BaseChannel>> {
        let (value, finished) = match checkpoint {
            Some(snapshot) => {
                let object = snapshot.as_object().ok_or_else(|| {
                    Error::Other(format!(
                        "LastValueAfterFinish checkpoint at '{}' must be an object",
                        self.key
                    ))
                })?;
                let value = object.get("value").cloned();
                let finished = object
                    .get("finished")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                (value, finished)
            }
            None => (None, false),
        };
        Ok(Box::new(Self {
            key: self.key.clone(),
            value,
            finished,
        }))
    }

    fn consume(&mut self) -> bool {
        if self.finished {
            self.finished = false;
            self.value = None;
            true
        } else {
            false
        }
    }

    fn finish(&mut self) -> bool {
        if !self.finished && self.value.is_some() {
            self.finished = true;
            true
        } else {
            false
        }
    }

    fn clone_channel(&self) -> Box<dyn BaseChannel> {
        Box::new(self.clone())
    }
}
