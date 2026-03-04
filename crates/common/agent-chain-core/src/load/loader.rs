use std::collections::HashMap;
use std::env;
use std::sync::LazyLock;

use serde_json::Value;

use super::mapping::{DEFAULT_NAMESPACES, DISALLOW_LOAD_FROM_PATH, get_all_serializable_mappings};
use super::serializable::LC_VERSION;
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct ReviverConfig {
    pub secrets_map: HashMap<String, String>,
    pub valid_namespaces: Vec<String>,
    pub secrets_from_env: bool,
    pub additional_import_mappings: HashMap<Vec<String>, Vec<String>>,
    pub ignore_unserializable_fields: bool,
}

impl Default for ReviverConfig {
    fn default() -> Self {
        Self {
            secrets_map: HashMap::new(),
            valid_namespaces: DEFAULT_NAMESPACES.iter().map(|s| s.to_string()).collect(),
            secrets_from_env: true,
            additional_import_mappings: HashMap::new(),
            ignore_unserializable_fields: false,
        }
    }
}

#[bon::bon]
impl ReviverConfig {
    #[builder]
    pub fn new(
        secrets_map: Option<HashMap<String, String>>,
        valid_namespaces: Option<Vec<String>>,
        #[builder(default = true)] secrets_from_env: bool,
        additional_import_mappings: Option<HashMap<Vec<String>, Vec<String>>>,
        #[builder(default)] ignore_unserializable_fields: bool,
    ) -> Self {
        let mut default_namespaces: Vec<String> =
            DEFAULT_NAMESPACES.iter().map(|s| s.to_string()).collect();
        if let Some(extra) = valid_namespaces {
            default_namespaces.extend(extra);
        }

        Self {
            secrets_map: secrets_map.unwrap_or_default(),
            valid_namespaces: default_namespaces,
            secrets_from_env,
            additional_import_mappings: additional_import_mappings.unwrap_or_default(),
            ignore_unserializable_fields,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reviver {
    config: ReviverConfig,
    import_mappings: HashMap<Vec<String>, Vec<String>>,
}

impl Reviver {
    pub fn new(mut config: ReviverConfig) -> Self {
        let mut import_mappings = get_all_serializable_mappings();
        import_mappings.extend(std::mem::take(&mut config.additional_import_mappings));

        Self {
            config,
            import_mappings,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(ReviverConfig::default())
    }

    pub fn revive(&self, value: &Value) -> Result<RevivedValue> {
        let Some(obj) = value.as_object() else {
            return Ok(RevivedValue::Value(value.clone()));
        };

        let lc = obj.get("lc").and_then(|v| v.as_i64());
        let type_ = obj.get("type").and_then(|v| v.as_str());
        let Some(id_arr) = obj.get("id").and_then(|v| v.as_array()) else {
            return Ok(RevivedValue::Value(value.clone()));
        };

        if lc != Some(LC_VERSION as i64) {
            return Ok(RevivedValue::Value(value.clone()));
        }

        let id: Vec<String> = id_arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        match type_ {
            Some("secret") => self.revive_secret(&id),
            Some("not_implemented") => self.revive_not_implemented(value),
            Some("constructor") => self.revive_constructor(&id, obj),
            _ => Ok(RevivedValue::Value(value.clone())),
        }
    }

    fn revive_secret(&self, id: &[String]) -> Result<RevivedValue> {
        let Some(key) = id.first() else {
            return Ok(RevivedValue::None);
        };

        if let Some(secret) = self.config.secrets_map.get(key) {
            return Ok(RevivedValue::String(secret.clone()));
        }

        if self.config.secrets_from_env
            && let Ok(value) = env::var(key)
            && !value.is_empty()
        {
            return Ok(RevivedValue::String(value));
        }

        Ok(RevivedValue::None)
    }

    fn revive_not_implemented(&self, value: &Value) -> Result<RevivedValue> {
        if self.config.ignore_unserializable_fields {
            return Ok(RevivedValue::None);
        }

        Err(Error::Deserialization {
            id: vec![],
            reason: format!(
                "Trying to load an object that doesn't implement serialization: {value:?}"
            ),
        })
    }

    fn revive_constructor(
        &self,
        id: &[String],
        obj: &serde_json::Map<String, Value>,
    ) -> Result<RevivedValue> {
        if id.is_empty() {
            return Err(Error::Deserialization {
                id: vec![],
                reason: "Constructor id cannot be empty".to_string(),
            });
        }

        let namespace = &id[..id.len() - 1];
        let name = id.last().unwrap().clone();

        let root_namespace = namespace.first().map(|s| s.as_str()).unwrap_or("");

        if namespace.len() == 1 && namespace[0] == "langchain" {
            return Err(Error::Deserialization {
                id: id.to_vec(),
                reason: "Invalid namespace".to_string(),
            });
        }

        if !self
            .config
            .valid_namespaces
            .iter()
            .any(|ns| ns == root_namespace)
        {
            return Err(Error::Deserialization {
                id: id.to_vec(),
                reason: format!("Invalid namespace: {root_namespace}"),
            });
        }

        let mapping_key: Vec<String> = id.to_vec();
        let resolved_path = if let Some(import_path) = self.import_mappings.get(&mapping_key) {
            import_path.clone()
        } else if DISALLOW_LOAD_FROM_PATH.contains(&root_namespace) {
            return Err(Error::Deserialization {
                id: mapping_key,
                reason: "cannot be deserialized in current version".to_string(),
            });
        } else {
            id.to_vec()
        };

        let kwargs = obj
            .get("kwargs")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let kwargs_value = Value::Object(kwargs);

        let constructor = lookup_constructor(id).or_else(|| lookup_constructor(&resolved_path));
        if let Some(constructor) = constructor
            && let Ok(value) = constructor(&kwargs_value)
        {
            return Ok(RevivedValue::Value(value));
        }

        Ok(RevivedValue::Constructor(ConstructorInfo {
            path: resolved_path,
            name,
            kwargs: kwargs_value,
        }))
    }
}

#[derive(Debug, Clone)]
pub enum RevivedValue {
    Value(Value),
    String(String),
    Constructor(ConstructorInfo),
    None,
}

impl RevivedValue {
    pub fn into_value(self) -> Value {
        match self {
            RevivedValue::Value(v) => v,
            RevivedValue::String(s) => Value::String(s),
            RevivedValue::Constructor(info) => {
                serde_json::json!({
                    "_type": "constructor",
                    "path": info.path,
                    "name": info.name,
                    "kwargs": info.kwargs,
                })
            }
            RevivedValue::None => Value::Null,
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, RevivedValue::None)
    }
}

#[derive(Debug, Clone)]
pub struct ConstructorInfo {
    pub path: Vec<String>,
    pub name: String,
    pub kwargs: Value,
}

pub fn loads(text: &str, config: Option<ReviverConfig>) -> Result<Value> {
    let value: Value = serde_json::from_str(text)?;
    load(value, config)
}

pub fn load(obj: Value, config: Option<ReviverConfig>) -> Result<Value> {
    let reviver = Reviver::new(config.unwrap_or_default());
    load_recursive(&obj, &reviver)
}

fn load_recursive(obj: &Value, reviver: &Reviver) -> Result<Value> {
    match obj {
        Value::Object(map) => {
            let mut loaded_obj = serde_json::Map::new();
            for (k, v) in map {
                loaded_obj.insert(k.clone(), load_recursive(v, reviver)?);
            }

            let loaded_value = Value::Object(loaded_obj);
            let revived = reviver.revive(&loaded_value)?;
            Ok(revived.into_value())
        }
        Value::Array(arr) => {
            let loaded: Result<Vec<Value>> =
                arr.iter().map(|v| load_recursive(v, reviver)).collect();
            Ok(Value::Array(loaded?))
        }
        _ => Ok(obj.clone()),
    }
}

pub fn loads_with_secrets(text: &str, secrets_map: HashMap<String, String>) -> Result<Value> {
    let config = ReviverConfig::builder().secrets_map(secrets_map).build();
    loads(text, Some(config))
}

pub fn loads_with_namespaces(text: &str, namespaces: Vec<String>) -> Result<Value> {
    let config = ReviverConfig::builder()
        .valid_namespaces(namespaces)
        .build();
    loads(text, Some(config))
}

pub type ConstructorFn = fn(&Value) -> Result<Value>;

pub struct ConstructorEntry {
    pub lc_id: fn() -> Vec<String>,
    pub constructor: ConstructorFn,
}

inventory::collect!(ConstructorEntry);

pub fn deserialize_constructor<T>(kwargs: &Value) -> Result<Value>
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let obj: T = serde_json::from_value(kwargs.clone())?;
    Ok(serde_json::to_value(&obj)?)
}

static CONSTRUCTOR_REGISTRY: LazyLock<HashMap<String, ConstructorFn>> = LazyLock::new(|| {
    let mut registry = HashMap::new();
    let mappings = get_all_serializable_mappings();

    for entry in inventory::iter::<ConstructorEntry> {
        let id = (entry.lc_id)();
        let key = id.join(":");
        registry.insert(key, entry.constructor);

        for (old_path, new_path) in &mappings {
            if *old_path == id {
                let mapped_key = new_path.join(":");
                registry.insert(mapped_key, entry.constructor);
            }
        }
    }

    registry
});

pub fn lookup_constructor(path: &[String]) -> Option<&'static ConstructorFn> {
    let key = path.join(":");
    CONSTRUCTOR_REGISTRY.get(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reviver_default() {
        let reviver = Reviver::with_defaults();
        assert!(reviver.config.secrets_from_env);
        assert!(!reviver.config.valid_namespaces.is_empty());
    }

    #[test]
    fn test_revive_simple_value() {
        let reviver = Reviver::with_defaults();
        let value = serde_json::json!({"key": "value"});
        let result = reviver.revive(&value).unwrap();

        match result {
            RevivedValue::Value(v) => {
                assert_eq!(v.get("key").and_then(|v| v.as_str()), Some("value"));
            }
            _ => panic!("Expected Value"),
        }
    }

    #[test]
    fn test_revive_secret_from_map() {
        let config = ReviverConfig::builder()
            .secrets_map(HashMap::from([(
                "MY_SECRET".to_string(),
                "secret_value".to_string(),
            )]))
            .build();
        let reviver = Reviver::new(config);

        let value = serde_json::json!({
            "lc": 1,
            "type": "secret",
            "id": ["MY_SECRET"]
        });

        let result = reviver.revive(&value).unwrap();
        match result {
            RevivedValue::String(s) => assert_eq!(s, "secret_value"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_revive_missing_secret() {
        let config = ReviverConfig::builder().secrets_from_env(false).build();
        let reviver = Reviver::new(config);

        let value = serde_json::json!({
            "lc": 1,
            "type": "secret",
            "id": ["NONEXISTENT_SECRET"]
        });

        let result = reviver.revive(&value).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_revive_not_implemented_error() {
        let reviver = Reviver::with_defaults();

        let value = serde_json::json!({
            "lc": 1,
            "type": "not_implemented",
            "id": ["some", "type"],
            "repr": "SomeType(...)"
        });

        let result = reviver.revive(&value);
        assert!(result.is_err());
    }

    #[test]
    fn test_revive_not_implemented_ignored() {
        let config = ReviverConfig::builder()
            .ignore_unserializable_fields(true)
            .build();
        let reviver = Reviver::new(config);

        let value = serde_json::json!({
            "lc": 1,
            "type": "not_implemented",
            "id": ["some", "type"],
            "repr": "SomeType(...)"
        });

        let result = reviver.revive(&value).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_revive_constructor() {
        let reviver = Reviver::with_defaults();

        let value = serde_json::json!({
            "lc": 1,
            "type": "constructor",
            "id": ["langchain_core", "messages", "ai", "AIMessage"],
            "kwargs": {
                "content": "Hello, world!"
            }
        });

        let result = reviver.revive(&value).unwrap();
        match result {
            RevivedValue::Value(v) => {
                assert_eq!(
                    v.get("content").and_then(|v| v.as_str()),
                    Some("Hello, world!")
                );
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
    fn test_revive_constructor_with_mapping() {
        let reviver = Reviver::with_defaults();

        let value = serde_json::json!({
            "lc": 1,
            "type": "constructor",
            "id": ["langchain", "schema", "messages", "AIMessage"],
            "kwargs": {
                "content": "Hello!"
            }
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
    fn test_revive_invalid_namespace() {
        let reviver = Reviver::with_defaults();

        let value = serde_json::json!({
            "lc": 1,
            "type": "constructor",
            "id": ["invalid_namespace", "SomeClass"],
            "kwargs": {}
        });

        let result = reviver.revive(&value);
        assert!(result.is_err());
    }

    #[test]
    fn test_loads_simple() {
        let json = r#"{"key": "value"}"#;
        let result = loads(json, None).unwrap();
        assert_eq!(result.get("key").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn test_loads_nested() {
        let json = r#"{
            "outer": {
                "lc": 1,
                "type": "secret",
                "id": ["TEST_KEY"]
            }
        }"#;

        let config = ReviverConfig::builder()
            .secrets_map(HashMap::from([(
                "TEST_KEY".to_string(),
                "resolved".to_string(),
            )]))
            .build();

        let result = loads(json, Some(config)).unwrap();
        assert_eq!(
            result.get("outer").and_then(|v| v.as_str()),
            Some("resolved")
        );
    }

    #[test]
    fn test_load_array() {
        let json = r#"[
            {"lc": 1, "type": "secret", "id": ["KEY1"]},
            {"lc": 1, "type": "secret", "id": ["KEY2"]}
        ]"#;

        let config = ReviverConfig::builder()
            .secrets_map(HashMap::from([
                ("KEY1".to_string(), "value1".to_string()),
                ("KEY2".to_string(), "value2".to_string()),
            ]))
            .build();

        let result = loads(json, Some(config)).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr[0].as_str(), Some("value1"));
        assert_eq!(arr[1].as_str(), Some("value2"));
    }
}
