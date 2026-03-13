use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use bon::bon;

use crate::error::{Error, Result};
use crate::messages::{AnyMessage, get_buffer_string};
use crate::runnables::base::Runnable;
use crate::runnables::config::RunnableConfig;

use super::base::BasePromptTemplate;
use super::chat::{BaseChatPromptTemplate, ChatPromptInput};
use super::message::BaseMessagePromptTemplate;
use super::prompt::PromptTemplate;
use super::string::{
    PromptTemplateFormat, StringPromptTemplate, check_valid_template, format_template,
    get_template_variables,
};

pub type ExampleSelectionFuture<'a> =
    std::pin::Pin<Box<dyn std::future::Future<Output = Vec<HashMap<String, String>>> + Send + 'a>>;

pub trait ExampleSelector: Send + Sync {
    fn add_example(&mut self, example: HashMap<String, String>) -> Option<String>;

    fn select_examples(
        &self,
        input_variables: &HashMap<String, String>,
    ) -> Vec<HashMap<String, String>>;

    fn aselect_examples(
        &self,
        input_variables: &HashMap<String, String>,
    ) -> ExampleSelectionFuture<'_> {
        let result = self.select_examples(input_variables);
        Box::pin(async move { result })
    }
}

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

pub(super) fn resolve_examples(
    examples: Option<&[HashMap<String, String>]>,
    selector: Option<&(dyn ExampleSelectorClone + Send + Sync)>,
    kwargs: &HashMap<String, String>,
) -> Result<Vec<HashMap<String, String>>> {
    if let Some(examples) = examples {
        Ok(examples.to_vec())
    } else if let Some(selector) = selector {
        Ok(selector.select_examples(kwargs))
    } else {
        Err(Error::InvalidConfig(
            "One of 'examples' and 'example_selector' should be provided".to_string(),
        ))
    }
}

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
    fn add_example(&mut self, example: HashMap<String, String>) -> Option<String> {
        self.examples.push(example);
        None
    }

    fn select_examples(
        &self,
        _input_variables: &HashMap<String, String>,
    ) -> Vec<HashMap<String, String>> {
        self.examples.clone()
    }
}

#[derive(Debug, Clone)]
pub struct FewShotPromptTemplate {
    examples: Option<Vec<HashMap<String, String>>>,

    example_selector: Option<Box<dyn ExampleSelectorClone + Send + Sync>>,

    example_prompt: PromptTemplate,

    suffix: String,

    example_separator: String,

    prefix: String,

    template_format: PromptTemplateFormat,

    input_variables: Vec<String>,

    partial_variables: HashMap<String, String>,

    validate_template: bool,
}

#[bon]
impl FewShotPromptTemplate {
    #[builder]
    pub fn new(
        examples: Vec<HashMap<String, String>>,
        example_prompt: PromptTemplate,
        suffix: String,
        prefix: Option<String>,
        #[builder(default = "\n\n".to_string())] example_separator: String,
        #[builder(default)] template_format: PromptTemplateFormat,
        #[builder(default)] validate_template: bool,
    ) -> Result<Self> {
        let mut template = Self {
            examples: Some(examples),
            example_selector: None,
            example_prompt,
            suffix,
            example_separator,
            prefix: prefix.unwrap_or_default(),
            template_format,
            input_variables: Vec::new(),
            partial_variables: HashMap::new(),
            validate_template,
        };
        template.infer_input_variables();
        Ok(template)
    }

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

    fn get_examples(
        &self,
        kwargs: &HashMap<String, String>,
    ) -> Result<Vec<HashMap<String, String>>> {
        resolve_examples(
            self.examples.as_deref(),
            self.example_selector.as_deref(),
            kwargs,
        )
    }
}

impl BasePromptTemplate for FewShotPromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn partial_variables(&self) -> HashMap<String, String> {
        self.partial_variables.clone()
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<String> {
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
    type Output = Vec<AnyMessage>;

    fn name(&self) -> Option<String> {
        Some("FewShotPromptTemplate".to_string())
    }
    async fn invoke(
        &self,
        input: Self::Input,
        _config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.format_messages(&input)
    }
}

impl StringPromptTemplate for FewShotPromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn partial_variables(&self) -> HashMap<String, String> {
        self.partial_variables.clone()
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

#[derive(Debug, Clone)]
pub struct FewShotChatMessagePromptTemplate {
    examples: Option<Vec<HashMap<String, String>>>,

    example_selector: Option<Box<dyn ExampleSelectorClone + Send + Sync>>,

    example_prompt: Box<dyn ExamplePrompt>,

    input_variables: Vec<String>,
}

pub trait ExamplePrompt: Send + Sync {
    fn input_variables(&self) -> Vec<String>;
    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<AnyMessage>>;
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

impl ExamplePrompt for super::chat::ChatPromptTemplate {
    fn input_variables(&self) -> Vec<String> {
        BasePromptTemplate::input_variables(self).to_vec()
    }
    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<AnyMessage>> {
        let input = ChatPromptInput::from(kwargs.clone());
        BaseChatPromptTemplate::format_chat_messages(self, &input)
    }
    fn clone_box(&self) -> Box<dyn ExamplePrompt> {
        Box::new(self.clone())
    }
}

impl FewShotChatMessagePromptTemplate {
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
        resolve_examples(
            self.examples.as_deref(),
            self.example_selector.as_deref(),
            kwargs,
        )
    }
}

impl BaseMessagePromptTemplate for FewShotChatMessagePromptTemplate {
    fn input_variables(&self) -> &[String] {
        &self.input_variables
    }

    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<AnyMessage>> {
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
        let input = ChatPromptInput::from(kwargs.clone());
        let messages = BaseChatPromptTemplate::format_chat_messages(self, &input)?;
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
    fn format_chat_messages(&self, input: &ChatPromptInput) -> Result<Vec<AnyMessage>> {
        BaseMessagePromptTemplate::format_messages(self, &input.variables)
    }

    fn pretty_repr(&self, _html: bool) -> String {
        "FewShotChatMessagePromptTemplate(pretty_repr not implemented)".to_string()
    }
}

#[async_trait]
impl Runnable for FewShotChatMessagePromptTemplate {
    type Input = ChatPromptInput;
    type Output = Vec<AnyMessage>;

    fn name(&self) -> Option<String> {
        Some("FewShotChatMessagePromptTemplate".to_string())
    }
    async fn invoke(
        &self,
        input: Self::Input,
        _config: Option<RunnableConfig>,
    ) -> Result<Self::Output> {
        self.format_chat_messages(&input)
    }
}

#[cfg(test)]
mod tests {
    use super::super::chat::ChatPromptTemplate;
    use super::*;
    use crate::messages::BaseMessage;

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

        let few_shot = FewShotPromptTemplate::builder()
            .examples(examples)
            .example_prompt(example_prompt)
            .suffix("Q: {question}\nA:".to_string())
            .build()
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

        let few_shot = FewShotPromptTemplate::builder()
            .examples(examples)
            .example_prompt(example_prompt)
            .suffix("{query}".to_string())
            .prefix("Examples:".to_string())
            .build()
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

        let few_shot = FewShotPromptTemplate::builder()
            .examples(examples)
            .example_prompt(example_prompt)
            .suffix("Answer: {question}".to_string())
            .prefix("Context: {context}".to_string())
            .build()
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
