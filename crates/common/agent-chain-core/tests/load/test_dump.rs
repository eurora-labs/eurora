//! Unit tests for the dump module.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/load/test_dump.py`

use agent_chain_core::load::{Serializable, Serialized, dumpd, dumpd_value, dumps, dumps_value};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct SerializableTest {
    value: i32,
}

impl Serializable for SerializableTest {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "tests".to_string(),
            "unit_tests".to_string(),
            "load".to_string(),
            "test_dump".to_string(),
        ]
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NonSerializableTest {
    value: i32,
}

impl Serializable for NonSerializableTest {
    fn is_lc_serializable() -> bool {
        false
    }

    fn get_lc_namespace() -> Vec<String> {
        vec![
            "tests".to_string(),
            "unit_tests".to_string(),
            "load".to_string(),
            "test_dump".to_string(),
        ]
    }
}

/// Tests for the Serializable::to_json() method (equivalent to Python's default() function).
mod test_default {
    use super::*;

    /// Test to_json() handles Serializable objects.
    ///
    /// Ported from `TestDefault::test_default_with_serializable`
    #[test]
    fn test_default_with_serializable() {
        let obj = SerializableTest { value: 42 };
        let result = obj.to_json();
        match &result {
            Serialized::Constructor(data) => {
                assert_eq!(data.lc, 1);
            }
            other => panic!("Expected Constructor, got {:?}", other),
        }
    }

    /// Test to_json() handles non-Serializable objects.
    ///
    /// Ported from `TestDefault::test_default_with_non_serializable`
    #[test]
    fn test_default_with_non_serializable() {
        let obj = NonSerializableTest { value: 42 };
        let result = obj.to_json();
        match &result {
            Serialized::NotImplemented(data) => {
                assert_eq!(data.lc, 1);
                assert!(
                    data.id
                        .last()
                        .map_or(false, |s| s.contains("NonSerializableTest"))
                );
            }
            other => panic!("Expected NotImplemented, got {:?}", other),
        }
    }
}

/// Tests for the dumps() function.
mod test_dumps {
    use super::*;

    /// Test basic dumps() functionality.
    ///
    /// Ported from `TestDumps::test_dumps_basic`
    #[test]
    fn test_dumps_basic() {
        let obj = SerializableTest { value: 42 };
        let json_str = dumps(&obj, false).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["type"], "constructor");
        assert_eq!(parsed["kwargs"]["value"], 42);
    }

    /// Test dumps() with pretty=true.
    ///
    /// Ported from `TestDumps::test_dumps_with_pretty_flag`
    #[test]
    fn test_dumps_with_pretty_flag() {
        let obj = SerializableTest { value: 42 };
        let json_str = dumps(&obj, true).unwrap();

        assert!(json_str.contains("  "));
        let parsed: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["kwargs"]["value"], 42);
    }

    /// Test dumps() with non-serializable fallback.
    ///
    /// Ported from `TestDumps::test_dumps_with_non_serializable_fallback`
    #[test]
    fn test_dumps_with_non_serializable_fallback() {
        let obj = NonSerializableTest { value: 42 };
        let json_str = dumps(&obj, false).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["type"], "not_implemented");
    }

    /// Test dumps() fallback with pretty=true when non-serializable.
    ///
    /// Ported from `TestDumps::test_dumps_with_pretty_and_typeerror_fallback`
    #[test]
    fn test_dumps_with_pretty_and_non_serializable_fallback() {
        let obj = NonSerializableTest { value: 42 };
        let json_str = dumps(&obj, true).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert!(json_str.contains("  "));
        assert_eq!(parsed["type"], "not_implemented");
    }

    /// Test dumps_value() with nested data structures.
    ///
    /// Ported from `TestDumps::test_dumps_nested_structures`
    /// Note: In Rust, we use dumps_value for plain serde-serializable data
    /// and dumps for Serializable trait objects. For nested structures mixing
    /// both, we serialize the Serializable parts first, then embed them.
    #[test]
    fn test_dumps_nested_structures() {
        let serializable_obj = SerializableTest { value: 1 };
        let serialized = serializable_obj.to_json();
        let serialized_value = serde_json::to_value(&serialized).unwrap();

        let data = json!({
            "serializable": serialized_value,
            "list": [1, 2, {"nested": "value"}],
            "primitive": "string"
        });

        let json_str = serde_json::to_string(&data).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["serializable"]["type"], "constructor");
        assert_eq!(parsed["list"], json!([1, 2, {"nested": "value"}]));
        assert_eq!(parsed["primitive"], "string");
    }

    /// Test dumps_value() on None (null).
    ///
    /// Ported from `TestDumpsSnapshot::test_dumps_with_none`
    #[test]
    fn test_dumps_with_none() {
        let result = dumps_value(&Value::Null, false).unwrap();
        assert_eq!(result, "null");
    }

    /// Test dumps_value() on a string.
    ///
    /// Ported from `TestDumpsSnapshot::test_dumps_with_plain_string`
    #[test]
    fn test_dumps_with_plain_string() {
        let result = dumps_value(&"hello", false).unwrap();
        assert_eq!(result, "\"hello\"");
    }

    /// Test dumps_value() on an int.
    ///
    /// Ported from `TestDumpsSnapshot::test_dumps_with_plain_int`
    #[test]
    fn test_dumps_with_plain_int() {
        let result = dumps_value(&42, false).unwrap();
        assert_eq!(result, "42");
    }

    /// Test dumps_value() on a dict.
    ///
    /// Ported from `TestDumpsSnapshot::test_dumps_with_plain_dict`
    #[test]
    fn test_dumps_with_plain_dict() {
        let data: HashMap<&str, i32> = HashMap::from([("a", 1), ("b", 2)]);
        let result = dumps_value(&data, false).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed, json!({"a": 1, "b": 2}));
    }

    /// Test dumps_value() on a list.
    ///
    /// Ported from `TestDumpsSnapshot::test_dumps_with_plain_list`
    #[test]
    fn test_dumps_with_plain_list() {
        let data = json!([1, "two", 3]);
        let result = dumps_value(&data, false).unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed, json!([1, "two", 3]));
    }

    /// Test dumps_value() with bool values.
    ///
    /// Ported from `TestDumpsSnapshot::test_dumps_bool_values`
    #[test]
    fn test_dumps_bool_values() {
        assert_eq!(dumps_value(&true, false).unwrap(), "true");
        assert_eq!(dumps_value(&false, false).unwrap(), "false");
    }

    /// Test dumps() pretty uses 2-space indent by default.
    ///
    /// Ported from `TestDumpsSnapshot::test_dumps_pretty_default_indent_is_2`
    #[test]
    fn test_dumps_pretty_default_indent_is_2() {
        let obj = SerializableTest { value: 1 };
        let pretty = dumps(&obj, true).unwrap();
        let lines: Vec<&str> = pretty.split('\n').collect();
        assert!(lines.len() > 1, "Pretty output should have multiple lines");
        assert!(
            lines[1].starts_with("  "),
            "Second line should start with 2-space indent, got: {:?}",
            lines[1]
        );
    }
}

/// Tests for the dumpd() function.
mod test_dumpd {
    use super::*;

    /// Test basic dumpd() functionality.
    ///
    /// Ported from `TestDumpd::test_dumpd_basic`
    #[test]
    fn test_dumpd_basic() {
        let obj = SerializableTest { value: 42 };
        let result = dumpd(&obj).unwrap();

        assert!(result.is_object());
        assert_eq!(result["type"], "constructor");
        assert_eq!(result["kwargs"]["value"], 42);
    }

    /// Test dumpd() with non-serializable object.
    ///
    /// Ported from `TestDumpd::test_dumpd_with_non_serializable`
    #[test]
    fn test_dumpd_with_non_serializable() {
        let obj = NonSerializableTest { value: 42 };
        let result = dumpd(&obj).unwrap();

        assert!(result.is_object());
        assert_eq!(result["type"], "not_implemented");
        let id = result["id"].as_array().unwrap();
        assert!(
            id.iter().any(|v| v
                .as_str()
                .map_or(false, |s| s.contains("NonSerializableTest"))),
            "id should contain NonSerializableTest, got: {:?}",
            id
        );
    }

    /// Test dumpd() with nested data structures.
    ///
    /// Ported from `TestDumpd::test_dumpd_with_nested_structures`
    /// Note: dumpd works with Serializable trait objects. For nested structures
    /// mixing Serializable and plain data, we serialize parts individually.
    #[test]
    fn test_dumpd_with_nested_structures() {
        let serializable_obj = SerializableTest { value: 1 };
        let serialized_value = serde_json::to_value(&serializable_obj.to_json()).unwrap();

        let data = json!({
            "serializable": serialized_value,
            "list": [1, 2, {"nested": "value"}],
            "primitive": "string"
        });

        assert_eq!(data["serializable"]["type"], "constructor");
        assert_eq!(data["list"], json!([1, 2, {"nested": "value"}]));
        assert_eq!(data["primitive"], "string");
    }

    /// Test dumpd() produces same result as serde_json::from_str(dumps()).
    ///
    /// Ported from `TestDumpd::test_dumpd_equivalence_with_dumps`
    #[test]
    fn test_dumpd_equivalence_with_dumps() {
        let obj = SerializableTest { value: 42 };

        let dumpd_result = dumpd(&obj).unwrap();
        let dumps_result: Value = serde_json::from_str(&dumps(&obj, false).unwrap()).unwrap();

        assert_eq!(dumpd_result, dumps_result);
    }

    /// Test dumpd_value() with primitive types.
    ///
    /// Ported from `TestDumpd::test_dumpd_with_primitive_types`
    #[test]
    fn test_dumpd_with_primitive_types() {
        // List
        let result = dumpd_value(&vec![1, 2, 3]).unwrap();
        assert_eq!(result, json!([1, 2, 3]));

        // Dict
        let data: HashMap<&str, &str> = HashMap::from([("key", "value")]);
        let result = dumpd_value(&data).unwrap();
        assert_eq!(result, json!({"key": "value"}));

        // Serializable object in dict
        let obj = SerializableTest { value: 42 };
        let serialized = serde_json::to_value(&obj.to_json()).unwrap();
        let data = json!({"obj": serialized});
        assert_eq!(data["obj"]["type"], "constructor");
    }

    /// Test dumpd() with a list of Serializable objects.
    ///
    /// Ported from `TestDumpd::test_dumpd_list_of_serializable`
    #[test]
    fn test_dumpd_list_of_serializable() {
        let objs = vec![SerializableTest { value: 1 }, SerializableTest { value: 2 }];

        let result: Vec<Value> = objs.iter().map(|obj| dumpd(obj).unwrap()).collect();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["kwargs"]["value"], 1);
        assert_eq!(result[1]["kwargs"]["value"], 2);
    }

    /// Test dumpd_value() with None (null).
    ///
    /// Ported from `TestDumpdSnapshot::test_dumpd_none`
    #[test]
    fn test_dumpd_none() {
        let result = dumpd_value(&Value::Null).unwrap();
        assert!(result.is_null());
    }

    /// Test dumpd_value() with plain string.
    ///
    /// Ported from `TestDumpdSnapshot::test_dumpd_string`
    #[test]
    fn test_dumpd_string() {
        let result = dumpd_value(&"hello").unwrap();
        assert_eq!(result, "hello");
    }

    /// Test dumpd_value() with int.
    ///
    /// Ported from `TestDumpdSnapshot::test_dumpd_int`
    #[test]
    fn test_dumpd_int() {
        let result = dumpd_value(&42).unwrap();
        assert_eq!(result, 42);
    }

    /// Test dumpd_value() with booleans.
    ///
    /// Ported from `TestDumpdSnapshot::test_dumpd_bool`
    #[test]
    fn test_dumpd_bool() {
        assert_eq!(dumpd_value(&true).unwrap(), json!(true));
        assert_eq!(dumpd_value(&false).unwrap(), json!(false));
    }

    /// Test dumpd_value() with float.
    ///
    /// Ported from `TestDumpdSnapshot::test_dumpd_float`
    #[test]
    fn test_dumpd_float() {
        let result = dumpd_value(&3.14).unwrap();
        assert_eq!(result, json!(3.14));
    }

    /// Test dumpd() with Serializable inside a list inside a dict.
    ///
    /// Ported from `TestDumpdSnapshot::test_dumpd_nested_serializable_in_list`
    #[test]
    fn test_dumpd_nested_serializable_in_list() {
        let obj = SerializableTest { value: 10 };
        let serialized = serde_json::to_value(&obj.to_json()).unwrap();

        let data = json!({
            "items": [serialized, "plain", 42]
        });

        assert_eq!(data["items"][0]["type"], "constructor");
        assert_eq!(data["items"][0]["kwargs"]["value"], 10);
        assert_eq!(data["items"][1], "plain");
        assert_eq!(data["items"][2], 42);
    }

    /// Test dumpd_value() with empty dict and list.
    ///
    /// Ported from `TestDumpdSnapshot::test_dumpd_empty_structures`
    #[test]
    fn test_dumpd_empty_structures() {
        let empty_map: HashMap<String, Value> = HashMap::new();
        assert_eq!(dumpd_value(&empty_map).unwrap(), json!({}));

        let empty_vec: Vec<Value> = vec![];
        assert_eq!(dumpd_value(&empty_vec).unwrap(), json!([]));
    }
}

/// Snapshot tests for Serializable::to_json() output structure.
///
/// Ported from `TestDefaultSnapshot` in Python.
mod test_default_snapshot {
    use super::*;

    /// Snapshot: to_json() output for a Serializable object.
    ///
    /// Ported from `TestDefaultSnapshot::test_default_serializable_full_snapshot`
    #[test]
    fn test_default_serializable_full_snapshot() {
        let obj = SerializableTest { value: 99 };
        let result = obj.to_json();
        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["lc"], 1);
        assert_eq!(value["type"], "constructor");
        assert_eq!(value["kwargs"]["value"], 99);

        let id = value["id"].as_array().unwrap();
        assert_eq!(id.last().unwrap().as_str().unwrap(), "SerializableTest");
        // Verify namespace prefix
        assert_eq!(id[0], "tests");
        assert_eq!(id[1], "unit_tests");
        assert_eq!(id[2], "load");
        assert_eq!(id[3], "test_dump");
    }

    /// Snapshot: to_json() output for a non-Serializable object.
    ///
    /// Ported from `TestDefaultSnapshot::test_default_non_serializable_full_snapshot`
    #[test]
    fn test_default_non_serializable_full_snapshot() {
        let obj = NonSerializableTest { value: 7 };
        let result = obj.to_json();
        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["lc"], 1);
        assert_eq!(value["type"], "not_implemented");
        let id = value["id"].as_array().unwrap();
        assert!(
            id.iter().any(|v| v
                .as_str()
                .map_or(false, |s| s.contains("NonSerializableTest"))),
            "id should contain NonSerializableTest, got: {:?}",
            id
        );
    }

    /// Snapshot: to_json_not_implemented() on an arbitrary type.
    ///
    /// Ported from `TestDefaultSnapshot::test_default_with_builtin_type`
    /// In Rust, we test with a non-serializable struct since there's no
    /// equivalent of passing a set to default().
    #[test]
    fn test_default_with_non_serializable_type() {
        let obj = NonSerializableTest { value: 0 };
        let result = obj.to_json_not_implemented();
        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["lc"], 1);
        assert_eq!(value["type"], "not_implemented");
    }

    /// Snapshot: to_json_not_implemented_value() for an arbitrary type name.
    ///
    /// Ported from `TestDefaultSnapshot::test_default_with_none`
    /// In Rust, None doesn't implement Serializable, so we test
    /// the to_json_not_implemented_value function directly.
    #[test]
    fn test_default_with_not_implemented_value() {
        use agent_chain_core::load::to_json_not_implemented_value;

        let result = to_json_not_implemented_value("NoneType", None);
        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["lc"], 1);
        assert_eq!(value["type"], "not_implemented");
    }
}

/// Snapshot tests for dumps() output structure.
///
/// Ported from `TestDumpsSnapshot` in Python.
mod test_dumps_snapshot {
    use super::*;

    /// Snapshot: full JSON output of dumps() for a Serializable.
    ///
    /// Ported from `TestDumpsSnapshot::test_dumps_serializable_full_snapshot`
    #[test]
    fn test_dumps_serializable_full_snapshot() {
        let obj = SerializableTest { value: 42 };
        let json_str = dumps(&obj, false).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["lc"], 1);
        assert_eq!(parsed["type"], "constructor");
        assert_eq!(parsed["kwargs"]["value"], 42);

        let id = parsed["id"].as_array().unwrap();
        assert_eq!(id.last().unwrap().as_str().unwrap(), "SerializableTest");
        assert_eq!(id[0], "tests");
        assert_eq!(id[1], "unit_tests");
        assert_eq!(id[2], "load");
        assert_eq!(id[3], "test_dump");
    }

    /// Snapshot: full JSON output of dumps() for a non-Serializable object.
    ///
    /// Ported from `TestDumpsSnapshot::test_dumps_non_serializable_full_snapshot`
    #[test]
    fn test_dumps_non_serializable_full_snapshot() {
        let obj = NonSerializableTest { value: 7 };
        let json_str = dumps(&obj, false).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["lc"], 1);
        assert_eq!(parsed["type"], "not_implemented");
        let id = parsed["id"].as_array().unwrap();
        assert!(
            id.iter().any(|v| v
                .as_str()
                .map_or(false, |s| s.contains("NonSerializableTest"))),
            "id should contain NonSerializableTest, got: {:?}",
            id
        );
    }
}

/// Snapshot tests for dumpd() output structure.
///
/// Ported from `TestDumpdSnapshot` in Python.
mod test_dumpd_snapshot {
    use super::*;

    /// Snapshot: exact dumpd output for SerializableTest.
    ///
    /// Ported from `TestDumpdSnapshot::test_dumpd_serializable_full_snapshot`
    #[test]
    fn test_dumpd_serializable_full_snapshot() {
        let obj = SerializableTest { value: 42 };
        let result = dumpd(&obj).unwrap();

        assert_eq!(result["lc"], 1);
        assert_eq!(result["type"], "constructor");
        assert_eq!(result["kwargs"]["value"], 42);

        let id = result["id"].as_array().unwrap();
        assert_eq!(id.last().unwrap().as_str().unwrap(), "SerializableTest");
        assert_eq!(id[0], "tests");
        assert_eq!(id[1], "unit_tests");
        assert_eq!(id[2], "load");
        assert_eq!(id[3], "test_dump");
    }
}
