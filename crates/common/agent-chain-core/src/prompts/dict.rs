//! Dict prompt template.
//!
//! This module provides dict prompt templates that format to dictionaries,
//! mirroring `langchain_core.prompts.dict` in Python.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::Result;

use super::string::{PromptTemplateFormat, format_template, get_template_variables};

/// Template represented by a dict.
///
/// Recognizes variables in f-string or mustache formatted string dict values.
/// Does NOT recognize variables in dict keys. Applies recursively.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::prompts::DictPromptTemplate;
/// use std::collections::HashMap;
///
/// let mut template = HashMap::new();
/// template.insert("name".to_string(), serde_json::json!("{user_name}"));
/// template.insert("age".to_string(), serde_json::json!("{user_age}"));
///
/// let dict_template = DictPromptTemplate::new(template, PromptTemplateFormat::FString);
///
/// let mut kwargs = HashMap::new();
/// kwargs.insert("user_name".to_string(), "Alice".to_string());
/// kwargs.insert("user_age".to_string(), "30".to_string());
///
/// let result = dict_template.format(&kwargs).unwrap();
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictPromptTemplate {
    /// The template dictionary.
    pub template: serde_json::Value,

    /// The format of the template.
    #[serde(default)]
    pub template_format: PromptTemplateFormat,
}

impl DictPromptTemplate {
    /// Create a new DictPromptTemplate.
    pub fn new(template: serde_json::Value, template_format: PromptTemplateFormat) -> Self {
        Self {
            template,
            template_format,
        }
    }

    /// Create a new DictPromptTemplate with f-string format.
    pub fn from_dict(template: serde_json::Value) -> Self {
        Self::new(template, PromptTemplateFormat::FString)
    }

    /// Get the input variables for this template.
    pub fn input_variables(&self) -> Vec<String> {
        get_input_variables(&self.template, self.template_format)
    }

    /// Format the template with the inputs.
    pub fn format(&self, kwargs: &HashMap<String, String>) -> Result<serde_json::Value> {
        insert_input_variables(&self.template, kwargs, self.template_format)
    }

    /// Async format the template with the inputs.
    pub async fn aformat(&self, kwargs: &HashMap<String, String>) -> Result<serde_json::Value> {
        self.format(kwargs)
    }

    /// Get the prompt type.
    pub fn prompt_type(&self) -> &str {
        "dict-prompt"
    }

    /// Get a pretty representation.
    pub fn pretty_repr(&self, _html: bool) -> String {
        format!("DictPromptTemplate({:?})", self.template)
    }
}

/// Get input variables from a template value recursively.
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

    // Remove duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    input_variables.retain(|v| seen.insert(v.clone()));

    input_variables
}

/// Insert input variables into a template value recursively.
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
                // Security warning for image_url paths
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
        // For non-string primitives, return as-is
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

        let dict_template = DictPromptTemplate::new(template, PromptTemplateFormat::Mustache);

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result = dict_template.format(&kwargs).unwrap();
        assert_eq!(result["greeting"], "Hello, World!");
    }
}
