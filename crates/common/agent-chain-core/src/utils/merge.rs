use serde_json::{Map, Value};

pub fn merge_dicts(left: Value, others: Vec<Value>) -> Result<Value, MergeError> {
    let mut merged = match left {
        Value::Object(map) => map,
        _ => return Err(MergeError::NotAnObject),
    };

    for right in others {
        let right_map = match right {
            Value::Object(map) => map,
            Value::Null => continue,
            _ => return Err(MergeError::NotAnObject),
        };

        for (right_k, right_v) in right_map {
            if !merged.contains_key(&right_k)
                || (right_v != Value::Null && merged.get(&right_k) == Some(&Value::Null))
            {
                merged.insert(right_k, right_v);
            } else if right_v == Value::Null {
                continue;
            } else {
                let left_v = merged.get(&right_k).expect("key exists in merged");

                if !values_same_type(left_v, &right_v) {
                    return Err(MergeError::TypeMismatch {
                        key: right_k.clone(),
                        left_type: type_name(left_v),
                        right_type: type_name(&right_v),
                    });
                }

                match (left_v.clone(), right_v.clone()) {
                    (Value::String(left_str), Value::String(right_str)) => {
                        if (right_k == "index" && left_str.starts_with("lc_"))
                            || ((right_k == "id"
                                || right_k == "output_version"
                                || right_k == "model_provider")
                                && left_str == right_str)
                        {
                            continue;
                        }
                        merged.insert(right_k, Value::String(left_str + &right_str));
                    }
                    (Value::Object(_), Value::Object(_)) => {
                        let merged_nested = merge_dicts(
                            merged.remove(&right_k).expect("key exists in merged"),
                            vec![right_v],
                        )?;
                        merged.insert(right_k, merged_nested);
                    }
                    (Value::Array(left_arr), Value::Array(right_arr)) => {
                        let merged_list = merge_lists(Some(left_arr), vec![Some(right_arr)])?;
                        merged.insert(right_k, Value::Array(merged_list.unwrap_or_default()));
                    }
                    (Value::Number(left_num), Value::Number(right_num)) => {
                        if left_num == right_num {
                            continue;
                        }
                        if let (Some(left_i), Some(right_i)) =
                            (left_num.as_i64(), right_num.as_i64())
                        {
                            merged.insert(
                                right_k,
                                Value::Number(serde_json::Number::from(left_i + right_i)),
                            );
                        } else if let (Some(left_f), Some(right_f)) =
                            (left_num.as_f64(), right_num.as_f64())
                            && let Some(num) = serde_json::Number::from_f64(left_f + right_f)
                        {
                            merged.insert(right_k, Value::Number(num));
                        }
                    }
                    (left_v, right_v) if left_v == right_v => {
                        continue;
                    }
                    (left_v, _) => {
                        return Err(MergeError::UnsupportedType {
                            key: right_k,
                            value_type: type_name(&left_v),
                        });
                    }
                }
            }
        }
    }

    Ok(Value::Object(merged))
}

pub fn merge_lists(
    left: Option<Vec<Value>>,
    others: Vec<Option<Vec<Value>>>,
) -> Result<Option<Vec<Value>>, MergeError> {
    let mut merged = left.map(|v| v.to_vec());

    for other in others {
        let Some(other_vec) = other else {
            continue;
        };

        if let Some(ref mut merged_vec) = merged {
            for e in other_vec {
                if let Value::Object(ref e_map) = e
                    && let Some(index) = e_map.get("index")
                {
                    let should_merge = match index {
                        Value::Number(n) => n.as_i64().is_some(),
                        Value::String(s) => s.starts_with("lc_"),
                        _ => false,
                    };

                    if should_merge {
                        let to_merge: Vec<usize> = merged_vec
                            .iter()
                            .enumerate()
                            .filter_map(|(i, e_left)| {
                                if let Value::Object(left_map) = e_left
                                    && left_map.get("index") == Some(index)
                                {
                                    return Some(i);
                                }
                                None
                            })
                            .collect();

                        if !to_merge.is_empty() {
                            let merge_idx = to_merge[0];
                            let left_elem = &merged_vec[merge_idx];

                            let left_type = left_elem
                                .as_object()
                                .and_then(|m| m.get("type"))
                                .and_then(|t| t.as_str());

                            let new_e: Value = if left_type.is_some() {
                                let e_type = e_map.get("type").and_then(|t| t.as_str());

                                if e_type == Some("non_standard") && e_map.contains_key("value") {
                                    if left_type != Some("non_standard") {
                                        let mut extras = Map::new();
                                        if let Some(Value::Object(value_map)) = e_map.get("value") {
                                            for (k, v) in value_map {
                                                if k != "type" {
                                                    extras.insert(k.clone(), v.clone());
                                                }
                                            }
                                        }
                                        Value::Object(
                                            [("extras".to_string(), Value::Object(extras))]
                                                .into_iter()
                                                .collect(),
                                        )
                                    } else {
                                        let mut new_map = Map::new();
                                        let mut value_map = Map::new();
                                        if let Some(Value::Object(orig_value)) = e_map.get("value")
                                        {
                                            for (k, v) in orig_value {
                                                if k != "type" {
                                                    value_map.insert(k.clone(), v.clone());
                                                }
                                            }
                                        }
                                        new_map
                                            .insert("value".to_string(), Value::Object(value_map));
                                        if let Some(idx) = e_map.get("index") {
                                            new_map.insert("index".to_string(), idx.clone());
                                        }
                                        Value::Object(new_map)
                                    }
                                } else {
                                    let mut new_map = Map::new();
                                    for (k, v) in e_map {
                                        if k != "type" {
                                            new_map.insert(k.clone(), v.clone());
                                        }
                                    }
                                    Value::Object(new_map)
                                }
                            } else {
                                e.clone()
                            };

                            let left_val = merged_vec.remove(merge_idx);
                            let merged_val = merge_dicts(left_val, vec![new_e])?;
                            merged_vec.insert(merge_idx, merged_val);
                        } else {
                            merged_vec.push(e);
                        }
                        continue;
                    }
                }
                merged_vec.push(e);
            }
        } else {
            merged = Some(other_vec);
        }
    }

    Ok(merged)
}

pub fn merge_obj(left: Value, right: Value) -> Result<Value, MergeError> {
    if left == Value::Null || right == Value::Null {
        return Ok(if left != Value::Null { left } else { right });
    }

    if !values_same_type(&left, &right) {
        return Err(MergeError::TypeMismatch {
            key: String::new(),
            left_type: type_name(&left),
            right_type: type_name(&right),
        });
    }

    match (&left, &right) {
        (Value::String(l), Value::String(r)) => Ok(Value::String(l.clone() + r)),
        (Value::Object(_), Value::Object(_)) => merge_dicts(left, vec![right]),
        (Value::Array(l), Value::Array(r)) => {
            let merged = merge_lists(Some(l.clone()), vec![Some(r.clone())])?;
            Ok(Value::Array(merged.unwrap_or_default()))
        }
        (l, r) if l == r => Ok(left),
        _ => Err(MergeError::UnableToMerge {
            left_type: type_name(&left),
            right_type: type_name(&right),
        }),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeError {
    NotAnObject,
    TypeMismatch {
        key: String,
        left_type: String,
        right_type: String,
    },
    UnsupportedType {
        key: String,
        value_type: String,
    },
    UnableToMerge {
        left_type: String,
        right_type: String,
    },
}

impl std::fmt::Display for MergeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeError::NotAnObject => write!(f, "Value is not a JSON object"),
            MergeError::TypeMismatch {
                key,
                left_type,
                right_type,
            } => {
                write!(
                    f,
                    "additional_kwargs[\"{}\"] already exists in this message, but with a different type. Left type: {}, Right type: {}",
                    key, left_type, right_type
                )
            }
            MergeError::UnsupportedType { key, value_type } => {
                write!(
                    f,
                    "Additional kwargs key {} already exists in left dict and value has unsupported type {}",
                    key, value_type
                )
            }
            MergeError::UnableToMerge {
                left_type,
                right_type,
            } => {
                write!(
                    f,
                    "Unable to merge {} and {}. Both must be of type str, dict, or list, or else be two equal objects",
                    left_type, right_type
                )
            }
        }
    }
}

impl std::error::Error for MergeError {}

fn values_same_type(left: &Value, right: &Value) -> bool {
    matches!(
        (left, right),
        (Value::Null, Value::Null)
            | (Value::Bool(_), Value::Bool(_))
            | (Value::Number(_), Value::Number(_))
            | (Value::String(_), Value::String(_))
            | (Value::Array(_), Value::Array(_))
            | (Value::Object(_), Value::Object(_))
    )
}

fn type_name(value: &Value) -> String {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_merge_dicts_basic() {
        let left = json!({"a": 1, "b": 2});
        let right = json!({"c": 3});
        let result = merge_dicts(left, vec![right]).unwrap();
        assert_eq!(result, json!({"a": 1, "b": 2, "c": 3}));
    }

    #[test]
    fn test_merge_dicts_null_handling() {
        let left = json!({"a": null});
        let right = json!({"a": "value"});
        let result = merge_dicts(left, vec![right]).unwrap();
        assert_eq!(result, json!({"a": "value"}));
    }

    #[test]
    fn test_merge_dicts_string_concatenation() {
        let left = json!({"text": "hello "});
        let right = json!({"text": "world"});
        let result = merge_dicts(left, vec![right]).unwrap();
        assert_eq!(result, json!({"text": "hello world"}));
    }

    #[test]
    fn test_merge_dicts_nested() {
        let left = json!({"outer": {"inner": "a"}});
        let right = json!({"outer": {"inner": "b"}});
        let result = merge_dicts(left, vec![right]).unwrap();
        assert_eq!(result, json!({"outer": {"inner": "ab"}}));
    }

    #[test]
    fn test_merge_lists_basic() {
        let left = Some(vec![json!(1), json!(2)]);
        let right = Some(vec![json!(3), json!(4)]);
        let result = merge_lists(left, vec![right]).unwrap();
        assert_eq!(result, Some(vec![json!(1), json!(2), json!(3), json!(4)]));
    }

    #[test]
    fn test_merge_lists_with_index() {
        let left = Some(vec![json!({"index": 0, "value": "a"})]);
        let right = Some(vec![json!({"index": 0, "value": "b"})]);
        let result = merge_lists(left, vec![right]).unwrap();
        assert_eq!(result, Some(vec![json!({"index": 0, "value": "ab"})]));
    }

    #[test]
    fn test_merge_obj_strings() {
        let left = json!("hello ");
        let right = json!("world");
        let result = merge_obj(left, right).unwrap();
        assert_eq!(result, json!("hello world"));
    }

    #[test]
    fn test_merge_obj_with_null() {
        let left = Value::Null;
        let right = json!("value");
        let result = merge_obj(left, right).unwrap();
        assert_eq!(result, json!("value"));
    }
}
