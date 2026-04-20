//! Mirrors `langgraph/channels/binop.py`.

use std::fmt;
use std::sync::Arc;

use crate::channels::{BaseChannel, Value};
use crate::errors::{Error, ErrorCode, Result, create_message};
use crate::types::Overwrite;

/// A fallible binary reducer. Takes ownership of the accumulator and the
/// next update, yielding the new accumulator (or an error).
pub type Reducer = Arc<dyn Fn(Value, Value) -> Result<Value> + Send + Sync>;

/// Stores the result of folding a binary operator over every value the
/// channel has ever received. A bare [`Overwrite`] (or `{"__overwrite__":
/// v}`-shaped JSON) bypasses the reducer and replaces the accumulator.
///
/// Unlike the Python implementation, which derives an identity element
/// from the declared `typ` (e.g. `int` → `0`, `list` → `[]`), this version
/// asks the caller for an explicit `initial` value or leaves the channel
/// empty until the first update — the Python trick relies on runtime type
/// introspection that has no Rust equivalent.
#[derive(Clone)]
pub struct BinaryOperatorAggregate {
    key: String,
    value: Option<Value>,
    initial: Option<Value>,
    operator: Reducer,
}

impl BinaryOperatorAggregate {
    pub fn new(operator: Reducer) -> Self {
        Self {
            key: String::new(),
            value: None,
            initial: None,
            operator,
        }
    }

    pub fn with_initial(operator: Reducer, initial: Value) -> Self {
        Self {
            key: String::new(),
            value: Some(initial.clone()),
            initial: Some(initial),
            operator,
        }
    }

    fn apply_overwrite(&mut self, value: Value, seen: &mut bool) -> Result<()> {
        if *seen {
            return Err(Error::InvalidUpdate(create_message(
                "Can receive only one Overwrite value per super-step.",
                ErrorCode::InvalidConcurrentGraphUpdate,
            )));
        }
        self.value = Some(value);
        *seen = true;
        Ok(())
    }
}

impl fmt::Debug for BinaryOperatorAggregate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BinaryOperatorAggregate")
            .field("key", &self.key)
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

impl BaseChannel for BinaryOperatorAggregate {
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
        let mut iter = values.into_iter();
        let mut seen_overwrite = false;

        if self.value.is_none() {
            let first = iter.next().expect("non-empty iterator");
            if let Some(inner) = Overwrite::from_value(&first) {
                self.apply_overwrite(inner, &mut seen_overwrite)?;
            } else {
                self.value = Some(first);
            }
        }

        for value in iter {
            if let Some(inner) = Overwrite::from_value(&value) {
                self.apply_overwrite(inner, &mut seen_overwrite)?;
                continue;
            }
            if seen_overwrite {
                continue;
            }
            let accumulator = self
                .value
                .take()
                .expect("accumulator seeded before first reduce step");
            self.value = Some((self.operator)(accumulator, value)?);
        }
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
            value: checkpoint.or_else(|| self.initial.clone()),
            initial: self.initial.clone(),
            operator: Arc::clone(&self.operator),
        }))
    }

    fn clone_channel(&self) -> Box<dyn BaseChannel> {
        Box::new(self.clone())
    }
}
