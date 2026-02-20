use serde::{Deserialize, Serialize};

use crate::load::Serializable;
use crate::messages::{
    AnyMessage, BaseMessage, ContentPart, HumanMessage, ImageDetail, ImageSource, MessageContent,
    get_buffer_string,
};

pub trait PromptValue: Serializable {
    fn to_string(&self) -> String;

    fn to_messages(&self) -> Vec<BaseMessage>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImageDetailLevel {
    #[default]
    Auto,
    Low,
    High,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ImageURL {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ImageDetailLevel>,
}

impl ImageURL {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            detail: None,
        }
    }

    pub fn with_detail(url: impl Into<String>, detail: ImageDetailLevel) -> Self {
        Self {
            url: Some(url.into()),
            detail: Some(detail),
        }
    }

    pub fn get_url(&self) -> &str {
        self.url.as_deref().unwrap_or("")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StringPromptValue {
    pub text: String,
}

impl StringPromptValue {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl PromptValue for StringPromptValue {
    fn to_string(&self) -> String {
        self.text.clone()
    }

    fn to_messages(&self) -> Vec<BaseMessage> {
        vec![BaseMessage::Human(
            HumanMessage::builder().content(&self.text).build(),
        )]
    }
}

impl Serializable for StringPromptValue {
    fn is_lc_serializable() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn get_lc_namespace() -> Vec<String>
    where
        Self: Sized,
    {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "base".to_string(),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatPromptValue {
    pub messages: Vec<BaseMessage>,
}

impl ChatPromptValue {
    pub fn new(messages: Vec<BaseMessage>) -> Self {
        Self { messages }
    }

    pub fn from_message(message: impl Into<BaseMessage>) -> Self {
        Self {
            messages: vec![message.into()],
        }
    }
}

impl PromptValue for ChatPromptValue {
    fn to_string(&self) -> String {
        get_buffer_string(&self.messages, "Human", "AI")
    }

    fn to_messages(&self) -> Vec<BaseMessage> {
        self.messages.clone()
    }
}

impl Serializable for ChatPromptValue {
    fn is_lc_serializable() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn get_lc_namespace() -> Vec<String>
    where
        Self: Sized,
    {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "chat".to_string(),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImagePromptValue {
    pub image_url: ImageURL,
}

impl ImagePromptValue {
    pub fn new(image_url: ImageURL) -> Self {
        Self { image_url }
    }

    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            image_url: ImageURL::new(url),
        }
    }

    pub fn from_url_with_detail(url: impl Into<String>, detail: ImageDetailLevel) -> Self {
        Self {
            image_url: ImageURL::with_detail(url, detail),
        }
    }
}

impl PromptValue for ImagePromptValue {
    fn to_string(&self) -> String {
        self.image_url.get_url().to_string()
    }

    fn to_messages(&self) -> Vec<BaseMessage> {
        let url = self.image_url.get_url().to_string();
        let detail = self.image_url.detail.as_ref().map(|d| match d {
            ImageDetailLevel::Auto => ImageDetail::Auto,
            ImageDetailLevel::Low => ImageDetail::Low,
            ImageDetailLevel::High => ImageDetail::High,
        });

        let content_part = ContentPart::Image {
            source: ImageSource::Url { url },
            detail,
        };

        vec![BaseMessage::Human(
            HumanMessage::builder()
                .content(MessageContent::Parts(vec![content_part]))
                .build(),
        )]
    }
}

impl Serializable for ImagePromptValue {
    fn is_lc_serializable() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn get_lc_namespace() -> Vec<String>
    where
        Self: Sized,
    {
        vec![
            "langchain".to_string(),
            "schema".to_string(),
            "prompt".to_string(),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatPromptValueConcrete {
    pub messages: Vec<AnyMessage>,
}

impl ChatPromptValueConcrete {
    pub fn new(messages: Vec<AnyMessage>) -> Self {
        Self { messages }
    }
}

impl PromptValue for ChatPromptValueConcrete {
    fn to_string(&self) -> String {
        get_buffer_string(&self.messages, "Human", "AI")
    }

    fn to_messages(&self) -> Vec<BaseMessage> {
        self.messages.clone()
    }
}

impl Serializable for ChatPromptValueConcrete {
    fn is_lc_serializable() -> bool
    where
        Self: Sized,
    {
        true
    }

    fn get_lc_namespace() -> Vec<String>
    where
        Self: Sized,
    {
        vec![
            "langchain".to_string(),
            "prompts".to_string(),
            "chat".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{AIMessage, SystemMessage};

    #[test]
    fn test_string_prompt_value() {
        let pv = StringPromptValue::new("Hello, world!");
        assert_eq!(pv.to_string(), "Hello, world!");

        let messages = pv.to_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content(), "Hello, world!");
    }

    #[test]
    fn test_chat_prompt_value() {
        let messages = vec![
            BaseMessage::System(
                SystemMessage::builder()
                    .content("You are a helpful assistant.")
                    .build(),
            ),
            BaseMessage::Human(HumanMessage::builder().content("Hello!").build()),
            BaseMessage::AI(AIMessage::builder().content("Hi there!").build()),
        ];
        let pv = ChatPromptValue::new(messages.clone());

        let result = pv.to_string();
        assert!(result.contains("System:"));
        assert!(result.contains("Human:"));
        assert!(result.contains("AI:"));

        let returned_messages = pv.to_messages();
        assert_eq!(returned_messages.len(), 3);
    }

    #[test]
    fn test_image_url() {
        let url = ImageURL::new("https://example.com/image.jpg");
        assert_eq!(url.get_url(), "https://example.com/image.jpg");
        assert!(url.detail.is_none());

        let url_with_detail =
            ImageURL::with_detail("https://example.com/image.jpg", ImageDetailLevel::High);
        assert_eq!(url_with_detail.detail, Some(ImageDetailLevel::High));
    }

    #[test]
    fn test_image_prompt_value() {
        let pv = ImagePromptValue::from_url("https://example.com/image.jpg");
        assert_eq!(pv.to_string(), "https://example.com/image.jpg");

        let messages = pv.to_messages();
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_chat_prompt_value_concrete() {
        let messages = vec![
            BaseMessage::Human(HumanMessage::builder().content("Hello!").build()),
            BaseMessage::AI(AIMessage::builder().content("Hi!").build()),
        ];
        let pv = ChatPromptValueConcrete::new(messages);

        assert_eq!(pv.to_messages().len(), 2);
    }

    #[test]
    fn test_serializable_namespaces() {
        assert_eq!(
            StringPromptValue::get_lc_namespace(),
            vec!["langchain", "prompts", "base"]
        );
        assert_eq!(
            ChatPromptValue::get_lc_namespace(),
            vec!["langchain", "prompts", "chat"]
        );
        assert_eq!(
            ImagePromptValue::get_lc_namespace(),
            vec!["langchain", "schema", "prompt"]
        );
    }
}
