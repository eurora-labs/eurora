use serde::Serialize;
use serde_json::Value;

use super::serializable::Serializable;

pub fn dumps<T: Serializable + Serialize>(obj: &T, pretty: bool) -> crate::Result<String> {
    let serialized = obj.to_json();
    let json = if pretty {
        serde_json::to_string_pretty(&serialized)?
    } else {
        serde_json::to_string(&serialized)?
    };
    Ok(json)
}

pub fn dumpd<T: Serializable + Serialize>(obj: &T) -> crate::Result<Value> {
    let serialized = obj.to_json();
    Ok(serde_json::to_value(&serialized)?)
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
