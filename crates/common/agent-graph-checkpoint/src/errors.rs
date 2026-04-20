//! Error type shared by the checkpoint crate.
//!
//! Mirrors the exceptions raised across `langgraph.checkpoint.base` and
//! `langgraph.checkpoint.serde.*`. Most variants are thin wrappers over the
//! underlying serde failure so call sites can distinguish a JSON error from a
//! msgpack error without matching on the inner type.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    /// JSON encode/decode failure.
    #[error("json serialization failed: {0}")]
    Json(#[from] serde_json::Error),

    /// Msgpack encode failure (`rmp_serde::encode::Error`).
    #[error("msgpack encoding failed: {0}")]
    MsgpackEncode(#[from] rmp_serde::encode::Error),

    /// Msgpack decode failure (`rmp_serde::decode::Error`).
    #[error("msgpack decoding failed: {0}")]
    MsgpackDecode(#[from] rmp_serde::decode::Error),

    /// A [`crate::codec::Serializer`] was handed a type tag it does not know
    /// how to decode. Mirrors the implicit assertion in Python's
    /// `SerializerProtocol.loads_typed`.
    #[error("unknown serializer type tag: {0}")]
    UnknownTypeTag(String),
}
