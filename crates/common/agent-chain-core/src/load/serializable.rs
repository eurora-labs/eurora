//! Serializable base trait and types.
//!
//! This module contains the core `Serializable` trait and related types,
//! mirroring `langchain_core.load.serializable`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;

/// Serialization version.
pub const LC_VERSION: i32 = 1;

/// Base serialized structure containing common fields.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseSerialized {
    /// The version of the serialization format.
    pub lc: i32,
    /// The unique identifier of the object (namespace path).
    pub id: Vec<String>,
    /// The name of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The graph of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

impl Default for BaseSerialized {
    fn default() -> Self {
        Self {
            lc: LC_VERSION,
            id: Vec::new(),
            name: None,
            graph: None,
        }
    }
}

/// Serialized constructor representation.
///
/// Used when an object can be serialized and reconstructed from its constructor.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedConstructor {
    /// The version of the serialization format.
    pub lc: i32,
    /// The type of serialization. Always "constructor".
    #[serde(rename = "type")]
    pub type_: String,
    /// The unique identifier of the object (namespace path).
    pub id: Vec<String>,
    /// The constructor arguments.
    pub kwargs: HashMap<String, Value>,
    /// The name of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The graph of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

impl SerializedConstructor {
    /// Create a new SerializedConstructor.
    pub fn new(id: Vec<String>, kwargs: HashMap<String, Value>) -> Self {
        Self {
            lc: LC_VERSION,
            type_: "constructor".to_string(),
            id,
            kwargs,
            name: None,
            graph: None,
        }
    }

    /// Set the name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the graph.
    pub fn with_graph(mut self, graph: HashMap<String, Value>) -> Self {
        self.graph = Some(graph);
        self
    }
}

/// Serialized secret representation.
///
/// Used to represent secret values that should not be serialized directly.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedSecret {
    /// The version of the serialization format.
    pub lc: i32,
    /// The type of serialization. Always "secret".
    #[serde(rename = "type")]
    pub type_: String,
    /// The unique identifier of the secret (usually environment variable name).
    pub id: Vec<String>,
    /// The name of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The graph of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

impl SerializedSecret {
    /// Create a new SerializedSecret.
    pub fn new(id: Vec<String>) -> Self {
        Self {
            lc: LC_VERSION,
            type_: "secret".to_string(),
            id,
            name: None,
            graph: None,
        }
    }

    /// Create a secret from a single secret id (environment variable name).
    pub fn from_secret_id(secret_id: impl Into<String>) -> Self {
        Self::new(vec![secret_id.into()])
    }
}

/// Serialized not implemented representation.
///
/// Used when an object cannot be serialized.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedNotImplemented {
    /// The version of the serialization format.
    pub lc: i32,
    /// The type of serialization. Always "not_implemented".
    #[serde(rename = "type")]
    pub type_: String,
    /// The unique identifier of the object (namespace path).
    pub id: Vec<String>,
    /// The representation of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repr: Option<String>,
    /// The name of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The graph of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

impl SerializedNotImplemented {
    /// Create a new SerializedNotImplemented.
    pub fn new(id: Vec<String>) -> Self {
        Self {
            lc: LC_VERSION,
            type_: "not_implemented".to_string(),
            id,
            repr: None,
            name: None,
            graph: None,
        }
    }

    /// Set the repr.
    pub fn with_repr(mut self, repr: impl Into<String>) -> Self {
        self.repr = Some(repr.into());
        self
    }
}

/// Union type for all serialized representations.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Serialized {
    /// A serialized constructor.
    #[serde(rename = "constructor")]
    Constructor(SerializedConstructorData),
    /// A serialized secret.
    #[serde(rename = "secret")]
    Secret(SerializedSecretData),
    /// A serialized not implemented.
    #[serde(rename = "not_implemented")]
    NotImplemented(SerializedNotImplementedData),
}

/// Data for SerializedConstructor without the type tag.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedConstructorData {
    /// The version of the serialization format.
    pub lc: i32,
    /// The unique identifier of the object (namespace path).
    pub id: Vec<String>,
    /// The constructor arguments.
    pub kwargs: HashMap<String, Value>,
    /// The name of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The graph of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

/// Data for SerializedSecret without the type tag.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedSecretData {
    /// The version of the serialization format.
    pub lc: i32,
    /// The unique identifier of the secret (usually environment variable name).
    pub id: Vec<String>,
    /// The name of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The graph of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

/// Data for SerializedNotImplemented without the type tag.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedNotImplementedData {
    /// The version of the serialization format.
    pub lc: i32,
    /// The unique identifier of the object (namespace path).
    pub id: Vec<String>,
    /// The representation of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repr: Option<String>,
    /// The name of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The graph of the object (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

impl From<SerializedConstructor> for Serialized {
    fn from(s: SerializedConstructor) -> Self {
        Serialized::Constructor(SerializedConstructorData {
            lc: s.lc,
            id: s.id,
            kwargs: s.kwargs,
            name: s.name,
            graph: s.graph,
        })
    }
}

impl From<SerializedSecret> for Serialized {
    fn from(s: SerializedSecret) -> Self {
        Serialized::Secret(SerializedSecretData {
            lc: s.lc,
            id: s.id,
            name: s.name,
            graph: s.graph,
        })
    }
}

impl From<SerializedNotImplemented> for Serialized {
    fn from(s: SerializedNotImplemented) -> Self {
        Serialized::NotImplemented(SerializedNotImplementedData {
            lc: s.lc,
            id: s.id,
            repr: s.repr,
            name: s.name,
            graph: s.graph,
        })
    }
}

/// Trait for objects that can be serialized to JSON for LangChain compatibility.
///
/// This trait provides the core serialization functionality, allowing objects
/// to be serialized and deserialized across different versions and implementations.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::load::Serializable;
///
/// struct MyModel {
///     name: String,
///     value: i32,
/// }
///
/// impl Serializable for MyModel {
///     fn is_lc_serializable() -> bool {
///         true
///     }
///
///     fn get_lc_namespace() -> Vec<String> {
///         vec!["my_package".to_string(), "models".to_string()]
///     }
/// }
/// ```
pub trait Serializable: Any + Send + Sync {
    /// Is this class serializable?
    ///
    /// By design, even if a type implements `Serializable`, it is not serializable
    /// by default. This is to prevent accidental serialization of objects that should
    /// not be serialized.
    ///
    /// Returns `false` by default.
    fn is_lc_serializable() -> bool
    where
        Self: Sized,
    {
        false
    }

    /// Get the namespace of the LangChain object.
    ///
    /// For example, if the class is `langchain.llms.openai.OpenAI`, then the
    /// namespace is `["langchain", "llms", "openai"]`.
    fn get_lc_namespace() -> Vec<String>
    where
        Self: Sized;

    /// A map of constructor argument names to secret ids.
    ///
    /// For example, `{"openai_api_key": "OPENAI_API_KEY"}`.
    fn lc_secrets(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    /// List of attribute names that should be included in the serialized kwargs.
    ///
    /// These attributes must be accepted by the constructor.
    fn lc_attributes(&self) -> HashMap<String, Value> {
        HashMap::new()
    }

    /// Return a unique identifier for this class for serialization purposes.
    ///
    /// The unique identifier is a list of strings that describes the path
    /// to the object.
    fn lc_id() -> Vec<String>
    where
        Self: Sized,
    {
        let mut id = Self::get_lc_namespace();
        id.push(
            std::any::type_name::<Self>()
                .rsplit("::")
                .next()
                .unwrap_or("Unknown")
                .to_string(),
        );
        id
    }

    /// Get the type name of this object.
    fn lc_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Serialize this object to JSON.
    ///
    /// Returns a `SerializedConstructor` if the object is serializable,
    /// or a `SerializedNotImplemented` if it is not.
    fn to_json(&self) -> Serialized
    where
        Self: Sized + Serialize,
    {
        if !Self::is_lc_serializable() {
            return self.to_json_not_implemented();
        }

        let kwargs: HashMap<String, Value> = match serde_json::to_value(self) {
            Ok(Value::Object(map)) => map
                .into_iter()
                .filter(|(k, v)| is_field_useful(k, v))
                .collect(),
            _ => HashMap::new(),
        };

        let secrets = self.lc_secrets();
        let kwargs = if secrets.is_empty() {
            kwargs
        } else {
            replace_secrets(kwargs, &secrets)
        };

        let mut final_kwargs = kwargs;
        for (key, value) in self.lc_attributes() {
            final_kwargs.insert(key, value);
        }

        SerializedConstructor::new(Self::lc_id(), final_kwargs).into()
    }

    /// Serialize a "not implemented" object.
    fn to_json_not_implemented(&self) -> Serialized {
        to_json_not_implemented_value(self.lc_type_name(), None)
    }
}

/// Check if a field is useful as a constructor argument.
///
/// Mirrors `_is_field_useful()` from `langchain_core.load.serializable`.
/// Filters out null values, empty strings, empty arrays, and empty objects.
fn is_field_useful(_key: &str, value: &Value) -> bool {
    !value.is_null()
}

/// Create a SerializedNotImplemented for an arbitrary object.
///
/// # Arguments
///
/// * `type_name` - The type name of the object.
/// * `repr` - Optional string representation.
pub fn to_json_not_implemented_value(type_name: &str, repr: Option<String>) -> Serialized {
    let id: Vec<String> = type_name.split("::").map(|s| s.to_string()).collect();

    let mut result = SerializedNotImplemented::new(id);
    if let Some(r) = repr {
        result = result.with_repr(r);
    }

    result.into()
}

/// Create a SerializedNotImplemented from a serde_json::Value.
///
/// Used as a fallback when normal serialization fails.
pub fn to_json_not_implemented(value: &Value) -> Serialized {
    let repr = serde_json::to_string_pretty(value).ok();
    to_json_not_implemented_value("serde_json::Value", repr)
}

/// Replace secrets in kwargs with SerializedSecret placeholders.
fn replace_secrets(
    mut kwargs: HashMap<String, Value>,
    secrets_map: &HashMap<String, String>,
) -> HashMap<String, Value> {
    for (path, secret_id) in secrets_map {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.len() == 1 {
            if kwargs.contains_key(path) {
                kwargs.insert(
                    path.clone(),
                    serde_json::to_value(SerializedSecret::from_secret_id(secret_id))
                        .expect("SerializedSecret serialization"),
                );
            }
        } else {
            replace_nested_secret(&mut kwargs, &parts, secret_id);
        }
    }
    kwargs
}

/// Replace a nested secret in kwargs.
fn replace_nested_secret(current: &mut HashMap<String, Value>, parts: &[&str], secret_id: &str) {
    if parts.is_empty() {
        return;
    }

    let key = parts[0];

    if parts.len() == 1 {
        if current.contains_key(key) {
            current.insert(
                key.to_string(),
                serde_json::to_value(SerializedSecret::from_secret_id(secret_id))
                    .expect("SerializedSecret serialization"),
            );
        }
    } else if let Some(Value::Object(map)) = current.get_mut(key) {
        let mut nested: HashMap<String, Value> =
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        replace_nested_secret(&mut nested, &parts[1..], secret_id);
        *map = nested.into_iter().collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialized_constructor() {
        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), Value::String("test".to_string()));

        let constructor = SerializedConstructor::new(
            vec![
                "langchain".to_string(),
                "llms".to_string(),
                "OpenAI".to_string(),
            ],
            kwargs,
        );

        assert_eq!(constructor.lc, 1);
        assert_eq!(constructor.type_, "constructor");
        assert_eq!(constructor.id.len(), 3);
    }

    #[test]
    fn test_serialized_secret() {
        let secret = SerializedSecret::from_secret_id("OPENAI_API_KEY");

        assert_eq!(secret.lc, 1);
        assert_eq!(secret.type_, "secret");
        assert_eq!(secret.id, vec!["OPENAI_API_KEY".to_string()]);
    }

    #[test]
    fn test_serialized_not_implemented() {
        let not_impl =
            SerializedNotImplemented::new(vec!["my_module".to_string(), "MyClass".to_string()])
                .with_repr("MyClass(...)".to_string());

        assert_eq!(not_impl.lc, 1);
        assert_eq!(not_impl.type_, "not_implemented");
        assert_eq!(not_impl.repr, Some("MyClass(...)".to_string()));
    }

    #[test]
    fn test_replace_secrets() {
        let mut kwargs = HashMap::new();
        kwargs.insert(
            "api_key".to_string(),
            Value::String("secret_value".to_string()),
        );
        kwargs.insert("model".to_string(), Value::String("gpt-4".to_string()));

        let mut secrets = HashMap::new();
        secrets.insert("api_key".to_string(), "OPENAI_API_KEY".to_string());

        let result = replace_secrets(kwargs, &secrets);

        assert!(result.get("api_key").unwrap().is_object());
        assert_eq!(
            result.get("model").unwrap(),
            &Value::String("gpt-4".to_string())
        );
    }
}
