use serde_json::Value;

pub fn stringify_value(val: &Value) -> String {
    match val {
        Value::String(s) => s.clone(),
        Value::Object(map) => {
            let inner = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, stringify_value(v)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\n{}", inner)
        }
        Value::Array(arr) => arr
            .iter()
            .map(stringify_value)
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
    }
}

pub fn stringify_dict(data: &std::collections::HashMap<String, String>) -> String {
    data.iter()
        .map(|(k, v)| format!("{}: {}\n", k, v))
        .collect()
}

pub fn comma_list(items: &[String]) -> String {
    items.join(", ")
}

pub fn sanitize_for_postgres(text: &str, replacement: &str) -> String {
    text.replace('\x00', replacement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_stringify_value_string() {
        assert_eq!(stringify_value(&json!("hello")), "hello");
    }

    #[test]
    fn test_stringify_value_number() {
        assert_eq!(stringify_value(&json!(42)), "42");
        assert_eq!(stringify_value(&json!(1.23)), "1.23");
    }

    #[test]
    fn test_stringify_value_bool() {
        assert_eq!(stringify_value(&json!(true)), "true");
        assert_eq!(stringify_value(&json!(false)), "false");
    }

    #[test]
    fn test_stringify_value_null() {
        assert_eq!(stringify_value(&json!(null)), "null");
    }

    #[test]
    fn test_stringify_value_array() {
        assert_eq!(stringify_value(&json!(["a", "b", "c"])), "a\nb\nc");
    }

    #[test]
    fn test_stringify_value_object() {
        let result = stringify_value(&json!({"key": "value"}));
        assert!(result.contains("key: value"));
    }

    #[test]
    fn test_comma_list() {
        let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        assert_eq!(comma_list(&items), "a, b, c");
    }

    #[test]
    fn test_comma_list_empty() {
        let items: Vec<String> = vec![];
        assert_eq!(comma_list(&items), "");
    }

    #[test]
    fn test_sanitize_for_postgres() {
        assert_eq!(sanitize_for_postgres("Hello\x00world", ""), "Helloworld");
        assert_eq!(sanitize_for_postgres("Hello\x00world", " "), "Hello world");
        assert_eq!(sanitize_for_postgres("No nulls here", ""), "No nulls here");
    }
}
