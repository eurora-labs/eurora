use std::collections::HashMap;

use bon::bon;
use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::string::{PromptTemplateFormat, format_template, get_template_variables};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictPromptTemplate {
    pub template: serde_json::Value,

    #[serde(default)]
    pub template_format: PromptTemplateFormat,
}

#[bon]
impl DictPromptTemplate {
    #[builder]
    pub fn new(
        template: serde_json::Value,
        #[builder(default)] template_format: PromptTemplateFormat,
    ) -> Self {
        Self {
            template,
            template_format,
        }
    }

    pub fn from_dict(template: serde_json::Value) -> Self {
        Self::builder().template(template).build()
    }

    pub fn input_variables(&self) -> Vec<String> {
        get_input_variables(&self.template, self.template_format)
    }

    pub fn format(&self, kwargs: &HashMap<String, String>) -> Result<serde_json::Value> {
        insert_input_variables(&self.template, kwargs, self.template_format)
    }

    pub async fn aformat(&self, kwargs: &HashMap<String, String>) -> Result<serde_json::Value> {
        self.format(kwargs)
    }

    pub fn prompt_type(&self) -> &str {
        "dict-prompt"
    }

    pub fn pretty_repr(&self, _html: bool) -> String {
        format!("DictPromptTemplate({:?})", self.template)
    }
}

fn get_input_variables(
    template: &serde_json::Value,
    template_format: PromptTemplateFormat,
) -> Vec<String> {
    let mut input_variables = Vec::new();

    match template {
        serde_json::Value::String(s) => {
            if let Ok(vars) = get_template_variables(s, template_format) {
                input_variables.extend(vars);
            }
        }
        serde_json::Value::Object(map) => {
            for value in map.values() {
                input_variables.extend(get_input_variables(value, template_format));
            }
        }
        serde_json::Value::Array(arr) => {
            for value in arr {
                input_variables.extend(get_input_variables(value, template_format));
            }
        }
        _ => {}
    }

    let mut seen = std::collections::HashSet::new();
    input_variables.retain(|v| seen.insert(v.clone()));

    input_variables
}

fn insert_input_variables(
    template: &serde_json::Value,
    inputs: &HashMap<String, String>,
    template_format: PromptTemplateFormat,
) -> Result<serde_json::Value> {
    match template {
        serde_json::Value::String(s) => {
            let formatted = format_template(s, template_format, inputs)?;
            Ok(serde_json::Value::String(formatted))
        }
        serde_json::Value::Object(map) => {
            let mut formatted = serde_json::Map::new();

            for (key, value) in map {
                if key == "image_url"
                    && let serde_json::Value::Object(inner) = value
                    && inner.contains_key("path")
                {
                    tracing::warn!(
                        target: "agent_chain_core::prompts",
                        "Specifying image inputs via file path in environments \
                         with user-input paths is a security vulnerability."
                    );
                }

                let formatted_value = insert_input_variables(value, inputs, template_format)?;
                formatted.insert(key.clone(), formatted_value);
            }

            Ok(serde_json::Value::Object(formatted))
        }
        serde_json::Value::Array(arr) => {
            let mut formatted = Vec::new();

            for value in arr {
                formatted.push(insert_input_variables(value, inputs, template_format)?);
            }

            Ok(serde_json::Value::Array(formatted))
        }
        _ => Ok(template.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_input_variables() {
        let template = serde_json::json!({
            "name": "{user_name}",
            "details": {
                "age": "{user_age}",
                "city": "{user_city}"
            }
        });

        let vars = get_input_variables(&template, PromptTemplateFormat::FString);
        assert!(vars.contains(&"user_name".to_string()));
        assert!(vars.contains(&"user_age".to_string()));
        assert!(vars.contains(&"user_city".to_string()));
    }

    #[test]
    fn test_format() {
        let template = serde_json::json!({
            "greeting": "Hello, {name}!",
            "info": {
                "age": "{age} years old"
            }
        });

        let dict_template = DictPromptTemplate::from_dict(template);

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "Alice".to_string());
        kwargs.insert("age".to_string(), "30".to_string());

        let result = dict_template.format(&kwargs).unwrap();

        assert_eq!(result["greeting"], "Hello, Alice!");
        assert_eq!(result["info"]["age"], "30 years old");
    }

    #[test]
    fn test_format_array() {
        let template = serde_json::json!({
            "items": ["{item1}", "{item2}"]
        });

        let dict_template = DictPromptTemplate::from_dict(template);

        let mut kwargs = HashMap::new();
        kwargs.insert("item1".to_string(), "first".to_string());
        kwargs.insert("item2".to_string(), "second".to_string());

        let result = dict_template.format(&kwargs).unwrap();

        assert_eq!(result["items"][0], "first");
        assert_eq!(result["items"][1], "second");
    }

    #[test]
    fn test_non_string_values() {
        let template = serde_json::json!({
            "name": "{user_name}",
            "count": 42,
            "active": true
        });

        let dict_template = DictPromptTemplate::from_dict(template);

        let mut kwargs = HashMap::new();
        kwargs.insert("user_name".to_string(), "Bob".to_string());

        let result = dict_template.format(&kwargs).unwrap();

        assert_eq!(result["name"], "Bob");
        assert_eq!(result["count"], 42);
        assert_eq!(result["active"], true);
    }

    #[test]
    fn test_mustache_format() {
        let template = serde_json::json!({
            "greeting": "Hello, {{name}}!"
        });

        let dict_template = DictPromptTemplate::builder()
            .template(template)
            .template_format(PromptTemplateFormat::Mustache)
            .build();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result = dict_template.format(&kwargs).unwrap();
        assert_eq!(result["greeting"], "Hello, World!");
    }
}
