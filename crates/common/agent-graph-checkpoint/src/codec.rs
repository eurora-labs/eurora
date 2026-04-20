//! Pluggable serialization used by checkpoint savers.
//!
//! Mirrors `langgraph.checkpoint.serde` but pared down to what Rust actually
//! needs. The Python allowlist machinery (`_msgpack.py`) guards against
//! `pickle`-style arbitrary-module deserialization; the same class of attack
//! doesn't exist here because every decodable type is named statically via a
//! serde impl, so we drop it.
//!
//! The trait deals in [`serde_json::Value`] so it can be used as a
//! `dyn Serializer`, matching the channel layer's erased value type. Callers
//! with typed data can use the free [`dumps`]/[`loads`] helpers, which
//! transparently round-trip via `Value`.

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::errors::Result;

pub mod json;
pub mod msgpack;

pub use json::JsonSerializer;
pub use msgpack::MsgpackSerializer;

/// Serialize and deserialize values for storage in a checkpoint backend.
///
/// Implementations return a `(type_tag, bytes)` pair so a storage layer can
/// record which codec produced the payload and hand the bytes back to the
/// right decoder on load. Tags are short ASCII strings (`"json"`,
/// `"msgpack"`) — they are intentionally not tied to the object's runtime
/// type, unlike Python's `type(obj).__name__` approach, which relies on
/// runtime type introspection that Rust doesn't provide.
pub trait Serializer: Send + Sync {
    /// Serialize a value, returning `(type_tag, bytes)`.
    fn dumps_typed(&self, value: &serde_json::Value) -> Result<(String, Vec<u8>)>;

    /// Deserialize a value previously produced by [`Self::dumps_typed`].
    ///
    /// Returns [`crate::errors::Error::UnknownTypeTag`] when the tag does not
    /// match this serializer's format.
    fn loads_typed(&self, type_tag: &str, bytes: &[u8]) -> Result<serde_json::Value>;
}

/// Serialize any `Serialize` value through a [`Serializer`].
///
/// Routes via `serde_json::Value` so the same code path handles both JSON and
/// msgpack backends — the intermediate `Value` is the lingua franca the
/// channel layer already operates in.
pub fn dumps<T: Serialize + ?Sized>(
    serializer: &dyn Serializer,
    value: &T,
) -> Result<(String, Vec<u8>)> {
    let intermediate = serde_json::to_value(value)?;
    serializer.dumps_typed(&intermediate)
}

/// Deserialize bytes produced by [`dumps`] into a typed value.
pub fn loads<T: DeserializeOwned>(
    serializer: &dyn Serializer,
    type_tag: &str,
    bytes: &[u8],
) -> Result<T> {
    let intermediate = serializer.loads_typed(type_tag, bytes)?;
    Ok(serde_json::from_value(intermediate)?)
}
