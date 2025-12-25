//! Usage utilities.
//!
//! Adapted from langchain_core/utils/usage.py

use serde_json::{Map, Value};
use std::collections::HashMap;
use std::collections::HashSet;

/// Perform an integer operation on nested dictionaries.
///
/// This function recursively applies an operation to integer values in
/// nested dictionaries.
///
/// # Arguments
///
/// * `left` - The first dictionary.
/// * `right` - The second dictionary.
/// * `op` - The operation to apply (e.g., addition, subtraction).
/// * `default` - The default value for missing keys.
/// * `max_depth` - Maximum recursion depth (default: 100).
///
/// # Returns
///
/// A new dictionary with the operation applied, or an error if max depth exceeded.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use agent_chain_core::utils::usage::{dict_int_op, dict_int_add};
///
/// let mut left = HashMap::new();
/// left.insert("a".to_string(), UsageValue::Int(1));
/// left.insert("b".to_string(), UsageValue::Int(2));
///
/// let mut right = HashMap::new();
/// right.insert("a".to_string(), UsageValue::Int(3));
/// right.insert("c".to_string(), UsageValue::Int(4));
///
/// let result = dict_int_add(&left, &right).unwrap();
/// // result["a"] == 4, result["b"] == 2, result["c"] == 4
/// ```
pub fn dict_int_op<F>(
    left: &HashMap<String, UsageValue>,
    right: &HashMap<String, UsageValue>,
    op: F,
    default: i64,
    max_depth: usize,
) -> Result<HashMap<String, UsageValue>, UsageError>
where
    F: Fn(i64, i64) -> i64 + Copy,
{
    dict_int_op_impl(left, right, op, default, 0, max_depth)
}

fn dict_int_op_impl<F>(
    left: &HashMap<String, UsageValue>,
    right: &HashMap<String, UsageValue>,
    op: F,
    default: i64,
    depth: usize,
    max_depth: usize,
) -> Result<HashMap<String, UsageValue>, UsageError>
where
    F: Fn(i64, i64) -> i64 + Copy,
{
    if depth >= max_depth {
        return Err(UsageError::MaxDepthExceeded(max_depth));
    }

    let mut combined = HashMap::new();
    let all_keys: std::collections::HashSet<_> = left.keys().chain(right.keys()).cloned().collect();

    for k in all_keys {
        let left_val = left.get(&k);
        let right_val = right.get(&k);

        match (left_val, right_val) {
            (Some(UsageValue::Int(l)), Some(UsageValue::Int(r))) => {
                combined.insert(k, UsageValue::Int(op(*l, *r)));
            }
            (Some(UsageValue::Int(l)), None) => {
                combined.insert(k, UsageValue::Int(op(*l, default)));
            }
            (None, Some(UsageValue::Int(r))) => {
                combined.insert(k, UsageValue::Int(op(default, *r)));
            }
            (Some(UsageValue::Dict(l)), Some(UsageValue::Dict(r))) => {
                let nested = dict_int_op_impl(l, r, op, default, depth + 1, max_depth)?;
                combined.insert(k, UsageValue::Dict(nested));
            }
            (Some(UsageValue::Dict(l)), None) => {
                let empty = HashMap::new();
                let nested = dict_int_op_impl(l, &empty, op, default, depth + 1, max_depth)?;
                combined.insert(k, UsageValue::Dict(nested));
            }
            (None, Some(UsageValue::Dict(r))) => {
                let empty = HashMap::new();
                let nested = dict_int_op_impl(&empty, r, op, default, depth + 1, max_depth)?;
                combined.insert(k, UsageValue::Dict(nested));
            }
            (Some(l), Some(r)) => {
                return Err(UsageError::TypeMismatch {
                    key: k,
                    left_type: l.type_name().to_string(),
                    right_type: r.type_name().to_string(),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    Ok(combined)
}

/// Add two usage dictionaries together.
///
/// # Arguments
///
/// * `left` - The first dictionary.
/// * `right` - The second dictionary.
///
/// # Returns
///
/// A new dictionary with values added together.
pub fn dict_int_add(
    left: &HashMap<String, UsageValue>,
    right: &HashMap<String, UsageValue>,
) -> Result<HashMap<String, UsageValue>, UsageError> {
    dict_int_op(left, right, |a, b| a + b, 0, 100)
}

/// Subtract one usage dictionary from another.
///
/// # Arguments
///
/// * `left` - The first dictionary.
/// * `right` - The dictionary to subtract.
///
/// # Returns
///
/// A new dictionary with values subtracted.
pub fn dict_int_sub(
    left: &HashMap<String, UsageValue>,
    right: &HashMap<String, UsageValue>,
) -> Result<HashMap<String, UsageValue>, UsageError> {
    dict_int_op(left, right, |a, b| a - b, 0, 100)
}

/// A value in a usage dictionary.
#[derive(Debug, Clone, PartialEq)]
pub enum UsageValue {
    /// An integer value.
    Int(i64),
    /// A nested dictionary.
    Dict(HashMap<String, UsageValue>),
}

impl UsageValue {
    /// Get the type name of this value.
    pub fn type_name(&self) -> &'static str {
        match self {
            UsageValue::Int(_) => "int",
            UsageValue::Dict(_) => "dict",
        }
    }

    /// Try to get the value as an integer.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            UsageValue::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to get the value as a dictionary.
    pub fn as_dict(&self) -> Option<&HashMap<String, UsageValue>> {
        match self {
            UsageValue::Dict(v) => Some(v),
            _ => None,
        }
    }
}

impl From<i64> for UsageValue {
    fn from(v: i64) -> Self {
        UsageValue::Int(v)
    }
}

impl From<i32> for UsageValue {
    fn from(v: i32) -> Self {
        UsageValue::Int(v as i64)
    }
}

impl From<HashMap<String, UsageValue>> for UsageValue {
    fn from(v: HashMap<String, UsageValue>) -> Self {
        UsageValue::Dict(v)
    }
}

/// Error types for usage operations.
#[derive(Debug, Clone, PartialEq)]
pub enum UsageError {
    /// Maximum recursion depth exceeded.
    MaxDepthExceeded(usize),
    /// Type mismatch between left and right values.
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
                    "Unknown value types for key '{}': {} and {}. Only dict and int values are supported.",
                    key, left_type, right_type
                )
            }
        }
    }
}

impl std::error::Error for UsageError {}

/// Perform an integer operation on nested JSON dictionaries.
///
/// This function recursively applies an operation to integer values in
/// nested JSON objects. This matches the Python `_dict_int_op` function
/// from `langchain_core.utils.usage`.
///
/// # Arguments
///
/// * `left` - The first JSON object.
/// * `right` - The second JSON object.
/// * `op` - The operation to apply (e.g., addition, subtraction).
/// * `default` - The default value for missing keys.
/// * `max_depth` - Maximum recursion depth (default: 100).
///
/// # Returns
///
/// A new JSON object with the operation applied, or an error if max depth exceeded.
pub fn dict_int_op_json<F>(
    left: &Value,
    right: &Value,
    op: F,
    default: i64,
    max_depth: usize,
) -> Result<Value, UsageError>
where
    F: Fn(i64, i64) -> i64 + Copy,
{
    dict_int_op_json_impl(left, right, op, default, 0, max_depth)
}

fn dict_int_op_json_impl<F>(
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

    for k in all_keys {
        let left_val = left_obj.get(&k);
        let right_val = right_obj.get(&k);

        match (left_val, right_val) {
            // Both are integers
            (Some(Value::Number(l)), Some(Value::Number(r)))
                if l.is_i64() && r.is_i64() =>
            {
                let l_int = l.as_i64().unwrap_or(default);
                let r_int = r.as_i64().unwrap_or(default);
                combined.insert(k, Value::Number(op(l_int, r_int).into()));
            }
            // Left is int, right is missing
            (Some(Value::Number(l)), None) if l.is_i64() => {
                let l_int = l.as_i64().unwrap_or(default);
                combined.insert(k, Value::Number(op(l_int, default).into()));
            }
            // Right is int, left is missing
            (None, Some(Value::Number(r))) if r.is_i64() => {
                let r_int = r.as_i64().unwrap_or(default);
                combined.insert(k, Value::Number(op(default, r_int).into()));
            }
            // Both are objects
            (Some(Value::Object(_)), Some(Value::Object(_))) => {
                let nested = dict_int_op_json_impl(
                    left_val.unwrap(),
                    right_val.unwrap(),
                    op,
                    default,
                    depth + 1,
                    max_depth,
                )?;
                combined.insert(k, nested);
            }
            // Left is object, right is missing
            (Some(Value::Object(_)), None) => {
                let nested = dict_int_op_json_impl(
                    left_val.unwrap(),
                    &Value::Object(Map::new()),
                    op,
                    default,
                    depth + 1,
                    max_depth,
                )?;
                combined.insert(k, nested);
            }
            // Right is object, left is missing
            (None, Some(Value::Object(_))) => {
                let nested = dict_int_op_json_impl(
                    &Value::Object(Map::new()),
                    right_val.unwrap(),
                    op,
                    default,
                    depth + 1,
                    max_depth,
                )?;
                combined.insert(k, nested);
            }
            // Neither present (shouldn't happen due to all_keys)
            (None, None) => {}
            // Type mismatch or unsupported types
            (Some(l), Some(r)) => {
                return Err(UsageError::TypeMismatch {
                    key: k,
                    left_type: json_type_name(l).to_string(),
                    right_type: json_type_name(r).to_string(),
                });
            }
            // One side has unsupported type
            (Some(v), None) | (None, Some(v)) => {
                // Just copy over non-int/non-object values
                combined.insert(k, v.clone());
            }
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

/// Add two JSON usage dictionaries together.
///
/// # Arguments
///
/// * `left` - The first JSON object.
/// * `right` - The second JSON object.
///
/// # Returns
///
/// A new JSON object with values added together.
pub fn dict_int_add_json(left: &Value, right: &Value) -> Result<Value, UsageError> {
    dict_int_op_json(left, right, |a, b| a + b, 0, 100)
}

/// Subtract one JSON usage dictionary from another, with floor at 0.
///
/// Token counts cannot be negative so the actual operation is `max(left - right, 0)`.
///
/// # Arguments
///
/// * `left` - The first JSON object.
/// * `right` - The JSON object to subtract.
///
/// # Returns
///
/// A new JSON object with values subtracted (floored at 0).
pub fn dict_int_sub_floor_json(left: &Value, right: &Value) -> Result<Value, UsageError> {
    dict_int_op_json(left, right, |a, b| (a - b).max(0), 0, 100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dict_int_add() {
        let mut left = HashMap::new();
        left.insert("a".to_string(), UsageValue::Int(1));
        left.insert("b".to_string(), UsageValue::Int(2));

        let mut right = HashMap::new();
        right.insert("a".to_string(), UsageValue::Int(3));
        right.insert("c".to_string(), UsageValue::Int(4));

        let result = dict_int_add(&left, &right).unwrap();

        assert_eq!(result.get("a").unwrap().as_int(), Some(4));
        assert_eq!(result.get("b").unwrap().as_int(), Some(2));
        assert_eq!(result.get("c").unwrap().as_int(), Some(4));
    }

    #[test]
    fn test_dict_int_add_nested() {
        let mut inner_left = HashMap::new();
        inner_left.insert("x".to_string(), UsageValue::Int(1));

        let mut left = HashMap::new();
        left.insert("nested".to_string(), UsageValue::Dict(inner_left));

        let mut inner_right = HashMap::new();
        inner_right.insert("x".to_string(), UsageValue::Int(2));
        inner_right.insert("y".to_string(), UsageValue::Int(3));

        let mut right = HashMap::new();
        right.insert("nested".to_string(), UsageValue::Dict(inner_right));

        let result = dict_int_add(&left, &right).unwrap();

        let nested = result.get("nested").unwrap().as_dict().unwrap();
        assert_eq!(nested.get("x").unwrap().as_int(), Some(3));
        assert_eq!(nested.get("y").unwrap().as_int(), Some(3));
    }

    #[test]
    fn test_dict_int_sub() {
        let mut left = HashMap::new();
        left.insert("a".to_string(), UsageValue::Int(5));
        left.insert("b".to_string(), UsageValue::Int(3));

        let mut right = HashMap::new();
        right.insert("a".to_string(), UsageValue::Int(2));

        let result = dict_int_sub(&left, &right).unwrap();

        assert_eq!(result.get("a").unwrap().as_int(), Some(3));
        assert_eq!(result.get("b").unwrap().as_int(), Some(3));
    }

    #[test]
    fn test_max_depth_exceeded() {
        fn create_nested(depth: usize) -> HashMap<String, UsageValue> {
            if depth == 0 {
                let mut m = HashMap::new();
                m.insert("value".to_string(), UsageValue::Int(1));
                m
            } else {
                let mut m = HashMap::new();
                m.insert(
                    "nested".to_string(),
                    UsageValue::Dict(create_nested(depth - 1)),
                );
                m
            }
        }

        let left = create_nested(150);
        let right = create_nested(150);

        let result = dict_int_op(&left, &right, |a, b| a + b, 0, 100);
        assert!(matches!(result, Err(UsageError::MaxDepthExceeded(_))));
    }

    #[test]
    fn test_dict_int_add_json() {
        let left = json!({
            "a": 1,
            "b": 2
        });

        let right = json!({
            "a": 3,
            "c": 4
        });

        let result = dict_int_add_json(&left, &right).unwrap();

        assert_eq!(result["a"], 4);
        assert_eq!(result["b"], 2);
        assert_eq!(result["c"], 4);
    }

    #[test]
    fn test_dict_int_add_json_nested() {
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

        let result = dict_int_add_json(&left, &right).unwrap();

        assert_eq!(result["input_tokens"], 5);
        assert_eq!(result["output_tokens"], 10);
        assert_eq!(result["total_tokens"], 15);
        assert_eq!(result["input_token_details"]["cache_read"], 3);
        assert_eq!(result["output_token_details"]["reasoning"], 4);
    }

    #[test]
    fn test_dict_int_sub_floor_json() {
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

        let result = dict_int_sub_floor_json(&left, &right).unwrap();

        assert_eq!(result["input_tokens"], 2);
        assert_eq!(result["output_tokens"], 2);
        assert_eq!(result["total_tokens"], 4);
        assert_eq!(result["input_token_details"]["cache_read"], 4);
        // reasoning should be 0 because 0 - 4 = -4, floored to 0
        assert_eq!(result["output_token_details"]["reasoning"], 0);
    }
}
