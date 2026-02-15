//! Prompt templates that contain few shot examples.
//!
//! This module provides few-shot prompt templates for adding examples to prompts,
//! mirroring `langchain_core.prompts.few_shot` in Python.

use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};
use crate::messages::{BaseMessage, get_buffer_string};

use super::base::{BasePromptTemplate, FormatOutputType};
use super::chat::{BaseChatPromptTemplate, ChatPromptTemplate};
use super::message::BaseMessagePromptTemplate;
use super::prompt::PromptTemplate;
use super::string::{PromptTemplateFormat, StringPromptTemplate, format_template};

/// Type alias for async example selection future.
pub type ExampleSelectionFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Vec<HashMap<String, String>>> + Send + 'a>>;

/// Trait for example selectors.
///
/// Example selectors dynamically select examples based on the input.
pub trait ExampleSelector: Send + Sync {
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

/// A simple example selector that always returns the same examples.
///
/// This selector always returns the same examples regardless of input,
/// matching the Python langchain_core implementation.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct StaticExampleSelector {
    examples: Vec<HashMap<String, String>>,
}

#[allow(dead_code)]
impl StaticExampleSelector {
    /// Create a new static example selector.
    pub fn new(examples: Vec<HashMap<String, String>>) -> Self {
        Self { examples }
    }
}

impl ExampleSelector for StaticExampleSelector {
    fn select_examples(
        &self,
        _input_variables: &HashMap<String, String>,
    ) -> Vec<HashMap<String, String>> {
        self.examples.clone()
    }
}

/// Prompt template that contains few shot examples.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::prompts::{FewShotPromptTemplate, PromptTemplate};
/// use std::collections::HashMap;
///
/// let examples = vec![
///     HashMap::from([
///         ("input".to_string(), "2+2".to_string()),
///         ("output".to_string(), "4".to_string()),
///     ]),
///     HashMap::from([
///         ("input".to_string(), "2+3".to_string()),
///         ("output".to_string(), "5".to_string()),
///     ]),
/// ];
///
/// let example_prompt = PromptTemplate::from_template("Input: {input}\nOutput: {output}").unwrap();
///
/// let few_shot = FewShotPromptTemplate::new(
///     examples,
///     example_prompt,
///     "Answer the question:\n{question}".to_string(),
///     None,
/// ).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct FewShotPromptTemplate {
    /// Examples to format into the prompt.
    examples: Option<Vec<HashMap<String, String>>>,

    /// ExampleSelector to choose the examples to format into the prompt.
    example_selector: Option<Box<dyn ExampleSelectorClone + Send + Sync>>,

    /// PromptTemplate used to format an individual example.
    example_prompt: PromptTemplate,

    /// A prompt template string to put after the examples.
    suffix: String,

    /// String separator used to join the prefix, the examples, and suffix.
    example_separator: String,

    /// A prompt template string to put before the examples.
    prefix: String,

    /// The format of the prompt template.
    template_format: PromptTemplateFormat,

    /// Input variables for this prompt.
    input_variables: Vec<String>,

    /// Partial variables for this prompt.
    partial_variables: HashMap<String, String>,

    /// Whether to validate the template.
    validate_template: bool,
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

impl FewShotPromptTemplate {
    /// Create a new FewShotPromptTemplate with examples.
    pub fn new(
        examples: Vec<HashMap<String, String>>,
        example_prompt: PromptTemplate,
        suffix: String,
        prefix: Option<String>,
    ) -> Result<Self> {
        let input_variables = example_prompt.input_variables.clone();

        Ok(Self {
            examples: Some(examples),
            example_selector: None,
            example_prompt,
            suffix,
            example_separator: "\n\n".to_string(),
            prefix: prefix.unwrap_or_default(),
            template_format: PromptTemplateFormat::FString,
            input_variables,
            partial_variables: HashMap::new(),
            validate_template: false,
        })
    }

    /// Create a new FewShotPromptTemplate with an example selector.
    pub fn with_selector(
        selector: impl ExampleSelectorClone + 'static,
        example_prompt: PromptTemplate,
        suffix: String,
        prefix: Option<String>,
    ) -> Result<Self> {
        let input_variables = example_prompt.input_variables.clone();

        Ok(Self {
            examples: None,
            example_selector: Some(Box::new(selector)),
            example_prompt,
            suffix,
            example_separator: "\n\n".to_string(),
            prefix: prefix.unwrap_or_default(),
            template_format: PromptTemplateFormat::FString,
            input_variables,
            partial_variables: HashMap::new(),
            validate_template: false,
        })
    }

    /// Set the example separator.
    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.example_separator = separator.into();
        self
    }

    /// Set the template format.
    pub fn with_format(mut self, format: PromptTemplateFormat) -> Self {
        self.template_format = format;
        self
    }

    /// Get examples based on kwargs.
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

    /// Merge partial and user variables.
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

        // Get examples
        let examples = self.get_examples(&kwargs)?;

        // Filter example keys to only those in example_prompt
        let example_vars = &self.example_prompt.input_variables;
        let filtered_examples: Vec<_> = examples
            .iter()
            .map(|e| {
                e.iter()
                    .filter(|(k, _)| example_vars.contains(k))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .collect();

        // Format examples
        let example_strings: Result<Vec<_>> = filtered_examples
            .iter()
            .map(|example| StringPromptTemplate::format(&self.example_prompt, example))
            .collect();
        let example_strings = example_strings?;

        // Create the overall template
        let pieces: Vec<&str> = std::iter::once(self.prefix.as_str())
            .chain(example_strings.iter().map(|s| s.as_str()))
            .chain(std::iter::once(self.suffix.as_str()))
            .filter(|p| !p.is_empty())
            .collect();

        let template = pieces.join(&self.example_separator);

        // Format the template with input variables
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
        // Note: Cannot call default save implementation due to recursion.
        // The save functionality for few-shot prompts is not fully supported.
        Err(Error::InvalidConfig(
            "Saving few-shot prompts is not currently supported".to_string(),
        ))
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

/// Chat prompt template that supports few-shot examples.
///
/// The high level structure of produced by this prompt template is a list of messages
/// consisting of prefix message(s), example message(s), and suffix message(s).
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::prompts::{FewShotChatMessagePromptTemplate, ChatPromptTemplate};
/// use std::collections::HashMap;
///
/// let examples = vec![
///     HashMap::from([
///         ("input".to_string(), "2+2".to_string()),
///         ("output".to_string(), "4".to_string()),
///     ]),
///     HashMap::from([
///         ("input".to_string(), "2+3".to_string()),
///         ("output".to_string(), "5".to_string()),
///     ]),
/// ];
///
/// let example_prompt = ChatPromptTemplate::from_messages(vec![
///     ("human", "What is {input}?").into(),
///     ("ai", "{output}").into(),
/// ]).unwrap();
///
/// let few_shot = FewShotChatMessagePromptTemplate::new(examples, example_prompt);
/// ```
#[derive(Debug, Clone)]
pub struct FewShotChatMessagePromptTemplate {
    /// Examples to format into the prompt.
    examples: Option<Vec<HashMap<String, String>>>,

    /// ExampleSelector to choose the examples to format into the prompt.
    example_selector: Option<Box<dyn ExampleSelectorClone + Send + Sync>>,

    /// The prompt template to format each example.
    example_prompt: ChatPromptTemplate,

    /// Input variables for this prompt (for example selector).
    input_variables: Vec<String>,
}

impl FewShotChatMessagePromptTemplate {
    /// Create a new FewShotChatMessagePromptTemplate with examples.
    pub fn new(examples: Vec<HashMap<String, String>>, example_prompt: ChatPromptTemplate) -> Self {
        Self {
            examples: Some(examples),
            example_selector: None,
            example_prompt,
            input_variables: Vec::new(),
        }
    }

    /// Create a new FewShotChatMessagePromptTemplate with an example selector.
    pub fn with_selector(
        selector: impl ExampleSelectorClone + 'static,
        example_prompt: ChatPromptTemplate,
        input_variables: Vec<String>,
    ) -> Self {
        Self {
            examples: None,
            example_selector: Some(Box::new(selector)),
            example_prompt,
            input_variables,
        }
    }

    /// Get examples based on kwargs.
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

        // Filter example keys to only those in example_prompt
        let example_vars = self.example_prompt.input_variables();
        let filtered_examples: Vec<_> = examples
            .iter()
            .map(|e| {
                e.iter()
                    .filter(|(k, _)| example_vars.contains(k))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .collect();

        // Format examples into messages
        let mut messages = Vec::new();
        for example in filtered_examples {
            let example_messages = self.example_prompt.format_messages(&example)?;
            messages.extend(example_messages);
        }

        Ok(messages)
    }

    fn pretty_repr(&self, _html: bool) -> String {
        format!(
            "FewShotChatMessagePromptTemplate(examples={:?})",
            self.examples
        )
    }
}

impl BaseChatPromptTemplate for FewShotChatMessagePromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        BaseMessagePromptTemplate::format_messages(self, kwargs)
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<String> {
        let messages = BaseChatPromptTemplate::format_messages(self, kwargs)?;
        Ok(get_buffer_string(&messages, "Human", "AI"))
    }

    fn pretty_repr(&self, html: bool) -> String {
        BaseMessagePromptTemplate::pretty_repr(self, html)
    }
}

#[cfg(test)]
mod tests {
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
