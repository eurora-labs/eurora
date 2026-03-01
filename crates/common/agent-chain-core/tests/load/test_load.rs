use std::collections::HashMap;

use agent_chain_core::load::{
    ConstructorInfo, RevivedValue, Reviver, ReviverConfig, dumpd, dumps, load, loads,
    loads_with_namespaces, loads_with_secrets,
};
use agent_chain_core::load::{
    DEFAULT_NAMESPACES, DISALLOW_LOAD_FROM_PATH, Serializable, get_all_serializable_mappings,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestSerializableModel {
    value: i32,
    name: String,
}

impl Serializable for TestSerializableModel {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec!["tests".to_string(), "load".to_string()]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SecretModel {
    api_key: String,
    name: String,
}

impl Serializable for SecretModel {
    fn is_lc_serializable() -> bool {
        true
    }

    fn get_lc_namespace() -> Vec<String> {
        vec!["tests".to_string(), "load".to_string()]
    }

    fn lc_secrets(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("api_key".to_string(), "MY_API_KEY".to_string());
        map
    }
}

#[test]
fn test_reviver_init_default() {
    let reviver = Reviver::with_defaults();
    let config = &reviver;
    let result = reviver.revive(&json!({"key": "value"})).unwrap();
    match result {
        RevivedValue::Value(v) => {
            assert_eq!(v.get("key").and_then(|v| v.as_str()), Some("value"));
        }
        _ => panic!("Expected Value variant"),
    }
    let _ = config;
}

#[test]
fn test_reviver_init_default_properties() {
    let config = ReviverConfig::default();
    assert!(config.secrets_map.is_empty());
    assert!(config.secrets_from_env);
    assert!(config.valid_namespaces.iter().any(|ns| ns == "langchain"));
    assert!(
        config
            .valid_namespaces
            .iter()
            .any(|ns| ns == "langchain_core")
    );
    assert!(!config.ignore_unserializable_fields);
}

#[test]
fn test_reviver_init_custom_namespaces() {
    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["tests".to_string(), "custom".to_string()])
        .build();
    let reviver = Reviver::new(config);

    let test_langchain = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_core", "messages", "ai", "AIMessage"],
        "kwargs": {"content": "hello"}
    });
    let result = reviver.revive(&test_langchain).unwrap();
    assert!(matches!(
        result,
        RevivedValue::Value(_) | RevivedValue::Constructor(_)
    ));

    let test_custom = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["tests", "module", "SomeClass"],
        "kwargs": {}
    });
    let result = reviver.revive(&test_custom).unwrap();
    assert!(matches!(result, RevivedValue::Constructor(_)));
}

#[test]
fn test_reviver_init_with_secrets_map() {
    let mut secrets = HashMap::new();
    secrets.insert("API_KEY".to_string(), "secret_value".to_string());
    let config = ReviverConfig::builder().secrets_map(secrets).build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "secret",
        "id": ["API_KEY"]
    });
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::String(s) => assert_eq!(s, "secret_value"),
        _ => panic!("Expected String variant"),
    }
}

#[test]
fn test_reviver_init_with_additional_import_mappings() {
    let mut mappings = HashMap::new();
    mappings.insert(
        vec![
            "custom".to_string(),
            "module".to_string(),
            "Class".to_string(),
        ],
        vec![
            "actual".to_string(),
            "module".to_string(),
            "Class".to_string(),
        ],
    );
    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["custom".to_string()])
        .additional_import_mappings(mappings.clone())
        .build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["custom", "module", "Class"],
        "kwargs": {"value": 42}
    });
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Constructor(info) => {
            assert_eq!(
                info.path,
                vec![
                    "actual".to_string(),
                    "module".to_string(),
                    "Class".to_string()
                ]
            );
        }
        _ => panic!("Expected Constructor variant"),
    }
}

#[test]
fn test_reviver_secret_from_map() {
    let mut secrets = HashMap::new();
    secrets.insert("API_KEY".to_string(), "secret_value".to_string());
    let config = ReviverConfig::builder().secrets_map(secrets).build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "secret",
        "id": ["API_KEY"]
    });
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::String(s) => assert_eq!(s, "secret_value"),
        _ => panic!("Expected String"),
    }
}

#[test]
fn test_reviver_secret_from_env() {
    let key = "TEST_REVIVER_SECRET_FROM_ENV_KEY";
    unsafe { std::env::set_var(key, "env_secret_value") };

    let config = ReviverConfig::builder().secrets_from_env(true).build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "secret",
        "id": [key]
    });
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::String(s) => assert_eq!(s, "env_secret_value"),
        _ => panic!("Expected String"),
    }

    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_reviver_secret_not_in_env_returns_none() {
    let key = "REVIVER_TEST_DEFINITELY_MISSING_KEY";
    unsafe { std::env::remove_var(key) };

    let config = ReviverConfig::builder().secrets_from_env(true).build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "secret",
        "id": [key]
    });
    let result = reviver.revive(&value).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_reviver_secret_from_env_disabled() {
    let key = "TEST_REVIVER_SECRET_ENV_DISABLED_KEY";
    unsafe { std::env::set_var(key, "env_value") };

    let config = ReviverConfig::builder().secrets_from_env(false).build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "secret",
        "id": [key]
    });
    let result = reviver.revive(&value).unwrap();
    assert!(result.is_none());

    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_reviver_not_implemented_raises_error() {
    let reviver = Reviver::with_defaults();

    let value = json!({
        "lc": 1,
        "type": "not_implemented",
        "id": ["some", "module", "Class"]
    });
    let result = reviver.revive(&value);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("doesn't implement serialization") || err_msg.contains("not_implemented")
    );
}

#[test]
fn test_reviver_not_implemented_with_ignore_flag() {
    let config = ReviverConfig::builder()
        .ignore_unserializable_fields(true)
        .build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "not_implemented",
        "id": ["some", "module", "Class"]
    });
    let result = reviver.revive(&value).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_reviver_constructor_deserialization() {
    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["langchain_core".to_string()])
        .build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_core", "messages", "ai", "AIMessage"],
        "kwargs": {"content": "hello"}
    });

    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => {
            assert_eq!(v.get("content").and_then(|v| v.as_str()), Some("hello"));
            assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("ai"));
        }
        RevivedValue::Constructor(info) => {
            assert_eq!(info.name, "AIMessage");
            assert!(info.path.contains(&"langchain_core".to_string()));
            assert_eq!(
                info.kwargs.get("content").and_then(|v| v.as_str()),
                Some("hello")
            );
        }
        _ => panic!("Expected Value or Constructor"),
    }
}

#[test]
fn test_reviver_invalid_namespace_raises_error() {
    let reviver = Reviver::with_defaults();

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["invalid_namespace", "module", "Class"],
        "kwargs": {}
    });

    let result = reviver.revive(&value);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid namespace"));
}

#[test]
fn test_reviver_root_langchain_namespace_raises_error() {
    let reviver = Reviver::with_defaults();

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain", "SomeClass"],
        "kwargs": {}
    });

    let result = reviver.revive(&value);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Invalid namespace"));
}

#[test]
fn test_reviver_with_import_mapping() {
    let mut mappings = HashMap::new();
    mappings.insert(
        vec![
            "old".to_string(),
            "namespace".to_string(),
            "Class".to_string(),
        ],
        vec![
            "langchain_core".to_string(),
            "messages".to_string(),
            "ai".to_string(),
            "AIMessage".to_string(),
        ],
    );
    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["old".to_string()])
        .additional_import_mappings(mappings)
        .build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["old", "namespace", "Class"],
        "kwargs": {"content": "hello"}
    });

    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => {
            assert_eq!(v.get("content").and_then(|v| v.as_str()), Some("hello"));
            assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("ai"));
        }
        RevivedValue::Constructor(info) => {
            assert!(info.path.contains(&"langchain_core".to_string()));
            assert_eq!(info.path.last().unwrap(), "AIMessage");
        }
        _ => panic!("Expected Value or Constructor"),
    }
}

#[test]
fn test_reviver_disallow_load_from_path() {
    let reviver = Reviver::with_defaults();

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_community", "some", "module", "Class"],
        "kwargs": {}
    });

    let result = reviver.revive(&value);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("cannot be deserialized"));
}

#[test]
fn test_reviver_passthrough_non_lc_dict() {
    let reviver = Reviver::with_defaults();

    let value = json!({"key": "value", "number": 42});
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => {
            assert_eq!(v, json!({"key": "value", "number": 42}));
        }
        _ => panic!("Expected Value passthrough"),
    }
}

#[test]
fn test_loads_basic_constructor() {
    let json_str = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_core", "messages", "ai", "AIMessage"],
        "kwargs": {"content": "hello"}
    })
    .to_string();

    let result = loads(&json_str, None).unwrap();
    assert!(result.is_object());
}

#[test]
fn test_loads_with_secrets_map() {
    let json_str = json!({
        "lc": 1,
        "type": "secret",
        "id": ["API_KEY"]
    })
    .to_string();

    let mut secrets = HashMap::new();
    secrets.insert("API_KEY".to_string(), "secret_value".to_string());
    let config = ReviverConfig::builder().secrets_map(secrets).build();

    let result = loads(&json_str, Some(config)).unwrap();
    assert_eq!(result.as_str(), Some("secret_value"));
}

#[test]
fn test_loads_with_secrets_from_env() {
    let key = "TEST_LOADS_SECRET_FROM_ENV_KEY";
    unsafe { std::env::set_var(key, "env_value") };

    let json_str = json!({
        "lc": 1,
        "type": "secret",
        "id": [key]
    })
    .to_string();

    let config = ReviverConfig::builder().secrets_from_env(true).build();
    let result = loads(&json_str, Some(config)).unwrap();
    assert_eq!(result.as_str(), Some("env_value"));

    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_loads_with_additional_import_mappings() {
    let mut mappings = HashMap::new();
    mappings.insert(
        vec![
            "old".to_string(),
            "namespace".to_string(),
            "Class".to_string(),
        ],
        vec![
            "langchain_core".to_string(),
            "messages".to_string(),
            "ai".to_string(),
            "AIMessage".to_string(),
        ],
    );

    let json_str = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["old", "namespace", "Class"],
        "kwargs": {"content": "hello"}
    })
    .to_string();

    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["old".to_string()])
        .additional_import_mappings(mappings)
        .build();

    let result = loads(&json_str, Some(config)).unwrap();
    assert!(result.is_object());
}

#[test]
fn test_loads_with_ignore_unserializable_fields() {
    let json_str = json!({
        "lc": 1,
        "type": "not_implemented",
        "id": ["some", "module", "Class"]
    })
    .to_string();

    let config = ReviverConfig::builder()
        .ignore_unserializable_fields(true)
        .build();
    let result = loads(&json_str, Some(config)).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_loads_nested_structure() {
    let json_str = json!({
        "data": {
            "lc": 1,
            "type": "constructor",
            "id": ["langchain_core", "messages", "ai", "AIMessage"],
            "kwargs": {"content": "hello"}
        },
        "list": [1, 2, 3]
    })
    .to_string();

    let result = loads(&json_str, None).unwrap();
    assert!(result.get("data").unwrap().is_object());
    assert_eq!(result.get("list").unwrap(), &json!([1, 2, 3]));
}

#[test]
fn test_load_basic() {
    let obj = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_core", "messages", "ai", "AIMessage"],
        "kwargs": {"content": "hello"}
    });

    let result = load(obj, None).unwrap();
    assert!(result.is_object());
}

#[test]
fn test_load_with_secrets_map() {
    let obj = json!({
        "lc": 1,
        "type": "secret",
        "id": ["API_KEY"]
    });

    let mut secrets = HashMap::new();
    secrets.insert("API_KEY".to_string(), "secret_value".to_string());
    let config = ReviverConfig::builder().secrets_map(secrets).build();

    let result = load(obj, Some(config)).unwrap();
    assert_eq!(result.as_str(), Some("secret_value"));
}

#[test]
fn test_load_with_secrets_from_env() {
    let key = "TEST_LOAD_SECRET_FROM_ENV_KEY";
    unsafe { std::env::set_var(key, "env_value") };

    let obj = json!({
        "lc": 1,
        "type": "secret",
        "id": [key]
    });

    let config = ReviverConfig::builder().secrets_from_env(true).build();
    let result = load(obj, Some(config)).unwrap();
    assert_eq!(result.as_str(), Some("env_value"));

    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_load_with_additional_import_mappings() {
    let mut mappings = HashMap::new();
    mappings.insert(
        vec![
            "old".to_string(),
            "namespace".to_string(),
            "Class".to_string(),
        ],
        vec![
            "langchain_core".to_string(),
            "messages".to_string(),
            "ai".to_string(),
            "AIMessage".to_string(),
        ],
    );

    let obj = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["old", "namespace", "Class"],
        "kwargs": {"content": "hello"}
    });

    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["old".to_string()])
        .additional_import_mappings(mappings)
        .build();

    let result = load(obj, Some(config)).unwrap();
    assert!(result.is_object());
}

#[test]
fn test_load_with_ignore_unserializable_fields() {
    let obj = json!({
        "lc": 1,
        "type": "not_implemented",
        "id": ["some", "module", "Class"]
    });

    let config = ReviverConfig::builder()
        .ignore_unserializable_fields(true)
        .build();
    let result = load(obj, Some(config)).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_load_nested_dict_structure() {
    let obj = json!({
        "outer": {
            "inner": {
                "lc": 1,
                "type": "constructor",
                "id": ["langchain_core", "messages", "ai", "AIMessage"],
                "kwargs": {"content": "hello"}
            }
        }
    });

    let result = load(obj, None).unwrap();
    let inner = result.get("outer").unwrap().get("inner").unwrap();
    assert!(inner.is_object());
}

#[test]
fn test_load_nested_list_structure() {
    let obj = json!([
        {
            "lc": 1,
            "type": "constructor",
            "id": ["langchain_core", "messages", "ai", "AIMessage"],
            "kwargs": {"content": "first"}
        },
        {
            "lc": 1,
            "type": "constructor",
            "id": ["langchain_core", "messages", "ai", "AIMessage"],
            "kwargs": {"content": "second"}
        }
    ]);

    let result = load(obj, None).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert!(arr[0].is_object());
    assert!(arr[1].is_object());
}

#[test]
fn test_load_primitive_types() {
    assert_eq!(load(json!("test"), None).unwrap(), json!("test"));

    assert_eq!(load(json!(42), None).unwrap(), json!(42));

    assert_eq!(load(json!(true), None).unwrap(), json!(true));

    assert_eq!(load(json!(null), None).unwrap(), json!(null));
}

#[test]
fn test_load_complex_nested_structure() {
    let obj = json!({
        "serializable": {
            "lc": 1,
            "type": "constructor",
            "id": ["langchain_core", "messages", "ai", "AIMessage"],
            "kwargs": {"content": "hello"}
        },
        "list_of_serializables": [
            {
                "lc": 1,
                "type": "constructor",
                "id": ["langchain_core", "messages", "ai", "AIMessage"],
                "kwargs": {"content": "first"}
            }
        ],
        "primitive": "string",
        "nested": {"data": [1, 2, 3]}
    });

    let result = load(obj, None).unwrap();
    assert!(result.get("serializable").unwrap().is_object());
    assert!(
        result
            .get("list_of_serializables")
            .unwrap()
            .as_array()
            .unwrap()[0]
            .is_object()
    );
    assert_eq!(result.get("primitive").unwrap().as_str(), Some("string"));
    assert_eq!(
        result.get("nested").unwrap().get("data").unwrap(),
        &json!([1, 2, 3])
    );
}

#[test]
fn test_load_with_empty_env_string() {
    let key = "TEST_LOAD_EMPTY_ENV_KEY";
    unsafe { std::env::set_var(key, "") };

    let obj = json!({
        "lc": 1,
        "type": "secret",
        "id": [key]
    });

    let config = ReviverConfig::builder().secrets_from_env(true).build();
    let result = load(obj, Some(config)).unwrap();
    assert!(result.is_null());

    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_round_trip_basic() {
    let original = TestSerializableModel {
        value: 42,
        name: "test".to_string(),
    };

    let serialized = dumpd(&original).unwrap();
    assert_eq!(
        serialized.get("type").and_then(|v| v.as_str()),
        Some("constructor")
    );
    assert_eq!(serialized.get("lc").and_then(|v| v.as_i64()), Some(1));

    let kwargs = serialized.get("kwargs").unwrap();
    assert_eq!(kwargs.get("value").and_then(|v| v.as_i64()), Some(42));
    assert_eq!(kwargs.get("name").and_then(|v| v.as_str()), Some("test"));
}

#[test]
fn test_round_trip_with_loads_dumps() {
    let original = TestSerializableModel {
        value: 42,
        name: "test".to_string(),
    };

    let json_str = dumps(&original, false).unwrap();
    assert!(json_str.contains("constructor"));

    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["tests".to_string()])
        .build();
    let loaded = loads(&json_str, Some(config)).unwrap();
    assert!(loaded.is_object());
}

#[test]
fn test_default_namespaces_exact_snapshot() {
    let expected = vec![
        "langchain",
        "langchain_core",
        "langchain_community",
        "langchain_anthropic",
        "langchain_groq",
        "langchain_google_genai",
        "langchain_aws",
        "langchain_openai",
        "langchain_google_vertexai",
        "langchain_mistralai",
        "langchain_fireworks",
        "langchain_xai",
        "langchain_sambanova",
        "langchain_perplexity",
    ];
    assert_eq!(DEFAULT_NAMESPACES, expected.as_slice());
}

#[test]
fn test_disallow_load_from_path_exact_snapshot() {
    assert_eq!(
        DISALLOW_LOAD_FROM_PATH,
        &["langchain_community", "langchain"]
    );
}

#[test]
fn test_reviver_non_lc_versioned_dict_passthrough() {
    let reviver = Reviver::with_defaults();

    let value = json!({"lc": 2, "type": "constructor", "id": ["a"], "kwargs": {}});
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => assert_eq!(v, value),
        _ => panic!("Expected Value passthrough for lc != 1"),
    }
}

#[test]
fn test_reviver_dict_without_lc_passthrough() {
    let reviver = Reviver::with_defaults();

    let value = json!({"type": "constructor", "id": ["a"], "kwargs": {}});
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => assert_eq!(v, value),
        _ => panic!("Expected Value passthrough for dict without lc"),
    }
}

#[test]
fn test_reviver_secret_without_id_passthrough() {
    let reviver = Reviver::with_defaults();

    let value = json!({"lc": 1, "type": "secret"});
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => assert_eq!(v, value),
        _ => panic!("Expected Value passthrough for secret without id"),
    }
}

#[test]
fn test_reviver_constructor_without_id_passthrough() {
    let reviver = Reviver::with_defaults();

    let value = json!({"lc": 1, "type": "constructor", "kwargs": {}});
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => assert_eq!(v, value),
        _ => panic!("Expected Value passthrough for constructor without id"),
    }
}

#[test]
fn test_reviver_constructor_with_empty_kwargs() {
    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["langchain_core".to_string()])
        .build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_core", "messages", "ai", "AIMessage"],
        "kwargs": {}
    });

    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Constructor(info) => {
            assert_eq!(info.name, "AIMessage");
            assert_eq!(info.kwargs, json!({}));
        }
        _ => panic!("Expected Constructor with empty kwargs"),
    }
}

#[test]
fn test_reviver_constructor_missing_kwargs_key() {
    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["langchain_core".to_string()])
        .build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_core", "messages", "ai", "AIMessage"]
    });

    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Constructor(info) => {
            assert_eq!(info.name, "AIMessage");
            assert_eq!(info.kwargs, json!({}));
        }
        _ => panic!("Expected Constructor with defaulted empty kwargs"),
    }
}

#[test]
fn test_reviver_secret_map_takes_priority_over_env() {
    let key = "TEST_REVIVER_PRIORITY_KEY";
    unsafe { std::env::set_var(key, "from_env") };

    let mut secrets = HashMap::new();
    secrets.insert(key.to_string(), "from_map".to_string());
    let config = ReviverConfig::builder()
        .secrets_map(secrets)
        .secrets_from_env(true)
        .build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "secret",
        "id": [key]
    });
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::String(s) => assert_eq!(s, "from_map"),
        _ => panic!("Expected secret from map, not env"),
    }

    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_reviver_valid_namespaces_merged_with_defaults() {
    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["my_custom_ns".to_string()])
        .build();

    for ns in DEFAULT_NAMESPACES {
        assert!(
            config.valid_namespaces.iter().any(|n| n == ns),
            "Default namespace '{}' should be present",
            ns
        );
    }
    assert!(config.valid_namespaces.iter().any(|n| n == "my_custom_ns"));
}

#[test]
fn test_reviver_no_custom_namespaces_uses_defaults() {
    let config = ReviverConfig::default();
    let expected: Vec<String> = DEFAULT_NAMESPACES.iter().map(|s| s.to_string()).collect();
    assert_eq!(config.valid_namespaces, expected);
}

#[test]
fn test_reviver_additional_import_mappings_override() {
    let custom_key = vec![
        "langchain".to_string(),
        "schema".to_string(),
        "messages".to_string(),
        "AIMessage".to_string(),
    ];
    let custom_value = vec![
        "tests".to_string(),
        "load".to_string(),
        "TestSerializableModel".to_string(),
    ];

    let mut custom_mapping = HashMap::new();
    custom_mapping.insert(custom_key.clone(), custom_value.clone());

    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["tests".to_string()])
        .additional_import_mappings(custom_mapping)
        .build();
    let reviver = Reviver::new(config);

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain", "schema", "messages", "AIMessage"],
        "kwargs": {"content": "hello"}
    });

    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => {
            assert_eq!(v.get("content").and_then(|v| v.as_str()), Some("hello"));
        }
        RevivedValue::Constructor(info) => {
            assert_eq!(info.path, custom_value);
        }
        _ => panic!("Expected Value or Constructor"),
    }
}

#[test]
fn test_reviver_import_mappings_without_additional() {
    let all_mappings = get_all_serializable_mappings();
    let reviver = Reviver::with_defaults();

    let key = vec![
        "langchain".to_string(),
        "schema".to_string(),
        "messages".to_string(),
        "AIMessage".to_string(),
    ];
    let expected = all_mappings.get(&key).unwrap();

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": key,
        "kwargs": {"content": "hello"}
    });
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => {
            assert_eq!(v.get("content").and_then(|v| v.as_str()), Some("hello"));
        }
        RevivedValue::Constructor(info) => {
            assert_eq!(&info.path, expected);
        }
        _ => panic!("Expected Value or Constructor"),
    }
}

#[test]
fn test_reviver_not_implemented_with_repr() {
    let reviver = Reviver::with_defaults();

    let value = json!({
        "lc": 1,
        "type": "not_implemented",
        "id": ["some", "Class"],
        "repr": "Class(x=1)"
    });

    let result = reviver.revive(&value);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("doesn't implement serialization") || err_msg.contains("not_implemented")
    );
}

#[test]
fn test_reviver_unknown_type_passthrough() {
    let reviver = Reviver::with_defaults();

    let value = json!({"lc": 1, "type": "unknown_type", "id": ["a"]});
    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => assert_eq!(v, value),
        _ => panic!("Expected Value passthrough for unknown type"),
    }
}

#[test]
fn test_reviver_langchain_core_direct_namespace() {
    let reviver = Reviver::with_defaults();

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_core", "messages", "ai", "AIMessage"],
        "kwargs": {"content": "hello"}
    });

    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => {
            assert_eq!(v.get("content").and_then(|v| v.as_str()), Some("hello"));
            assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("ai"));
        }
        RevivedValue::Constructor(info) => {
            assert_eq!(info.name, "AIMessage");
            assert!(info.path.contains(&"langchain_core".to_string()));
        }
        _ => panic!("Expected Value or Constructor for langchain_core namespace"),
    }
}

#[test]
fn test_loads_invalid_json_raises_error() {
    let result = loads("not valid json{{{", None);
    assert!(result.is_err());
}

#[test]
fn test_loads_plain_json_string() {
    let result = loads("\"hello\"", None).unwrap();
    assert_eq!(result, json!("hello"));
}

#[test]
fn test_loads_plain_json_number() {
    let result = loads("42", None).unwrap();
    assert_eq!(result, json!(42));
}

#[test]
fn test_loads_plain_json_null() {
    let result = loads("null", None).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_loads_plain_json_array() {
    let result = loads("[1, 2, 3]", None).unwrap();
    assert_eq!(result, json!([1, 2, 3]));
}

#[test]
fn test_loads_secret_not_in_map_or_env() {
    let key = "LOADS_TEST_NONEXISTENT_KEY";
    unsafe { std::env::remove_var(key) };

    let json_str = json!({
        "lc": 1,
        "type": "secret",
        "id": [key]
    })
    .to_string();

    let config = ReviverConfig::builder()
        .secrets_map(HashMap::new())
        .secrets_from_env(true)
        .build();
    let result = loads(&json_str, Some(config)).unwrap();
    assert!(result.is_null());
}

#[test]
fn test_loads_with_secrets_from_env_false() {
    let key = "TEST_LOADS_SECRET_ENV_FALSE_KEY";
    unsafe { std::env::set_var(key, "from_env") };

    let json_str = json!({
        "lc": 1,
        "type": "secret",
        "id": [key]
    })
    .to_string();

    let config = ReviverConfig::builder().secrets_from_env(false).build();
    let result = loads(&json_str, Some(config)).unwrap();
    assert!(result.is_null());

    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_load_deeply_nested_mixed() {
    let obj = json!({
        "level1": {
            "level2": [
                {
                    "level3": {
                        "lc": 1,
                        "type": "constructor",
                        "id": ["langchain_core", "messages", "ai", "AIMessage"],
                        "kwargs": {"content": "deep"}
                    }
                }
            ]
        }
    });

    let result = load(obj, None).unwrap();
    let inner = result
        .get("level1")
        .unwrap()
        .get("level2")
        .unwrap()
        .as_array()
        .unwrap()[0]
        .get("level3")
        .unwrap();
    assert!(inner.is_object());
}

#[test]
fn test_load_empty_dict() {
    let result = load(json!({}), None).unwrap();
    assert_eq!(result, json!({}));
}

#[test]
fn test_load_empty_list() {
    let result = load(json!([]), None).unwrap();
    assert_eq!(result, json!([]));
}

#[test]
fn test_load_float() {
    let result = load(json!(3.15), None).unwrap();
    assert_eq!(result.as_f64(), Some(3.15));
}

#[test]
fn test_load_nested_secrets() {
    let key = "TEST_LOAD_NESTED_SECRET_KEY";
    unsafe { std::env::set_var(key, "nested_secret_value") };

    let obj = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_core", "messages", "ai", "AIMessage"],
        "kwargs": {
            "content": {
                "lc": 1,
                "type": "secret",
                "id": [key]
            }
        }
    });

    let config = ReviverConfig::builder().secrets_from_env(true).build();
    let result = load(obj, Some(config)).unwrap();
    assert_eq!(
        result.get("content").and_then(|v| v.as_str()),
        Some("nested_secret_value")
    );

    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_load_list_with_mixed_types() {
    let obj = json!([
        "string",
        42,
        true,
        null,
        {
            "lc": 1,
            "type": "constructor",
            "id": ["langchain_core", "messages", "ai", "AIMessage"],
            "kwargs": {"content": "in_list"}
        }
    ]);

    let result = load(obj, None).unwrap();
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0], json!("string"));
    assert_eq!(arr[1], json!(42));
    assert_eq!(arr[2], json!(true));
    assert!(arr[3].is_null());
    assert!(arr[4].is_object());
}

#[test]
fn test_round_trip_preserves_all_fields() {
    let original = TestSerializableModel {
        value: 99,
        name: "round_trip_test".to_string(),
    };

    let serialized = dumpd(&original).unwrap();

    assert_eq!(serialized.get("lc").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(
        serialized.get("type").and_then(|v| v.as_str()),
        Some("constructor")
    );

    let kwargs = serialized.get("kwargs").unwrap();
    assert_eq!(kwargs.get("value").and_then(|v| v.as_i64()), Some(99));
    assert_eq!(
        kwargs.get("name").and_then(|v| v.as_str()),
        Some("round_trip_test")
    );

    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["tests".to_string()])
        .build();
    let loaded = load(serialized, Some(config)).unwrap();
    assert!(loaded.is_object());
}

#[test]
fn test_round_trip_with_secrets() {
    let original = SecretModel {
        api_key: "secret123".to_string(),
        name: "test".to_string(),
    };

    let serialized = dumpd(&original).unwrap();

    let kwargs = serialized.get("kwargs").unwrap();
    let api_key_value = kwargs.get("api_key").unwrap();
    assert_eq!(
        api_key_value.get("type").and_then(|v| v.as_str()),
        Some("secret")
    );
    assert_eq!(
        api_key_value
            .get("id")
            .and_then(|v| v.as_array())
            .map(|arr| { arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>() }),
        Some(vec!["MY_API_KEY"])
    );

    let mut secrets = HashMap::new();
    secrets.insert("MY_API_KEY".to_string(), "secret123".to_string());
    let config = ReviverConfig::builder()
        .valid_namespaces(vec!["tests".to_string()])
        .secrets_map(secrets)
        .build();
    let loaded = load(serialized, Some(config)).unwrap();
    assert!(loaded.is_object());

    let loaded_kwargs = loaded.get("kwargs").unwrap();
    assert_eq!(
        loaded_kwargs.get("api_key").and_then(|v| v.as_str()),
        Some("secret123")
    );
    assert_eq!(
        loaded_kwargs.get("name").and_then(|v| v.as_str()),
        Some("test")
    );
}

#[test]
fn test_loads_with_secrets_convenience() {
    let json_str = json!({
        "lc": 1,
        "type": "secret",
        "id": ["MY_KEY"]
    })
    .to_string();

    let mut secrets = HashMap::new();
    secrets.insert("MY_KEY".to_string(), "my_value".to_string());

    let result = loads_with_secrets(&json_str, secrets).unwrap();
    assert_eq!(result.as_str(), Some("my_value"));
}

#[test]
fn test_loads_with_namespaces_convenience() {
    let json_str = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["custom_ns", "module", "SomeClass"],
        "kwargs": {"value": 42}
    })
    .to_string();

    let result = loads_with_namespaces(&json_str, vec!["custom_ns".to_string()]).unwrap();
    assert!(result.is_object());
}

#[test]
fn test_revived_value_to_value() {
    let rv = RevivedValue::Value(json!({"key": "value"}));
    assert_eq!(rv.to_value(), json!({"key": "value"}));

    let rv = RevivedValue::String("hello".to_string());
    assert_eq!(rv.to_value(), json!("hello"));

    let rv = RevivedValue::None;
    assert!(rv.to_value().is_null());

    let rv = RevivedValue::Constructor(ConstructorInfo {
        path: vec!["langchain_core".to_string(), "AIMessage".to_string()],
        name: "AIMessage".to_string(),
        kwargs: json!({"content": "hello"}),
    });
    let v = rv.to_value();
    assert!(v.is_object());
    assert_eq!(v.get("_type").and_then(|v| v.as_str()), Some("constructor"));
    assert_eq!(v.get("name").and_then(|v| v.as_str()), Some("AIMessage"));
}

#[test]
fn test_revived_value_is_none() {
    assert!(RevivedValue::None.is_none());
    assert!(!RevivedValue::Value(json!(null)).is_none());
    assert!(!RevivedValue::String("".to_string()).is_none());
}

#[test]
fn test_reviver_config_builder_chain() {
    let mut secrets = HashMap::new();
    secrets.insert("key".to_string(), "value".to_string());

    let mut mappings = HashMap::new();
    mappings.insert(vec!["old".to_string()], vec!["new".to_string()]);

    let config = ReviverConfig::builder()
        .secrets_map(secrets.clone())
        .valid_namespaces(vec!["custom".to_string()])
        .secrets_from_env(false)
        .additional_import_mappings(mappings.clone())
        .ignore_unserializable_fields(true)
        .build();

    assert_eq!(config.secrets_map, secrets);
    assert!(config.valid_namespaces.contains(&"custom".to_string()));
    assert!(!config.secrets_from_env);
    assert_eq!(config.additional_import_mappings, mappings);
    assert!(config.ignore_unserializable_fields);
}

#[test]
fn test_reviver_constructor_empty_id_raises_error() {
    let reviver = Reviver::with_defaults();

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": [],
        "kwargs": {}
    });

    let result = reviver.revive(&value);
    assert!(result.is_err());
}

#[test]
fn test_reviver_constructor_with_mapping_old_schema() {
    let reviver = Reviver::with_defaults();

    let value = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain", "schema", "messages", "AIMessage"],
        "kwargs": {"content": "Hello!"}
    });

    let result = reviver.revive(&value).unwrap();
    match result {
        RevivedValue::Value(v) => {
            assert_eq!(v.get("content").and_then(|v| v.as_str()), Some("Hello!"));
            assert_eq!(v.get("type").and_then(|v| v.as_str()), Some("ai"));
        }
        RevivedValue::Constructor(info) => {
            assert_eq!(info.name, "AIMessage");
            assert!(info.path.contains(&"langchain_core".to_string()));
        }
        _ => panic!("Expected Value or Constructor"),
    }
}

#[test]
fn test_load_recursive_processes_kwargs_secrets() {
    let key = "TEST_RECURSIVE_KWARGS_KEY";
    unsafe { std::env::set_var(key, "resolved_value") };

    let json_str = json!({
        "wrapper": {
            "lc": 1,
            "type": "constructor",
            "id": ["langchain_core", "messages", "ai", "AIMessage"],
            "kwargs": {
                "content": "hello",
                "metadata": {
                    "secret": {
                        "lc": 1,
                        "type": "secret",
                        "id": [key]
                    }
                }
            }
        }
    })
    .to_string();

    let config = ReviverConfig::builder().secrets_from_env(true).build();
    let result = loads(&json_str, Some(config)).unwrap();

    let wrapper = result.get("wrapper").unwrap();
    let _metadata = wrapper.get("response_metadata").unwrap();
    assert_eq!(
        wrapper.get("content").and_then(|v| v.as_str()),
        Some("hello")
    );

    unsafe { std::env::remove_var(key) };
}

#[test]
fn test_all_serializable_mappings_contains_all_sources() {
    let combined = get_all_serializable_mappings();

    let key = vec![
        "langchain".to_string(),
        "schema".to_string(),
        "messages".to_string(),
        "AIMessage".to_string(),
    ];
    assert!(combined.contains_key(&key));
}

#[test]
fn test_default_namespaces_contains_core() {
    assert!(DEFAULT_NAMESPACES.contains(&"langchain_core"));
    assert!(DEFAULT_NAMESPACES.contains(&"langchain_openai"));
    assert!(DEFAULT_NAMESPACES.contains(&"langchain"));
}

#[test]
fn test_disallow_load_from_path_contents() {
    assert!(DISALLOW_LOAD_FROM_PATH.contains(&"langchain_community"));
    assert!(DISALLOW_LOAD_FROM_PATH.contains(&"langchain"));
    assert!(!DISALLOW_LOAD_FROM_PATH.contains(&"langchain_core"));
}

#[test]
fn test_round_trip_document() {
    use agent_chain_core::documents::Document;

    let doc = Document::builder().page_content("Hello, World!").build();
    let serialized = dumps(&doc, false).unwrap();
    let loaded = loads(&serialized, None).unwrap();

    assert_eq!(
        loaded.get("page_content").and_then(|v| v.as_str()),
        Some("Hello, World!")
    );
    assert!(loaded.get("_type").is_none());
}

#[test]
fn test_round_trip_document_with_metadata() {
    use agent_chain_core::documents::Document;

    let doc = Document::builder()
        .page_content("Test content")
        .metadata(HashMap::from([(
            "source".to_string(),
            serde_json::Value::String("test.txt".to_string()),
        )]))
        .build();

    let serialized = dumpd(&doc).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert_eq!(
        loaded.get("page_content").and_then(|v| v.as_str()),
        Some("Test content")
    );
    assert_eq!(
        loaded
            .get("metadata")
            .and_then(|v| v.get("source"))
            .and_then(|v| v.as_str()),
        Some("test.txt")
    );
}

#[test]
fn test_round_trip_human_message() {
    use agent_chain_core::messages::HumanMessage;

    let msg = HumanMessage::builder()
        .content("What is the meaning of life?")
        .build();
    let serialized = dumpd(&msg).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert_eq!(
        loaded.get("content").and_then(|v| v.as_str()),
        Some("What is the meaning of life?")
    );
    assert_eq!(loaded.get("type").and_then(|v| v.as_str()), Some("human"));
}

#[test]
fn test_round_trip_ai_message() {
    use agent_chain_core::messages::AIMessage;

    let msg = AIMessage::builder().content("42").build();
    let serialized = dumpd(&msg).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert_eq!(loaded.get("content").and_then(|v| v.as_str()), Some("42"));
    assert_eq!(loaded.get("type").and_then(|v| v.as_str()), Some("ai"));
}

#[test]
fn test_round_trip_system_message() {
    use agent_chain_core::messages::SystemMessage;

    let msg = SystemMessage::builder()
        .content("You are a helpful assistant.")
        .build();
    let serialized = dumpd(&msg).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert_eq!(
        loaded.get("content").and_then(|v| v.as_str()),
        Some("You are a helpful assistant.")
    );
    assert_eq!(loaded.get("type").and_then(|v| v.as_str()), Some("system"));
}

#[test]
fn test_round_trip_tool_message() {
    use agent_chain_core::messages::ToolMessage;

    let msg = ToolMessage::builder()
        .content("result data")
        .tool_call_id("call_123")
        .build();
    let serialized = dumpd(&msg).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert_eq!(
        loaded.get("content").and_then(|v| v.as_str()),
        Some("result data")
    );
    assert_eq!(
        loaded.get("tool_call_id").and_then(|v| v.as_str()),
        Some("call_123")
    );
}

#[test]
fn test_round_trip_prompt_template() {
    use agent_chain_core::prompts::PromptTemplate;

    let prompt = PromptTemplate::from_template("Hello, {name}!").unwrap();
    let serialized = dumpd(&prompt).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert_eq!(
        loaded.get("template").and_then(|v| v.as_str()),
        Some("Hello, {name}!")
    );
    assert_eq!(
        loaded.get("template_format").and_then(|v| v.as_str()),
        Some("f-string")
    );
    let input_vars = loaded
        .get("input_variables")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    assert_eq!(input_vars, vec!["name"]);
}

#[test]
fn test_round_trip_str_output_parser() {
    use agent_chain_core::output_parsers::StrOutputParser;

    let parser = StrOutputParser::new();
    let serialized = dumpd(&parser).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert!(loaded.is_object());
    assert!(loaded.get("_type").is_none());
}

#[test]
fn test_round_trip_old_namespace_mapping() {
    let serialized = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain", "schema", "messages", "HumanMessage"],
        "kwargs": {
            "content": "mapped message",
            "type": "human"
        }
    });

    let loaded = load(serialized, None).unwrap();

    assert_eq!(
        loaded.get("content").and_then(|v| v.as_str()),
        Some("mapped message")
    );
}

#[test]
fn test_round_trip_unknown_type_falls_back_to_constructor_info() {
    let serialized = json!({
        "lc": 1,
        "type": "constructor",
        "id": ["langchain_core", "unknown_module", "UnknownType"],
        "kwargs": {
            "field": "value"
        }
    });

    let loaded = load(serialized, None).unwrap();

    assert_eq!(
        loaded.get("_type").and_then(|v| v.as_str()),
        Some("constructor")
    );
}

#[test]
fn test_round_trip_chat_prompt_template() {
    use agent_chain_core::prompts::ChatPromptTemplate;

    let template = ChatPromptTemplate::from_messages(vec![
        ("system", "You are a helpful assistant.").into(),
        ("human", "{question}").into(),
    ])
    .unwrap();

    let serialized = dumpd(&template).unwrap();

    assert_eq!(serialized.get("lc").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(
        serialized.get("type").and_then(|v| v.as_str()),
        Some("constructor")
    );

    let kwargs = serialized.get("kwargs").unwrap();
    let input_vars = kwargs
        .get("input_variables")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();
    assert_eq!(input_vars, vec!["question"]);

    let loaded = load(serialized, None).unwrap();
    assert!(loaded.is_object());
    assert!(loaded.get("_type").is_none());

    let messages = loaded.get("messages").and_then(|v| v.as_array());
    assert!(messages.is_some());
    assert_eq!(messages.unwrap().len(), 2);
}

#[test]
fn test_round_trip_chat_prompt_template_with_placeholder() {
    use agent_chain_core::prompts::{ChatPromptTemplate, MessageLikeRepresentation};

    let template = ChatPromptTemplate::from_messages(vec![
        ("system", "You are a helpful assistant.").into(),
        MessageLikeRepresentation::placeholder("history", false),
        ("human", "{question}").into(),
    ])
    .unwrap();

    let serialized = dumpd(&template).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert!(loaded.is_object());
    let messages = loaded.get("messages").and_then(|v| v.as_array());
    assert!(messages.is_some());
    assert_eq!(messages.unwrap().len(), 3);
}

#[test]
fn test_round_trip_human_message_prompt_template() {
    use agent_chain_core::prompts::HumanMessagePromptTemplate;

    let template = HumanMessagePromptTemplate::from_template("Hello, {name}!").unwrap();
    let serialized = dumpd(&template).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert!(loaded.is_object());
    assert!(loaded.get("_type").is_none());
    let prompt = loaded.get("prompt");
    assert!(prompt.is_some());
}

#[test]
fn test_round_trip_system_message_prompt_template() {
    use agent_chain_core::prompts::SystemMessagePromptTemplate;

    let template = SystemMessagePromptTemplate::from_template("You are {role}.").unwrap();
    let serialized = dumpd(&template).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert!(loaded.is_object());
    assert!(loaded.get("_type").is_none());
}

#[test]
fn test_round_trip_messages_placeholder() {
    use agent_chain_core::prompts::MessagesPlaceholder;

    let placeholder = MessagesPlaceholder::builder()
        .variable_name("history")
        .optional(true)
        .build();
    let serialized = dumpd(&placeholder).unwrap();
    let loaded = load(serialized, None).unwrap();

    assert!(loaded.is_object());
    assert!(loaded.get("_type").is_none());
    assert_eq!(
        loaded.get("variable_name").and_then(|v| v.as_str()),
        Some("history")
    );
    assert_eq!(loaded.get("optional").and_then(|v| v.as_bool()), Some(true));
}
