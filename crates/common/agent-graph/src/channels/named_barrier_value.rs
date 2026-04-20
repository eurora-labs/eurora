//! Mirrors `langgraph/channels/named_barrier_value.py`.
//!
//! Python's `NamedBarrierValue` accepts a `set[Value]` of names. Because
//! `serde_json::Value` is neither `Hash` nor `Eq`, the Rust port restricts
//! barrier names to [`String`]. Python's test suite only ever exercises the
//! channel with string names, so this is a faithful narrowing.

use std::collections::BTreeSet;

use crate::channels::{BaseChannel, Value};
use crate::errors::{Error, Result};

/// Waits until all named values are received before making the value
/// available. [`BaseChannel::consume`] resets the barrier.
#[derive(Debug, Clone)]
pub struct NamedBarrierValue {
    key: String,
    names: BTreeSet<String>,
    seen: BTreeSet<String>,
}

impl NamedBarrierValue {
    pub fn new<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            key: String::new(),
            names: names.into_iter().map(Into::into).collect(),
            seen: BTreeSet::new(),
        }
    }
}

fn checkpoint_from_seen(seen: &BTreeSet<String>) -> Option<Value> {
    if seen.is_empty() {
        None
    } else {
        Some(Value::Array(
            seen.iter().map(|s| Value::String(s.clone())).collect(),
        ))
    }
}

fn seen_from_checkpoint(
    key: &str,
    checkpoint: Option<Value>,
    kind: &'static str,
) -> Result<BTreeSet<String>> {
    match checkpoint {
        None => Ok(BTreeSet::new()),
        Some(Value::Array(items)) => items
            .into_iter()
            .map(|item| {
                item.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                    Error::Other(format!(
                        "{kind} checkpoint at '{key}' contains a non-string element"
                    ))
                })
            })
            .collect(),
        Some(other) => Err(Error::Other(format!(
            "{kind} checkpoint at '{key}' must be an array, got {other}"
        ))),
    }
}

impl BaseChannel for NamedBarrierValue {
    fn key(&self) -> &str {
        &self.key
    }

    fn set_key(&mut self, key: String) {
        self.key = key;
    }

    fn update(&mut self, values: Vec<Value>) -> Result<bool> {
        let mut updated = false;
        for value in values {
            let name = value.as_str().ok_or_else(|| {
                Error::InvalidUpdate(format!(
                    "At key '{}': NamedBarrierValue expects string updates, got {value}",
                    self.key
                ))
            })?;
            if !self.names.contains(name) {
                return Err(Error::InvalidUpdate(format!(
                    "At key '{}': Value {name} not in {:?}",
                    self.key, self.names
                )));
            }
            if self.seen.insert(name.to_owned()) {
                updated = true;
            }
        }
        Ok(updated)
    }

    fn get(&self) -> Result<Value> {
        if self.seen == self.names {
            Ok(Value::Null)
        } else {
            Err(Error::EmptyChannel(self.key.clone()))
        }
    }

    fn is_available(&self) -> bool {
        self.seen == self.names
    }

    fn checkpoint(&self) -> Option<Value> {
        checkpoint_from_seen(&self.seen)
    }

    fn from_checkpoint(&self, checkpoint: Option<Value>) -> Result<Box<dyn BaseChannel>> {
        let seen = seen_from_checkpoint(&self.key, checkpoint, "NamedBarrierValue")?;
        Ok(Box::new(Self {
            key: self.key.clone(),
            names: self.names.clone(),
            seen,
        }))
    }

    fn consume(&mut self) -> bool {
        if self.seen == self.names {
            self.seen.clear();
            true
        } else {
            false
        }
    }

    fn clone_channel(&self) -> Box<dyn BaseChannel> {
        Box::new(self.clone())
    }
}

/// Same as [`NamedBarrierValue`] but only makes the value available after
/// [`BaseChannel::finish`] is called.
#[derive(Debug, Clone)]
pub struct NamedBarrierValueAfterFinish {
    key: String,
    names: BTreeSet<String>,
    seen: BTreeSet<String>,
    finished: bool,
}

impl NamedBarrierValueAfterFinish {
    pub fn new<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            key: String::new(),
            names: names.into_iter().map(Into::into).collect(),
            seen: BTreeSet::new(),
            finished: false,
        }
    }
}

impl BaseChannel for NamedBarrierValueAfterFinish {
    fn key(&self) -> &str {
        &self.key
    }

    fn set_key(&mut self, key: String) {
        self.key = key;
    }

    fn update(&mut self, values: Vec<Value>) -> Result<bool> {
        let mut updated = false;
        for value in values {
            let name = value.as_str().ok_or_else(|| {
                Error::InvalidUpdate(format!(
                    "At key '{}': NamedBarrierValueAfterFinish expects string updates, got {value}",
                    self.key
                ))
            })?;
            if !self.names.contains(name) {
                return Err(Error::InvalidUpdate(format!(
                    "At key '{}': Value {name} not in {:?}",
                    self.key, self.names
                )));
            }
            if self.seen.insert(name.to_owned()) {
                updated = true;
            }
        }
        Ok(updated)
    }

    fn get(&self) -> Result<Value> {
        if self.finished && self.seen == self.names {
            Ok(Value::Null)
        } else {
            Err(Error::EmptyChannel(self.key.clone()))
        }
    }

    fn is_available(&self) -> bool {
        self.finished && self.seen == self.names
    }

    fn checkpoint(&self) -> Option<Value> {
        if self.seen.is_empty() && !self.finished {
            None
        } else {
            Some(serde_json::json!({
                "seen": self.seen.iter().collect::<Vec<_>>(),
                "finished": self.finished,
            }))
        }
    }

    fn from_checkpoint(&self, checkpoint: Option<Value>) -> Result<Box<dyn BaseChannel>> {
        let (seen, finished) = match checkpoint {
            None => (BTreeSet::new(), false),
            Some(snapshot) => {
                let object = snapshot.as_object().ok_or_else(|| {
                    Error::Other(format!(
                        "NamedBarrierValueAfterFinish checkpoint at '{}' must be an object",
                        self.key
                    ))
                })?;
                let seen_value = object
                    .get("seen")
                    .cloned()
                    .unwrap_or_else(|| Value::Array(Vec::new()));
                let seen = seen_from_checkpoint(
                    &self.key,
                    Some(seen_value),
                    "NamedBarrierValueAfterFinish",
                )?;
                let finished = object
                    .get("finished")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                (seen, finished)
            }
        };
        Ok(Box::new(Self {
            key: self.key.clone(),
            names: self.names.clone(),
            seen,
            finished,
        }))
    }

    fn consume(&mut self) -> bool {
        if self.finished && self.seen == self.names {
            self.finished = false;
            self.seen.clear();
            true
        } else {
            false
        }
    }

    fn finish(&mut self) -> bool {
        if !self.finished && self.seen == self.names {
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
