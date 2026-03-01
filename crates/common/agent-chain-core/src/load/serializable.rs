use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::HashMap;

pub const LC_VERSION: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BaseSerialized {
    pub lc: i32,
    pub id: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedConstructor {
    pub lc: i32,
    #[serde(rename = "type")]
    pub type_: String,
    pub id: Vec<String>,
    pub kwargs: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

#[bon::bon]
impl SerializedConstructor {
    #[builder]
    pub fn new(
        id: Vec<String>,
        kwargs: HashMap<String, Value>,
        #[builder(into)] name: Option<String>,
        graph: Option<HashMap<String, Value>>,
    ) -> Self {
        Self {
            lc: LC_VERSION,
            type_: "constructor".to_string(),
            id,
            kwargs,
            name,
            graph,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedSecret {
    pub lc: i32,
    #[serde(rename = "type")]
    pub type_: String,
    pub id: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

impl SerializedSecret {
    pub fn new(id: Vec<String>) -> Self {
        Self {
            lc: LC_VERSION,
            type_: "secret".to_string(),
            id,
            name: None,
            graph: None,
        }
    }

    pub fn from_secret_id(secret_id: impl Into<String>) -> Self {
        Self::new(vec![secret_id.into()])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedNotImplemented {
    pub lc: i32,
    #[serde(rename = "type")]
    pub type_: String,
    pub id: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

#[bon::bon]
impl SerializedNotImplemented {
    #[builder]
    pub fn new(id: Vec<String>, #[builder(into)] repr: Option<String>) -> Self {
        Self {
            lc: LC_VERSION,
            type_: "not_implemented".to_string(),
            id,
            repr,
            name: None,
            graph: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Serialized {
    #[serde(rename = "constructor")]
    Constructor(SerializedConstructorData),
    #[serde(rename = "secret")]
    Secret(SerializedSecretData),
    #[serde(rename = "not_implemented")]
    NotImplemented(SerializedNotImplementedData),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedConstructorData {
    pub lc: i32,
    pub id: Vec<String>,
    pub kwargs: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedSecretData {
    pub lc: i32,
    pub id: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedNotImplementedData {
    pub lc: i32,
    pub id: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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

pub trait Serializable: Any + Send + Sync {
    fn is_lc_serializable() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn get_lc_namespace() -> Vec<String>
    where
        Self: Sized;

    fn lc_secrets(&self) -> HashMap<String, String> {
        HashMap::new()
    }

    fn lc_attributes(&self) -> HashMap<String, Value> {
        HashMap::new()
    }

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

    fn lc_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

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

        SerializedConstructor::builder()
            .id(Self::lc_id())
            .kwargs(final_kwargs)
            .build()
            .into()
    }

    fn to_json_not_implemented(&self) -> Serialized {
        to_json_not_implemented_value(self.lc_type_name(), None)
    }
}

fn is_field_useful(_key: &str, value: &Value) -> bool {
    !value.is_null()
}

pub fn to_json_not_implemented_value(type_name: &str, repr: Option<String>) -> Serialized {
    let id: Vec<String> = type_name.split("::").map(|s| s.to_string()).collect();

    SerializedNotImplemented::builder()
        .id(id)
        .maybe_repr(repr)
        .build()
        .into()
}

pub fn to_json_not_implemented(value: &Value) -> Serialized {
    let repr = serde_json::to_string_pretty(value).ok();
    to_json_not_implemented_value("serde_json::Value", repr)
}

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

        let constructor = SerializedConstructor::builder()
            .id(vec![
                "langchain".to_string(),
                "llms".to_string(),
                "OpenAI".to_string(),
            ])
            .kwargs(kwargs)
            .build();

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
        let not_impl = SerializedNotImplemented::builder()
            .id(vec!["my_module".to_string(), "MyClass".to_string()])
            .repr("MyClass(...)")
            .build();

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
