use serde_json::{Map, Value};
use std::collections::HashSet;

pub fn dereference_refs(
    schema_obj: &Value,
    full_schema: Option<&Value>,
    skip_keys: Option<&[&str]>,
) -> Value {
    let full = full_schema.unwrap_or(schema_obj);
    let keys_to_skip: Vec<&str> = skip_keys
        .map(|k| k.to_vec())
        .unwrap_or_else(|| vec!["$defs"]);
    let shallow = skip_keys.is_none();
    let mut processed_refs = HashSet::new();

    dereference_refs_helper(
        schema_obj,
        full,
        &mut processed_refs,
        &keys_to_skip,
        shallow,
    )
}

fn dereference_refs_helper(
    obj: &Value,
    full_schema: &Value,
    processed_refs: &mut HashSet<String>,
    skip_keys: &[&str],
    shallow_refs: bool,
) -> Value {
    match obj {
        Value::Object(map) if map.contains_key("$ref") => {
            let ref_path = map.get("$ref").and_then(|v| v.as_str()).unwrap_or("");

            let additional_properties: Map<String, Value> = map
                .iter()
                .filter(|(k, _)| *k != "$ref")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            if processed_refs.contains(ref_path) {
                return Value::Object(process_dict_properties(
                    &additional_properties,
                    full_schema,
                    processed_refs,
                    skip_keys,
                    shallow_refs,
                ));
            }

            processed_refs.insert(ref_path.to_string());

            let referenced_object = retrieve_ref(ref_path, full_schema);
            let resolved_reference = dereference_refs_helper(
                &referenced_object,
                full_schema,
                processed_refs,
                skip_keys,
                shallow_refs,
            );

            processed_refs.remove(ref_path);

            if additional_properties.is_empty() {
                return resolved_reference;
            }

            let mut merged_result = Map::new();
            if let Value::Object(resolved_map) = &resolved_reference {
                for (k, v) in resolved_map {
                    merged_result.insert(k.clone(), v.clone());
                }
            }

            let processed_additional = process_dict_properties(
                &additional_properties,
                full_schema,
                processed_refs,
                skip_keys,
                shallow_refs,
            );
            for (k, v) in processed_additional {
                merged_result.insert(k, v);
            }

            Value::Object(merged_result)
        }
        Value::Object(map) => Value::Object(process_dict_properties(
            map,
            full_schema,
            processed_refs,
            skip_keys,
            shallow_refs,
        )),
        Value::Array(arr) => {
            let processed: Vec<Value> = arr
                .iter()
                .map(|item| {
                    dereference_refs_helper(
                        item,
                        full_schema,
                        processed_refs,
                        skip_keys,
                        shallow_refs,
                    )
                })
                .collect();
            Value::Array(processed)
        }
        _ => obj.clone(),
    }
}

fn process_dict_properties(
    properties: &Map<String, Value>,
    full_schema: &Value,
    processed_refs: &mut HashSet<String>,
    skip_keys: &[&str],
    shallow_refs: bool,
) -> Map<String, Value> {
    let mut result = Map::new();

    for (key, value) in properties {
        if skip_keys.contains(&key.as_str()) {
            result.insert(key.clone(), value.clone());
        } else {
            match value {
                Value::Object(_) | Value::Array(_) => {
                    result.insert(
                        key.clone(),
                        dereference_refs_helper(
                            value,
                            full_schema,
                            processed_refs,
                            skip_keys,
                            shallow_refs,
                        ),
                    );
                }
                _ => {
                    result.insert(key.clone(), value.clone());
                }
            }
        }
    }

    result
}

fn retrieve_ref(path: &str, schema: &Value) -> Value {
    let components: Vec<&str> = path.split('/').collect();

    if components.first() != Some(&"#") {
        return Value::Null;
    }

    let mut current = schema;

    for component in components.iter().skip(1) {
        current = match current {
            Value::Object(map) => map.get(*component).unwrap_or(&Value::Null),
            Value::Array(arr) => {
                if let Ok(index) = component.parse::<usize>() {
                    arr.get(index).unwrap_or(&Value::Null)
                } else {
                    &Value::Null
                }
            }
            _ => &Value::Null,
        };
    }

    current.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dereference_refs_basic() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"$ref": "#/$defs/string_type"}
            },
            "$defs": {
                "string_type": {"type": "string"}
            }
        });

        let result = dereference_refs(&schema, None, None);

        assert_eq!(result["properties"]["name"]["type"], json!("string"));
    }

    #[test]
    fn test_dereference_refs_mixed() {
        let schema = json!({
            "properties": {
                "name": {
                    "$ref": "#/$defs/base",
                    "description": "User name"
                }
            },
            "$defs": {
                "base": {"type": "string", "minLength": 1}
            }
        });

        let result = dereference_refs(&schema, None, Some(&[]));

        assert_eq!(result["properties"]["name"]["type"], json!("string"));
        assert_eq!(
            result["properties"]["name"]["description"],
            json!("User name")
        );
    }

    #[test]
    fn test_dereference_refs_circular() {
        let schema = json!({
            "properties": {
                "user": {"$ref": "#/$defs/User"}
            },
            "$defs": {
                "User": {
                    "type": "object",
                    "properties": {
                        "friend": {"$ref": "#/$defs/User"}
                    }
                }
            }
        });

        let _ = dereference_refs(&schema, None, Some(&[]));
    }

    #[test]
    fn test_retrieve_ref() {
        let schema = json!({
            "$defs": {
                "Person": {
                    "type": "object"
                }
            }
        });

        let result = retrieve_ref("#/$defs/Person", &schema);
        assert_eq!(result["type"], json!("object"));
    }
}
