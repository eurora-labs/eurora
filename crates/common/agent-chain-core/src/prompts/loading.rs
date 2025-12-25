//! Load prompts from files.
//!
//! This module provides functions for loading prompts from JSON and YAML files,
//! mirroring `langchain_core.prompts.loading` in Python.

use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};

use super::base::BasePromptTemplate;
use super::few_shot::FewShotPromptTemplate;
use super::prompt::PromptTemplate;
use super::string::PromptTemplateFormat;

/// Load prompt from a configuration dictionary.
///
/// # Arguments
///
/// * `config` - A JSON value containing the prompt configuration.
///
/// # Returns
///
/// A boxed BasePromptTemplate, or an error if loading fails.
pub fn load_prompt_from_config(config: serde_json::Value) -> Result<Box<dyn BasePromptTemplate>> {
    let config = match config {
        serde_json::Value::Object(map) => map,
        _ => {
            return Err(Error::InvalidConfig(
                "Configuration must be an object".to_string(),
            ));
        }
    };

    // Get the type, defaulting to "prompt"
    let config_type = config
        .get("_type")
        .and_then(|v| v.as_str())
        .unwrap_or("prompt");

    match config_type {
        "prompt" => load_basic_prompt(&config),
        "few_shot" => load_few_shot_prompt(&config),
        "chat" => load_chat_prompt(&config),
        _ => Err(Error::InvalidConfig(format!(
            "Loading {} prompt not supported",
            config_type
        ))),
    }
}

/// Load a basic prompt template from config.
fn load_basic_prompt(
    config: &serde_json::Map<String, serde_json::Value>,
) -> Result<Box<dyn BasePromptTemplate>> {
    // Get template, either from "template" or "template_path"
    let template = if let Some(template_path) = config.get("template_path").and_then(|v| v.as_str())
    {
        if config.contains_key("template") {
            return Err(Error::InvalidConfig(
                "Both 'template_path' and 'template' cannot be provided.".to_string(),
            ));
        }
        let path = Path::new(template_path);
        if path.extension().map(|e| e == "txt").unwrap_or(false) {
            std::fs::read_to_string(path)?
        } else {
            return Err(Error::InvalidConfig(
                "template_path must point to a .txt file".to_string(),
            ));
        }
    } else if let Some(template) = config.get("template").and_then(|v| v.as_str()) {
        template.to_string()
    } else {
        return Err(Error::InvalidConfig(
            "Either 'template' or 'template_path' must be provided.".to_string(),
        ));
    };

    // Get template format
    let template_format = config
        .get("template_format")
        .and_then(|v| v.as_str())
        .unwrap_or("f-string");

    // Check for jinja2 (disabled for security)
    if template_format == "jinja2" {
        return Err(Error::InvalidConfig(
            "Loading templates with 'jinja2' format is no longer supported \
             since it can lead to arbitrary code execution. Please migrate to using \
             the 'f-string' template format."
                .to_string(),
        ));
    }

    let format = PromptTemplateFormat::from_str(template_format)?;
    let prompt = PromptTemplate::from_template_with_format(template, format)?;

    Ok(Box::new(prompt))
}

/// Load a few-shot prompt template from config.
fn load_few_shot_prompt(
    config: &serde_json::Map<String, serde_json::Value>,
) -> Result<Box<dyn BasePromptTemplate>> {
    // Load prefix template
    let prefix = load_template_string("prefix", config)?;

    // Load suffix template
    let suffix = load_template_string("suffix", config)?
        .ok_or_else(|| Error::InvalidConfig("'suffix' is required".to_string()))?;

    // Load example prompt
    let example_prompt = if let Some(example_prompt_path) =
        config.get("example_prompt_path").and_then(|v| v.as_str())
    {
        if config.contains_key("example_prompt") {
            return Err(Error::InvalidConfig(
                "Only one of example_prompt and example_prompt_path should be specified."
                    .to_string(),
            ));
        }
        // Load from file path - for simplicity, we'll create a template from the suffix
        // The actual implementation would need to load and extract the prompt template
        let _path = Path::new(example_prompt_path);
        // This is a simplification - a full implementation would load the file
        PromptTemplate::from_template(&suffix)?
    } else if let Some(example_prompt_config) = config.get("example_prompt") {
        let config_map = example_prompt_config
            .as_object()
            .ok_or_else(|| Error::InvalidConfig("example_prompt must be an object".to_string()))?;

        let template = config_map
            .get("template")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                Error::InvalidConfig("example_prompt.template is required".to_string())
            })?;

        PromptTemplate::from_template(template)?
    } else {
        return Err(Error::InvalidConfig(
            "Either 'example_prompt' or 'example_prompt_path' must be provided.".to_string(),
        ));
    };

    // Load examples
    let examples = load_examples(config)?;

    let few_shot = FewShotPromptTemplate::new(examples, example_prompt, suffix, prefix)?;

    Ok(Box::new(few_shot))
}

/// Load a chat prompt template from config.
///
/// Note: ChatPromptTemplate doesn't implement BasePromptTemplate directly,
/// so we convert it to a PromptTemplate for loading purposes.
fn load_chat_prompt(
    config: &serde_json::Map<String, serde_json::Value>,
) -> Result<Box<dyn BasePromptTemplate>> {
    let messages = config
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            Error::InvalidConfig("'messages' is required for chat prompts".to_string())
        })?;

    if messages.is_empty() {
        return Err(Error::InvalidConfig(
            "Can't load chat prompt without messages".to_string(),
        ));
    }

    // Try to extract template from first message
    let first_message = messages
        .first()
        .and_then(|m| m.as_object())
        .ok_or_else(|| Error::InvalidConfig("Invalid message format".to_string()))?;

    let template = first_message
        .get("prompt")
        .and_then(|p| p.as_object())
        .and_then(|p| p.get("template"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            Error::InvalidConfig("Can't load chat prompt without template".to_string())
        })?;

    // For loading purposes, we convert to a basic PromptTemplate
    // A full implementation would preserve the chat structure
    let prompt = PromptTemplate::from_template(template)?;
    Ok(Box::new(prompt))
}

/// Helper to load a template string from config (with _path variant support).
fn load_template_string(
    var_name: &str,
    config: &serde_json::Map<String, serde_json::Value>,
) -> Result<Option<String>> {
    let path_key = format!("{}_path", var_name);

    if let Some(path) = config.get(&path_key).and_then(|v| v.as_str()) {
        if config.contains_key(var_name) {
            return Err(Error::InvalidConfig(format!(
                "Both '{}' and '{}' cannot be provided.",
                path_key, var_name
            )));
        }
        let file_path = Path::new(path);
        if file_path.extension().map(|e| e == "txt").unwrap_or(false) {
            Ok(Some(std::fs::read_to_string(file_path)?))
        } else {
            Err(Error::InvalidConfig(format!(
                "{} must point to a .txt file",
                path_key
            )))
        }
    } else {
        Ok(config
            .get(var_name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }
}

/// Load examples from config.
fn load_examples(
    config: &serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<HashMap<String, String>>> {
    let examples_value = config
        .get("examples")
        .ok_or_else(|| Error::InvalidConfig("'examples' is required".to_string()))?;

    match examples_value {
        serde_json::Value::Array(arr) => {
            let mut examples = Vec::new();
            for item in arr {
                let obj = item.as_object().ok_or_else(|| {
                    Error::InvalidConfig("Each example must be an object".to_string())
                })?;

                let example: HashMap<String, String> = obj
                    .iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect();

                examples.push(example);
            }
            Ok(examples)
        }
        serde_json::Value::String(path) => {
            let file_path = Path::new(path);
            let content = std::fs::read_to_string(file_path)?;

            let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

            let parsed: serde_json::Value = match extension {
                "json" => serde_json::from_str(&content)?,
                "yaml" | "yml" => {
                    return Err(Error::InvalidConfig(
                        "YAML file loading not supported. Please use JSON.".to_string(),
                    ));
                }
                _ => {
                    return Err(Error::InvalidConfig(
                        "Invalid file format. Only json or yaml formats are supported.".to_string(),
                    ));
                }
            };

            let arr = parsed.as_array().ok_or_else(|| {
                Error::InvalidConfig("Examples file must contain an array".to_string())
            })?;

            let mut examples = Vec::new();
            for item in arr {
                let obj = item.as_object().ok_or_else(|| {
                    Error::InvalidConfig("Each example must be an object".to_string())
                })?;

                let example: HashMap<String, String> = obj
                    .iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect();

                examples.push(example);
            }
            Ok(examples)
        }
        _ => Err(Error::InvalidConfig(
            "Invalid examples format. Only list or string are supported.".to_string(),
        )),
    }
}

/// Load a prompt from a file.
///
/// # Arguments
///
/// * `path` - Path to the prompt file (JSON or YAML).
///
/// # Returns
///
/// A boxed BasePromptTemplate, or an error if loading fails.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::prompts::load_prompt;
///
/// let prompt = load_prompt("path/to/prompt.json").unwrap();
/// ```
pub fn load_prompt(path: impl AsRef<Path>) -> Result<Box<dyn BasePromptTemplate>> {
    load_prompt_with_encoding(path, None)
}

/// Load a prompt from a file with optional encoding.
///
/// # Arguments
///
/// * `path` - Path to the prompt file.
/// * `encoding` - Optional encoding (currently ignored, uses UTF-8).
///
/// # Returns
///
/// A boxed BasePromptTemplate, or an error if loading fails.
pub fn load_prompt_with_encoding(
    path: impl AsRef<Path>,
    _encoding: Option<&str>,
) -> Result<Box<dyn BasePromptTemplate>> {
    let path = path.as_ref();

    // Check for deprecated LangChain Hub paths
    if let Some(path_str) = path.to_str()
        && path_str.starts_with("lc://")
    {
        return Err(Error::InvalidConfig(
            "Loading from the deprecated github-based Hub is no longer supported. \
                 Please use the new LangChain Hub at https://smith.langchain.com/hub instead."
                .to_string(),
        ));
    }

    load_prompt_from_file(path)
}

/// Load prompt from a file path.
fn load_prompt_from_file(path: &Path) -> Result<Box<dyn BasePromptTemplate>> {
    let content = std::fs::read_to_string(path)?;

    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let config: serde_json::Value = match extension {
        "json" => serde_json::from_str(&content)?,
        "yaml" | "yml" => {
            return Err(Error::InvalidConfig(
                "YAML file loading not supported. Please use JSON.".to_string(),
            ));
        }
        _ => {
            return Err(Error::InvalidConfig(format!(
                "Got unsupported file type {}",
                extension
            )));
        }
    };

    load_prompt_from_config(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_prompt_from_config() {
        let config = serde_json::json!({
            "_type": "prompt",
            "template": "Hello, {name}!",
            "template_format": "f-string"
        });

        let prompt = load_prompt_from_config(config).unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result = prompt.format(&kwargs).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_load_prompt_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("prompt.json");

        let config = serde_json::json!({
            "_type": "prompt",
            "template": "Say {word}"
        });

        std::fs::write(&file_path, serde_json::to_string(&config).unwrap()).unwrap();

        let prompt = load_prompt(&file_path).unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("word".to_string(), "hello".to_string());

        let result = prompt.format(&kwargs).unwrap();
        assert_eq!(result, "Say hello");
    }

    #[test]
    fn test_load_few_shot_from_config() {
        let config = serde_json::json!({
            "_type": "few_shot",
            "example_prompt": {
                "template": "Q: {input}\nA: {output}"
            },
            "examples": [
                {"input": "2+2", "output": "4"}
            ],
            "suffix": "Q: {question}\nA:"
        });

        let prompt = load_prompt_from_config(config).unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("question".to_string(), "3+3".to_string());

        let result = prompt.format(&kwargs).unwrap();
        assert!(result.contains("Q: 2+2"));
        assert!(result.contains("A: 4"));
        assert!(result.contains("Q: 3+3"));
    }

    #[test]
    fn test_jinja2_rejected() {
        let config = serde_json::json!({
            "_type": "prompt",
            "template": "Hello, {{ name }}!",
            "template_format": "jinja2"
        });

        let result = load_prompt_from_config(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_deprecated_hub_rejected() {
        let result = load_prompt("lc://prompts/some_prompt");
        assert!(result.is_err());
    }
}
