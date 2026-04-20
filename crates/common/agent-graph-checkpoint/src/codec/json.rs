//! JSON implementation of [`crate::codec::Serializer`].

use serde_json::Value;

use crate::codec::Serializer;
use crate::errors::{Error, Result};

/// Type tag emitted by [`JsonSerializer`].
pub const TYPE_TAG: &str = "json";

/// Serialize values as UTF-8 JSON via `serde_json`.
///
/// Stateless and safe to share behind `Arc` or `&'static`. Use this when
/// checkpoints need to remain human-readable or when the storage backend
/// requires text (e.g. a plain SQL `TEXT` column).
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonSerializer;

impl JsonSerializer {
    pub const fn new() -> Self {
        Self
    }
}

impl Serializer for JsonSerializer {
    fn dumps_typed(&self, value: &Value) -> Result<(String, Vec<u8>)> {
        Ok((TYPE_TAG.to_owned(), serde_json::to_vec(value)?))
    }

    fn loads_typed(&self, type_tag: &str, bytes: &[u8]) -> Result<Value> {
        if type_tag != TYPE_TAG {
            return Err(Error::UnknownTypeTag(type_tag.to_owned()));
        }
        Ok(serde_json::from_slice(bytes)?)
    }
}
