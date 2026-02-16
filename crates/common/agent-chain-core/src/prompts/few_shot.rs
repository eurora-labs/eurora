//! Prompt templates that contain few shot examples.
//!
//! This module provides few-shot prompt templates for adding examples to prompts,
//! mirroring `langchain_core.prompts.few_shot` in Python.

use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::messages::{BaseMessage, get_buffer_string};
use crate::prompt_values::{ChatPromptValue, StringPromptValue};
use crate::runnables::base::Runnable;
use crate::runnables::config::{RunnableConfig, ensure_config};

use super::base::{BasePromptTemplate, FormatOutputType};
use super::chat::BaseChatPromptTemplate;
use super::message::BaseMessagePromptTemplate;
use super::prompt::PromptTemplate;
use super::string::{
    PromptTemplateFormat, StringPromptTemplate, check_valid_template, format_template,
    get_template_variables,
};

/// Type alias for async example selection future.
pub type ExampleSelectionFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Vec<HashMap<String, String>>> + Send + 'a>>;

/// Trait for example selectors used by few-shot prompt templates.
///
/// Mirrors `BaseExampleSelector` from Python but with string-typed examples
/// matching the prompt template usage pattern.
pub trait ExampleSelector: Send + Sync {
    /// Add new example to store.
    fn add_example(&mut self, example: HashMap<String, String>) -> Option<String> {
        let _ = example;
        None
    }

    /// Select examples based on the input variables.
    fn select_examples(
        &self,
        input_variables: &HashMap<String, String>,
    ) -> Vec<HashMap<String, String>>;

    /// Async select examples based on the input variables.
    fn aselect_examples(
        &self,
        input_variables: &HashMap<String, String>,
    ) -> ExampleSelectionFuture<'_> {
        let result = self.select_examples(input_variables);
        Box::pin(async move { result })
    }
}

/// Helper trait for cloning example selectors.
pub trait ExampleSelectorClone: ExampleSelector {
    fn clone_box(&self) -> Box<dyn ExampleSelectorClone + Send + Sync>;
}

impl<T: ExampleSelector + Clone + Send + Sync + 'static> ExampleSelectorClone for T {
    fn clone_box(&self) -> Box<dyn ExampleSelectorClone + Send + Sync> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn ExampleSelectorClone + Send + Sync> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

impl std::fmt::Debug for Box<dyn ExampleSelectorClone + Send + Sync> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExampleSelector")
    }
}

/// A simple example selector that always returns the same examples.
#[cfg(test)]
#[derive(Debug, Clone)]
struct StaticExampleSelector {
    examples: Vec<HashMap<String, String>>,
}

#[cfg(test)]
impl StaticExampleSelector {
    fn new(examples: Vec<HashMap<String, String>>) -> Self {
        Self { examples }
    }
}

#[cfg(test)]
impl ExampleSelector for StaticExampleSelector {
    fn select_examples(
        &self,
        _input_variables: &HashMap<String, String>,
    ) -> Vec<HashMap<String, String>> {
        self.examples.clone()
    }
}

// ---------------------------------------------------------------------------
// FewShotPromptTemplate
// ---------------------------------------------------------------------------

/// Prompt template that contains few shot examples.
///
/// Direct port of `langchain_core.prompts.few_shot.FewShotPromptTemplate`.
#[derive(Debug, Clone)]
pub struct FewShotPromptTemplate {
    /// Examples to format into the prompt.
    /// Either this or example_selector should be provided.
    examples: Option<Vec<HashMap<String, String>>>,

    /// ExampleSelector to choose the examples to format into the prompt.
    /// Either this or examples should be provided.
    example_selector: Option<Box<dyn ExampleSelectorClone + Send + Sync>>,

    /// PromptTemplate used to format an individual example.
    example_prompt: PromptTemplate,

    /// A prompt template string to put after the examples.
    suffix: String,

    /// String separator used to join the prefix, the examples, and suffix.
    example_separator: String,

    /// A prompt template string to put before the examples.
    prefix: String,

    /// The format of the prompt template. Options are: 'f-string', 'jinja2'.
    template_format: PromptTemplateFormat,

    /// Input variables for this prompt.
    input_variables: Vec<String>,

    /// Partial variables for this prompt.
    partial_variables: HashMap<String, String>,

    /// Whether or not to try validating the template.
    validate_template: bool,
}

impl FewShotPromptTemplate {
    /// Create a new FewShotPromptTemplate with examples.
    pub fn new(
        examples: Vec<HashMap<String, String>>,
        example_prompt: PromptTemplate,
        suffix: String,
        prefix: Option<String>,
    ) -> Result<Self> {
        let mut template = Self {
            examples: Some(examples),
            example_selector: None,
            example_prompt,
            suffix,
            example_separator: "\n\n".to_string(),
            prefix: prefix.unwrap_or_default(),
            template_format: PromptTemplateFormat::FString,
            input_variables: Vec::new(),
            partial_variables: HashMap::new(),
            validate_template: false,
        };
        template.infer_input_variables();
        Ok(template)
    }

    /// Create a new FewShotPromptTemplate with an example selector.
    pub fn with_selector(
        selector: impl ExampleSelectorClone + 'static,
        example_prompt: PromptTemplate,
        suffix: String,
        prefix: Option<String>,
    ) -> Result<Self> {
        let mut template = Self {
            examples: None,
            example_selector: Some(Box::new(selector)),
            example_prompt,
            suffix,
            example_separator: "\n\n".to_string(),
            prefix: prefix.unwrap_or_default(),
            template_format: PromptTemplateFormat::FString,
            input_variables: Vec::new(),
            partial_variables: HashMap::new(),
            validate_template: false,
        };
        template.infer_input_variables();
        Ok(template)
    }

    /// Set the example separator.
    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.example_separator = separator.into();
        self
    }

    /// Set the template format.
    pub fn with_format(mut self, format: PromptTemplateFormat) -> Self {
        self.template_format = format;
        self.infer_input_variables();
        self
    }

    /// Set the validate_template flag.
    pub fn with_validate_template(mut self, validate: bool) -> Self {
        self.validate_template = validate;
        self
    }

    /// Infer input variables from prefix + suffix template, matching Python's
    /// `template_is_valid` model_validator.
    fn infer_input_variables(&mut self) {
        if self.validate_template {
            let combined = format!("{}{}", self.prefix, self.suffix);
            let mut check_vars = self.input_variables.clone();
            check_vars.extend(self.partial_variables.keys().cloned());
            if let Err(error) = check_valid_template(&combined, self.template_format, &check_vars) {
                tracing::warn!("Template validation warning: {}", error);
            }
        } else {
            let combined = format!("{}{}", self.prefix, self.suffix);
            if let Ok(template_vars) = get_template_variables(&combined, self.template_format) {
                self.input_variables = template_vars
                    .into_iter()
                    .filter(|v| !self.partial_variables.contains_key(v))
                    .collect();
                self.input_variables.sort();
            }
        }
    }

    /// Get examples based on kwargs.
    ///
    /// Direct port of Python `_get_examples`.
    fn get_examples(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, String>>> {
        if let Some(ref examples) = self.examples {
            Ok(examples.clone())
        } else if let Some(ref selector) = self.example_selector {
            Ok(selector.select_examples(kwargs))
        } else {
            Err(Error::InvalidConfig(
                "One of 'examples' and 'example_selector' should be provided".to_string(),
            ))
        }
    }

    /// Async get examples based on kwargs.
    ///
    /// Direct port of Python `_aget_examples`.
    #[allow(dead_code)]
    async fn aget_examples(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, String>>> {
        if let Some(ref examples) = self.examples {
            Ok(examples.clone())
        } else if let Some(ref selector) = self.example_selector {
            Ok(selector.aselect_examples(kwargs).await)
        } else {
            Err(Error::InvalidConfig(
                "One of 'examples' and 'example_selector' should be provided".to_string(),
            ))
        }
    }

    fn merge_partial_and_user_variables(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut merged = self.partial_variables.clone();
        merged.extend(kwargs.clone());
        merged
    }
}

impl BasePromptTemplate for FewShotPromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn partial_variables(&self) -> &HashMap<String, String> {
        &self.partial_variables
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<FormatOutputType> {
        let kwargs = self.merge_partial_and_user_variables(kwargs);

        let examples = self.get_examples(&kwargs)?;

        let example_vars = &self.example_prompt.input_variables;
        let filtered_examples: Vec<HashMap<String, String>> = examples
            .iter()
            .map(|e| {
                e.iter()
                    .filter(|(k, _)| example_vars.contains(k))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .collect();

        let example_strings: Result<Vec<_>> = filtered_examples
            .iter()
            .map(|example| StringPromptTemplate::format(&self.example_prompt, example))
            .collect();
        let example_strings = example_strings?;

        let pieces: Vec<&str> = std::iter::once(self.prefix.as_str())
            .chain(example_strings.iter().map(|s| s.as_str()))
            .chain(std::iter::once(self.suffix.as_str()))
            .filter(|p| !p.is_empty())
            .collect();

        let template = pieces.join(&self.example_separator);

        format_template(&template, self.template_format, &kwargs)
    }

    fn partial(&self, kwargs: HashMap<String, String>) -> Result<Box<dyn BasePromptTemplate>> {
        let new_vars: Vec<_> = self
            .input_variables
            .iter()
            .filter(|v| !kwargs.contains_key(*v))
            .cloned()
            .collect();

        let mut new_partials = self.partial_variables.clone();
        new_partials.extend(kwargs);

        Ok(Box::new(Self {
            examples: self.examples.clone(),
            example_selector: self.example_selector.clone(),
            example_prompt: self.example_prompt.clone(),
            suffix: self.suffix.clone(),
            example_separator: self.example_separator.clone(),
            prefix: self.prefix.clone(),
            template_format: self.template_format,
            input_variables: new_vars,
            partial_variables: new_partials,
            validate_template: self.validate_template,
        }))
    }

    fn prompt_type(&self) -> &str {
        "few_shot"
    }

    fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "_type": self.prompt_type(),
            "examples": self.examples,
            "suffix": self.suffix,
            "prefix": self.prefix,
            "example_separator": self.example_separator,
            "template_format": self.template_format,
        })
    }

    fn save(&self, _file_path: &Path) -> Result<()> {
        if self.example_selector.is_some() {
            return Err(Error::InvalidConfig(
                "Saving an example selector is not currently supported".to_string(),
            ));
        }
        Err(Error::NotImplemented(
            "Saving few-shot prompts is not currently supported".to_string(),
        ))
    }
}

#[async_trait]
impl Runnable for FewShotPromptTemplate {
    type Input = HashMap<String, String>;
    type Output = StringPromptValue;

    fn name(&self) -> Option<String> {
        Some("FewShotPromptTemplate".to_string())
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let _config = ensure_config(config);
        BasePromptTemplate::validate_input(self, &input)?;
        let text = BasePromptTemplate::format(self, &input)?;
        Ok(StringPromptValue::new(text))
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.invoke(input, config)
    }
}

impl StringPromptTemplate for FewShotPromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn partial_variables(&self) -> &HashMap<String, String> {
        &self.partial_variables
    }

    fn template_format(&self) -> PromptTemplateFormat {
        self.template_format
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<String> {
        BasePromptTemplate::format(self, kwargs)
    }

    fn pretty_repr(&self, _html: bool) -> String {
        format!(
            "FewShotPromptTemplate(prefix={:?}, suffix={:?}, examples={:?})",
            self.prefix, self.suffix, self.examples
        )
    }
}

// ---------------------------------------------------------------------------
// FewShotChatMessagePromptTemplate
// ---------------------------------------------------------------------------

/// Chat prompt template that supports few-shot examples.
///
/// Direct port of `langchain_core.prompts.few_shot.FewShotChatMessagePromptTemplate`.
#[derive(Debug, Clone)]
pub struct FewShotChatMessagePromptTemplate {
    /// Examples to format into the prompt.
    examples: Option<Vec<HashMap<String, String>>>,

    /// ExampleSelector to choose the examples to format into the prompt.
    example_selector: Option<Box<dyn ExampleSelectorClone + Send + Sync>>,

    /// The prompt template to format each example.
    example_prompt: Box<dyn ExamplePrompt>,

    /// Input variables for this prompt (for example selector).
    input_variables: Vec<String>,
}

/// Trait for prompt templates that can format examples.
///
/// This abstracts over both `PromptTemplate` (BaseMessagePromptTemplate)
/// and `ChatPromptTemplate` (BaseChatPromptTemplate) as Python allows both
/// as `example_prompt`.
pub trait ExamplePrompt: Send + Sync {
    fn input_variables(&self) -> Vec<String>;
    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>>;
    fn clone_box(&self) -> Box<dyn ExamplePrompt>;
}

impl std::fmt::Debug for dyn ExamplePrompt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExamplePrompt(vars={:?})", self.input_variables())
    }
}

impl Clone for Box<dyn ExamplePrompt> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Wrapper for ChatPromptTemplate as an ExamplePrompt.
impl ExamplePrompt for super::chat::ChatPromptTemplate {
    fn input_variables(&self) -> Vec<String> {
        BasePromptTemplate::input_variables(self).to_vec()
    }
    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        BaseChatPromptTemplate::format_messages(self, kwargs)
    }
    fn clone_box(&self) -> Box<dyn ExamplePrompt> {
        Box::new(self.clone())
    }
}

impl FewShotChatMessagePromptTemplate {
    /// Create a new FewShotChatMessagePromptTemplate with examples.
    pub fn new(
        examples: Vec<HashMap<String, String>>,
        example_prompt: impl ExamplePrompt + 'static,
    ) -> Self {
        Self {
            examples: Some(examples),
            example_selector: None,
            example_prompt: Box::new(example_prompt),
            input_variables: Vec::new(),
        }
    }

    /// Create a new FewShotChatMessagePromptTemplate with an example selector.
    pub fn with_selector(
        selector: impl ExampleSelectorClone + 'static,
        example_prompt: impl ExamplePrompt + 'static,
        input_variables: Vec<String>,
    ) -> Self {
        Self {
            examples: None,
            example_selector: Some(Box::new(selector)),
            example_prompt: Box::new(example_prompt),
            input_variables,
        }
    }

    fn get_examples(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, String>>> {
        if let Some(ref examples) = self.examples {
            Ok(examples.clone())
        } else if let Some(ref selector) = self.example_selector {
            Ok(selector.select_examples(kwargs))
        } else {
            Err(Error::InvalidConfig(
                "One of 'examples' and 'example_selector' should be provided".to_string(),
            ))
        }
    }

    #[allow(dead_code)]
    async fn aget_examples(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, String>>> {
        if let Some(ref examples) = self.examples {
            Ok(examples.clone())
        } else if let Some(ref selector) = self.example_selector {
            Ok(selector.aselect_examples(kwargs).await)
        } else {
            Err(Error::InvalidConfig(
                "One of 'examples' and 'example_selector' should be provided".to_string(),
            ))
        }
    }
}

impl BaseMessagePromptTemplate for FewShotChatMessagePromptTemplate {
    fn input_variables(&self) -> Vec<String> {
        self.input_variables.clone()
    }

    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        let examples = self.get_examples(kwargs)?;

        let example_vars = self.example_prompt.input_variables();
        let filtered_examples: Vec<HashMap<String, String>> = examples
            .iter()
            .map(|e| {
                e.iter()
                    .filter(|(k, _)| example_vars.contains(k))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .collect();

        let mut messages = Vec::new();
        for example in &filtered_examples {
            let example_messages = self.example_prompt.format_messages(example)?;
            messages.extend(example_messages);
        }

        Ok(messages)
    }

    fn pretty_repr(&self, _html: bool) -> String {
        "FewShotChatMessagePromptTemplate(pretty_repr not implemented)".to_string()
    }
}

impl BasePromptTemplate for FewShotChatMessagePromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<String> {
        let messages = BaseChatPromptTemplate::format_messages(self, kwargs)?;
        Ok(get_buffer_string(&messages, "Human", "AI"))
    }

    fn partial(&self, _kwargs: HashMap<String, String>) -> Result<Box<dyn BasePromptTemplate>> {
        Err(crate::error::Error::NotImplemented(
            "partial is not supported for FewShotChatMessagePromptTemplate".into(),
        ))
    }

    fn prompt_type(&self) -> &str {
        "few_shot_chat"
    }

    fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "_type": self.prompt_type(),
            "input_variables": self.input_variables,
        })
    }
}

impl BaseChatPromptTemplate for FewShotChatMessagePromptTemplate {
    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        BaseMessagePromptTemplate::format_messages(self, kwargs)
    }

    fn pretty_repr(&self, _html: bool) -> String {
        "FewShotChatMessagePromptTemplate(pretty_repr not implemented)".to_string()
    }
}

#[async_trait]
impl Runnable for FewShotChatMessagePromptTemplate {
    type Input = HashMap<String, String>;
    type Output = ChatPromptValue;

    fn name(&self) -> Option<String> {
        Some("FewShotChatMessagePromptTemplate".to_string())
    }

    fn invoke(&self, input: Self::Input, config: Option<RunnableConfig>) -> Result<Self::Output> {
        let _config = ensure_config(config);
        BasePromptTemplate::validate_input(self, &input)?;
        let messages = BaseChatPromptTemplate::format_messages(self, &input)?;
        Ok(ChatPromptValue::new(messages))
    }

    async fn ainvoke(
        &self,
        input: Self::Input,
        config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.invoke(input, config)
    }
}

#[cfg(test)]
mod tests {
    use super::super::chat::ChatPromptTemplate;
    use super::*;

    #[test]
    fn test_few_shot_prompt_template() {
        let examples = vec![
            HashMap::from([
                ("input".to_string(), "2+2".to_string()),
                ("output".to_string(), "4".to_string()),
            ]),
            HashMap::from([
                ("input".to_string(), "2+3".to_string()),
                ("output".to_string(), "5".to_string()),
            ]),
        ];

        let example_prompt = PromptTemplate::from_template("Q: {input}\nA: {output}").unwrap();

        let few_shot = FewShotPromptTemplate::new(
            examples,
            example_prompt,
            "Q: {question}\nA:".to_string(),
            None,
        )
        .unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("question".to_string(), "2+4".to_string());

        let result = BasePromptTemplate::format(&few_shot, &kwargs).unwrap();
        assert!(result.contains("Q: 2+2"));
        assert!(result.contains("A: 4"));
        assert!(result.contains("Q: 2+3"));
        assert!(result.contains("A: 5"));
        assert!(result.contains("Q: 2+4"));
    }

    #[test]
    fn test_few_shot_with_prefix() {
        let examples = vec![HashMap::from([
            ("input".to_string(), "hi".to_string()),
            ("output".to_string(), "hello".to_string()),
        ])];

        let example_prompt = PromptTemplate::from_template("{input} -> {output}").unwrap();

        let few_shot = FewShotPromptTemplate::new(
            examples,
            example_prompt,
            "{query}".to_string(),
            Some("Examples:".to_string()),
        )
        .unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("query".to_string(), "bye".to_string());

        let result = BasePromptTemplate::format(&few_shot, &kwargs).unwrap();
        assert!(result.starts_with("Examples:"));
        assert!(result.contains("hi -> hello"));
        assert!(result.ends_with("bye"));
    }

    #[test]
    fn test_few_shot_infers_input_variables() {
        let examples = vec![HashMap::from([("input".to_string(), "test".to_string())])];

        let example_prompt = PromptTemplate::from_template("{input}").unwrap();

        let few_shot = FewShotPromptTemplate::new(
            examples,
            example_prompt,
            "Answer: {question}".to_string(),
            Some("Context: {context}".to_string()),
        )
        .unwrap();

        let mut vars = few_shot.input_variables.clone();
        vars.sort();
        assert_eq!(vars, vec!["context", "question"]);
    }

    #[test]
    fn test_few_shot_chat_message_template() {
        let examples = vec![HashMap::from([
            ("input".to_string(), "2+2".to_string()),
            ("output".to_string(), "4".to_string()),
        ])];

        let example_prompt = ChatPromptTemplate::from_messages(vec![
            ("human", "What is {input}?").into(),
            ("ai", "{output}").into(),
        ])
        .unwrap();

        let few_shot = FewShotChatMessagePromptTemplate::new(examples, example_prompt);

        let kwargs = HashMap::new();
        let messages = BaseMessagePromptTemplate::format_messages(&few_shot, &kwargs).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content(), "What is 2+2?");
        assert_eq!(messages[1].content(), "4");
    }

    #[test]
    fn test_static_example_selector() {
        let examples = vec![HashMap::from([("key".to_string(), "value".to_string())])];

        let selector = StaticExampleSelector::new(examples.clone());
        let selected = selector.select_examples(&HashMap::new());

        assert_eq!(selected, examples);
    }
}
