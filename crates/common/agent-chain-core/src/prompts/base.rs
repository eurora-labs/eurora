use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::prompt_values::{PromptValue, StringPromptValue};

pub type FormatOutputType = String;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromptTemplateConfig {
    pub input_variables: Vec<String>,

    #[serde(default)]
    pub optional_variables: Vec<String>,

    #[serde(default)]
    pub input_types: HashMap<String, String>,

    #[serde(default)]
    pub partial_variables: HashMap<String, String>,

    #[serde(default)]
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

pub trait BasePromptTemplate: Send + Sync {
    fn input_variables(&self) -> &[String];

    fn optional_variables(&self) -> &[String] {
        &[]
    }

    fn input_types(&self) -> &HashMap<String, String> {
        static EMPTY: std::sync::LazyLock<HashMap<String, String>> =
            std::sync::LazyLock::new(HashMap::new);
        &EMPTY
    }

    fn partial_variables(&self) -> &HashMap<String, String> {
        static EMPTY: std::sync::LazyLock<HashMap<String, String>> =
            std::sync::LazyLock::new(HashMap::new);
        &EMPTY
    }

    fn metadata(&self) -> Option<&HashMap<String, serde_json::Value>> {
        None
    }

    fn tags(&self) -> Option<&[String]> {
        None
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<FormatOutputType>;

    fn aformat(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FormatOutputType>> + Send + '_>>
    {
        let result = self.format(kwargs);
        Box::pin(async move { result })
    }

    fn format_prompt(&self, kwargs: &HashMap<String, String>) -> Result<Box<dyn PromptValue>> {
        let text = self.format(kwargs)?;
        Ok(Box::new(StringPromptValue::new(text)))
    }

    fn aformat_prompt(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Box<dyn PromptValue>>> + Send + '_>,
    > {
        let result = self.format_prompt(kwargs);
        Box::pin(async move { result })
    }

    fn partial(&self, kwargs: HashMap<String, String>) -> Result<Box<dyn BasePromptTemplate>>;

    fn prompt_type(&self) -> &str;

    fn validate_variable_names(&self) -> Result<()> {
        if self.input_variables().contains(&"stop".to_string()) {
            return Err(Error::InvalidConfig(
                "Cannot have an input variable named 'stop', as it is used internally. \
                 Please rename."
                    .to_string(),
            ));
        }

        if self.partial_variables().contains_key("stop") {
            return Err(Error::InvalidConfig(
                "Cannot have a partial variable named 'stop', as it is used internally. \
                 Please rename."
                    .to_string(),
            ));
        }

        let input_set: std::collections::HashSet<_> =
            self.input_variables().iter().cloned().collect();
        let partial_set: std::collections::HashSet<_> =
            self.partial_variables().keys().cloned().collect();

        let overlap: Vec<_> = input_set.intersection(&partial_set).collect();
        if !overlap.is_empty() {
            return Err(Error::InvalidConfig(format!(
                "Found overlapping input and partial variables: {:?}",
                overlap
            )));
        }

        Ok(())
    }

    fn validate_input(&self, inner_input: &HashMap<String, String>) -> Result<()> {
        let input_vars: std::collections::HashSet<_> =
            self.input_variables().iter().cloned().collect();
        let provided: std::collections::HashSet<_> = inner_input.keys().cloned().collect();

        let missing: Vec<_> = input_vars.difference(&provided).collect();
        if !missing.is_empty() {
            let example_key = missing[0];
            return Err(Error::InvalidConfig(format!(
                "Input is missing variables {:?}. Expected: {:?}, Received: {:?}\n\
                 Note: if you intended {{{}}} to be part of the string and not a variable, \
                 please escape it with double curly braces like: '{{{{{}}}}}'.",
                missing,
                self.input_variables(),
                inner_input.keys().collect::<Vec<_>>(),
                example_key,
                example_key
            )));
        }

        Ok(())
    }

    fn merge_partial_and_user_variables(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut merged = self.partial_variables().clone();
        merged.extend(kwargs.clone());
        merged
    }

    fn to_dict(&self) -> serde_json::Value;

    fn save(&self, file_path: &Path) -> Result<()> {
        if !self.partial_variables().is_empty() {
            return Err(Error::InvalidConfig(
                "Cannot save prompt with partial variables.".to_string(),
            ));
        }

        let prompt_dict = self.to_dict();

        if prompt_dict.get("_type").is_none() {
            return Err(Error::InvalidConfig(
                "Prompt does not support saving.".to_string(),
            ));
        }

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let extension = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match extension {
            "json" => {
                let json = serde_json::to_string_pretty(&prompt_dict)?;
                std::fs::write(file_path, json)?;
            }
            "yaml" | "yml" => {
                return Err(Error::InvalidConfig(
                    "YAML serialization not supported. Please use JSON.".to_string(),
                ));
            }
            _ => {
                return Err(Error::InvalidConfig(format!(
                    "{} must be json or yaml",
                    file_path.display()
                )));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub page_content: String,

    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl Document {
    pub fn new(page_content: impl Into<String>) -> Self {
        Self {
            page_content: page_content.into(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(
        page_content: impl Into<String>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            page_content: page_content.into(),
            metadata,
        }
    }
}

fn get_document_info(
    doc: &Document,
    input_variables: &[String],
) -> Result<HashMap<String, String>> {
    let mut base_info = HashMap::new();
    base_info.insert("page_content".to_string(), doc.page_content.clone());

    for (key, value) in &doc.metadata {
        base_info.insert(key.clone(), value.clone());
    }

    let base_keys: std::collections::HashSet<_> = base_info.keys().cloned().collect();
    let required: std::collections::HashSet<_> = input_variables.iter().cloned().collect();

    let missing: Vec<_> = required.difference(&base_keys).collect();
    if !missing.is_empty() {
        let required_metadata: Vec<_> = input_variables
            .iter()
            .filter(|iv| *iv != "page_content")
            .collect();
        return Err(Error::InvalidConfig(format!(
            "Document prompt requires documents to have metadata variables: {:?}. \
             Received document with missing metadata: {:?}.",
            required_metadata, missing
        )));
    }

    let result: HashMap<_, _> = input_variables
        .iter()
        .filter_map(|k| base_info.get(k).map(|v| (k.clone(), v.clone())))
        .collect();

    Ok(result)
}

pub fn format_document(doc: &Document, prompt: &dyn BasePromptTemplate) -> Result<String> {
    let info = get_document_info(doc, prompt.input_variables())?;
    prompt.format(&info)
}

pub async fn aformat_document(doc: &Document, prompt: &dyn BasePromptTemplate) -> Result<String> {
    let info = get_document_info(doc, prompt.input_variables())?;
    prompt.aformat(&info).await
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPromptTemplate {
        input_variables: Vec<String>,
        template: String,
    }

    impl BasePromptTemplate for TestPromptTemplate {
        fn input_variables(&self) -> &[String] {
            &self.input_variables
        }

        fn format(&self, kwargs: &HashMap<String, String>) -> Result<FormatOutputType> {
            let mut result = self.template.clone();
            for (key, value) in kwargs {
                result = result.replace(&format!("{{{}}}", key), value);
            }
            Ok(result)
        }

        fn partial(&self, kwargs: HashMap<String, String>) -> Result<Box<dyn BasePromptTemplate>> {
            let new_vars: Vec<_> = self
                .input_variables
                .iter()
                .filter(|v| !kwargs.contains_key(*v))
                .cloned()
                .collect();

            let mut new_template = self.template.clone();
            for (key, value) in &kwargs {
                new_template = new_template.replace(&format!("{{{}}}", key), value);
            }

            Ok(Box::new(TestPromptTemplate {
                input_variables: new_vars,
                template: new_template,
            }))
        }

        fn prompt_type(&self) -> &str {
            "test"
        }

        fn to_dict(&self) -> serde_json::Value {
            serde_json::json!({
                "_type": "test",
                "input_variables": self.input_variables,
                "template": self.template,
            })
        }
    }

    #[test]
    fn test_format_document() {
        let mut metadata = HashMap::new();
        metadata.insert("page".to_string(), "1".to_string());

        let doc = Document::with_metadata("This is a joke", metadata);

        let prompt = TestPromptTemplate {
            input_variables: vec!["page".to_string(), "page_content".to_string()],
            template: "Page {page}: {page_content}".to_string(),
        };

        let result = format_document(&doc, &prompt).unwrap();
        assert_eq!(result, "Page 1: This is a joke");
    }

    #[test]
    fn test_validate_variable_names() {
        let prompt = TestPromptTemplate {
            input_variables: vec!["stop".to_string()],
            template: "{stop}".to_string(),
        };

        let result = prompt.validate_variable_names();
        assert!(result.is_err());
    }
}
