//! Anthropic (Claude) chat model implementation.
//!
//! This module provides the `ChatAnthropic` struct which implements the
//! `ChatModel` trait for Anthropic's Claude models.

use std::collections::HashMap;
use std::env;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use backon::{ConstantBuilder, Retryable};
use futures::Stream;
use serde::Deserialize;

use crate::ToolChoice;
use crate::callbacks::{CallbackManagerForLLMRun, Callbacks};
use crate::chat_models::{
    BaseChatModel, ChatChunk, ChatModelConfig, ChatStream, LangSmithParams, UsageMetadata,
};
use crate::error::{Error, Result};
use crate::language_models::{BaseLanguageModel, LanguageModelConfig, LanguageModelInput};
use crate::messages::{AIMessage, BaseMessage, ToolCall};
use crate::outputs::{ChatGeneration, ChatResult, LLMResult};
use crate::tools::{BaseTool, ToolDefinition};

/// Default API base URL for Anthropic.
const DEFAULT_API_BASE: &str = "https://api.anthropic.com/v1";

/// Default Anthropic API version.
const DEFAULT_API_VERSION: &str = "2023-06-01";

/// Default max tokens if not specified.
const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Anthropic chat model (Claude).
///
/// This struct implements the `ChatModel` trait for Anthropic's Claude models.
/// It follows the LangChain pattern of provider-specific implementations.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::providers::ChatAnthropic;
///
/// let model = ChatAnthropic::new("claude-sonnet-4-5-20250929")
///     .temperature(0.7)
///     .max_tokens(1024);
///
/// let messages = vec![HumanMessage::builder().content("Hello!").build().into()];
/// let response = model.generate(messages, GenerateConfig::default()).await?;
/// ```
#[derive(Debug, Clone)]
pub struct ChatAnthropic {
    /// Model name/identifier.
    model: String,
    /// Temperature for generation (0.0 - 1.0).
    temperature: Option<f64>,
    /// Maximum tokens to generate.
    max_tokens: u32,
    /// API key for authentication.
    api_key: Option<String>,
    /// Base URL for API requests.
    api_base: String,
    /// API version header.
    api_version: String,
    /// Top-k sampling parameter.
    top_k: Option<u32>,
    /// Top-p (nucleus) sampling parameter.
    top_p: Option<f64>,
    /// Stop sequences.
    stop_sequences: Option<Vec<String>>,
    /// Request timeout in seconds.
    timeout: Option<u64>,
    /// Maximum number of retries.
    max_retries: u32,
    /// Additional model kwargs.
    model_kwargs: HashMap<String, serde_json::Value>,
    /// Chat model configuration.
    chat_model_config: ChatModelConfig,
    /// Language model configuration.
    language_model_config: LanguageModelConfig,
    /// HTTP client.
    #[allow(dead_code)]
    client: reqwest::Client,
    /// Tools bound to this model via `bind_tools()`.
    bound_tools: Vec<ToolDefinition>,
    /// Tool choice for bound tools.
    bound_tool_choice: Option<ToolChoice>,
}

impl ChatAnthropic {
    /// Create a new ChatAnthropic instance.
    ///
    /// # Arguments
    ///
    /// * `model` - The model name (e.g., "claude-sonnet-4-5-20250929").
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            temperature: None,
            max_tokens: DEFAULT_MAX_TOKENS,
            api_key: None,
            api_base: DEFAULT_API_BASE.to_string(),
            api_version: DEFAULT_API_VERSION.to_string(),
            top_k: None,
            top_p: None,
            stop_sequences: None,
            timeout: None,
            max_retries: 2,
            model_kwargs: HashMap::new(),
            chat_model_config: ChatModelConfig::new(),
            language_model_config: LanguageModelConfig::new(),
            client: reqwest::Client::new(),
            bound_tools: Vec::new(),
            bound_tool_choice: None,
        }
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set the maximum tokens to generate.
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = max;
        self
    }

    /// Set the API key.
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set the API base URL.
    pub fn api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }

    /// Set the top-k parameter.
    pub fn top_k(mut self, k: u32) -> Self {
        self.top_k = Some(k);
        self
    }

    /// Set the top-p parameter.
    pub fn top_p(mut self, p: f64) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Set stop sequences.
    pub fn stop_sequences(mut self, sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(sequences);
        self
    }

    /// Set request timeout in seconds.
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(seconds);
        self
    }

    /// Set maximum retries.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Get the API key, checking environment variable if not set directly.
    fn get_api_key(&self) -> Result<String> {
        self.api_key
            .clone()
            .or_else(|| env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| Error::missing_config("ANTHROPIC_API_KEY"))
    }

    /// Build the HTTP client with configured timeout.
    fn build_client(&self) -> reqwest::Client {
        let mut builder = reqwest::Client::builder();
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(std::time::Duration::from_secs(timeout));
        }
        builder.build().unwrap_or_else(|_| reqwest::Client::new())
    }

    /// Convert messages to Anthropic API format.
    fn format_messages(
        &self,
        messages: &[BaseMessage],
    ) -> (Option<String>, Vec<serde_json::Value>) {
        let mut system_message = None;
        let mut conversation = Vec::new();

        for msg in messages {
            match msg {
                BaseMessage::System(m) => {
                    system_message = Some(m.content.as_text().to_string());
                }
                BaseMessage::Human(m) => {
                    conversation.push(serde_json::json!({
                        "role": "user",
                        "content": m.content.as_text()
                    }));
                }
                BaseMessage::AI(m) => {
                    let mut content: Vec<serde_json::Value> = Vec::new();

                    if !m.content().is_empty() {
                        content.push(serde_json::json!({
                            "type": "text",
                            "text": m.content()
                        }));
                    }

                    for tool_call in &m.tool_calls {
                        content.push(serde_json::json!({
                            "type": "tool_use",
                            "id": tool_call.id,
                            "name": tool_call.name,
                            "input": tool_call.args
                        }));
                    }

                    conversation.push(serde_json::json!({
                        "role": "assistant",
                        "content": content
                    }));
                }
                BaseMessage::Tool(m) => {
                    conversation.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": m.tool_call_id,
                            "content": m.content
                        }]
                    }));
                }
                BaseMessage::Chat(m) => {
                    // Map chat messages based on role
                    let role = match m.role.as_str() {
                        "user" | "human" => "user",
                        "assistant" | "ai" => "assistant",
                        _ => "user", // Default to user for unknown roles
                    };
                    conversation.push(serde_json::json!({
                        "role": role,
                        "content": m.content
                    }));
                }
                BaseMessage::Function(m) => {
                    // Function messages are legacy, treat like tool results
                    conversation.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": m.name, // Use function name as tool_use_id
                            "content": m.content
                        }]
                    }));
                }
                BaseMessage::Remove(_) => {
                    // RemoveMessage is used for message management, not sent to API
                    continue;
                }
            }
        }

        (system_message, conversation)
    }

    /// Build the request payload.
    fn build_request_payload(
        &self,
        messages: &[BaseMessage],
        stop: Option<Vec<String>>,
        tools: Option<&[serde_json::Value]>,
    ) -> serde_json::Value {
        let (system_message, conversation_messages) = self.format_messages(messages);

        let mut payload = serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "messages": conversation_messages
        });

        if let Some(system) = system_message {
            payload["system"] = serde_json::Value::String(system);
        }

        if let Some(temp) = self.temperature {
            payload["temperature"] = serde_json::json!(temp);
        }

        if let Some(k) = self.top_k {
            payload["top_k"] = serde_json::json!(k);
        }

        if let Some(p) = self.top_p {
            payload["top_p"] = serde_json::json!(p);
        }

        let stop_sequences = stop.or_else(|| self.stop_sequences.clone());
        if let Some(stop) = stop_sequences {
            payload["stop_sequences"] = serde_json::json!(stop);
        }

        if let Some(tools) = tools
            && !tools.is_empty()
        {
            payload["tools"] = serde_json::Value::Array(tools.to_vec());
        }

        // Add any additional model kwargs
        if let serde_json::Value::Object(ref mut obj) = payload {
            for (k, v) in &self.model_kwargs {
                obj.insert(k.clone(), v.clone());
            }
        }

        payload
    }

    /// Parse the API response into a ChatResult.
    fn parse_response(&self, response: AnthropicResponse) -> ChatResult {
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();

        for content in response.content {
            match content {
                AnthropicContent::Text { text } => {
                    text_content.push_str(&text);
                }
                AnthropicContent::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCall::builder().name(name).args(input).id(id).build());
                }
            }
        }

        let message = AIMessage::builder()
            .content(text_content)
            .tool_calls(tool_calls)
            .build();

        let generation = ChatGeneration::new(message.into());
        ChatResult::new(vec![generation])
    }

    /// Extract AIMessage from ChatResult.
    fn extract_ai_message(result: ChatResult) -> Result<AIMessage> {
        if result.generations.is_empty() {
            return Err(Error::other("No generations returned"));
        }
        match result.generations[0].message.clone() {
            BaseMessage::AI(msg) => Ok(msg),
            _ => Err(Error::other("Expected AI message")),
        }
    }
}

#[async_trait]
impl BaseLanguageModel for ChatAnthropic {
    fn llm_type(&self) -> &str {
        "anthropic-chat"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn config(&self) -> &LanguageModelConfig {
        &self.language_model_config
    }

    async fn generate_prompt(
        &self,
        prompts: Vec<LanguageModelInput>,
        stop: Option<Vec<String>>,
        _callbacks: Option<Callbacks>,
    ) -> Result<LLMResult> {
        let mut all_generations = Vec::new();
        for prompt in prompts {
            let messages = prompt.to_messages();
            let result = self
                ._generate_internal(messages, stop.clone(), None)
                .await?;
            all_generations.push(result.generations.into_iter().map(|g| g.into()).collect());
        }
        Ok(LLMResult::new(all_generations))
    }

    fn get_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        LangSmithParams {
            ls_provider: Some("anthropic".to_string()),
            ls_model_name: Some(self.model.clone()),
            ls_model_type: Some("chat".to_string()),
            ls_temperature: self.temperature,
            ls_max_tokens: Some(self.max_tokens),
            ls_stop: stop.map(|s| s.to_vec()),
        }
    }
}

#[async_trait]
impl BaseChatModel for ChatAnthropic {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.chat_model_config
    }

    async fn _generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        if !self.bound_tools.is_empty() {
            let ai_message = self
                .generate_with_tools_internal(
                    messages,
                    &self.bound_tools,
                    self.bound_tool_choice.as_ref(),
                    stop,
                )
                .await?;
            let generation = ChatGeneration::new(ai_message.into());
            return Ok(ChatResult::new(vec![generation]));
        }
        self._generate_internal(messages, stop, None).await
    }

    async fn generate_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessage> {
        self.generate_with_tools_internal(messages, tools, tool_choice, stop)
            .await
    }

    fn bind_tools(
        &self,
        tools: &[Arc<dyn BaseTool>],
        tool_choice: Option<ToolChoice>,
    ) -> Result<Box<dyn BaseChatModel>> {
        let mut bound = self.clone();
        bound.bound_tools = tools.iter().map(|t| t.definition()).collect();
        bound.bound_tool_choice = tool_choice;
        Ok(Box::new(bound))
    }

    fn with_structured_output(
        &self,
        schema: serde_json::Value,
        _include_raw: bool,
    ) -> Result<Box<dyn BaseChatModel>> {
        let name = schema
            .get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("structured_output")
            .to_string();
        let description = schema
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let tool_def = ToolDefinition {
            name,
            description,
            parameters: schema,
        };
        let mut bound = self.clone();
        bound.bound_tools = vec![tool_def];
        bound.bound_tool_choice = Some(ToolChoice::String("any".to_string()));
        Ok(Box::new(bound))
    }
}

impl ChatAnthropic {
    /// Send an HTTP request and deserialize the JSON response.
    ///
    /// Returns an `Error::Api` for non-success status codes and
    /// `Error::Http` for transport failures. The caller can use
    /// `Error::is_retryable()` to decide whether to retry.
    async fn send_json_request<T: serde::de::DeserializeOwned>(
        &self,
        payload: &serde_json::Value,
    ) -> Result<T> {
        let api_key = self.get_api_key()?;
        let client = self.build_client();

        let resp = client
            .post(format!("{}/messages", self.api_base))
            .header("x-api-key", &api_key)
            .header("anthropic-version", &self.api_version)
            .header("content-type", "application/json")
            .json(payload)
            .send()
            .await
            .map_err(Error::Http)?;

        if resp.status().is_success() {
            resp.json::<T>().await.map_err(|e| {
                Error::Json(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )))
            })
        } else {
            let status = resp.status().as_u16();
            let error_text = resp.text().await.unwrap_or_default();
            Err(Error::api(status, error_text))
        }
    }

    /// Build a `backon` retry strategy from `self.max_retries`.
    fn retry_strategy(&self) -> ConstantBuilder {
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_millis(0))
            .with_max_times(self.max_retries as usize)
    }

    /// Internal generate implementation.
    async fn _generate_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let payload = self.build_request_payload(&messages, stop, None);

        let resp: AnthropicResponse = (|| self.send_json_request(&payload))
            .retry(self.retry_strategy())
            .when(|e| e.is_retryable())
            .await?;

        Ok(self.parse_response(resp))
    }

    /// Internal generate with tools implementation.
    async fn generate_with_tools_internal(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessage> {
        // Convert tool definitions to Anthropic format
        let anthropic_tools: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters
                })
            })
            .collect();

        let tools_option = if anthropic_tools.is_empty() {
            None
        } else {
            Some(anthropic_tools.as_slice())
        };
        let mut payload = self.build_request_payload(&messages, stop, tools_option);

        // Add tool_choice if specified
        if let Some(choice) = tool_choice {
            match choice {
                ToolChoice::String(s) => {
                    match s.as_str() {
                        "auto" => payload["tool_choice"] = serde_json::json!({"type": "auto"}),
                        "any" => payload["tool_choice"] = serde_json::json!({"type": "any"}),
                        "none" => {
                            // Don't send tool_choice for None
                        }
                        _ => payload["tool_choice"] = serde_json::json!({"type": "auto"}),
                    }
                }
                ToolChoice::Structured { choice_type, name } => {
                    if (choice_type == "tool" || choice_type == "function")
                        && let Some(tool_name) = name
                    {
                        payload["tool_choice"] =
                            serde_json::json!({"type": "tool", "name": tool_name});
                    }
                }
            }
        }

        let resp: AnthropicResponse = (|| self.send_json_request(&payload))
            .retry(self.retry_strategy())
            .when(|e| e.is_retryable())
            .await?;

        let result = self.parse_response(resp);
        Self::extract_ai_message(result)
    }

    /// Internal stream implementation.
    #[allow(dead_code)]
    async fn stream_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatStream> {
        let api_key = self.get_api_key()?;
        let client = self.build_client();
        let mut payload = self.build_request_payload(&messages, stop, None);

        // Enable streaming
        payload["stream"] = serde_json::json!(true);

        let response = client
            .post(format!("{}/messages", self.api_base))
            .header("x-api-key", &api_key)
            .header("anthropic-version", &self.api_version)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(Error::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::api(status, error_text));
        }

        // Create a stream from the SSE response
        let stream = async_stream::stream! {
            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut usage: Option<UsageMetadata> = None;
            let mut stop_reason: Option<String> = None;

            use futures::StreamExt;

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete SSE events
                        while let Some(event_end) = buffer.find("\n\n") {
                            let event_data = buffer[..event_end].to_string();
                            buffer = buffer[event_end + 2..].to_string();

                            // Parse SSE event
                            for line in event_data.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        continue;
                                    }

                                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                                        match event {
                                            AnthropicStreamEvent::ContentBlockDelta { delta, .. } => {
                                                if let Some(text) = delta.text {
                                                    yield Ok(ChatChunk::new(text));
                                                }
                                            }
                                            AnthropicStreamEvent::MessageDelta { delta, usage: u } => {
                                                stop_reason = delta.stop_reason;
                                                if let Some(u) = u {
                                                    usage = Some(UsageMetadata::new(
                                                        u.input_tokens.unwrap_or(0) as i64,
                                                        u.output_tokens as i64,
                                                    ));
                                                }
                                            }
                                            AnthropicStreamEvent::MessageStop => {
                                                yield Ok(ChatChunk::final_chunk(usage.take(), stop_reason.take()));
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(Error::Http(e));
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream) as Pin<Box<dyn Stream<Item = Result<ChatChunk>> + Send>>)
    }
}

/// Anthropic API response structure.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

/// Anthropic content block.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

/// Anthropic usage information.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic streaming event types.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "message_start")]
    MessageStart {
        #[allow(dead_code)]
        message: serde_json::Value,
    },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        #[allow(dead_code)]
        index: u32,
        #[allow(dead_code)]
        content_block: serde_json::Value,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        #[allow(dead_code)]
        index: u32,
        delta: ContentDelta,
    },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop {
        #[allow(dead_code)]
        index: u32,
    },
    #[serde(rename = "message_delta")]
    MessageDelta {
        #[allow(dead_code)]
        delta: MessageDelta,
        #[allow(dead_code)]
        usage: Option<StreamUsage>,
    },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(other)]
    Unknown,
}

/// Content delta in streaming.
#[derive(Debug, Deserialize)]
struct ContentDelta {
    #[serde(rename = "type")]
    _type: Option<String>,
    text: Option<String>,
}

/// Message delta in streaming.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MessageDelta {
    stop_reason: Option<String>,
}

/// Stream usage information.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamUsage {
    input_tokens: Option<u32>,
    output_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let model = ChatAnthropic::new("claude-sonnet-4-5-20250929");
        assert_eq!(model.model, "claude-sonnet-4-5-20250929");
        assert_eq!(model.max_tokens, DEFAULT_MAX_TOKENS);
        assert!(model.temperature.is_none());
    }

    #[test]
    fn test_builder_methods() {
        let model = ChatAnthropic::new("claude-sonnet-4-5-20250929")
            .temperature(0.7)
            .max_tokens(1024)
            .top_k(40)
            .top_p(0.9)
            .api_key("test-key");

        assert_eq!(model.temperature, Some(0.7));
        assert_eq!(model.max_tokens, 1024);
        assert_eq!(model.top_k, Some(40));
        assert_eq!(model.top_p, Some(0.9));
        assert_eq!(model.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_llm_type() {
        let model = ChatAnthropic::new("claude-sonnet-4-5-20250929");
        assert_eq!(model.llm_type(), "anthropic-chat");
    }
}
