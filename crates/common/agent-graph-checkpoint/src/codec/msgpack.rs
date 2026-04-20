//! Msgpack implementation of [`crate::codec::Serializer`].
//!
//! Uses `rmp-serde`'s compact named representation — the same shape
//! `serde_json` emits, just binary-packed. This is the default codec for
//! checkpoint backends because the payloads are smaller and faster to parse
//! than JSON without sacrificing schema fidelity.

use serde_json::Value;

use crate::codec::Serializer;
use crate::errors::{Error, Result};

/// Type tag emitted by [`MsgpackSerializer`].
pub const TYPE_TAG: &str = "msgpack";

#[derive(Debug, Default, Clone, Copy)]
pub struct MsgpackSerializer;

impl MsgpackSerializer {
    pub const fn new() -> Self {
        Self
    }
}

impl Serializer for MsgpackSerializer {
    fn dumps_typed(&self, value: &Value) -> Result<(String, Vec<u8>)> {
        Ok((TYPE_TAG.to_owned(), rmp_serde::to_vec_named(value)?))
    }

    fn loads_typed(&self, type_tag: &str, bytes: &[u8]) -> Result<Value> {
        if type_tag != TYPE_TAG {
            return Err(Error::UnknownTypeTag(type_tag.to_owned()));
        }
        Ok(rmp_serde::from_slice(bytes)?)
    }
}
