//! Load LangChain objects from JSON strings or objects.
//!
//! This module provides functions for deserializing LangChain objects from JSON,
//! mirroring `langchain_core.load.load`.
//!
//! # Warning
//!
//! The `load` and `loads` functions can instantiate arbitrary types based on the
//! serialized data. Be careful when using with untrusted input.

use serde_json::Value;
use std::collections::HashMap;
use std::env;

use super::mapping::{DEFAULT_NAMESPACES, DISALLOW_LOAD_FROM_PATH, get_all_serializable_mappings};
use super::serializable::LC_VERSION;
use crate::error::{Error, Result};

/// Configuration for the Reviver.
#[derive(Debug, Clone)]
pub struct ReviverConfig {
    /// A map of secrets to load. If a secret is not found in the map,
    /// it will be loaded from the environment if `secrets_from_env` is `true`.
    pub secrets_map: HashMap<String, String>,
    /// A list of additional namespaces (modules) to allow to be deserialized.
    pub valid_namespaces: Vec<String>,
    /// Whether to load secrets from the environment.
    pub secrets_from_env: bool,
    /// Additional import mappings to override or extend the default mappings.
    pub additional_import_mappings: HashMap<Vec<String>, Vec<String>>,
    /// Whether to ignore unserializable fields (return None instead of error).
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

impl ReviverConfig {
    /// Create a new ReviverConfig with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the secrets map.
    pub fn with_secrets_map(mut self, secrets_map: HashMap<String, String>) -> Self {
        self.secrets_map = secrets_map;
        self
    }

    /// Add additional valid namespaces.
    pub fn with_valid_namespaces(mut self, namespaces: Vec<String>) -> Self {
        self.valid_namespaces.extend(namespaces);
        self
    }

    /// Set whether to load secrets from the environment.
    pub fn with_secrets_from_env(mut self, secrets_from_env: bool) -> Self {
        self.secrets_from_env = secrets_from_env;
        self
    }

    /// Add additional import mappings.
    pub fn with_additional_import_mappings(
        mut self,
        mappings: HashMap<Vec<String>, Vec<String>>,
    ) -> Self {
        self.additional_import_mappings = mappings;
        self
    }

    /// Set whether to ignore unserializable fields.
    pub fn with_ignore_unserializable_fields(mut self, ignore: bool) -> Self {
        self.ignore_unserializable_fields = ignore;
        self
    }
}

/// Reviver for JSON objects.
///
/// The Reviver is responsible for transforming serialized LangChain objects
/// back into their original form. It handles:
/// - Secret resolution (from secrets_map or environment)
/// - Namespace validation
/// - Type mapping (for backwards compatibility)
#[derive(Debug, Clone)]
pub struct Reviver {
    config: ReviverConfig,
    import_mappings: HashMap<Vec<String>, Vec<String>>,
}

impl Reviver {
    /// Create a new Reviver with the given configuration.
    pub fn new(config: ReviverConfig) -> Self {
        let mut import_mappings = get_all_serializable_mappings();
        import_mappings.extend(config.additional_import_mappings.clone());

        Self {
            config,
            import_mappings,
        }
    }

    /// Create a new Reviver with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(ReviverConfig::default())
    }

    /// Revive a JSON value.
    ///
    /// This method processes a JSON value and:
    /// - Resolves secrets
    /// - Validates namespaces
    /// - Maps old namespaces to new ones
    ///
    /// # Arguments
    ///
    /// * `value` - The JSON value to revive.
    ///
    /// # Returns
    ///
    /// The revived value, or an error if revival fails.
    pub fn revive(&self, value: &Value) -> Result<RevivedValue> {
        let Some(obj) = value.as_object() else {
            return Ok(RevivedValue::Value(value.clone()));
        };

        let lc = obj.get("lc").and_then(|v| v.as_i64());
        let type_ = obj.get("type").and_then(|v| v.as_str());
        let id = obj.get("id").and_then(|v| v.as_array());

        if lc != Some(LC_VERSION as i64) || id.is_none() {
            return Ok(RevivedValue::Value(value.clone()));
        }

        let id: Vec<String> = id
            .expect("checked is_none above")
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

    /// Revive a secret value.
    fn revive_secret(&self, id: &[String]) -> Result<RevivedValue> {
        if id.is_empty() {
            return Ok(RevivedValue::None);
        }

        let key = &id[0];

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

    /// Revive a not_implemented value.
    fn revive_not_implemented(&self, value: &Value) -> Result<RevivedValue> {
        if self.config.ignore_unserializable_fields {
            return Ok(RevivedValue::None);
        }

        Err(Error::Other(format!(
            "Trying to load an object that doesn't implement serialization: {:?}",
            value
        )))
    }

    /// Revive a constructor value.
    fn revive_constructor(
        &self,
        id: &[String],
        obj: &serde_json::Map<String, Value>,
    ) -> Result<RevivedValue> {
        if id.is_empty() {
            return Err(Error::Other("Constructor id cannot be empty".to_string()));
        }

        let namespace: Vec<String> = id[..id.len() - 1].to_vec();
        let name = id.last().expect("checked non-empty above").clone();

        let root_namespace = namespace.first().map(|s| s.as_str()).unwrap_or("");

        if namespace == vec!["langchain".to_string()] {
            return Err(Error::Other(format!("Invalid namespace: {:?}", id)));
        }

        if !self
            .config
            .valid_namespaces
            .iter()
            .any(|ns| ns == root_namespace)
        {
            return Err(Error::Other(format!("Invalid namespace: {:?}", id)));
        }

        let mapping_key: Vec<String> = id.to_vec();
        let resolved_path = if let Some(import_path) = self.import_mappings.get(&mapping_key) {
            import_path.clone()
        } else if DISALLOW_LOAD_FROM_PATH.contains(&root_namespace) {
            return Err(Error::Other(format!(
                "Trying to deserialize something that cannot be deserialized \
                 in current version of langchain-core: {:?}",
                mapping_key
            )));
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

/// Result of reviving a JSON value.
#[derive(Debug, Clone)]
pub enum RevivedValue {
    /// A simple value (unchanged from input).
    Value(Value),
    /// A string value (typically a resolved secret).
    String(String),
    /// A constructor to instantiate.
    Constructor(ConstructorInfo),
    /// No value (for ignored unserializable fields or missing secrets).
    None,
}

impl RevivedValue {
    /// Convert to a serde_json::Value.
    pub fn to_value(&self) -> Value {
        match self {
            RevivedValue::Value(v) => v.clone(),
            RevivedValue::String(s) => Value::String(s.clone()),
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

    /// Check if this is a None value.
    pub fn is_none(&self) -> bool {
        matches!(self, RevivedValue::None)
    }
}

/// Information about a constructor to instantiate.
#[derive(Debug, Clone)]
pub struct ConstructorInfo {
    /// The full path to the type (namespace + name).
    pub path: Vec<String>,
    /// The name of the type.
    pub name: String,
    /// The constructor arguments.
    pub kwargs: Value,
}

/// Revive a LangChain object from a JSON string.
///
/// # Warning
///
/// This function can instantiate arbitrary types based on the serialized data.
/// Be careful when using with untrusted input.
///
/// # Arguments
///
/// * `text` - The JSON string to parse and revive.
/// * `config` - Optional configuration for the reviver.
///
/// # Returns
///
/// The revived value.
///
/// # Errors
///
/// Returns an error if parsing or revival fails.
pub fn loads(text: &str, config: Option<ReviverConfig>) -> Result<Value> {
    let value: Value = serde_json::from_str(text)?;
    load(value, config)
}

/// Revive a LangChain object from a JSON value.
///
/// Use this if you already have a parsed JSON object,
/// e.g., from `serde_json::from_str` or similar.
///
/// # Warning
///
/// This function can instantiate arbitrary types based on the serialized data.
/// Be careful when using with untrusted input.
///
/// # Arguments
///
/// * `obj` - The JSON value to revive.
/// * `config` - Optional configuration for the reviver.
///
/// # Returns
///
/// The revived value.
///
/// # Errors
///
/// Returns an error if revival fails.
pub fn load(obj: Value, config: Option<ReviverConfig>) -> Result<Value> {
    let reviver = Reviver::new(config.unwrap_or_default());
    load_recursive(&obj, &reviver)
}

/// Recursively load a JSON value.
fn load_recursive(obj: &Value, reviver: &Reviver) -> Result<Value> {
    match obj {
        Value::Object(map) => {
            let mut loaded_obj = serde_json::Map::new();
            for (k, v) in map {
                loaded_obj.insert(k.clone(), load_recursive(v, reviver)?);
            }

            let loaded_value = Value::Object(loaded_obj);
            let revived = reviver.revive(&loaded_value)?;
            Ok(revived.to_value())
        }
        Value::Array(arr) => {
            let loaded: Result<Vec<Value>> =
                arr.iter().map(|v| load_recursive(v, reviver)).collect();
            Ok(Value::Array(loaded?))
        }
        _ => Ok(obj.clone()),
    }
}

/// Load with secrets from a map.
///
/// Convenience function that creates a ReviverConfig with the given secrets map.
///
/// # Arguments
///
/// * `text` - The JSON string to parse and revive.
/// * `secrets_map` - A map of secret names to their values.
///
/// # Returns
///
/// The revived value.
pub fn loads_with_secrets(text: &str, secrets_map: HashMap<String, String>) -> Result<Value> {
    let config = ReviverConfig::new().with_secrets_map(secrets_map);
    loads(text, Some(config))
}

/// Load with additional valid namespaces.
///
/// Convenience function that creates a ReviverConfig with additional namespaces.
///
/// # Arguments
///
/// * `text` - The JSON string to parse and revive.
/// * `namespaces` - Additional namespaces to allow.
///
/// # Returns
///
/// The revived value.
pub fn loads_with_namespaces(text: &str, namespaces: Vec<String>) -> Result<Value> {
    let config = ReviverConfig::new().with_valid_namespaces(namespaces);
    loads(text, Some(config))
}

use std::sync::LazyLock;

use crate::agents::{AgentAction, AgentFinish};
use crate::documents::Document;
use crate::messages::{
    AIMessage, AIMessageChunk, ChatMessage, ChatMessageChunk, HumanMessage, HumanMessageChunk,
    SystemMessage, SystemMessageChunk, ToolMessage,
};
use crate::output_parsers::StrOutputParser;
use crate::prompt_values::{ChatPromptValue, StringPromptValue};
use crate::prompts::{
    AIMessagePromptTemplate, ChatMessagePromptTemplate, ChatPromptTemplate,
    HumanMessagePromptTemplate, MessagesPlaceholder, PromptTemplate, SystemMessagePromptTemplate,
};

type ConstructorFn = fn(&Value) -> Result<Value>;

fn register_constructor<T>(registry: &mut HashMap<String, ConstructorFn>)
where
    T: serde::de::DeserializeOwned + serde::Serialize + super::serializable::Serializable,
{
    let id = T::lc_id();
    let key = id.join(":");
    let constructor: ConstructorFn = |kwargs| {
        let obj: T = serde_json::from_value(kwargs.clone())?;
        let value = serde_json::to_value(&obj)?;
        Ok(value)
    };
    registry.insert(key, constructor);

    let mappings = get_all_serializable_mappings();
    for (old_path, new_path) in &mappings {
        if *old_path == id {
            let mapped_key = new_path.join(":");
            registry.insert(mapped_key, constructor);
        }
    }
}

static CONSTRUCTOR_REGISTRY: LazyLock<HashMap<String, ConstructorFn>> = LazyLock::new(|| {
    let mut registry = HashMap::new();
    register_constructor::<AIMessage>(&mut registry);
    register_constructor::<HumanMessage>(&mut registry);
    register_constructor::<SystemMessage>(&mut registry);
    register_constructor::<ToolMessage>(&mut registry);
    register_constructor::<ChatMessage>(&mut registry);
    register_constructor::<Document>(&mut registry);
    register_constructor::<PromptTemplate>(&mut registry);
    register_constructor::<ChatPromptTemplate>(&mut registry);
    register_constructor::<MessagesPlaceholder>(&mut registry);
    register_constructor::<HumanMessagePromptTemplate>(&mut registry);
    register_constructor::<AIMessagePromptTemplate>(&mut registry);
    register_constructor::<SystemMessagePromptTemplate>(&mut registry);
    register_constructor::<ChatMessagePromptTemplate>(&mut registry);
    register_constructor::<StrOutputParser>(&mut registry);
    register_constructor::<AIMessageChunk>(&mut registry);
    register_constructor::<HumanMessageChunk>(&mut registry);
    register_constructor::<SystemMessageChunk>(&mut registry);
    register_constructor::<ChatMessageChunk>(&mut registry);
    register_constructor::<StringPromptValue>(&mut registry);
    register_constructor::<ChatPromptValue>(&mut registry);
    register_constructor::<AgentAction>(&mut registry);
    register_constructor::<AgentFinish>(&mut registry);
    registry
});

/// Look up a constructor function by lc_id path.
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
        let config = ReviverConfig::new().with_secrets_map(HashMap::from([(
            "MY_SECRET".to_string(),
            "secret_value".to_string(),
        )]));
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
        let config = ReviverConfig::new().with_secrets_from_env(false);
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
        let config = ReviverConfig::new().with_ignore_unserializable_fields(true);
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

        let config = ReviverConfig::new().with_secrets_map(HashMap::from([(
            "TEST_KEY".to_string(),
            "resolved".to_string(),
        )]));

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

        let config = ReviverConfig::new().with_secrets_map(HashMap::from([
            ("KEY1".to_string(), "value1".to_string()),
            ("KEY2".to_string(), "value2".to_string()),
        ]));

        let result = loads(json, Some(config)).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr[0].as_str(), Some("value1"));
        assert_eq!(arr[1].as_str(), Some("value2"));
    }
}
