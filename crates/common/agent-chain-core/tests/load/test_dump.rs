//! Unit tests for the dump module.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/load/test_dump.py`

use agent_chain_core::load::{Serializable, Serialized, dumpd, dumps};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
                        .is_some_and(|s| s.contains("NonSerializableTest"))
                );
            }
            other => panic!("Expected NotImplemented, got {:?}", other),
        }
    }
}

/// Tests for the dumps() function.
mod test_dumps {
    use super::*;

    /// Ported from `TestDumps::test_dumps_basic`
    #[test]
    fn test_dumps_basic() {
        let obj = SerializableTest { value: 42 };
        let json_str = dumps(&obj, false).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["type"], "constructor");
        assert_eq!(parsed["kwargs"]["value"], 42);
    }

    /// Ported from `TestDumps::test_dumps_with_pretty_flag`
    #[test]
    fn test_dumps_with_pretty_flag() {
        let obj = SerializableTest { value: 42 };
        let json_str = dumps(&obj, true).unwrap();

        assert!(json_str.contains("  "));
        let parsed: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["kwargs"]["value"], 42);
    }

    /// Ported from `TestDumps::test_dumps_with_non_serializable_fallback`
    #[test]
    fn test_dumps_with_non_serializable_fallback() {
        let obj = NonSerializableTest { value: 42 };
        let json_str = dumps(&obj, false).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["type"], "not_implemented");
    }

    /// Ported from `TestDumps::test_dumps_with_pretty_and_typeerror_fallback`
    #[test]
    fn test_dumps_with_pretty_and_non_serializable_fallback() {
        let obj = NonSerializableTest { value: 42 };
        let json_str = dumps(&obj, true).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        assert!(json_str.contains("  "));
        assert_eq!(parsed["type"], "not_implemented");
    }

    /// Ported from `TestDumps::test_dumps_nested_structures`
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

    /// Ported from `TestDumpd::test_dumpd_basic`
    #[test]
    fn test_dumpd_basic() {
        let obj = SerializableTest { value: 42 };
        let result = dumpd(&obj).unwrap();

        assert!(result.is_object());
        assert_eq!(result["type"], "constructor");
        assert_eq!(result["kwargs"]["value"], 42);
    }

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
                .is_some_and(|s| s.contains("NonSerializableTest"))),
            "id should contain NonSerializableTest, got: {:?}",
            id
        );
    }

    /// Ported from `TestDumpd::test_dumpd_with_nested_structures`
    #[test]
    fn test_dumpd_with_nested_structures() {
        let serializable_obj = SerializableTest { value: 1 };
        let serialized_value = serde_json::to_value(serializable_obj.to_json()).unwrap();

        let data = json!({
            "serializable": serialized_value,
            "list": [1, 2, {"nested": "value"}],
            "primitive": "string"
        });

        assert_eq!(data["serializable"]["type"], "constructor");
        assert_eq!(data["list"], json!([1, 2, {"nested": "value"}]));
        assert_eq!(data["primitive"], "string");
    }

    /// Ported from `TestDumpd::test_dumpd_equivalence_with_dumps`
    #[test]
    fn test_dumpd_equivalence_with_dumps() {
        let obj = SerializableTest { value: 42 };

        let dumpd_result = dumpd(&obj).unwrap();
        let dumps_result: Value = serde_json::from_str(&dumps(&obj, false).unwrap()).unwrap();

        assert_eq!(dumpd_result, dumps_result);
    }

    /// Ported from `TestDumpd::test_dumpd_list_of_serializable`
    #[test]
    fn test_dumpd_list_of_serializable() {
        let objs = [SerializableTest { value: 1 }, SerializableTest { value: 2 }];

        let result: Vec<Value> = objs.iter().map(|obj| dumpd(obj).unwrap()).collect();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["kwargs"]["value"], 1);
        assert_eq!(result[1]["kwargs"]["value"], 2);
    }
}

/// Snapshot tests for Serializable::to_json() output structure.
///
/// Ported from `TestDefaultSnapshot` in Python.
mod test_default_snapshot {
    use super::*;

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
        assert_eq!(id[0], "tests");
        assert_eq!(id[1], "unit_tests");
        assert_eq!(id[2], "load");
        assert_eq!(id[3], "test_dump");
    }

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
                .is_some_and(|s| s.contains("NonSerializableTest"))),
            "id should contain NonSerializableTest, got: {:?}",
            id
        );
    }

    /// Ported from `TestDefaultSnapshot::test_default_with_builtin_type`
    #[test]
    fn test_default_with_non_serializable_type() {
        let obj = NonSerializableTest { value: 0 };
        let result = obj.to_json_not_implemented();
        let value = serde_json::to_value(&result).unwrap();

        assert_eq!(value["lc"], 1);
        assert_eq!(value["type"], "not_implemented");
    }

    /// Ported from `TestDefaultSnapshot::test_default_with_none`
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
                .is_some_and(|s| s.contains("NonSerializableTest"))),
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
