use std::collections::HashMap;
use std::path::Path;

use tracing::warn;

use crate::error::{Error, Result};

use super::base::BasePromptTemplate;
use super::chat::ChatPromptTemplate;
use super::few_shot::FewShotPromptTemplate;
use super::prompt::PromptTemplate;
use super::string::PromptTemplateFormat;

type LoaderFn =
    fn(&mut serde_json::Map<String, serde_json::Value>) -> Result<Box<dyn BasePromptTemplate>>;

fn get_type_to_loader() -> HashMap<&'static str, LoaderFn> {
    let mut map: HashMap<&str, LoaderFn> = HashMap::new();
    map.insert("prompt", load_basic_prompt);
    map.insert("few_shot", load_few_shot_prompt);
    map.insert("chat", load_chat_prompt);
    map
}

pub fn load_prompt_from_config(config: serde_json::Value) -> Result<Box<dyn BasePromptTemplate>> {
    let mut config = match config {
        serde_json::Value::Object(map) => map,
        _ => {
            return Err(Error::InvalidConfig(
                "Configuration must be an object".to_string(),
            ));
        }
    };

    if !config.contains_key("_type") {
        warn!("No `_type` key found, defaulting to `prompt`.");
    }

    let config_type = config
        .remove("_type")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "prompt".to_string());

    let loaders = get_type_to_loader();
    let loader = loaders.get(config_type.as_str()).ok_or_else(|| {
        Error::InvalidConfig(format!("Loading {} prompt not supported", config_type))
    })?;

    loader(&mut config)
}

fn load_template(var_name: &str, config: &mut serde_json::Map<String, serde_json::Value>) {
    let path_key = format!("{}_path", var_name);

    if let Some(path_value) = config.remove(&path_key) {
        if config.contains_key(var_name) {
            warn!("Both '{}' and '{}' cannot be provided.", path_key, var_name);
            return;
        }

        if let Some(path_str) = path_value.as_str() {
            let path = Path::new(path_str);
            if path.extension().and_then(|e| e.to_str()) == Some("txt")
                && let Ok(content) = std::fs::read_to_string(path)
            {
                config.insert(var_name.to_string(), serde_json::Value::String(content));
            }
        }
    }
}

fn load_examples_from_config(
    config: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<()> {
    let examples = config
        .get("examples")
        .ok_or_else(|| Error::InvalidConfig("'examples' is required".to_string()))?;

    match examples {
        serde_json::Value::Array(_) => Ok(()),
        serde_json::Value::String(path_str) => {
            let path = Path::new(path_str);
            let content = std::fs::read_to_string(path)?;

            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
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

            config.insert("examples".to_string(), parsed);
            Ok(())
        }
        _ => Err(Error::InvalidConfig(
            "Invalid examples format. Only list or string are supported.".to_string(),
        )),
    }
}

fn extract_examples(
    config: &serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<HashMap<String, String>>> {
    let examples_value = config
        .get("examples")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::InvalidConfig("'examples' must be an array".to_string()))?;

    let mut examples = Vec::new();
    for item in examples_value {
        let obj = item
            .as_object()
            .ok_or_else(|| Error::InvalidConfig("Each example must be an object".to_string()))?;

        let example: HashMap<String, String> = obj
            .iter()
            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
            .collect();

        examples.push(example);
    }
    Ok(examples)
}

fn load_basic_prompt(
    config: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<Box<dyn BasePromptTemplate>> {
    load_template("template", config);

    let template = config
        .get("template")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::InvalidConfig(
                "Either 'template' or 'template_path' must be provided.".to_string(),
            )
        })?
        .to_string();

    let template_format = config
        .get("template_format")
        .and_then(|v| v.as_str())
        .unwrap_or("f-string");

    if template_format == "jinja2" {
        return Err(Error::InvalidConfig(format!(
            "Loading templates with '{}' format is no longer supported \
             since it can lead to arbitrary code execution. Please migrate to using \
             the 'f-string' template format, which does not suffer from this issue.",
            template_format
        )));
    }

    let format: PromptTemplateFormat = template_format.parse()?;
    let prompt = PromptTemplate::from_template_with_format(template, format)?;

    Ok(Box::new(prompt))
}

fn load_few_shot_prompt(
    config: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<Box<dyn BasePromptTemplate>> {
    load_template("suffix", config);
    load_template("prefix", config);

    let example_prompt = if let Some(example_prompt_path) = config.remove("example_prompt_path") {
        if config.contains_key("example_prompt") {
            return Err(Error::InvalidConfig(
                "Only one of example_prompt and example_prompt_path should be specified."
                    .to_string(),
            ));
        }
        let path_str = example_prompt_path.as_str().ok_or_else(|| {
            Error::InvalidConfig("example_prompt_path must be a string".to_string())
        })?;
        let loaded = load_prompt(path_str)?;
        let dict = loaded.to_dict();
        let template_str = dict
            .get("template")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                Error::InvalidConfig("Loaded example_prompt must have a template".to_string())
            })?;
        PromptTemplate::from_template(template_str)?
    } else if let Some(example_prompt_config) = config.remove("example_prompt") {
        let loaded = load_prompt_from_config(example_prompt_config)?;
        let dict = loaded.to_dict();
        let template_str = dict
            .get("template")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                Error::InvalidConfig("example_prompt must have a template".to_string())
            })?;
        PromptTemplate::from_template(template_str)?
    } else {
        return Err(Error::InvalidConfig(
            "Either 'example_prompt' or 'example_prompt_path' must be provided.".to_string(),
        ));
    };

    load_examples_from_config(config)?;
    let examples = extract_examples(config)?;

    let suffix = config
        .get("suffix")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let prefix = config
        .get("prefix")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let few_shot = FewShotPromptTemplate::new(examples, example_prompt, suffix, prefix)?;

    Ok(Box::new(few_shot))
}

fn load_chat_prompt(
    config: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<Box<dyn BasePromptTemplate>> {
    let messages = config
        .remove("messages")
        .and_then(|v| match v {
            serde_json::Value::Array(arr) => Some(arr),
            _ => None,
        })
        .ok_or_else(|| {
            Error::InvalidConfig("'messages' is required for chat prompts".to_string())
        })?;

    let template = messages
        .first()
        .and_then(|m| m.as_object())
        .and_then(|m| m.get("prompt"))
        .and_then(|p| p.as_object())
        .and_then(|p| p.get("template"))
        .and_then(|t| t.as_str())
        .ok_or_else(|| {
            Error::InvalidConfig("Can't load chat prompt without template".to_string())
        })?;

    let chat_prompt = ChatPromptTemplate::from_template(template)?;
    Ok(Box::new(chat_prompt))
}

pub fn load_prompt(path: impl AsRef<Path>) -> Result<Box<dyn BasePromptTemplate>> {
    load_prompt_with_encoding(path, None)
}

pub fn load_prompt_with_encoding(
    path: impl AsRef<Path>,
    _encoding: Option<&str>,
) -> Result<Box<dyn BasePromptTemplate>> {
    let path = path.as_ref();

    if let Some(path_str) = path.to_str()
        && path_str.starts_with("lc://")
    {
        return Err(Error::Other(
            "Loading from the deprecated github-based Hub is no longer supported. \
                 Please use the new LangChain Hub at https://smith.langchain.com/hub instead."
                .to_string(),
        ));
    }

    load_prompt_from_file(path)
}

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
    fn test_load_prompt_default_type() {
        let config = serde_json::json!({
            "template": "Hello, {name}!"
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
    fn test_load_chat_prompt_from_config() {
        let config = serde_json::json!({
            "_type": "chat",
            "messages": [
                {
                    "prompt": {
                        "template": "Hello, {name}!"
                    }
                }
            ],
            "input_variables": ["name"]
        });

        let prompt = load_prompt_from_config(config).unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result = prompt.format(&kwargs).unwrap();
        assert!(result.contains("Hello, World!"));
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

    #[test]
    fn test_unsupported_type() {
        let config = serde_json::json!({
            "_type": "unknown_type",
            "template": "test"
        });

        let result = load_prompt_from_config(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_prompt_with_template_path() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("template.txt");
        std::fs::write(&template_path, "Hello, {name}!").unwrap();

        let config = serde_json::json!({
            "_type": "prompt",
            "template_path": template_path.to_str().unwrap()
        });

        let prompt = load_prompt_from_config(config).unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), "World".to_string());

        let result = prompt.format(&kwargs).unwrap();
        assert_eq!(result, "Hello, World!");
    }
}
