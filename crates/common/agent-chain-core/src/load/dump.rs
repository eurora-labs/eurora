//! Dump objects to JSON.
//!
//! Mirrors `langchain_core.load.dump`.

use serde::Serialize;
use serde_json::Value;

use super::serializable::{Serializable, to_json_not_implemented_value};

/// Return a JSON string representation of a Serializable object.
///
/// Mirrors Python's `dumps(obj, *, pretty=False)`. If serialization of the
/// `to_json()` output fails, falls back to a `SerializedNotImplemented`
/// representation (matching Python's `except TypeError` branch).
pub fn dumps<T: Serializable + Serialize>(obj: &T, pretty: bool) -> crate::Result<String> {
    let serialized = obj.to_json();
    let result = if pretty {
        serde_json::to_string_pretty(&serialized)
    } else {
        serde_json::to_string(&serialized)
    };
    match result {
        Ok(json) => Ok(json),
        Err(_) => {
            let fallback = to_json_not_implemented_value(obj.lc_type_name(), None);
            if pretty {
                serde_json::to_string_pretty(&fallback).map_err(crate::Error::from)
            } else {
                serde_json::to_string(&fallback).map_err(crate::Error::from)
            }
        }
    }
}

/// Return a dict-like representation of a Serializable object.
///
/// Mirrors Python's `dumpd(obj)` -- roundtrips through `dumps` then parses
/// back into a `Value`.
pub fn dumpd<T: Serializable + Serialize>(obj: &T) -> crate::Result<Value> {
    let json_string = dumps(obj, false)?;
    serde_json::from_str(&json_string).map_err(crate::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

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
}
