//! OpenAI chat model implementation.
//!
//! This module provides the `ChatOpenAI` struct which implements the
//! `ChatModel` trait for OpenAI's GPT models.
//!
//! # Built-in Tools (Responses API)
//!
//! OpenAI's Responses API supports built-in server-side tools like web search.
//! These can be used via `ChatOpenAI::with_builtin_tools()`:
//!
//! ```ignore
//! use agent_chain_core::providers::ChatOpenAI;
//! use agent_chain_core::providers::openai::BuiltinTool;
//!
//! let model = ChatOpenAI::new("gpt-4o")
//!     .with_responses_api(true)
//!     .with_builtin_tools(vec![BuiltinTool::WebSearch]);
//!
//! let messages = vec![HumanMessage::builder().content("What is the latest news?").build().into()];
//! let response = model.generate(messages, None).await?;
//! ```
//!
//! # Streaming with Responses API
//!
//! The Responses API also supports streaming for real-time token output:
//!
//! ```ignore
//! use agent_chain_core::providers::ChatOpenAI;
//! use agent_chain_core::providers::openai::BuiltinTool;
//! use futures::StreamExt;
//!
//! let model = ChatOpenAI::new("gpt-4o")
//!     .with_builtin_tools(vec![BuiltinTool::WebSearch]);
//!
//! let messages = vec![HumanMessage::builder().content("What is the latest news?").build().into()];
//! let mut stream = model.stream(messages, None).await?;
//!
//! while let Some(chunk) = stream.next().await {
//!     match chunk {
//!         Ok(c) => print!("{}", c.content),
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::env;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::ToolChoice;
use crate::callbacks::AsyncCallbackManagerForLLMRun;
use crate::callbacks::CallbackManagerForLLMRun;
use crate::callbacks::Callbacks;
use crate::chat_models::{
    BaseChatModel, ChatChunk, ChatModelConfig, ChatStream, LangSmithParams, UsageMetadata,
};
use crate::error::{Error, Result};
use crate::language_models::ChatGenerationStream;
use crate::language_models::{BaseLanguageModel, LanguageModelConfig, LanguageModelInput};
use crate::messages::{
    AIMessage, BaseMessage, ContentPart, ImageDetail, ImageSource, MessageContent, ToolCall,
};
use crate::outputs::ChatGenerationChunk;
use crate::outputs::{ChatGeneration, ChatResult, LLMResult};
use crate::tools::ToolDefinition;

/// Default API base URL for OpenAI.
const DEFAULT_API_BASE: &str = "https://api.openai.com/v1";

/// Built-in tools supported by OpenAI's Responses API.
///
/// These are server-side tools that OpenAI executes automatically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinTool {
    /// Web search tool - searches the web for relevant information.
    WebSearch,
    /// Web search preview (older version).
    WebSearchPreview,
    /// File search tool - searches through uploaded files.
    FileSearch,
    /// Code interpreter tool - executes code.
    CodeInterpreter,
    /// Computer use preview tool.
    ComputerUsePreview,
    /// Image generation tool.
    ImageGeneration,
}

impl BuiltinTool {
    /// Convert to OpenAI API format.
    pub fn to_api_format(&self) -> serde_json::Value {
        match self {
            BuiltinTool::WebSearch => serde_json::json!({"type": "web_search"}),
            BuiltinTool::WebSearchPreview => serde_json::json!({"type": "web_search_preview"}),
            BuiltinTool::FileSearch => serde_json::json!({"type": "file_search"}),
            BuiltinTool::CodeInterpreter => serde_json::json!({"type": "code_interpreter"}),
            BuiltinTool::ComputerUsePreview => serde_json::json!({"type": "computer_use_preview"}),
            BuiltinTool::ImageGeneration => serde_json::json!({"type": "image_generation"}),
        }
    }
}

/// Text annotation in a response (e.g., citations from web search).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextAnnotation {
    /// Type of annotation (e.g., "url_citation").
    #[serde(rename = "type")]
    pub annotation_type: String,
    /// Start index in the text.
    pub start_index: Option<u32>,
    /// End index in the text.
    pub end_index: Option<u32>,
    /// URL for URL citations.
    pub url: Option<String>,
    /// Title for URL citations.
    pub title: Option<String>,
}

/// Content block in a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Text content with optional annotations.
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(default)]
        annotations: Vec<TextAnnotation>,
    },
    /// Output text (Responses API format).
    #[serde(rename = "output_text")]
    OutputText {
        text: String,
        #[serde(default)]
        annotations: Vec<TextAnnotation>,
    },
    /// Refusal content.
    #[serde(rename = "refusal")]
    Refusal { refusal: String },
}

/// OpenAI chat model (GPT).
///
/// This struct implements the `ChatModel` trait for OpenAI's GPT models.
/// It follows the LangChain pattern of provider-specific implementations.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::providers::ChatOpenAI;
///
/// let model = ChatOpenAI::new("gpt-4o")
///     .temperature(0.7)
///     .max_tokens(1024);
///
/// let messages = vec![HumanMessage::builder().content("Hello!").build().into()];
/// let response = model.generate(messages, None).await?;
/// ```
///
/// # Using Built-in Tools (Responses API)
///
/// ```ignore
/// use agent_chain_core::providers::ChatOpenAI;
/// use agent_chain_core::providers::openai::BuiltinTool;
///
/// let model = ChatOpenAI::new("gpt-4o")
///     .with_responses_api(true)
///     .with_builtin_tools(vec![BuiltinTool::WebSearch]);
///
/// let messages = vec![HumanMessage::builder().content("What happened today?").build().into()];
/// let response = model.generate(messages, None).await?;
/// // Response will include web search results with citations
/// ```
#[derive(Debug, Clone)]
pub struct ChatOpenAI {
    /// Model name/identifier.
    model: String,
    /// Temperature for generation (0.0 - 2.0).
    temperature: Option<f64>,
    /// Maximum tokens to generate.
    max_tokens: Option<u32>,
    /// API key for authentication.
    api_key: Option<String>,
    /// Base URL for API requests.
    api_base: String,
    /// Organization ID.
    organization: Option<String>,
    /// Top-p (nucleus) sampling parameter.
    top_p: Option<f64>,
    /// Frequency penalty.
    frequency_penalty: Option<f64>,
    /// Presence penalty.
    presence_penalty: Option<f64>,
    /// Stop sequences.
    stop: Option<Vec<String>>,
    /// Request timeout in seconds.
    timeout: Option<u64>,
    /// Maximum number of retries.
    max_retries: u32,
    /// Additional model kwargs.
    model_kwargs: HashMap<String, serde_json::Value>,
    /// Whether to stream responses.
    streaming: bool,
    /// Seed for generation.
    seed: Option<i32>,
    /// Whether to return logprobs.
    logprobs: Option<bool>,
    /// Number of most likely tokens to return at each position.
    top_logprobs: Option<u32>,
    /// Modify likelihood of specified tokens.
    logit_bias: Option<HashMap<i32, i32>>,
    /// Number of chat completions to generate.
    n: Option<u32>,
    /// Reasoning effort for reasoning models (Chat Completions API).
    reasoning_effort: Option<String>,
    /// Reasoning parameters for reasoning models (Responses API).
    reasoning: Option<HashMap<String, serde_json::Value>>,
    /// Verbosity level for reasoning models (Responses API).
    verbosity: Option<String>,
    /// Whether to include usage metadata in streaming output.
    stream_usage: Option<bool>,
    /// Additional fields to include in Responses API generations.
    include: Option<Vec<String>>,
    /// Latency tier for request ('auto', 'default', or 'flex').
    service_tier: Option<String>,
    /// Whether OpenAI may store response data.
    store: Option<bool>,
    /// Truncation strategy for Responses API ('auto' or 'disabled').
    truncation: Option<String>,
    /// Whether to use the Responses API instead of Chat Completions API.
    use_responses_api: Option<bool>,
    /// Whether to pass previous_response_id automatically.
    use_previous_response_id: bool,
    /// Version of AIMessage output format.
    output_version: Option<String>,
    /// Built-in tools to use with the Responses API.
    builtin_tools: Vec<BuiltinTool>,
    /// Parameters to disable for certain models.
    disabled_params: Option<HashMap<String, Option<serde_json::Value>>>,
    /// Extra body parameters to pass to the API.
    extra_body: Option<HashMap<String, serde_json::Value>>,
    /// Chat model configuration.
    chat_model_config: ChatModelConfig,
    /// Language model configuration.
    language_model_config: LanguageModelConfig,
    /// HTTP client.
    #[allow(dead_code)]
    client: reqwest::Client,
}

impl ChatOpenAI {
    /// Create a new ChatOpenAI instance.
    ///
    /// # Arguments
    ///
    /// * `model` - The model name (e.g., "gpt-4o", "gpt-4-turbo").
    pub fn new(model: impl Into<String>) -> Self {
        let model_name = model.into();

        // Validate and set temperature for o1 models (match Python behavior)
        let temperature = if model_name.to_lowercase().starts_with("o1") {
            Some(1.0)
        } else {
            None
        };

        Self {
            model: model_name,
            temperature,
            max_tokens: None,
            api_key: None,
            api_base: DEFAULT_API_BASE.to_string(),
            organization: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            timeout: None,
            max_retries: 2,
            model_kwargs: HashMap::new(),
            streaming: false,
            seed: None,
            logprobs: None,
            top_logprobs: None,
            logit_bias: None,
            n: None,
            reasoning_effort: None,
            reasoning: None,
            verbosity: None,
            stream_usage: None,
            include: None,
            service_tier: None,
            store: None,
            truncation: None,
            use_responses_api: None,
            use_previous_response_id: false,
            output_version: None,
            builtin_tools: Vec::new(),
            disabled_params: None,
            extra_body: None,
            chat_model_config: ChatModelConfig::new(),
            language_model_config: LanguageModelConfig::new(),
            client: reqwest::Client::new(),
        }
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set the maximum tokens to generate.
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
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

    /// Set the organization ID.
    pub fn organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    /// Set the top-p parameter.
    pub fn top_p(mut self, p: f64) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Set the frequency penalty.
    pub fn frequency_penalty(mut self, penalty: f64) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    /// Set the presence penalty.
    pub fn presence_penalty(mut self, penalty: f64) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    /// Set stop sequences.
    pub fn stop(mut self, sequences: Vec<String>) -> Self {
        self.stop = Some(sequences);
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

    /// Enable streaming mode.
    pub fn streaming(mut self, enabled: bool) -> Self {
        self.streaming = enabled;
        self
    }

    /// Set the seed for generation.
    pub fn seed(mut self, seed: i32) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set whether to return logprobs.
    pub fn logprobs(mut self, enabled: bool) -> Self {
        self.logprobs = Some(enabled);
        self
    }

    /// Set number of most likely tokens to return at each position.
    pub fn top_logprobs(mut self, count: u32) -> Self {
        self.top_logprobs = Some(count);
        self
    }

    /// Set logit bias for specified tokens.
    pub fn logit_bias(mut self, bias: HashMap<i32, i32>) -> Self {
        self.logit_bias = Some(bias);
        self
    }

    /// Set number of completions to generate.
    pub fn n(mut self, count: u32) -> Self {
        self.n = Some(count);
        self
    }

    /// Set reasoning effort for reasoning models (Chat Completions API).
    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    /// Set reasoning parameters for reasoning models (Responses API).
    pub fn reasoning(mut self, params: HashMap<String, serde_json::Value>) -> Self {
        self.reasoning = Some(params);
        self
    }

    /// Set verbosity level for reasoning models (Responses API).
    pub fn verbosity(mut self, level: impl Into<String>) -> Self {
        self.verbosity = Some(level.into());
        self
    }

    /// Set whether to include usage metadata in streaming output.
    pub fn stream_usage(mut self, enabled: bool) -> Self {
        self.stream_usage = Some(enabled);
        self
    }

    /// Set additional fields to include in Responses API generations.
    pub fn include(mut self, fields: Vec<String>) -> Self {
        self.include = Some(fields);
        self
    }

    /// Set latency tier for request.
    pub fn service_tier(mut self, tier: impl Into<String>) -> Self {
        self.service_tier = Some(tier.into());
        self
    }

    /// Set whether OpenAI may store response data.
    pub fn store(mut self, enabled: bool) -> Self {
        self.store = Some(enabled);
        self
    }

    /// Set truncation strategy for Responses API.
    pub fn truncation(mut self, strategy: impl Into<String>) -> Self {
        self.truncation = Some(strategy.into());
        self
    }

    /// Set whether to pass previous_response_id automatically.
    pub fn use_previous_response_id(mut self, enabled: bool) -> Self {
        self.use_previous_response_id = enabled;
        self
    }

    /// Set AIMessage output format version.
    pub fn output_version(mut self, version: impl Into<String>) -> Self {
        self.output_version = Some(version.into());
        self
    }

    /// Enable or disable the Responses API.
    ///
    /// The Responses API is required for built-in tools like web search.
    pub fn with_responses_api(mut self, enabled: bool) -> Self {
        self.use_responses_api = Some(enabled);
        self
    }

    /// Set built-in tools to use with the Responses API.
    ///
    /// This automatically enables the Responses API.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use agent_chain_core::providers::ChatOpenAI;
    /// use agent_chain_core::providers::openai::BuiltinTool;
    ///
    /// let model = ChatOpenAI::new("gpt-4o")
    ///     .with_builtin_tools(vec![BuiltinTool::WebSearch]);
    /// ```
    pub fn with_builtin_tools(mut self, tools: Vec<BuiltinTool>) -> Self {
        self.builtin_tools = tools;
        if !self.builtin_tools.is_empty() {
            self.use_responses_api = Some(true);
        }
        self
    }

    /// Set disabled parameters.
    pub fn disabled_params(mut self, params: HashMap<String, Option<serde_json::Value>>) -> Self {
        self.disabled_params = Some(params);
        self
    }

    /// Set extra body parameters.
    pub fn extra_body(mut self, body: HashMap<String, serde_json::Value>) -> Self {
        self.extra_body = Some(body);
        self
    }

    /// Filter out disabled parameters from a payload.
    fn filter_disabled_params(&self, payload: &mut serde_json::Value) {
        if let Some(ref disabled) = self.disabled_params
            && let Some(obj) = payload.as_object_mut()
        {
            for (key, default_value) in disabled {
                if let Some(default) = default_value {
                    // Replace with default value
                    obj.insert(key.clone(), default.clone());
                } else {
                    // Remove the parameter
                    obj.remove(key);
                }
            }
        }
    }

    /// Helper function to determine if Responses API should be used.
    /// Matches Python's _use_responses_api logic.
    pub fn should_use_responses_api(&self, has_builtin_tools: bool) -> bool {
        // Explicit setting takes precedence
        if let Some(use_api) = self.use_responses_api {
            return use_api;
        }

        // Check if we have parameters that require Responses API
        if has_builtin_tools
            || self.reasoning.is_some()
            || self.verbosity.is_some()
            || self.truncation.is_some()
            || self.include.is_some()
            || self.use_previous_response_id
        {
            return true;
        }

        // Check if output_version requires Responses API
        if self.output_version.as_deref() == Some("responses/v1") {
            return true;
        }

        // Check if model prefers Responses API
        if self.model.to_lowercase().contains("gpt-5.2-pro") {
            return true;
        }

        false
    }

    /// Helper to check if a tool is a built-in tool (not a function).
    #[allow(dead_code)]
    fn is_builtin_tool(tool: &serde_json::Value) -> bool {
        if let Some(tool_type) = tool.get("type").and_then(|t| t.as_str()) {
            tool_type != "function"
        } else {
            false
        }
    }

    /// Get the API key, checking environment variable if not set directly.
    fn get_api_key(&self) -> Result<String> {
        self.api_key
            .clone()
            .or_else(|| env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| Error::missing_config("OPENAI_API_KEY"))
    }

    /// Build the HTTP client with configured timeout.
    fn build_client(&self) -> reqwest::Client {
        let mut builder = reqwest::Client::builder();
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(std::time::Duration::from_secs(timeout));
        }
        builder.build().unwrap_or_else(|_| reqwest::Client::new())
    }

    /// Convert messages to OpenAI API format.
    pub fn format_messages(&self, messages: &[BaseMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .filter_map(|msg| match msg {
                BaseMessage::System(m) => Some(serde_json::json!({
                    "role": "system",
                    "content": m.content.as_text()
                })),
                BaseMessage::Human(m) => {
                    let content = match &m.content {
                        MessageContent::Text(text) => serde_json::json!(text),
                        MessageContent::Parts(parts) => {
                            let content_parts: Vec<serde_json::Value> = parts
                                .iter()
                                .map(|part| match part {
                                    ContentPart::Text { text } => {
                                        serde_json::json!({
                                            "type": "text",
                                            "text": text
                                        })
                                    }
                                    ContentPart::Image { source, detail } => {
                                        let url = match source {
                                            ImageSource::Url { url } => url.clone(),
                                            ImageSource::Base64 { media_type, data } => {
                                                format!("data:{};base64,{}", media_type, data)
                                            }
                                            ImageSource::FileId { file_id } => file_id.clone(),
                                        };
                                        let mut image_url = serde_json::json!({ "url": url });
                                        if let Some(d) = detail {
                                            image_url["detail"] = serde_json::json!(match d {
                                                ImageDetail::Low => "low",
                                                ImageDetail::High => "high",
                                                ImageDetail::Auto => "auto",
                                            });
                                        }
                                        serde_json::json!({
                                            "type": "image_url",
                                            "image_url": image_url
                                        })
                                    }
                                    ContentPart::Other(value) => value.clone(),
                                })
                                .collect();
                            serde_json::Value::Array(content_parts)
                        }
                    };
                    Some(serde_json::json!({
                        "role": "user",
                        "content": content
                    }))
                }
                BaseMessage::AI(m) => {
                    let mut message = serde_json::json!({
                        "role": "assistant",
                    });

                    if !m.content().is_empty() {
                        message["content"] = serde_json::json!(m.content());
                    }

                    // Combine tool_calls and invalid_tool_calls
                    let mut all_tool_calls: Vec<serde_json::Value> = m
                        .tool_calls
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.args.to_string()
                                }
                            })
                        })
                        .collect();

                    // Add invalid tool calls (sent as-is for error recovery)
                    for itc in &m.invalid_tool_calls {
                        all_tool_calls.push(serde_json::json!({
                            "id": itc.id,
                            "type": "function",
                            "function": {
                                "name": itc.name,
                                "arguments": itc.args.as_deref().unwrap_or("")
                            }
                        }));
                    }

                    if !all_tool_calls.is_empty() {
                        message["tool_calls"] = serde_json::Value::Array(all_tool_calls);
                    }

                    Some(message)
                }
                BaseMessage::Tool(m) => Some(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": m.tool_call_id,
                    "content": m.content
                })),
                BaseMessage::Remove(_) => {
                    // RemoveMessage is used for message management, not sent to API
                    None
                }
                BaseMessage::Chat(m) => Some(serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })),
                BaseMessage::Function(m) => Some(serde_json::json!({
                    "role": "function",
                    "name": m.name,
                    "content": m.content
                })),
            })
            .collect()
    }

    /// Build the request payload.
    pub fn build_request_payload(
        &self,
        messages: &[BaseMessage],
        stop: Option<Vec<String>>,
        tools: Option<&[serde_json::Value]>,
        stream: bool,
    ) -> serde_json::Value {
        let formatted_messages = self.format_messages(messages);

        let mut payload = serde_json::json!({
            "model": self.model,
            "messages": formatted_messages
        });

        if let Some(max_tokens) = self.max_tokens {
            payload["max_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(temp) = self.temperature {
            payload["temperature"] = serde_json::json!(temp);
        }

        if let Some(p) = self.top_p {
            payload["top_p"] = serde_json::json!(p);
        }

        if let Some(fp) = self.frequency_penalty {
            payload["frequency_penalty"] = serde_json::json!(fp);
        }

        if let Some(pp) = self.presence_penalty {
            payload["presence_penalty"] = serde_json::json!(pp);
        }

        let stop_sequences = stop.or_else(|| self.stop.clone());
        if let Some(stop) = stop_sequences {
            payload["stop"] = serde_json::json!(stop);
        }

        if let Some(tools) = tools
            && !tools.is_empty()
        {
            payload["tools"] = serde_json::Value::Array(tools.to_vec());
        }

        if stream {
            payload["stream"] = serde_json::json!(true);
        }

        // Add optional parameters
        if let Some(ref effort) = self.reasoning_effort {
            payload["reasoning_effort"] = serde_json::json!(effort);
        }

        if let Some(seed) = self.seed {
            payload["seed"] = serde_json::json!(seed);
        }

        if let Some(logprobs) = self.logprobs {
            payload["logprobs"] = serde_json::json!(logprobs);
        }

        if let Some(top_logprobs) = self.top_logprobs {
            payload["top_logprobs"] = serde_json::json!(top_logprobs);
        }

        if let Some(ref bias) = self.logit_bias {
            payload["logit_bias"] = serde_json::json!(bias);
        }

        if let Some(n) = self.n {
            payload["n"] = serde_json::json!(n);
        }

        if let Some(ref service_tier) = self.service_tier {
            payload["service_tier"] = serde_json::json!(service_tier);
        }

        if let Some(store) = self.store {
            payload["store"] = serde_json::json!(store);
        }

        // Add any additional model kwargs
        if let Some(obj) = payload.as_object_mut() {
            for (k, v) in &self.model_kwargs {
                obj.insert(k.clone(), v.clone());
            }
        }

        // Add extra_body parameters
        if let Some(ref extra) = self.extra_body
            && let Some(obj) = payload.as_object_mut()
        {
            for (k, v) in extra {
                obj.insert(k.clone(), v.clone());
            }
        }

        // Filter out disabled parameters
        self.filter_disabled_params(&mut payload);

        payload
    }

    /// Build the request payload for the Responses API.
    pub fn build_responses_api_payload(
        &self,
        messages: &[BaseMessage],
        stop: Option<Vec<String>>,
        tools: Option<&[serde_json::Value]>,
        stream: bool,
    ) -> serde_json::Value {
        let input = self.format_messages_for_responses_api(messages);

        let mut payload = serde_json::json!({
            "model": self.model,
            "input": input
        });

        if let Some(max_tokens) = self.max_tokens {
            payload["max_output_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(temp) = self.temperature {
            payload["temperature"] = serde_json::json!(temp);
        }

        if let Some(p) = self.top_p {
            payload["top_p"] = serde_json::json!(p);
        }

        let stop_sequences = stop.or_else(|| self.stop.clone());
        if let Some(stop) = stop_sequences {
            payload["stop"] = serde_json::json!(stop);
        }

        // Add built-in tools
        let mut all_tools: Vec<serde_json::Value> = self
            .builtin_tools
            .iter()
            .map(|t| t.to_api_format())
            .collect();

        // Add function tools
        if let Some(tools) = tools {
            for tool in tools {
                // Convert from Chat Completions format to Responses API format
                if let Some(function) = tool.get("function") {
                    all_tools.push(serde_json::json!({
                        "type": "function",
                        "name": function.get("name"),
                        "description": function.get("description"),
                        "parameters": function.get("parameters")
                    }));
                } else {
                    all_tools.push(tool.clone());
                }
            }
        }

        if !all_tools.is_empty() {
            payload["tools"] = serde_json::Value::Array(all_tools);
        }

        if stream {
            payload["stream"] = serde_json::json!(true);
        }

        // Add reasoning parameters
        if let Some(ref reasoning) = self.reasoning {
            payload["reasoning"] = serde_json::json!(reasoning);
        } else if let Some(ref effort) = self.reasoning_effort {
            payload["reasoning"] = serde_json::json!({"effort": effort});
        }

        // Add verbosity
        if let Some(ref verbosity) = self.verbosity {
            payload["text"] =
                serde_json::json!({"format": {"type": "text"}, "verbosity": verbosity});
        }

        // Add include fields
        if let Some(ref include) = self.include {
            payload["include"] = serde_json::json!(include);
        }

        // Add truncation
        if let Some(ref truncation) = self.truncation {
            payload["truncation"] = serde_json::json!(truncation);
        }

        // Add service_tier
        if let Some(ref service_tier) = self.service_tier {
            payload["service_tier"] = serde_json::json!(service_tier);
        }

        // Add store
        if let Some(store) = self.store {
            payload["store"] = serde_json::json!(store);
        }

        // Add any additional model kwargs
        if let Some(obj) = payload.as_object_mut() {
            for (k, v) in &self.model_kwargs {
                obj.insert(k.clone(), v.clone());
            }
        }

        payload
    }

    /// Build the request payload for the Responses API (non-streaming).
    #[allow(dead_code)]
    fn build_responses_api_payload_non_streaming(
        &self,
        messages: &[BaseMessage],
        stop: Option<Vec<String>>,
        tools: Option<&[serde_json::Value]>,
    ) -> serde_json::Value {
        let input = self.format_messages_for_responses_api(messages);

        let mut payload = serde_json::json!({
            "model": self.model,
            "input": input
        });

        if let Some(max_tokens) = self.max_tokens {
            payload["max_output_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(temp) = self.temperature {
            payload["temperature"] = serde_json::json!(temp);
        }

        if let Some(p) = self.top_p {
            payload["top_p"] = serde_json::json!(p);
        }

        let stop_sequences = stop.or_else(|| self.stop.clone());
        if let Some(stop) = stop_sequences {
            payload["stop"] = serde_json::json!(stop);
        }

        // Add built-in tools
        let mut all_tools: Vec<serde_json::Value> = self
            .builtin_tools
            .iter()
            .map(|t| t.to_api_format())
            .collect();

        // Add function tools
        if let Some(tools) = tools {
            for tool in tools {
                // Convert from Chat Completions format to Responses API format
                if let Some(function) = tool.get("function") {
                    all_tools.push(serde_json::json!({
                        "type": "function",
                        "name": function.get("name"),
                        "description": function.get("description"),
                        "parameters": function.get("parameters")
                    }));
                } else {
                    all_tools.push(tool.clone());
                }
            }
        }

        if !all_tools.is_empty() {
            payload["tools"] = serde_json::Value::Array(all_tools);
        }

        // Add any additional model kwargs
        if let Some(obj) = payload.as_object_mut() {
            for (k, v) in &self.model_kwargs {
                obj.insert(k.clone(), v.clone());
            }
        }

        payload
    }

    /// Stream responses using the Responses API.
    async fn stream_responses_api(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatStream> {
        let api_key = self.get_api_key()?;
        let client = self.build_client();
        let payload = self.build_responses_api_payload(&messages, stop, None, true);

        let mut request = client
            .post(format!("{}/responses", self.api_base))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json");

        if let Some(ref org) = self.organization {
            request = request.header("OpenAI-Organization", org);
        }

        let response = request.json(&payload).send().await.map_err(Error::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::api(status, error_text));
        }

        let stream = async_stream::stream! {
            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut accumulated_text = String::new();
            let mut usage: Option<UsageMetadata> = None;
            let mut finish_reason: Option<String> = None;
            let mut annotations: Vec<TextAnnotation> = Vec::new();

            use futures::StreamExt;

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete SSE events (lines ending with \n\n or single \n for data lines)
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].to_string();
                            buffer = buffer[line_end + 1..].to_string();

                            // Skip empty lines
                            if line.is_empty() || line == "\r" {
                                continue;
                            }

                            // Parse SSE data lines
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    // Final chunk with metadata
                                    yield Ok(ChatChunk::final_chunk(usage.take(), finish_reason.take()));
                                    continue;
                                }

                                // Parse the JSON event
                                if let Ok(event) = serde_json::from_str::<ResponsesStreamEvent>(data) {
                                    match event.event_type.as_str() {
                                        "response.output_text.delta" => {
                                            // Text content delta
                                            if let Some(delta) = event.delta {
                                                accumulated_text.push_str(&delta);
                                                yield Ok(ChatChunk::new(delta));
                                            }
                                        }
                                        "response.output_text.annotation.added" => {
                                            // Annotation added
                                            if let Some(annotation) = event.annotation {
                                                annotations.push(annotation);
                                            }
                                        }
                                        "response.completed" | "response.incomplete" => {
                                            // Response complete - extract usage and status from the response
                                            if let Some(resp) = event.response {
                                                if let Some(resp_usage) = resp.usage {
                                                    usage = Some(UsageMetadata::new(
                                                        resp_usage.input_tokens as i64,
                                                        resp_usage.output_tokens as i64,
                                                    ));
                                                }
                                                // Set finish_reason based on status
                                                finish_reason = resp.status;
                                            }
                                            // Final chunk with metadata
                                            yield Ok(ChatChunk::final_chunk(usage.take(), finish_reason.take()));
                                        }
                                        "response.function_call_arguments.delta" => {
                                            // Function call arguments delta
                                            if let Some(delta) = event.delta {
                                                // We could accumulate this but for now just note it
                                                // The full function call will be in the completed response
                                                let _ = delta;
                                            }
                                        }
                                        "response.refusal.delta" => {
                                            // Refusal content delta
                                            if let Some(delta) = event.delta {
                                                yield Ok(ChatChunk::new(delta));
                                            }
                                        }
                                        _ => {
                                            // Other event types (response.created, response.output_item.added, etc.)
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

    /// Format messages for the Responses API.
    pub fn format_messages_for_responses_api(
        &self,
        messages: &[BaseMessage],
    ) -> Vec<serde_json::Value> {
        let mut input = Vec::new();

        for msg in messages {
            match msg {
                BaseMessage::System(m) => {
                    input.push(serde_json::json!({
                        "role": "system",
                        "content": m.content.as_text()
                    }));
                }
                BaseMessage::Human(m) => {
                    let content = match &m.content {
                        MessageContent::Text(text) => serde_json::json!(text),
                        MessageContent::Parts(parts) => {
                            let content_parts: Vec<serde_json::Value> = parts
                                .iter()
                                .map(|part| match part {
                                    ContentPart::Text { text } => {
                                        serde_json::json!({
                                            "type": "input_text",
                                            "text": text
                                        })
                                    }
                                    ContentPart::Image { source, detail } => {
                                        let url = match source {
                                            ImageSource::Url { url } => url.clone(),
                                            ImageSource::Base64 { media_type, data } => {
                                                format!("data:{};base64,{}", media_type, data)
                                            }
                                            ImageSource::FileId { file_id } => file_id.clone(),
                                        };
                                        let mut block = serde_json::json!({
                                            "type": "input_image",
                                            "image_url": url
                                        });
                                        if let Some(d) = detail {
                                            block["detail"] = serde_json::json!(match d {
                                                ImageDetail::Low => "low",
                                                ImageDetail::High => "high",
                                                ImageDetail::Auto => "auto",
                                            });
                                        }
                                        block
                                    }
                                    ContentPart::Other(value) => value.clone(),
                                })
                                .collect();
                            serde_json::Value::Array(content_parts)
                        }
                    };
                    input.push(serde_json::json!({
                        "role": "user",
                        "content": content
                    }));
                }
                BaseMessage::AI(m) => {
                    // Add message content
                    if !m.content().is_empty() || m.tool_calls.is_empty() {
                        input.push(serde_json::json!({
                            "type": "message",
                            "role": "assistant",
                            "content": [{
                                "type": "output_text",
                                "text": m.content(),
                                "annotations": []
                            }]
                        }));
                    }

                    // Add function calls
                    for tc in &m.tool_calls {
                        input.push(serde_json::json!({
                            "type": "function_call",
                            "name": tc.name,
                            "arguments": tc.args.to_string(),
                            "call_id": tc.id
                        }));
                    }
                }
                BaseMessage::Tool(m) => {
                    input.push(serde_json::json!({
                        "type": "function_call_output",
                        "call_id": m.tool_call_id,
                        "output": m.content
                    }));
                }
                BaseMessage::Remove(_) => {
                    // RemoveMessage is used for message management, not sent to API
                    continue;
                }
                BaseMessage::Chat(m) => {
                    input.push(serde_json::json!({
                        "role": m.role,
                        "content": m.content
                    }));
                }
                BaseMessage::Function(m) => {
                    input.push(serde_json::json!({
                        "type": "function_call_output",
                        "name": m.name,
                        "output": m.content
                    }));
                }
            }
        }

        input
    }

    /// Parse the API response into a ChatResult.
    fn parse_response(&self, response: OpenAIResponse) -> ChatResult {
        // Extract finish_reason before consuming choices
        let _finish_reason = response
            .choices
            .first()
            .and_then(|c| c.finish_reason.clone());

        let choice = response.choices.into_iter().next();

        let (content, tool_calls) = match choice {
            Some(c) => {
                let content = c.message.content.unwrap_or_default();
                let tool_calls: Vec<ToolCall> = c
                    .message
                    .tool_calls
                    .unwrap_or_default()
                    .into_iter()
                    .map(|tc| {
                        let args: serde_json::Value =
                            serde_json::from_str(&tc.function.arguments).unwrap_or_default();
                        ToolCall::builder()
                            .name(tc.function.name)
                            .args(args)
                            .id(tc.id)
                            .build()
                    })
                    .collect();
                (content, tool_calls)
            }
            None => (String::new(), Vec::new()),
        };

        let ai_message = AIMessage::builder().content(content).tool_calls(tool_calls);
        let ai_message = match response.usage {
            Some(ref usage) => ai_message
                .usage_metadata(Self::create_usage_metadata(usage))
                .build(),
            None => ai_message.build(),
        };

        let mut response_metadata = HashMap::new();
        response_metadata.insert("model_name".to_string(), serde_json::json!(response.model));

        let generation = ChatGeneration::new(BaseMessage::AI(
            AIMessage::builder()
                .content(ai_message.content.clone())
                .tool_calls(ai_message.tool_calls.clone())
                .maybe_usage_metadata(ai_message.usage_metadata.clone())
                .response_metadata(response_metadata)
                .build(),
        ));
        ChatResult::new(vec![generation])
    }

    /// Parse the Responses API response into a ChatResult.
    fn parse_responses_api_response(&self, response: ResponsesApiResponse) -> ChatResult {
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();
        let mut annotations: Vec<TextAnnotation> = Vec::new();

        for output in &response.output {
            match output {
                ResponsesOutput::Message { content, .. } => {
                    for block in content {
                        if let ResponsesContent::OutputText {
                            text,
                            annotations: anns,
                        } = block
                        {
                            text_content.push_str(text);
                            annotations.extend(anns.clone());
                        }
                    }
                }
                ResponsesOutput::FunctionCall {
                    name,
                    arguments,
                    call_id,
                    ..
                } => {
                    let args: serde_json::Value =
                        serde_json::from_str(arguments).unwrap_or_default();
                    tool_calls.push(
                        ToolCall::builder()
                            .name(name.clone())
                            .args(args)
                            .id(call_id.clone())
                            .build(),
                    );
                }
                ResponsesOutput::WebSearchCall { .. } => {
                    // Web search is handled internally by OpenAI
                    // Results are included in the text content with annotations
                }
                ResponsesOutput::Other(_) => {}
            }
        }

        let ai_message = AIMessage::builder()
            .content(text_content)
            .tool_calls(tool_calls);
        let usage_metadata = response
            .usage
            .as_ref()
            .map(|u| UsageMetadata::new(u.input_tokens as i64, u.output_tokens as i64));

        let ai_message = if let Some(usage) = usage_metadata {
            ai_message.usage_metadata(usage).build()
        } else {
            ai_message.build()
        };

        let mut response_metadata = HashMap::new();
        response_metadata.insert("model_name".to_string(), serde_json::json!(response.model));
        if let Some(ref status) = response.status {
            response_metadata.insert("status".to_string(), serde_json::json!(status));
        }

        let generation = ChatGeneration::new(BaseMessage::AI(
            AIMessage::builder()
                .content(ai_message.content.clone())
                .tool_calls(ai_message.tool_calls.clone())
                .maybe_usage_metadata(ai_message.usage_metadata.clone())
                .response_metadata(response_metadata)
                .build(),
        ));
        ChatResult::new(vec![generation])
    }

    /// Generate using the Responses API.
    async fn generate_responses_api(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatResult> {
        let api_key = self.get_api_key()?;
        let client = self.build_client();
        let payload = self.build_responses_api_payload(&messages, stop, None, false);

        let mut last_error = None;
        for _ in 0..=self.max_retries {
            let mut request = client
                .post(format!("{}/responses", self.api_base))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json");

            if let Some(ref org) = self.organization {
                request = request.header("OpenAI-Organization", org);
            }

            let response = request.json(&payload).send().await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<ResponsesApiResponse>().await {
                            Ok(responses_resp) => {
                                return Ok(self.parse_responses_api_response(responses_resp));
                            }
                            Err(e) => {
                                return Err(Error::Json(serde_json::Error::io(
                                    std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        e.to_string(),
                                    ),
                                )));
                            }
                        }
                    } else {
                        let status = resp.status().as_u16();
                        let error_text = resp.text().await.unwrap_or_default();
                        last_error = Some(Error::api(status, error_text));

                        // Don't retry 4xx errors (client errors), except 429 (rate limit)
                        if (400..500).contains(&status) && status != 429 {
                            break;
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(Error::Http(e));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::other("Unknown error")))
    }
}

#[async_trait]
impl BaseLanguageModel for ChatOpenAI {
    fn llm_type(&self) -> &str {
        "openai-chat"
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
        // Convert prompts to message batches and generate
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
        // Check model_kwargs for overrides (matching Python _get_ls_params)
        let model_name = self
            .model_kwargs
            .get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.model.clone());

        let temperature = self
            .model_kwargs
            .get("temperature")
            .and_then(|v| v.as_f64())
            .or(self.temperature);

        let max_tokens = self
            .model_kwargs
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .or(self.max_tokens);

        LangSmithParams {
            ls_provider: Some("openai".to_string()),
            ls_model_name: Some(model_name),
            ls_model_type: Some("chat".to_string()),
            ls_temperature: temperature,
            ls_max_tokens: max_tokens,
            ls_stop: stop.map(|s| s.to_vec()),
        }
    }
}

#[async_trait]
impl BaseChatModel for ChatOpenAI {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.chat_model_config
    }

    /// Indicate that async streaming is implemented.
    fn has_astream_impl(&self) -> bool {
        true
    }

    async fn _generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        self._generate_internal(messages, stop, None).await
    }

    /// Async streaming implementation that calls the internal stream method
    /// and converts ChatChunk to ChatGenerationChunk.
    async fn _astream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&AsyncCallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        // Call the internal stream implementation
        let chat_stream = self.stream_internal(messages, stop).await?;

        // Convert ChatChunk stream to ChatGenerationChunk stream
        let generation_stream = async_stream::stream! {
            use futures::StreamExt;

            let mut pinned_stream = chat_stream;

            while let Some(result) = pinned_stream.next().await {
                match result {
                    Ok(chat_chunk) => {
                        // Convert ChatChunk to ChatGenerationChunk
                        let message = AIMessage::builder()
                            .content(&chat_chunk.content)
                            .tool_calls(chat_chunk.tool_calls.clone())
                            .maybe_usage_metadata(chat_chunk.usage_metadata.clone())
                            .build();
                        let generation_chunk = ChatGenerationChunk::new(message.into());
                        yield Ok(generation_chunk);
                    }
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                }
            }
        };

        Ok(Box::pin(generation_stream) as ChatGenerationStream)
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

    /// Invoke the model with input and optional stop sequences.
    ///
    /// This overrides the default `BaseChatModel::invoke` to support stop sequences.
    async fn invoke(&self, input: LanguageModelInput) -> Result<AIMessage> {
        self.invoke_with_stop(input, None).await
    }
}

impl ChatOpenAI {
    /// Invoke the model with input and optional stop sequences.
    ///
    /// This is the primary method for generating a response from the model.
    /// It converts the input to messages and calls the internal generate method.
    ///
    /// # Arguments
    ///
    /// * `input` - The input to the model (string, messages, or PromptValue).
    /// * `stop` - Optional stop sequences that will cause the model to stop generating.
    ///
    /// # Returns
    ///
    /// An `AIMessage` containing the model's response.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use agent_chain::providers::openai::ChatOpenAI;
    /// use agent_chain_core::language_models::LanguageModelInput;
    ///
    /// let model = ChatOpenAI::new("gpt-4o");
    /// let response = model.invoke_with_stop(
    ///     LanguageModelInput::from("Hello, world!"),
    ///     Some(vec!["STOP".to_string()]),
    /// ).await?;
    /// ```
    pub async fn invoke_with_stop(
        &self,
        input: LanguageModelInput,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessage> {
        let messages = input.to_messages();
        let result = self._generate_internal(messages, stop, None).await?;

        if result.generations.is_empty() {
            return Err(Error::other("No generations returned"));
        }

        match result.generations[0].message.clone() {
            BaseMessage::AI(message) => Ok(message),
            _ => Err(Error::other("Unexpected message type")),
        }
    }

    /// Internal generate implementation.
    async fn _generate_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        // Use Responses API if enabled or if using built-in tools
        if self.should_use_responses_api(!self.builtin_tools.is_empty()) {
            return self.generate_responses_api(messages, stop).await;
        }

        let api_key = self.get_api_key()?;
        let client = self.build_client();
        let payload = self.build_request_payload(&messages, stop, None, false);

        let mut last_error = None;
        for _ in 0..=self.max_retries {
            let mut request = client
                .post(format!("{}/chat/completions", self.api_base))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json");

            if let Some(ref org) = self.organization {
                request = request.header("OpenAI-Organization", org);
            }

            let response = request.json(&payload).send().await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<OpenAIResponse>().await {
                            Ok(openai_resp) => {
                                return Ok(self.parse_response(openai_resp));
                            }
                            Err(e) => {
                                return Err(Error::Json(serde_json::Error::io(
                                    std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        e.to_string(),
                                    ),
                                )));
                            }
                        }
                    } else {
                        let status = resp.status().as_u16();
                        let error_text = resp.text().await.unwrap_or_default();
                        last_error = Some(Error::api(status, error_text));

                        // Don't retry 4xx errors (client errors), except 429 (rate limit)
                        if (400..500).contains(&status) && status != 429 {
                            break;
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(Error::Http(e));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::other("Unknown error")))
    }

    /// Internal generate with tools implementation.
    async fn generate_with_tools_internal(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessage> {
        // Convert tool definitions to OpenAI format
        let openai_tools: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters
                    }
                })
            })
            .collect();

        // Use Responses API if enabled or if using built-in tools
        if self.should_use_responses_api(!self.builtin_tools.is_empty()) {
            let api_key = self.get_api_key()?;
            let client = self.build_client();
            let payload =
                self.build_responses_api_payload(&messages, stop, Some(&openai_tools), false);

            let mut last_error = None;
            for _ in 0..=self.max_retries {
                let mut request = client
                    .post(format!("{}/responses", self.api_base))
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json");

                if let Some(ref org) = self.organization {
                    request = request.header("OpenAI-Organization", org);
                }

                let response = request.json(&payload).send().await;

                match response {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            match resp.json::<ResponsesApiResponse>().await {
                                Ok(responses_resp) => {
                                    let result = self.parse_responses_api_response(responses_resp);
                                    return Self::extract_ai_message(result);
                                }
                                Err(e) => {
                                    return Err(Error::Json(serde_json::Error::io(
                                        std::io::Error::new(
                                            std::io::ErrorKind::InvalidData,
                                            e.to_string(),
                                        ),
                                    )));
                                }
                            }
                        } else {
                            let status = resp.status().as_u16();
                            let error_text = resp.text().await.unwrap_or_default();
                            last_error = Some(Error::api(status, error_text));

                            if (400..500).contains(&status) && status != 429 {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        last_error = Some(Error::Http(e));
                    }
                }
            }

            return Err(last_error.unwrap_or_else(|| Error::other("Unknown error")));
        }

        // Use Chat Completions API
        let api_key = self.get_api_key()?;
        let client = self.build_client();
        let mut payload = self.build_request_payload(&messages, stop, Some(&openai_tools), false);

        // Add tool_choice if specified
        if let Some(choice) = tool_choice {
            let choice_json = match choice {
                ToolChoice::String(s) => serde_json::json!(s),
                ToolChoice::Structured { choice_type, name } => {
                    if choice_type == "tool" || choice_type == "function" {
                        serde_json::json!({
                            "type": "function",
                            "function": {"name": name}
                        })
                    } else {
                        serde_json::json!(choice_type)
                    }
                }
            };
            payload["tool_choice"] = choice_json;
        }

        let mut last_error = None;
        for _ in 0..=self.max_retries {
            let mut request = client
                .post(format!("{}/chat/completions", self.api_base))
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json");

            if let Some(ref org) = self.organization {
                request = request.header("OpenAI-Organization", org);
            }

            let response = request.json(&payload).send().await;

            match response {
                Ok(resp) => {
                    if resp.status().is_success() {
                        match resp.json::<OpenAIResponse>().await {
                            Ok(openai_resp) => {
                                let result = self.parse_response(openai_resp);
                                return Self::extract_ai_message(result);
                            }
                            Err(e) => {
                                return Err(Error::Json(serde_json::Error::io(
                                    std::io::Error::new(
                                        std::io::ErrorKind::InvalidData,
                                        e.to_string(),
                                    ),
                                )));
                            }
                        }
                    } else {
                        let status = resp.status().as_u16();
                        let error_text = resp.text().await.unwrap_or_default();
                        last_error = Some(Error::api(status, error_text));

                        if (400..500).contains(&status) && status < 500 && status != 429 {
                            break;
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(Error::Http(e));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| Error::other("Unknown error")))
    }

    /// Create usage metadata from OpenAI usage response, including token details.
    fn create_usage_metadata(usage: &OpenAIUsage) -> UsageMetadata {
        let mut metadata =
            UsageMetadata::new(usage.prompt_tokens as i64, usage.completion_tokens as i64);

        // Add input token details
        if let Some(ref details) = usage.prompt_tokens_details {
            metadata.input_token_details = Some(crate::messages::InputTokenDetails {
                cache_read: details.cached_tokens.map(|t| t as i64),
                cache_creation: None,
                audio: details.audio_tokens.map(|t| t as i64),
            });
        }

        // Add output token details
        if let Some(ref details) = usage.completion_tokens_details {
            metadata.output_token_details = Some(crate::messages::OutputTokenDetails {
                reasoning: details.reasoning_tokens.map(|t| t as i64),
                audio: details.audio_tokens.map(|t| t as i64),
            });
        }

        metadata
    }

    /// Extract AIMessage from ChatResult
    fn extract_ai_message(result: ChatResult) -> Result<AIMessage> {
        if result.generations.is_empty() {
            return Err(Error::other("No generations returned"));
        }
        match result.generations[0].message.clone() {
            BaseMessage::AI(msg) => Ok(msg),
            _ => Err(Error::other("Expected AI message")),
        }
    }

    /// Internal stream implementation.
    #[allow(dead_code)]
    async fn stream_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatStream> {
        // Use Responses API if enabled or if using built-in tools
        if self.should_use_responses_api(!self.builtin_tools.is_empty()) {
            return self.stream_responses_api(messages, stop).await;
        }

        let api_key = self.get_api_key()?;
        let client = self.build_client();
        let payload = self.build_request_payload(&messages, stop, None, true);

        let mut request = client
            .post(format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json");

        if let Some(ref org) = self.organization {
            request = request.header("OpenAI-Organization", org);
        }

        let response = request.json(&payload).send().await.map_err(Error::Http)?;

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
            let mut finish_reason: Option<String> = None;
            // Accumulate tool call deltas: index -> (id, name, arguments)
            let mut tool_call_acc: std::collections::HashMap<u32, (String, String, String)> =
                std::collections::HashMap::new();

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
                                        // Build accumulated tool calls into final chunk
                                        let mut final_chunk = ChatChunk::final_chunk(usage.take(), finish_reason.take());
                                        if !tool_call_acc.is_empty() {
                                            let mut tool_calls: Vec<_> = tool_call_acc.drain().collect();
                                            tool_calls.sort_by_key(|(idx, _)| *idx);
                                            let tcs: Vec<ToolCall> = tool_calls
                                                .into_iter()
                                                .map(|(_, (id, name, args))| {
                                                    let parsed_args: serde_json::Value =
                                                        serde_json::from_str(&args).unwrap_or_default();
                                                    ToolCall::builder()
                                                        .name(name)
                                                        .args(parsed_args)
                                                        .id(id)
                                                        .build()
                                                })
                                                .collect();
                                            final_chunk.tool_calls = tcs;
                                        }
                                        yield Ok(final_chunk);
                                        continue;
                                    }

                                    if let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(data) {
                                        if let Some(choice) = chunk.choices.first() {
                                            if let Some(ref content) = choice.delta.content {
                                                yield Ok(ChatChunk::new(content.clone()));
                                            }
                                            // Accumulate tool call deltas
                                            if let Some(ref tcs) = choice.delta.tool_calls {
                                                for tc in tcs {
                                                    let entry = tool_call_acc
                                                        .entry(tc.index)
                                                        .or_insert_with(|| (String::new(), String::new(), String::new()));
                                                    if let Some(ref id) = tc.id {
                                                        entry.0 = id.clone();
                                                    }
                                                    if let Some(ref func) = tc.function {
                                                        if let Some(ref name) = func.name {
                                                            entry.1 = name.clone();
                                                        }
                                                        if let Some(ref args) = func.arguments {
                                                            entry.2.push_str(args);
                                                        }
                                                    }
                                                }
                                            }
                                            if let Some(ref reason) = choice.finish_reason {
                                                finish_reason = Some(reason.clone());
                                            }
                                        }
                                        if let Some(ref u) = chunk.usage {
                                            usage = Some(UsageMetadata::new(
                                                u.prompt_tokens as i64,
                                                u.completion_tokens as i64,
                                            ));
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

/// OpenAI API response structure.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OpenAIResponse {
    model: String,
    choices: Vec<OpenAIChoice>,
    pub usage: Option<OpenAIUsage>,
}

/// OpenAI choice in response.
#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

/// OpenAI message in response.
#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

/// OpenAI tool call in response.
#[derive(Debug, Deserialize)]
struct OpenAIToolCall {
    id: String,
    function: OpenAIFunction,
}

/// OpenAI function in tool call.
#[derive(Debug, Deserialize)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

/// OpenAI usage information.
#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    #[allow(dead_code)]
    total_tokens: Option<u32>,
    prompt_tokens_details: Option<TokenDetails>,
    completion_tokens_details: Option<TokenDetails>,
}

/// Token usage breakdown details.
#[derive(Debug, Deserialize)]
struct TokenDetails {
    cached_tokens: Option<u32>,
    audio_tokens: Option<u32>,
    reasoning_tokens: Option<u32>,
}

/// OpenAI streaming chunk.
#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
    usage: Option<OpenAIUsage>,
}

/// OpenAI streaming choice.
#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

/// OpenAI delta in streaming.
#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
    #[allow(dead_code)]
    role: Option<String>,
    tool_calls: Option<Vec<OpenAIStreamToolCall>>,
}

/// Tool call delta in streaming.
#[derive(Debug, Clone, Deserialize)]
struct OpenAIStreamToolCall {
    index: u32,
    id: Option<String>,
    function: Option<OpenAIStreamFunction>,
}

/// Function delta in streaming tool call.
#[derive(Debug, Clone, Deserialize)]
struct OpenAIStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

// ============================================================================
// Responses API types
// ============================================================================

/// OpenAI Responses API response structure.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ResponsesApiResponse {
    model: String,
    output: Vec<ResponsesOutput>,
    usage: Option<ResponsesUsage>,
    status: Option<String>,
}

/// Output item in Responses API.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum ResponsesOutput {
    #[serde(rename = "message")]
    Message {
        role: String,
        content: Vec<ResponsesContent>,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        name: String,
        arguments: String,
        call_id: String,
        #[serde(default)]
        id: Option<String>,
    },
    #[serde(rename = "web_search_call")]
    WebSearchCall {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        status: Option<String>,
    },
    #[serde(untagged)]
    Other(serde_json::Value),
}

/// Content block in Responses API.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum ResponsesContent {
    #[serde(rename = "output_text")]
    OutputText {
        text: String,
        #[serde(default)]
        annotations: Vec<TextAnnotation>,
    },
    #[serde(rename = "refusal")]
    Refusal { refusal: String },
    #[serde(untagged)]
    Other(serde_json::Value),
}

/// Responses API usage information.
#[derive(Debug, Deserialize)]
struct ResponsesUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Responses API streaming event.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ResponsesStreamEvent {
    /// Event type (e.g., "response.output_text.delta", "response.completed")
    #[serde(rename = "type")]
    event_type: String,
    /// Delta text content (for text delta events)
    #[serde(default)]
    delta: Option<String>,
    /// Annotation (for annotation events)
    #[serde(default)]
    annotation: Option<TextAnnotation>,
    /// Response object (for completed/incomplete events)
    #[serde(default)]
    response: Option<ResponsesStreamResponse>,
    /// Output index
    #[serde(default)]
    output_index: Option<u32>,
    /// Content index
    #[serde(default)]
    content_index: Option<u32>,
    /// Item ID
    #[serde(default)]
    item_id: Option<String>,
}

/// Response object in streaming events.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct ResponsesStreamResponse {
    /// Response ID
    #[serde(default)]
    id: Option<String>,
    /// Model used
    #[serde(default)]
    model: Option<String>,
    /// Usage information
    #[serde(default)]
    usage: Option<ResponsesUsage>,
    /// Output items
    #[serde(default)]
    output: Option<Vec<serde_json::Value>>,
    /// Status
    #[serde(default)]
    status: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let model = ChatOpenAI::new("gpt-4o");
        assert_eq!(model.model, "gpt-4o");
        assert!(model.max_tokens.is_none());
        assert!(model.temperature.is_none());
    }

    #[test]
    fn test_builder_methods() {
        let model = ChatOpenAI::new("gpt-4o")
            .temperature(0.7)
            .max_tokens(1024)
            .top_p(0.9)
            .frequency_penalty(0.5)
            .presence_penalty(0.5)
            .api_key("test-key")
            .organization("test-org");

        assert_eq!(model.temperature, Some(0.7));
        assert_eq!(model.max_tokens, Some(1024));
        assert_eq!(model.top_p, Some(0.9));
        assert_eq!(model.frequency_penalty, Some(0.5));
        assert_eq!(model.presence_penalty, Some(0.5));
        assert_eq!(model.api_key, Some("test-key".to_string()));
        assert_eq!(model.organization, Some("test-org".to_string()));
    }

    #[test]
    fn test_llm_type() {
        use crate::language_models::BaseLanguageModel;
        let model = ChatOpenAI::new("gpt-4o");
        assert_eq!(model.llm_type(), "openai-chat");
    }
}
