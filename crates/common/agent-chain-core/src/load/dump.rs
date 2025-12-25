//! Dump objects to JSON.
//!
//! This module provides functions for serializing LangChain objects to JSON,
//! mirroring `langchain_core.load.dump`.

use serde::Serialize;
use serde_json::Value;

use super::serializable::{Serializable, Serialized, to_json_not_implemented};

/// Return a default serialized value for an object.
///
/// If the object is serializable, returns its JSON representation.
/// Otherwise, returns a SerializedNotImplemented.
///
/// # Arguments
///
/// * `obj` - The object to serialize.
pub fn default_serializer<T: Serialize>(obj: &T) -> Value {
    match serde_json::to_value(obj) {
        Ok(v) => v,
        Err(_) => serde_json::to_value(to_json_not_implemented(&Value::Null)).unwrap_or_default(),
    }
}

/// Serialize a Serializable object to a JSON string.
///
/// # Arguments
///
/// * `obj` - The object to serialize.
/// * `pretty` - Whether to pretty print the JSON. If `true`, the JSON will be
///     indented with 2 spaces.
///
/// # Returns
///
/// A JSON string representation of the object.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn dumps<T: Serializable + Serialize>(obj: &T, pretty: bool) -> crate::Result<String> {
    let serialized = obj.to_json();
    if pretty {
        serde_json::to_string_pretty(&serialized).map_err(crate::Error::from)
    } else {
        serde_json::to_string(&serialized).map_err(crate::Error::from)
    }
}

/// Serialize any serde-serializable object to a JSON string.
///
/// This is a fallback for objects that don't implement Serializable.
///
/// # Arguments
///
/// * `obj` - The object to serialize.
/// * `pretty` - Whether to pretty print the JSON.
///
/// # Returns
///
/// A JSON string representation of the object.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn dumps_value<T: Serialize>(obj: &T, pretty: bool) -> crate::Result<String> {
    if pretty {
        serde_json::to_string_pretty(obj).map_err(crate::Error::from)
    } else {
        serde_json::to_string(obj).map_err(crate::Error::from)
    }
}

/// Serialize a Serialized enum to a JSON string.
///
/// # Arguments
///
/// * `serialized` - The Serialized enum to convert to string.
/// * `pretty` - Whether to pretty print the JSON.
///
/// # Returns
///
/// A JSON string representation.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn dumps_serialized(serialized: &Serialized, pretty: bool) -> crate::Result<String> {
    if pretty {
        serde_json::to_string_pretty(serialized).map_err(crate::Error::from)
    } else {
        serde_json::to_string(serialized).map_err(crate::Error::from)
    }
}

/// Serialize a Serializable object to a Value (dict-like structure).
///
/// # Arguments
///
/// * `obj` - The object to serialize.
///
/// # Returns
///
/// A serde_json::Value representation of the object.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn dumpd<T: Serializable + Serialize>(obj: &T) -> crate::Result<Value> {
    let json_string = dumps(obj, false)?;
    serde_json::from_str(&json_string).map_err(crate::Error::from)
}

/// Serialize any serde-serializable object to a Value.
///
/// This is a fallback for objects that don't implement Serializable.
///
/// # Arguments
///
/// * `obj` - The object to serialize.
///
/// # Returns
///
/// A serde_json::Value representation of the object.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn dumpd_value<T: Serialize>(obj: &T) -> crate::Result<Value> {
    serde_json::to_value(obj).map_err(crate::Error::from)
}

/// Serialize a Serialized enum to a Value.
///
/// # Arguments
///
/// * `serialized` - The Serialized enum to convert.
///
/// # Returns
///
/// A serde_json::Value representation.
///
/// # Errors
///
/// Returns an error if serialization fails.
pub fn dumpd_serialized(serialized: &Serialized) -> crate::Result<Value> {
    serde_json::to_value(serialized).map_err(crate::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Debug, Serialize, Deserialize)]
    struct TestModel {
        name: String,
        value: i32,
    }

    impl Serializable for TestModel {
        fn is_lc_serializable() -> bool {
            true
        }

        fn get_lc_namespace() -> Vec<String> {
            vec!["test".to_string(), "models".to_string()]
        }
    }

    #[test]
    fn test_dumps_serializable() {
        let model = TestModel {
            name: "test".to_string(),
            value: 42,
        };

        let json = dumps(&model, false).unwrap();
        assert!(json.contains("constructor"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_dumps_pretty() {
        let model = TestModel {
            name: "test".to_string(),
            value: 42,
        };

        let json = dumps(&model, true).unwrap();
        assert!(json.contains('\n'));
    }

    #[test]
    fn test_dumpd() {
        let model = TestModel {
            name: "test".to_string(),
            value: 42,
        };

        let value = dumpd(&model).unwrap();
        assert!(value.is_object());
        assert_eq!(
            value.get("type").and_then(|v| v.as_str()),
            Some("constructor")
        );
    }

    #[test]
    fn test_dumps_value() {
        let data = HashMap::from([("key", "value")]);
        let json = dumps_value(&data, false).unwrap();
        assert!(json.contains("key"));
        assert!(json.contains("value"));
    }
}
