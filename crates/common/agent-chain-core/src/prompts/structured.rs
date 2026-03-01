use std::collections::HashMap;

use bon::bon;
use serde_json::Value;

use async_trait::async_trait;

use crate::api::{BetaParams, warn_beta};
use crate::error::{Error, Result};
use crate::messages::BaseMessage;
use crate::prompt_values::{ChatPromptValue, PromptValue};
use crate::runnables::base::Runnable;
use crate::runnables::config::{RunnableConfig, ensure_config};

use super::base::BasePromptTemplate;
use super::chat::{BaseChatPromptTemplate, ChatPromptTemplate, MessageLikeRepresentation};
use super::string::PromptTemplateFormat;

#[derive(Debug, Clone)]
pub struct StructuredPrompt {
    chat_template: ChatPromptTemplate,
    pub schema: Value,
    pub structured_output_kwargs: HashMap<String, Value>,
}

#[bon]
impl StructuredPrompt {
    #[builder]
    pub fn new(
        messages: Vec<MessageLikeRepresentation>,
        schema: Value,
        #[builder(default)] structured_output_kwargs: HashMap<String, Value>,
        #[builder(default)] template_format: PromptTemplateFormat,
    ) -> Result<Self> {
        warn_beta(
            BetaParams {
                message: Some("StructuredPrompt is in beta. It is actively being worked on,                           so the API may change.".to_string()),
                ..Default::default()
            },
            module_path!(),
        );

        if schema.is_null()
            || (schema.is_object() && schema.as_object().is_none_or(|o| o.is_empty()))
        {
            return Err(Error::InvalidConfig(format!(
                "Must pass in a non-empty structured output schema. Received: {}",
                schema
            )));
        }

        let chat_template =
            ChatPromptTemplate::from_messages_with_format(messages, template_format)?;

        Ok(Self {
            chat_template,
            schema,
            structured_output_kwargs,
        })
    }

    pub fn from_messages_and_schema(
        messages: Vec<MessageLikeRepresentation>,
        schema: Value,
    ) -> Result<Self> {
        Self::builder().messages(messages).schema(schema).build()
    }

    pub fn chat_template(&self) -> &ChatPromptTemplate {
        &self.chat_template
    }
}

impl BasePromptTemplate for StructuredPrompt {
    fn input_variables(&self) -> &[String] {
        self.chat_template.input_variables()
    }

    fn optional_variables(&self) -> &[String] {
        self.chat_template.optional_variables()
    }

    fn partial_variables(&self) -> &HashMap<String, String> {
        self.chat_template.partial_variables()
    }

    fn format(&self, kwargs: &HashMap<String, String>) -> Result<String> {
        let messages = self.format_messages(kwargs)?;
        let prompt_value = crate::prompt_values::ChatPromptValue::new(messages);
        Ok(prompt_value.to_string())
    }

    fn format_prompt(&self, kwargs: &HashMap<String, String>) -> Result<Box<dyn PromptValue>> {
        let messages = self.format_messages(kwargs)?;
        Ok(Box::new(crate::prompt_values::ChatPromptValue::new(
            messages,
        )))
    }

    fn partial(&self, _kwargs: HashMap<String, String>) -> Result<Box<dyn BasePromptTemplate>> {
        Err(crate::error::Error::NotImplemented(
            "partial is not supported for StructuredPrompt".into(),
        ))
    }

    fn prompt_type(&self) -> &str {
        "structured"
    }

    fn to_dict(&self) -> serde_json::Value {
        serde_json::json!({
            "_type": self.prompt_type(),
            "input_variables": self.input_variables(),
            "schema": self.schema,
        })
    }
}

impl BaseChatPromptTemplate for StructuredPrompt {
    fn format_messages(&self, kwargs: &HashMap<String, String>) -> Result<Vec<BaseMessage>> {
        self.chat_template.format_messages(kwargs)
    }

    fn pretty_repr(&self, html: bool) -> String {
        self.chat_template.pretty_repr(html)
    }
}

#[async_trait]
impl Runnable for StructuredPrompt {
    type Input = HashMap<String, String>;
    type Output = ChatPromptValue;

    fn name(&self) -> Option<String> {
        Some("StructuredPrompt".to_string())
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
    use super::*;
    use serde_json::json;

    #[test]
    fn test_structured_prompt_creation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "value": {"type": "integer"}
            }
        });

        let prompt = StructuredPrompt::new(
            vec![
                ("system", "Extract structured data.").into(),
                ("human", "{input}").into(),
            ],
            schema.clone(),
        )
        .unwrap();

        assert_eq!(prompt.input_variables(), &["input"]);
        assert_eq!(prompt.schema, schema);
    }

    #[test]
    fn test_structured_prompt_format_messages() {
        let schema = json!({"type": "object"});

        let prompt = StructuredPrompt::new(
            vec![
                ("system", "You extract data.").into(),
                ("human", "{text}").into(),
            ],
            schema,
        )
        .unwrap();

        let mut kwargs = HashMap::new();
        kwargs.insert("text".to_string(), "Hello world".to_string());

        let messages = prompt.format_messages(&kwargs).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content(), "You extract data.");
        assert_eq!(messages[1].content(), "Hello world");
    }

    #[test]
    fn test_structured_prompt_rejects_empty_schema() {
        let result = StructuredPrompt::new(vec![("human", "test").into()], json!({}));
        assert!(result.is_err());

        let result = StructuredPrompt::new(vec![("human", "test").into()], Value::Null);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_messages_and_schema() {
        let schema = json!({"type": "object", "properties": {}});

        let prompt =
            StructuredPrompt::from_messages_and_schema(vec![("human", "{input}").into()], schema)
                .unwrap();

        assert_eq!(prompt.input_variables(), &["input"]);
    }
}
