//! Usage utilities.
//!
//! Adapted from langchain_core/utils/usage.py

use std::collections::HashMap;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
