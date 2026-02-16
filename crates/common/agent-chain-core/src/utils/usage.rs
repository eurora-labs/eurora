use serde_json::{Map, Value};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq)]
pub enum UsageError {
    MaxDepthExceeded(usize),
    TypeMismatch {
        key: String,
        left_type: String,
        right_type: String,
    },
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsageError::MaxDepthExceeded(depth) => {
                write!(f, "max_depth={} exceeded, unable to combine dicts", depth)
            }
            UsageError::TypeMismatch {
                key,
                left_type,
                right_type,
            } => {
                write!(
                    f,
                    "Unknown value types for key \'{}\': {} and {}. Only dict and int values are supported.",
                    key, left_type, right_type
                )
            }
        }
    }
}

impl std::error::Error for UsageError {}

pub fn dict_int_op<F>(
    left: &Value,
    right: &Value,
    op: F,
    default: i64,
    max_depth: usize,
) -> Result<Value, UsageError>
where
    F: Fn(i64, i64) -> i64 + Copy,
{
    dict_int_op_impl(left, right, op, default, 0, max_depth)
}

fn dict_int_op_impl<F>(
    left: &Value,
    right: &Value,
    op: F,
    default: i64,
    depth: usize,
    max_depth: usize,
) -> Result<Value, UsageError>
where
    F: Fn(i64, i64) -> i64 + Copy,
{
    if depth >= max_depth {
        return Err(UsageError::MaxDepthExceeded(max_depth));
    }

    let empty_map = Map::new();
    let left_obj = left.as_object().unwrap_or(&empty_map);
    let right_obj = right.as_object().unwrap_or(&empty_map);

    let all_keys: HashSet<_> = left_obj.keys().chain(right_obj.keys()).cloned().collect();

    let mut combined = Map::new();

    let default_int = Value::Number(default.into());
    let default_obj = Value::Object(Map::new());

    for key in all_keys {
        let left_val = left_obj.get(&key);
        let right_val = right_obj.get(&key);

        let left_default_int = left_val.unwrap_or(&default_int);
        let right_default_int = right_val.unwrap_or(&default_int);

        let left_default_obj = left_val.unwrap_or(&default_obj);
        let right_default_obj = right_val.unwrap_or(&default_obj);

        if left_default_int.is_i64() && right_default_int.is_i64() {
            let left_int = left_default_int.as_i64().unwrap_or(default);
            let right_int = right_default_int.as_i64().unwrap_or(default);
            combined.insert(key, Value::Number(op(left_int, right_int).into()));
        } else if left_default_obj.is_object() && right_default_obj.is_object() {
            let nested = dict_int_op_impl(
                left_default_obj,
                right_default_obj,
                op,
                default,
                depth + 1,
                max_depth,
            )?;
            combined.insert(key, nested);
        } else {
            let types: Vec<String> = [left_val, right_val]
                .iter()
                .filter_map(|v| v.map(json_type_name).map(String::from))
                .collect();
            return Err(UsageError::TypeMismatch {
                key,
                left_type: types.first().cloned().unwrap_or_default(),
                right_type: types.last().cloned().unwrap_or_default(),
            });
        }
    }

    Ok(Value::Object(combined))
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dict_int_op_add() {
        let left = json!({
            "a": 1,
            "b": 2
        });

        let right = json!({
            "a": 3,
            "c": 4
        });

        let result = dict_int_op(&left, &right, |a, b| a + b, 0, 100).unwrap();

        assert_eq!(result["a"], 4);
        assert_eq!(result["b"], 2);
        assert_eq!(result["c"], 4);
    }

    #[test]
    fn test_dict_int_op_add_nested() {
        let left = json!({
            "input_tokens": 5,
            "output_tokens": 0,
            "total_tokens": 5,
            "input_token_details": {
                "cache_read": 3
            }
        });

        let right = json!({
            "input_tokens": 0,
            "output_tokens": 10,
            "total_tokens": 10,
            "output_token_details": {
                "reasoning": 4
            }
        });

        let result = dict_int_op(&left, &right, |a, b| a + b, 0, 100).unwrap();

        assert_eq!(result["input_tokens"], 5);
        assert_eq!(result["output_tokens"], 10);
        assert_eq!(result["total_tokens"], 15);
        assert_eq!(result["input_token_details"]["cache_read"], 3);
        assert_eq!(result["output_token_details"]["reasoning"], 4);
    }

    #[test]
    fn test_dict_int_op_sub() {
        let left = json!({
            "a": 5,
            "b": 3
        });

        let right = json!({
            "a": 2
        });

        let result = dict_int_op(&left, &right, |a, b| a - b, 0, 100).unwrap();

        assert_eq!(result["a"], 3);
        assert_eq!(result["b"], 3);
    }

    #[test]
    fn test_max_depth_exceeded() {
        fn create_nested(depth: usize) -> Value {
            if depth == 0 {
                json!({"value": 1})
            } else {
                json!({"nested": create_nested(depth - 1)})
            }
        }

        let left = create_nested(150);
        let right = create_nested(150);

        let result = dict_int_op(&left, &right, |a, b| a + b, 0, 100);
        assert!(matches!(result, Err(UsageError::MaxDepthExceeded(_))));
    }

    #[test]
    fn test_dict_int_op_sub_floor() {
        let left = json!({
            "input_tokens": 5,
            "output_tokens": 10,
            "total_tokens": 15,
            "input_token_details": {
                "cache_read": 4
            }
        });

        let right = json!({
            "input_tokens": 3,
            "output_tokens": 8,
            "total_tokens": 11,
            "output_token_details": {
                "reasoning": 4
            }
        });

        let result = dict_int_op(&left, &right, |a, b| (a - b).max(0), 0, 100).unwrap();

        assert_eq!(result["input_tokens"], 2);
        assert_eq!(result["output_tokens"], 2);
        assert_eq!(result["total_tokens"], 4);
        assert_eq!(result["input_token_details"]["cache_read"], 4);
        assert_eq!(result["output_token_details"]["reasoning"], 0);
    }
}
