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
    pub id: Vec<String>,
    pub kwargs: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

impl SerializedConstructor {
    pub fn new(id: Vec<String>, kwargs: HashMap<String, Value>) -> Self {
        Self {
            lc: LC_VERSION,
            id,
            kwargs,
            name: None,
            graph: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SerializedSecret {
    pub lc: i32,
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
    pub id: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph: Option<HashMap<String, Value>>,
}

impl SerializedNotImplemented {
    pub fn new(id: Vec<String>, repr: Option<String>) -> Self {
        Self {
            lc: LC_VERSION,
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
    Constructor(SerializedConstructor),
    #[serde(rename = "secret")]
    Secret(SerializedSecret),
    #[serde(rename = "not_implemented")]
    NotImplemented(SerializedNotImplemented),
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
            Ok(Value::Object(map)) => map.into_iter().filter(|(_, v)| !v.is_null()).collect(),
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

        Serialized::Constructor(SerializedConstructor::new(Self::lc_id(), final_kwargs))
    }

    fn to_json_not_implemented(&self) -> Serialized {
        to_json_not_implemented_value(self.lc_type_name(), None)
    }
}

pub fn to_json_not_implemented_value(type_name: &str, repr: Option<String>) -> Serialized {
    let id: Vec<String> = type_name.split("::").map(|s| s.to_string()).collect();
    Serialized::NotImplemented(SerializedNotImplemented::new(id, repr))
}

pub fn to_json_not_implemented(value: &Value) -> Serialized {
    let repr = serde_json::to_string_pretty(value).ok();
    to_json_not_implemented_value("serde_json::Value", repr)
}

fn secret_value(secret_id: &str) -> Value {
    serde_json::to_value(Serialized::Secret(SerializedSecret::from_secret_id(
        secret_id,
    )))
    .expect("Serialized serialization cannot fail")
}

fn replace_secrets(
    mut kwargs: HashMap<String, Value>,
    secrets_map: &HashMap<String, String>,
) -> HashMap<String, Value> {
    for (path, secret_id) in secrets_map {
        let parts: Vec<&str> = path.split('.').collect();

        if parts.len() == 1 {
            if kwargs.contains_key(path) {
                kwargs.insert(path.clone(), secret_value(secret_id));
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
            current.insert(key.to_string(), secret_value(secret_id));
        }
    } else if let Some(Value::Object(map)) = current.get_mut(key) {
        replace_nested_secret_in_map(map, &parts[1..], secret_id);
    }
}

fn replace_nested_secret_in_map(
    map: &mut serde_json::Map<String, Value>,
    parts: &[&str],
    secret_id: &str,
) {
    if parts.is_empty() {
        return;
    }

    let key = parts[0];

    if parts.len() == 1 {
        if map.contains_key(key) {
            map.insert(key.to_string(), secret_value(secret_id));
        }
    } else if let Some(Value::Object(nested)) = map.get_mut(key) {
        replace_nested_secret_in_map(nested, &parts[1..], secret_id);
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
        assert_eq!(constructor.id.len(), 3);
    }

    #[test]
    fn test_serialized_secret() {
        let secret = SerializedSecret::from_secret_id("OPENAI_API_KEY");

        assert_eq!(secret.lc, 1);
        assert_eq!(secret.id, vec!["OPENAI_API_KEY".to_string()]);
    }

    #[test]
    fn test_serialized_not_implemented() {
        let not_impl = SerializedNotImplemented::new(
            vec!["my_module".to_string(), "MyClass".to_string()],
            Some("MyClass(...)".to_string()),
        );

        assert_eq!(not_impl.lc, 1);
        assert_eq!(not_impl.repr, Some("MyClass(...)".to_string()));
    }

    #[test]
    fn test_serialized_roundtrip() {
        let constructor = Serialized::Constructor(SerializedConstructor::new(
            vec!["test".into()],
            HashMap::new(),
        ));
        let json = serde_json::to_value(&constructor).unwrap();
        assert_eq!(
            json.get("type").and_then(|v| v.as_str()),
            Some("constructor")
        );

        let secret = Serialized::Secret(SerializedSecret::from_secret_id("KEY"));
        let json = serde_json::to_value(&secret).unwrap();
        assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("secret"));

        let not_impl =
            Serialized::NotImplemented(SerializedNotImplemented::new(vec!["test".into()], None));
        let json = serde_json::to_value(&not_impl).unwrap();
        assert_eq!(
            json.get("type").and_then(|v| v.as_str()),
            Some("not_implemented")
        );
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
