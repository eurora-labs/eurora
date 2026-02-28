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
//! let response = model.generate(messages, GenerateConfig::default()).await?;
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
use backon::{ConstantBuilder, Retryable};
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
use crate::language_models::{ChatModelRunnable, ToolLike, extract_tool_name_from_schema};
use crate::messages::{
    AIMessage, BaseMessage, ContentPart, ImageDetail, ImageSource, InvalidToolCall, MessageContent,
    ToolCall,
};
use crate::outputs::ChatGenerationChunk;
use crate::outputs::{ChatGeneration, ChatResult, LLMResult};
use crate::runnables::base::Runnable;
use crate::tools::ToolDefinition;

/// Default API base URL for OpenAI.
const DEFAULT_API_BASE: &str = "https://api.openai.com/v1";

/// Well-known built-in tool type names for the Responses API.
const WELL_KNOWN_TOOLS: &[&str] = &[
    "file_search",
    "web_search_preview",
    "web_search",
    "computer_use_preview",
    "code_interpreter",
    "mcp",
    "image_generation",
];

/// Built-in tools supported by OpenAI's Responses API.
///
/// These are server-side tools that OpenAI executes automatically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinTool {
    WebSearch,
    WebSearchPreview,
    FileSearch,
    CodeInterpreter,
    ComputerUsePreview,
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
    #[serde(rename = "type")]
    pub annotation_type: String,
    pub start_index: Option<u32>,
    pub end_index: Option<u32>,
    pub url: Option<String>,
    pub title: Option<String>,
}

/// Content block in a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(default)]
        annotations: Vec<TextAnnotation>,
    },
    #[serde(rename = "output_text")]
    OutputText {
        text: String,
        #[serde(default)]
        annotations: Vec<TextAnnotation>,
    },
    #[serde(rename = "refusal")]
    Refusal { refusal: String },
}

/// Returns true if the model name indicates an o-series reasoning model.
fn is_o_series_model(model: &str) -> bool {
    let lower = model.to_lowercase();
    lower.starts_with("o1") || lower.starts_with("o3") || lower.starts_with("o4")
}

/// Returns true if the model prefers the Responses API.
fn model_prefers_responses_api(model: &str) -> bool {
    model.to_lowercase().contains("gpt-5.2-pro")
}

/// Returns true if a tool JSON value is a built-in tool (not a function).
fn is_builtin_tool(tool: &serde_json::Value) -> bool {
    tool.get("type")
        .and_then(|t| t.as_str())
        .is_some_and(|t| t != "function")
}

/// Check whether a payload dict contains parameters that require the Responses API.
fn payload_requires_responses_api(payload: &serde_json::Value) -> bool {
    let uses_builtin_tools = payload
        .get("tools")
        .and_then(|t| t.as_array())
        .is_some_and(|tools| tools.iter().any(is_builtin_tool));

    let has_responses_only_args = [
        "include",
        "previous_response_id",
        "reasoning",
        "text",
        "truncation",
    ]
    .iter()
    .any(|key| payload.get(*key).is_some());

    uses_builtin_tools || has_responses_only_args
}

/// OpenAI chat model (GPT).
///
/// Implements the `BaseChatModel` trait for OpenAI's GPT models.
/// Supports both the Chat Completions API and the Responses API.
#[derive(Clone)]
pub struct ChatOpenAI {
    model: String,
    temperature: Option<f64>,
    max_tokens: Option<u32>,
    api_key: Option<String>,
    api_base: String,
    organization: Option<String>,
    top_p: Option<f64>,
    frequency_penalty: Option<f64>,
    presence_penalty: Option<f64>,
    stop: Option<Vec<String>>,
    timeout: Option<u64>,
    max_retries: u32,
    model_kwargs: HashMap<String, serde_json::Value>,
    streaming: bool,
    seed: Option<i32>,
    logprobs: Option<bool>,
    top_logprobs: Option<u32>,
    logit_bias: Option<HashMap<i32, i32>>,
    n: Option<u32>,
    reasoning_effort: Option<String>,
    reasoning: Option<HashMap<String, serde_json::Value>>,
    verbosity: Option<String>,
    stream_usage: Option<bool>,
    include: Option<Vec<String>>,
    service_tier: Option<String>,
    store: Option<bool>,
    truncation: Option<String>,
    use_responses_api: Option<bool>,
    use_previous_response_id: bool,
    output_version: Option<String>,
    builtin_tools: Vec<BuiltinTool>,
    disabled_params: Option<HashMap<String, Option<serde_json::Value>>>,
    extra_body: Option<HashMap<String, serde_json::Value>>,
    chat_model_config: ChatModelConfig,
    language_model_config: LanguageModelConfig,
    /// Tools bound to this model via `bind_tools()`.
    bound_tools: Vec<ToolDefinition>,
    /// Tool choice for bound tools.
    bound_tool_choice: Option<ToolChoice>,
    /// Whether bound tools should be called with strict schema validation.
    bound_strict: Option<bool>,
    /// Whether to allow parallel tool calls.
    bound_parallel_tool_calls: Option<bool>,
    /// Response format for structured output (e.g., JSON schema).
    response_format: Option<serde_json::Value>,
    /// Whether to include response headers in response_metadata.
    include_response_headers: bool,
    /// Proxy URL for HTTP requests.
    proxy: Option<String>,
    /// Prediction content for predicted output tokens.
    prediction: Option<serde_json::Value>,
    /// Callable API key getter function.
    api_key_fn: Option<std::sync::Arc<dyn Fn() -> String + Send + Sync>>,
}

impl std::fmt::Debug for ChatOpenAI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatOpenAI")
            .field("model", &self.model)
            .field("temperature", &self.temperature)
            .field("max_tokens", &self.max_tokens)
            .field("api_base", &self.api_base)
            .field("streaming", &self.streaming)
            .field("n", &self.n)
            .field("api_key_fn", &self.api_key_fn.as_ref().map(|_| "<fn>"))
            .finish_non_exhaustive()
    }
}

impl ChatOpenAI {
    /// Create a new ChatOpenAI instance.
    pub fn new(model: impl Into<String>) -> Self {
        let model_name = model.into();
        let model_lower = model_name.to_lowercase();

        let temperature = if model_lower.starts_with("o1") {
            Some(1.0)
        } else {
            None
        };

        let organization = env::var("OPENAI_ORG_ID")
            .ok()
            .or_else(|| env::var("OPENAI_ORGANIZATION").ok());

        let api_base = env::var("OPENAI_API_BASE")
            .ok()
            .or_else(|| env::var("OPENAI_BASE_URL").ok())
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string());

        let stream_usage = if api_base == DEFAULT_API_BASE {
            Some(true)
        } else {
            None
        };

        Self {
            model: model_name,
            temperature,
            max_tokens: None,
            api_key: None,
            api_base,
            organization,
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
            stream_usage,
            include: None,
            service_tier: None,
            store: None,
            truncation: None,
            use_responses_api: None,
            use_previous_response_id: false,
            output_version: env::var("LC_OUTPUT_VERSION").ok(),
            builtin_tools: Vec::new(),
            disabled_params: None,
            extra_body: None,
            chat_model_config: ChatModelConfig::new(),
            language_model_config: LanguageModelConfig::new(),
            bound_tools: Vec::new(),
            bound_tool_choice: None,
            bound_strict: None,
            bound_parallel_tool_calls: None,
            response_format: None,
            include_response_headers: false,
            proxy: None,
            prediction: None,
            api_key_fn: None,
        }
    }

    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn api_base(mut self, base: impl Into<String>) -> Self {
        self.api_base = base.into();
        self
    }

    pub fn organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    pub fn top_p(mut self, p: f64) -> Self {
        self.top_p = Some(p);
        self
    }

    pub fn frequency_penalty(mut self, penalty: f64) -> Self {
        self.frequency_penalty = Some(penalty);
        self
    }

    pub fn presence_penalty(mut self, penalty: f64) -> Self {
        self.presence_penalty = Some(penalty);
        self
    }

    pub fn stop(mut self, sequences: Vec<String>) -> Self {
        self.stop = Some(sequences);
        self
    }

    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(seconds);
        self
    }

    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn streaming(mut self, enabled: bool) -> Self {
        self.streaming = enabled;
        self
    }

    pub fn seed(mut self, seed: i32) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn logprobs(mut self, enabled: bool) -> Self {
        self.logprobs = Some(enabled);
        self
    }

    pub fn top_logprobs(mut self, count: u32) -> Self {
        self.top_logprobs = Some(count);
        self
    }

    pub fn logit_bias(mut self, bias: HashMap<i32, i32>) -> Self {
        self.logit_bias = Some(bias);
        self
    }

    pub fn n(mut self, count: u32) -> Self {
        self.n = Some(count);
        self
    }

    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
        self
    }

    pub fn reasoning(mut self, params: HashMap<String, serde_json::Value>) -> Self {
        self.reasoning = Some(params);
        self
    }

    pub fn verbosity(mut self, level: impl Into<String>) -> Self {
        self.verbosity = Some(level.into());
        self
    }

    pub fn stream_usage(mut self, enabled: bool) -> Self {
        self.stream_usage = Some(enabled);
        self
    }

    pub fn include(mut self, fields: Vec<String>) -> Self {
        self.include = Some(fields);
        self
    }

    pub fn service_tier(mut self, tier: impl Into<String>) -> Self {
        self.service_tier = Some(tier.into());
        self
    }

    pub fn store(mut self, enabled: bool) -> Self {
        self.store = Some(enabled);
        self
    }

    pub fn truncation(mut self, strategy: impl Into<String>) -> Self {
        self.truncation = Some(strategy.into());
        self
    }

    pub fn use_previous_response_id(mut self, enabled: bool) -> Self {
        self.use_previous_response_id = enabled;
        self
    }

    pub fn output_version(mut self, version: impl Into<String>) -> Self {
        self.output_version = Some(version.into());
        self
    }

    pub fn with_responses_api(mut self, enabled: bool) -> Self {
        self.use_responses_api = Some(enabled);
        self
    }

    pub fn with_builtin_tools(mut self, tools: Vec<BuiltinTool>) -> Self {
        self.builtin_tools = tools;
        if !self.builtin_tools.is_empty() {
            self.use_responses_api = Some(true);
        }
        self
    }

    pub fn disabled_params(mut self, params: HashMap<String, Option<serde_json::Value>>) -> Self {
        self.disabled_params = Some(params);
        self
    }

    pub fn extra_body(mut self, body: HashMap<String, serde_json::Value>) -> Self {
        self.extra_body = Some(body);
        self
    }

    pub fn model_kwargs(mut self, kwargs: HashMap<String, serde_json::Value>) -> Self {
        self.model_kwargs = kwargs;
        self
    }

    pub fn response_format(mut self, format: serde_json::Value) -> Self {
        self.response_format = Some(format);
        self
    }

    pub fn include_response_headers(mut self, enabled: bool) -> Self {
        self.include_response_headers = enabled;
        self
    }

    pub fn openai_proxy(mut self, proxy_url: impl Into<String>) -> Self {
        self.proxy = Some(proxy_url.into());
        self
    }

    pub fn prediction(mut self, prediction: serde_json::Value) -> Self {
        self.prediction = Some(prediction);
        self
    }

    pub fn api_key_fn(mut self, f: impl Fn() -> String + Send + Sync + 'static) -> Self {
        self.api_key_fn = Some(std::sync::Arc::new(f));
        self
    }

    /// Filter out disabled parameters from a payload.
    /// Matches Python `_filter_disabled_params`: supports both `None` (remove entirely)
    /// and a list of disabled values.
    fn filter_disabled_params(&self, payload: &mut serde_json::Value) {
        if let Some(ref disabled) = self.disabled_params
            && let Some(obj) = payload.as_object_mut()
        {
            for (key, default_or_list) in disabled {
                match default_or_list {
                    None => {
                        obj.remove(key);
                    }
                    Some(serde_json::Value::Array(disabled_values)) => {
                        if let Some(current) = obj.get(key)
                            && disabled_values.contains(current)
                        {
                            obj.remove(key);
                        }
                    }
                    Some(default) => {
                        obj.insert(key.clone(), default.clone());
                    }
                }
            }
        }
    }

    /// Determine if Responses API should be used.
    /// Matches Python's `BaseChatOpenAI._use_responses_api` + module-level `_use_responses_api`.
    pub fn should_use_responses_api(&self, payload: Option<&serde_json::Value>) -> bool {
        if let Some(use_api) = self.use_responses_api {
            return use_api;
        }

        if !self.builtin_tools.is_empty()
            || self.reasoning.is_some()
            || self.verbosity.is_some()
            || self.truncation.is_some()
            || self.include.is_some()
            || self.use_previous_response_id
        {
            return true;
        }

        if self.output_version.as_deref() == Some("responses/v1") {
            return true;
        }

        if model_prefers_responses_api(&self.model) {
            return true;
        }

        if let Some(p) = payload
            && payload_requires_responses_api(p)
        {
            return true;
        }

        false
    }

    /// Get the API key, checking callable, direct value, or environment variable.
    fn get_api_key(&self) -> Result<String> {
        if let Some(ref f) = self.api_key_fn {
            return Ok(f());
        }
        self.api_key
            .clone()
            .or_else(|| env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| Error::missing_config("OPENAI_API_KEY"))
    }

    /// Build the HTTP client with configured timeout and proxy.
    fn build_client(&self) -> Result<reqwest::Client> {
        let mut builder = reqwest::Client::builder();
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(std::time::Duration::from_secs(timeout));
        }
        if let Some(ref proxy_url) = self.proxy {
            let proxy = reqwest::Proxy::all(proxy_url)
                .map_err(|e| Error::other(format!("Invalid proxy URL: {e}")))?;
            builder = builder.proxy(proxy);
        }
        builder
            .build()
            .map_err(|e| Error::other(format!("Failed to build HTTP client: {e}")))
    }

    /// Determine whether to include usage metadata in streaming output.
    fn should_stream_usage(&self) -> bool {
        self.stream_usage.unwrap_or(false)
    }

    /// Effective temperature after model-specific validation.
    /// gpt-5 (non-chat) models only support temperature=1.
    fn effective_temperature(&self) -> Option<f64> {
        let model_lower = self.model.to_lowercase();

        if model_lower.starts_with("gpt-5")
            && !model_lower.contains("chat")
            && self.reasoning_effort.as_deref() != Some("none")
            && self
                .reasoning
                .as_ref()
                .and_then(|r| r.get("effort"))
                .and_then(|e| e.as_str())
                .is_none_or(|e| e != "none")
        {
            match self.temperature {
                Some(t) if t != 1.0 => None,
                other => other,
            }
        } else {
            self.temperature
        }
    }

    /// Format message content, filtering out block types not supported by OpenAI.
    /// Matches Python `_format_message_content`.
    fn format_message_content(content: &serde_json::Value) -> serde_json::Value {
        if let Some(arr) = content.as_array() {
            let filtered: Vec<serde_json::Value> = arr
                .iter()
                .filter(|block| {
                    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    !matches!(block_type, "tool_use" | "thinking" | "reasoning_content")
                })
                .cloned()
                .collect();
            serde_json::Value::Array(filtered)
        } else {
            content.clone()
        }
    }

    /// Convert messages to OpenAI Chat Completions API format.
    /// Matches Python `_convert_message_to_dict`.
    pub fn format_messages(&self, messages: &[BaseMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .filter_map(|msg| self.convert_message_to_dict(msg))
            .collect()
    }

    /// Convert a single message to OpenAI dict format.
    fn convert_message_to_dict(&self, msg: &BaseMessage) -> Option<serde_json::Value> {
        match msg {
            BaseMessage::System(m) => {
                let role = m
                    .additional_kwargs
                    .get("__openai_role__")
                    .and_then(|v| v.as_str())
                    .unwrap_or("system");
                let mut message = serde_json::json!({
                    "role": role,
                    "content": m.content.as_text()
                });
                if let Some(ref name) = m.name {
                    message["name"] = serde_json::json!(name);
                }
                Some(message)
            }
            BaseMessage::Human(m) => {
                let content = match &m.content {
                    MessageContent::Text(text) => serde_json::json!(text),
                    MessageContent::Parts(parts) => {
                        let content_parts: Vec<serde_json::Value> = parts
                            .iter()
                            .map(|part| match part {
                                ContentPart::Text { text } => {
                                    serde_json::json!({"type": "text", "text": text})
                                }
                                ContentPart::Image { source, detail } => {
                                    let url = match source {
                                        ImageSource::Url { url } => url.clone(),
                                        ImageSource::Base64 { media_type, data } => {
                                            format!("data:{media_type};base64,{data}")
                                        }
                                        ImageSource::FileId { file_id } => file_id.clone(),
                                    };
                                    let mut image_url = serde_json::json!({"url": url});
                                    if let Some(d) = detail {
                                        image_url["detail"] = serde_json::json!(match d {
                                            ImageDetail::Low => "low",
                                            ImageDetail::High => "high",
                                            ImageDetail::Auto => "auto",
                                        });
                                    }
                                    serde_json::json!({"type": "image_url", "image_url": image_url})
                                }
                                ContentPart::Other(value) => value.clone(),
                            })
                            .collect();
                        let raw = serde_json::Value::Array(content_parts);
                        Self::format_message_content(&raw)
                    }
                };
                let mut message = serde_json::json!({"role": "user", "content": content});
                if let Some(ref name) = m.name {
                    message["name"] = serde_json::json!(name);
                }
                Some(message)
            }
            BaseMessage::AI(m) => {
                let mut message = serde_json::json!({"role": "assistant"});

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

                let has_tool_calls = message.get("tool_calls").is_some();
                if has_tool_calls {
                    let content_str = m.text();
                    if content_str.is_empty() {
                        message["content"] = serde_json::Value::Null;
                    } else {
                        message["content"] = serde_json::json!(content_str);
                    }
                } else if !m.content.is_empty() {
                    message["content"] = serde_json::json!(m.content.as_text());
                }

                if let Some(ref name) = m.name {
                    message["name"] = serde_json::json!(name);
                }

                Some(message)
            }
            BaseMessage::Tool(m) => Some(serde_json::json!({
                "role": "tool",
                "tool_call_id": m.tool_call_id,
                "content": m.content
            })),
            BaseMessage::Remove(_) => None,
            BaseMessage::Chat(m) => {
                let mut message = serde_json::json!({
                    "role": m.role,
                    "content": m.content.as_text()
                });
                if let Some(ref name) = m.name {
                    message["name"] = serde_json::json!(name);
                }
                Some(message)
            }
            BaseMessage::Function(m) => Some(serde_json::json!({
                "role": "function",
                "name": m.name,
                "content": m.content
            })),
        }
    }

    /// Format messages for the Responses API.
    /// Matches Python `_construct_responses_api_input`.
    pub fn format_messages_for_responses_api(
        &self,
        messages: &[BaseMessage],
    ) -> Vec<serde_json::Value> {
        let mut input = Vec::new();

        for msg in messages {
            match msg {
                BaseMessage::System(m) => {
                    let role = m
                        .additional_kwargs
                        .get("__openai_role__")
                        .and_then(|v| v.as_str())
                        .unwrap_or("system");
                    input.push(serde_json::json!({
                        "role": role,
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
                                        serde_json::json!({"type": "input_text", "text": text})
                                    }
                                    ContentPart::Image { source, detail } => {
                                        let url = match source {
                                            ImageSource::Url { url } => url.clone(),
                                            ImageSource::Base64 { media_type, data } => {
                                                format!("data:{media_type};base64,{data}")
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
                    input.push(serde_json::json!({"role": "user", "content": content}));
                }
                BaseMessage::AI(m) => {
                    if !m.content.is_empty() || m.tool_calls.is_empty() {
                        input.push(serde_json::json!({
                            "type": "message",
                            "role": "assistant",
                            "content": [{
                                "type": "output_text",
                                "text": m.content.as_text(),
                                "annotations": []
                            }]
                        }));
                    }

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
                BaseMessage::Remove(_) => continue,
                BaseMessage::Chat(m) => {
                    input.push(serde_json::json!({
                        "role": m.role,
                        "content": m.content.as_text()
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

    /// Build the request payload for the Chat Completions API.
    /// Matches Python `ChatOpenAI._get_request_payload`.
    pub fn build_request_payload(
        &self,
        messages: &[BaseMessage],
        stop: Option<Vec<String>>,
        tools: Option<&[serde_json::Value]>,
        stream: bool,
    ) -> serde_json::Value {
        let mut formatted_messages = self.format_messages(messages);

        if is_o_series_model(&self.model) {
            for message in &mut formatted_messages {
                if message.get("role").and_then(|r| r.as_str()) == Some("system") {
                    message["role"] = serde_json::json!("developer");
                }
            }
        }

        let mut payload = serde_json::json!({
            "model": self.model,
            "messages": formatted_messages
        });

        if let Some(max_tokens) = self.max_tokens {
            payload["max_completion_tokens"] = serde_json::json!(max_tokens);
        }

        if let Some(temp) = self.effective_temperature() {
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
            if self.should_stream_usage() {
                payload["stream_options"] = serde_json::json!({"include_usage": true});
            }
        }

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

        if let Some(ref response_format) = self.response_format {
            payload["response_format"] = response_format.clone();
        }

        if let Some(ref prediction) = self.prediction {
            payload["prediction"] = prediction.clone();
        }

        if let Some(parallel) = self.bound_parallel_tool_calls {
            payload["parallel_tool_calls"] = serde_json::json!(parallel);
        }

        if let Some(obj) = payload.as_object_mut() {
            for (k, v) in &self.model_kwargs {
                obj.insert(k.clone(), v.clone());
            }
        }

        if let Some(ref extra) = self.extra_body
            && let Some(obj) = payload.as_object_mut()
        {
            for (k, v) in extra {
                obj.insert(k.clone(), v.clone());
            }
        }

        self.filter_disabled_params(&mut payload);
        payload
    }

    /// Build the request payload for the Responses API.
    /// Matches Python `_construct_responses_api_payload`.
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

        if let Some(temp) = self.effective_temperature() {
            payload["temperature"] = serde_json::json!(temp);
        }

        if let Some(p) = self.top_p {
            payload["top_p"] = serde_json::json!(p);
        }

        let stop_sequences = stop.or_else(|| self.stop.clone());
        if let Some(stop) = stop_sequences {
            payload["stop"] = serde_json::json!(stop);
        }

        let mut all_tools: Vec<serde_json::Value> = self
            .builtin_tools
            .iter()
            .map(|t| t.to_api_format())
            .collect();

        if let Some(tools) = tools {
            for tool in tools {
                if let Some(function) = tool.get("function") {
                    let mut flat = serde_json::json!({"type": "function"});
                    if let Some(name) = function.get("name") {
                        flat["name"] = name.clone();
                    }
                    if let Some(desc) = function.get("description") {
                        flat["description"] = desc.clone();
                    }
                    if let Some(params) = function.get("parameters") {
                        flat["parameters"] = params.clone();
                    }
                    if let Some(strict) = function.get("strict") {
                        flat["strict"] = strict.clone();
                    }
                    all_tools.push(flat);
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

        if let Some(ref reasoning) = self.reasoning {
            payload["reasoning"] = serde_json::json!(reasoning);
        } else if let Some(ref effort) = self.reasoning_effort {
            payload["reasoning"] = serde_json::json!({"effort": effort});
        }

        if let Some(ref verbosity) = self.verbosity {
            if let Some(text_obj) = payload.get_mut("text").and_then(|t| t.as_object_mut()) {
                text_obj.insert("verbosity".to_string(), serde_json::json!(verbosity));
            } else {
                payload["text"] = serde_json::json!({"verbosity": verbosity});
            }
        }

        if let Some(ref include) = self.include {
            payload["include"] = serde_json::json!(include);
        }

        if let Some(ref truncation) = self.truncation {
            payload["truncation"] = serde_json::json!(truncation);
        }

        if let Some(ref service_tier) = self.service_tier {
            payload["service_tier"] = serde_json::json!(service_tier);
        }

        if let Some(store) = self.store {
            payload["store"] = serde_json::json!(store);
        }

        if let Some(ref response_format) = self.response_format {
            payload["text"] = serde_json::json!({"format": response_format});
        }

        if let Some(ref prediction) = self.prediction {
            payload["prediction"] = prediction.clone();
        }

        if let Some(obj) = payload.as_object_mut() {
            for (k, v) in &self.model_kwargs {
                obj.insert(k.clone(), v.clone());
            }
        }

        if let Some(ref extra) = self.extra_body
            && let Some(obj) = payload.as_object_mut()
        {
            for (k, v) in extra {
                obj.insert(k.clone(), v.clone());
            }
        }

        self.filter_disabled_params(&mut payload);
        payload
    }

    /// Stream responses using the Responses API.
    async fn stream_responses_api(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        tools: Option<&[serde_json::Value]>,
    ) -> Result<ChatStream> {
        let api_key = self.get_api_key()?;
        let client = self.build_client()?;
        let payload = self.build_responses_api_payload(&messages, stop, tools, true);

        let mut request = client
            .post(format!("{}/responses", self.api_base))
            .header("Authorization", format!("Bearer {api_key}"))
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
            let mut usage: Option<UsageMetadata> = None;
            let mut finish_reason: Option<String> = None;
            let mut tool_call_acc: std::collections::HashMap<String, (String, String, String)> =
                std::collections::HashMap::new();

            use futures::StreamExt;

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].to_string();
                            buffer = buffer[line_end + 1..].to_string();

                            if line.is_empty() || line == "\r" {
                                continue;
                            }

                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    let mut final_chunk = ChatChunk::final_chunk(usage.take(), finish_reason.take());
                                    if !tool_call_acc.is_empty() {
                                        let tcs: Vec<ToolCall> = tool_call_acc
                                            .drain()
                                            .map(|(_, (id, name, args))| {
                                                let parsed_args = serde_json::from_str(&args)
                                                    .unwrap_or(serde_json::Value::Object(Default::default()));
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

                                if let Ok(event) = serde_json::from_str::<ResponsesStreamEvent>(data) {
                                    match event.event_type.as_str() {
                                        "response.output_text.delta" => {
                                            if let Some(delta) = event.delta {
                                                yield Ok(ChatChunk::new(delta));
                                            }
                                        }
                                        "response.output_text.annotation.added" => {
                                        }
                                        "response.function_call_arguments.delta" => {
                                            if let Some(delta) = event.delta && let Some(call_id) = event.call_id.as_ref().or(event.item_id.as_ref()) {
                                                    let entry = tool_call_acc
                                                        .entry(call_id.clone())
                                                        .or_insert_with(|| (call_id.clone(), String::new(), String::new()));
                                                    entry.2.push_str(&delta);
                                            }
                                        }
                                        "response.output_item.added" => {
                                            if let Some(ref item) = event.item && item.get("type").and_then(|t| t.as_str()) == Some("function_call") && let (Some(call_id), Some(name)) = (
                                                        item.get("call_id").and_then(|v| v.as_str()),
                                                        item.get("name").and_then(|v| v.as_str()),
                                                    ) {
                                                        let entry = tool_call_acc
                                                            .entry(call_id.to_string())
                                                            .or_insert_with(|| (call_id.to_string(), String::new(), String::new()));
                                                        entry.1 = name.to_string();
                                            }
                                        }
                                        "response.completed" | "response.incomplete" => {
                                            if let Some(resp) = event.response {
                                                if let Some(ref resp_usage) = resp.usage {
                                                    usage = Some(Self::create_usage_metadata_responses(
                                                        resp_usage,
                                                        resp.service_tier.as_deref(),
                                                    ));
                                                }
                                                finish_reason = resp.status;
                                            }
                                            let mut final_chunk = ChatChunk::final_chunk(usage.take(), finish_reason.take());
                                            if !tool_call_acc.is_empty() {
                                                let tcs: Vec<ToolCall> = tool_call_acc
                                                    .drain()
                                                    .map(|(_, (id, name, args))| {
                                                        let parsed_args = serde_json::from_str(&args)
                                                            .unwrap_or(serde_json::Value::Object(Default::default()));
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
                                        }
                                        "response.refusal.delta" => {
                                            if let Some(delta) = event.delta {
                                                yield Ok(ChatChunk::new(delta));
                                            }
                                        }
                                        _ => {}
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

    /// Parse a Chat Completions API response.
    /// Matches Python `_create_chat_result`.
    fn parse_response(&self, response: OpenAIResponse) -> Result<ChatResult> {
        if let Some(ref error) = response.error {
            return Err(Error::api(0, error.to_string()));
        }

        let token_usage = response.usage.as_ref();
        let mut generations = Vec::new();

        for choice in &response.choices {
            let content = choice.message.content.clone().unwrap_or_default();

            let mut tool_calls = Vec::new();
            let mut invalid_tool_calls = Vec::new();

            for tc in choice.message.tool_calls.as_deref().unwrap_or_default() {
                match serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                    Ok(args) => {
                        tool_calls.push(
                            ToolCall::builder()
                                .name(&tc.function.name)
                                .args(args)
                                .id(tc.id.clone())
                                .build(),
                        );
                    }
                    Err(e) => {
                        invalid_tool_calls.push(
                            InvalidToolCall::builder()
                                .maybe_name(Some(tc.function.name.clone()))
                                .maybe_args(Some(tc.function.arguments.clone()))
                                .maybe_id(Some(tc.id.clone()))
                                .maybe_error(Some(e.to_string()))
                                .build(),
                        );
                    }
                }
            }

            let usage_metadata = token_usage
                .map(|u| Self::create_usage_metadata(u, response.service_tier.as_deref()));

            let mut generation_info = HashMap::new();
            if let Some(ref reason) = choice.finish_reason {
                generation_info.insert("finish_reason".to_string(), serde_json::json!(reason));
            }
            if let Some(ref logprobs) = choice.logprobs {
                generation_info.insert("logprobs".to_string(), logprobs.clone());
            }

            let mut response_metadata = HashMap::new();
            response_metadata.insert("model_name".to_string(), serde_json::json!(response.model));
            response_metadata.insert("model_provider".to_string(), serde_json::json!("openai"));
            if let Some(ref fp) = response.system_fingerprint {
                response_metadata.insert("system_fingerprint".to_string(), serde_json::json!(fp));
            }
            if let Some(ref id) = response.id {
                response_metadata.insert("id".to_string(), serde_json::json!(id));
            }
            if let Some(ref tier) = response.service_tier {
                response_metadata.insert("service_tier".to_string(), serde_json::json!(tier));
            }

            let ai_message = AIMessage::builder()
                .content(content)
                .tool_calls(tool_calls)
                .invalid_tool_calls(invalid_tool_calls)
                .maybe_usage_metadata(usage_metadata)
                .response_metadata(response_metadata)
                .build();

            let generation = if generation_info.is_empty() {
                ChatGeneration::new(BaseMessage::AI(ai_message))
            } else {
                ChatGeneration::with_info(BaseMessage::AI(ai_message), generation_info)
            };
            generations.push(generation);
        }

        let mut llm_output = HashMap::new();
        llm_output.insert("model_name".to_string(), serde_json::json!(response.model));
        llm_output.insert("model_provider".to_string(), serde_json::json!("openai"));
        if let Some(ref fp) = response.system_fingerprint {
            llm_output.insert("system_fingerprint".to_string(), serde_json::json!(fp));
        }
        if let Some(ref id) = response.id {
            llm_output.insert("id".to_string(), serde_json::json!(id));
        }
        if let Some(ref usage) = response.usage {
            llm_output.insert(
                "token_usage".to_string(),
                serde_json::json!({
                    "prompt_tokens": usage.prompt_tokens,
                    "completion_tokens": usage.completion_tokens,
                    "total_tokens": usage.total_tokens,
                }),
            );
        }

        Ok(ChatResult::with_llm_output(generations, llm_output))
    }

    /// Parse a Responses API response.
    /// Matches Python `_construct_lc_result_from_responses_api`.
    fn parse_responses_api_response(&self, response: ResponsesApiResponse) -> Result<ChatResult> {
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();
        let mut invalid_tool_calls = Vec::new();

        for output in &response.output {
            match output {
                ResponsesOutput::Message { content, .. } => {
                    for block in content {
                        if let ResponsesContent::OutputText { text, .. } = block {
                            text_content.push_str(text);
                        }
                    }
                }
                ResponsesOutput::FunctionCall {
                    name,
                    arguments,
                    call_id,
                    ..
                } => match serde_json::from_str::<serde_json::Value>(arguments) {
                    Ok(args) => {
                        tool_calls.push(
                            ToolCall::builder()
                                .name(name.clone())
                                .args(args)
                                .id(call_id.clone())
                                .build(),
                        );
                    }
                    Err(e) => {
                        invalid_tool_calls.push(
                            InvalidToolCall::builder()
                                .maybe_name(Some(name.clone()))
                                .maybe_args(Some(arguments.clone()))
                                .maybe_id(Some(call_id.clone()))
                                .maybe_error(Some(e.to_string()))
                                .build(),
                        );
                    }
                },
                ResponsesOutput::WebSearchCall {}
                | ResponsesOutput::FileSearchCall {}
                | ResponsesOutput::CodeInterpreterCall {}
                | ResponsesOutput::Other => {}
            }
        }

        let usage_metadata = response
            .usage
            .as_ref()
            .map(|u| Self::create_usage_metadata_responses(u, response.service_tier.as_deref()));

        let mut response_metadata = HashMap::new();
        response_metadata.insert("model_name".to_string(), serde_json::json!(response.model));
        response_metadata.insert("model_provider".to_string(), serde_json::json!("openai"));
        if let Some(ref status) = response.status {
            response_metadata.insert("status".to_string(), serde_json::json!(status));
        }
        if let Some(ref id) = response.id {
            response_metadata.insert("id".to_string(), serde_json::json!(id));
        }
        if let Some(ref tier) = response.service_tier {
            response_metadata.insert("service_tier".to_string(), serde_json::json!(tier));
        }

        let ai_message = AIMessage::builder()
            .content(text_content)
            .tool_calls(tool_calls)
            .invalid_tool_calls(invalid_tool_calls)
            .maybe_usage_metadata(usage_metadata)
            .response_metadata(response_metadata)
            .build();

        let generation = ChatGeneration::new(BaseMessage::AI(ai_message));
        Ok(ChatResult::new(vec![generation]))
    }

    /// Send an HTTP request and deserialize the JSON response.
    async fn send_json_request<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        payload: &serde_json::Value,
    ) -> Result<T> {
        let api_key = self.get_api_key()?;
        let client = self.build_client()?;

        let mut request = client
            .post(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json");

        if let Some(ref org) = self.organization {
            request = request.header("OpenAI-Organization", org);
        }

        let resp = request.json(payload).send().await.map_err(Error::Http)?;

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

    /// Send an HTTP request and return the response with headers.
    async fn send_json_request_with_headers<T: serde::de::DeserializeOwned>(
        &self,
        url: &str,
        payload: &serde_json::Value,
    ) -> Result<(T, HashMap<String, String>)> {
        let api_key = self.get_api_key()?;
        let client = self.build_client()?;

        let mut request = client
            .post(url)
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json");

        if let Some(ref org) = self.organization {
            request = request.header("OpenAI-Organization", org);
        }

        let resp = request.json(payload).send().await.map_err(Error::Http)?;

        if resp.status().is_success() {
            let headers: HashMap<String, String> = resp
                .headers()
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();
            let body = resp.json::<T>().await.map_err(|e| {
                Error::Json(serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    e.to_string(),
                )))
            })?;
            Ok((body, headers))
        } else {
            let status = resp.status().as_u16();
            let error_text = resp.text().await.unwrap_or_default();
            Err(Error::api(status, error_text))
        }
    }

    /// Build a retry strategy from `self.max_retries`.
    fn retry_strategy(&self) -> ConstantBuilder {
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_millis(0))
            .with_max_times(self.max_retries as usize)
    }

    /// Generate using the Responses API.
    async fn generate_responses_api(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatResult> {
        let url = format!("{}/responses", self.api_base);
        let payload = self.build_responses_api_payload(&messages, stop, None, false);

        if self.include_response_headers {
            let (resp, headers): (ResponsesApiResponse, HashMap<String, String>) =
                (|| self.send_json_request_with_headers(&url, &payload))
                    .retry(self.retry_strategy())
                    .when(|e| e.is_retryable())
                    .await?;
            let mut result = self.parse_responses_api_response(resp)?;
            self.inject_headers_into_result(&mut result, &headers);
            Ok(result)
        } else {
            let resp: ResponsesApiResponse = (|| self.send_json_request(&url, &payload))
                .retry(self.retry_strategy())
                .when(|e| e.is_retryable())
                .await?;
            self.parse_responses_api_response(resp)
        }
    }

    /// Inject captured HTTP headers into all generations' response_metadata.
    fn inject_headers_into_result(
        &self,
        result: &mut ChatResult,
        headers: &HashMap<String, String>,
    ) {
        let headers_value = serde_json::to_value(headers).unwrap_or_default();
        for generation in &mut result.generations {
            if let BaseMessage::AI(ref mut ai) = generation.message {
                ai.response_metadata
                    .insert("headers".to_string(), headers_value.clone());
            }
        }
    }

    /// Create usage metadata from OpenAI token usage.
    fn create_usage_metadata(usage: &OpenAIUsage, service_tier: Option<&str>) -> UsageMetadata {
        let input_tokens = usage.prompt_tokens as i64;
        let output_tokens = usage.completion_tokens as i64;
        let mut metadata = UsageMetadata::new(input_tokens, output_tokens);

        let tier = match service_tier {
            Some("priority" | "flex") => service_tier,
            _ => None,
        };

        let cached_tokens = usage
            .prompt_tokens_details
            .as_ref()
            .and_then(|d| d.cached_tokens)
            .map(|t| t as i64);
        let audio_input = usage
            .prompt_tokens_details
            .as_ref()
            .and_then(|d| d.audio_tokens)
            .map(|t| t as i64);
        let reasoning_tokens = usage
            .completion_tokens_details
            .as_ref()
            .and_then(|d| d.reasoning_tokens)
            .map(|t| t as i64);
        let audio_output = usage
            .completion_tokens_details
            .as_ref()
            .and_then(|d| d.audio_tokens)
            .map(|t| t as i64);

        if cached_tokens.is_some() || audio_input.is_some() || tier.is_some() {
            let mut input_details = crate::messages::InputTokenDetails {
                audio: audio_input,
                ..Default::default()
            };
            if let Some(tier_name) = tier {
                let cache_key = format!("{tier_name}_cache_read");
                if let Some(val) = cached_tokens {
                    input_details.extra.insert(cache_key.clone(), val);
                }
                let net = input_tokens - cached_tokens.unwrap_or(0);
                input_details.extra.insert(tier_name.to_string(), net);
            } else {
                input_details.cache_read = cached_tokens;
            }
            metadata.input_token_details = Some(input_details);
        }

        if reasoning_tokens.is_some() || audio_output.is_some() || tier.is_some() {
            let mut output_details = crate::messages::OutputTokenDetails {
                audio: audio_output,
                ..Default::default()
            };
            if let Some(tier_name) = tier {
                let reasoning_key = format!("{tier_name}_reasoning");
                if let Some(val) = reasoning_tokens {
                    output_details.extra.insert(reasoning_key.clone(), val);
                }
                let net = output_tokens - reasoning_tokens.unwrap_or(0);
                output_details.extra.insert(tier_name.to_string(), net);
            } else {
                output_details.reasoning = reasoning_tokens;
            }
            metadata.output_token_details = Some(output_details);
        }

        metadata
    }

    fn create_usage_metadata_responses(
        usage: &ResponsesUsage,
        service_tier: Option<&str>,
    ) -> UsageMetadata {
        let input_tokens = usage.input_tokens as i64;
        let output_tokens = usage.output_tokens as i64;
        let total_tokens = usage
            .total_tokens
            .map(|t| t as i64)
            .unwrap_or(input_tokens + output_tokens);
        let mut metadata = UsageMetadata {
            input_tokens,
            output_tokens,
            total_tokens,
            input_token_details: None,
            output_token_details: None,
        };

        let tier = match service_tier {
            Some("priority" | "flex") => service_tier,
            _ => None,
        };

        let cached_tokens = usage
            .input_tokens_details
            .as_ref()
            .and_then(|d| d.cached_tokens)
            .map(|t| t as i64);
        let reasoning_tokens = usage
            .output_tokens_details
            .as_ref()
            .and_then(|d| d.reasoning_tokens)
            .map(|t| t as i64);

        if cached_tokens.is_some() || tier.is_some() {
            let mut input_details = crate::messages::InputTokenDetails::default();
            if let Some(tier_name) = tier {
                let cache_key = format!("{tier_name}_cache_read");
                if let Some(val) = cached_tokens {
                    input_details.extra.insert(cache_key, val);
                }
                let net = input_tokens - cached_tokens.unwrap_or(0);
                input_details.extra.insert(tier_name.to_string(), net);
            } else {
                input_details.cache_read = cached_tokens;
            }
            metadata.input_token_details = Some(input_details);
        }

        if reasoning_tokens.is_some() || tier.is_some() {
            let mut output_details = crate::messages::OutputTokenDetails::default();
            if let Some(tier_name) = tier {
                let reasoning_key = format!("{tier_name}_reasoning");
                if let Some(val) = reasoning_tokens {
                    output_details.extra.insert(reasoning_key, val);
                }
                let net = output_tokens - reasoning_tokens.unwrap_or(0);
                output_details.extra.insert(tier_name.to_string(), net);
            } else {
                output_details.reasoning = reasoning_tokens;
            }
            metadata.output_token_details = Some(output_details);
        }

        metadata
    }

    /// Extract AIMessage from ChatResult, consuming it to avoid unnecessary cloning.
    fn extract_ai_message(result: ChatResult) -> Result<AIMessage> {
        let generation = result
            .generations
            .into_iter()
            .next()
            .ok_or_else(|| Error::other("No generations returned"))?;
        match generation.message {
            BaseMessage::AI(msg) => Ok(msg),
            _ => Err(Error::other("Expected AI message")),
        }
    }

    /// Internal stream implementation for Chat Completions API.
    async fn stream_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatStream> {
        self.stream_internal_with_tools(messages, stop, None, None)
            .await
    }

    async fn stream_internal_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        tools: Option<&[ToolDefinition]>,
        tool_choice: Option<&ToolChoice>,
    ) -> Result<ChatStream> {
        if self.should_use_responses_api(None) {
            let openai_tools: Option<Vec<serde_json::Value>> =
                tools.filter(|t| !t.is_empty()).map(|tools| {
                    tools
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
                        .collect()
                });
            return self
                .stream_responses_api(messages, stop, openai_tools.as_deref())
                .await;
        }

        let api_key = self.get_api_key()?;
        let client = self.build_client()?;

        let openai_tools: Option<Vec<serde_json::Value>> =
            tools.filter(|t| !t.is_empty()).map(|tools| {
                tools
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
                    .collect()
            });

        let mut payload =
            self.build_request_payload(&messages, stop, openai_tools.as_deref(), true);

        if let Some(choice) = tool_choice {
            let choice_json = match choice {
                ToolChoice::String(s) => {
                    if s == "any" {
                        serde_json::json!("required")
                    } else if WELL_KNOWN_TOOLS.contains(&s.as_str()) {
                        serde_json::json!({"type": s})
                    } else {
                        serde_json::json!(s)
                    }
                }
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

        let mut request = client
            .post(format!("{}/chat/completions", self.api_base))
            .header("Authorization", format!("Bearer {api_key}"))
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
            let mut usage: Option<UsageMetadata> = None;
            let mut finish_reason: Option<String> = None;
            let mut tool_call_acc: std::collections::HashMap<u32, (String, String, String)> =
                std::collections::HashMap::new();

            use futures::StreamExt;

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        while let Some(event_end) = buffer.find("\n\n") {
                            let event_data = buffer[..event_end].to_string();
                            buffer = buffer[event_end + 2..].to_string();

                            for line in event_data.lines() {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" {
                                        let mut final_chunk = ChatChunk::final_chunk(usage.take(), finish_reason.take());
                                        if !tool_call_acc.is_empty() {
                                            let mut sorted: Vec<_> = tool_call_acc.drain().collect();
                                            sorted.sort_by_key(|(idx, _)| *idx);
                                            let tcs: Vec<ToolCall> = sorted
                                                .into_iter()
                                                .map(|(_, (id, name, args))| {
                                                    let parsed_args = serde_json::from_str(&args)
                                                        .unwrap_or(serde_json::Value::Object(Default::default()));
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

                                    match serde_json::from_str::<OpenAIStreamChunk>(data) {
                                        Ok(chunk) => {
                                            if let Some(choice) = chunk.choices.first() {
                                                if let Some(ref content) = choice.delta.content {
                                                    yield Ok(ChatChunk::new(content.clone()));
                                                }
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
                                                usage = Some(Self::create_usage_metadata(u, chunk.service_tier.as_deref()));
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!("Failed to parse SSE chunk: {e}, data: {data}");
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

    fn has_astream_impl(&self) -> bool {
        true
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

    async fn _astream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&AsyncCallbackManagerForLLMRun>,
    ) -> Result<ChatGenerationStream> {
        let chat_stream = if !self.bound_tools.is_empty() {
            self.stream_internal_with_tools(
                messages,
                stop,
                Some(&self.bound_tools),
                self.bound_tool_choice.as_ref(),
            )
            .await?
        } else {
            self.stream_internal(messages, stop).await?
        };

        let generation_stream = async_stream::stream! {
            use futures::StreamExt;
            let mut pinned_stream = chat_stream;
            while let Some(result) = pinned_stream.next().await {
                match result {
                    Ok(chat_chunk) => {
                        let message = AIMessage::builder()
                            .content(&chat_chunk.content)
                            .tool_calls(chat_chunk.tool_calls.clone())
                            .maybe_usage_metadata(chat_chunk.usage_metadata.clone())
                            .build();
                        yield Ok(ChatGenerationChunk::new(message.into()));
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

    fn bind_tools(
        &self,
        tools: &[ToolLike],
        tool_choice: Option<ToolChoice>,
    ) -> Result<Box<dyn BaseChatModel>> {
        let mut bound = self.clone();
        bound.bound_tools = tools.iter().map(|t| t.to_definition()).collect();
        bound.bound_tool_choice = tool_choice;
        Ok(Box::new(bound))
    }



    fn with_structured_output(
        &self,
        schema: serde_json::Value,
        include_raw: bool,
    ) -> Result<
        Box<dyn Runnable<Input = LanguageModelInput, Output = serde_json::Value> + Send + Sync>,
    > {
        let tool_name = extract_tool_name_from_schema(&schema);
        let tool_like = ToolLike::Schema(schema);
        let bound_model = self.bind_tools(&[tool_like], Some(ToolChoice::any()))?;

        let output_parser =
            crate::output_parsers::openai_tools::JsonOutputKeyToolsParser::new(&tool_name)
                .with_first_tool_only(true);

        let model_runnable = ChatModelRunnable::new(std::sync::Arc::from(bound_model));

        if include_raw {
            Ok(Box::new(
                crate::language_models::StructuredOutputWithRaw::new(model_runnable, output_parser),
            ))
        } else {
            let chain = crate::runnables::base::pipe(model_runnable, output_parser);
            Ok(Box::new(chain))
        }
    }
}

impl ChatOpenAI {
    /// Bind tools with additional options (strict, parallel_tool_calls, response_format).
    pub fn bind_tools_with_options(
        &self,
        tools: &[ToolLike],
        tool_choice: Option<ToolChoice>,
        strict: Option<bool>,
        parallel_tool_calls: Option<bool>,
        response_format: Option<serde_json::Value>,
    ) -> Result<Box<dyn BaseChatModel>> {
        let mut bound = self.clone();
        bound.bound_tools = tools.iter().map(|t| t.to_definition()).collect();
        bound.bound_tool_choice = tool_choice;
        bound.bound_strict = strict;
        bound.bound_parallel_tool_calls = parallel_tool_calls;
        if let Some(fmt) = response_format {
            bound.response_format = Some(fmt);
        }
        Ok(Box::new(bound))
    }

    /// Enhanced structured output with method, strict, and tools parameters.
    /// 
    /// `method`: "function_calling" (default) or "json_schema"
    /// `strict`: Whether to enforce strict schema validation
    /// `tools`: Additional tools to combine with structured output
    pub fn with_structured_output_options(
        &self,
        schema: serde_json::Value,
        include_raw: bool,
        method: Option<&str>,
        strict: Option<bool>,
        tools: Option<&[ToolLike]>,
    ) -> Result<
        Box<dyn Runnable<Input = LanguageModelInput, Output = serde_json::Value> + Send + Sync>,
    > {
        let method = method.unwrap_or("function_calling");
        
        match method {
            "json_schema" => {
                let mut model = self.clone();
                let schema_name = schema
                    .get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("response_format");
                let mut json_schema = serde_json::json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": schema_name,
                        "schema": schema
                    }
                });
                if strict == Some(true) {
                    json_schema["json_schema"]["strict"] = serde_json::json!(true);
                    if let Some(schema_obj) = json_schema
                        .get_mut("json_schema")
                        .and_then(|js| js.get_mut("schema"))
                        .and_then(|s| s.as_object_mut())
                    {
                        schema_obj.insert(
                            "additionalProperties".to_string(),
                            serde_json::json!(false),
                        );
                        if let Some(props) = schema_obj.get("properties")
                            && let Some(props_obj) = props.as_object()
                        {
                            let all_keys: Vec<String> = props_obj.keys().cloned().collect();
                            schema_obj.insert("required".to_string(), serde_json::json!(all_keys));
                        }
                    }
                }
                model.response_format = Some(json_schema);
                
                if let Some(extra_tools) = tools {
                    model.bound_tools = extra_tools.iter().map(|t| t.to_definition()).collect();
                    model.bound_strict = strict;
                }
                
                let parse_json_content = crate::runnables::base::RunnableLambda::new(
                    |ai_msg: AIMessage| -> crate::error::Result<serde_json::Value> {
                        let content = ai_msg.text();
                        serde_json::from_str(&content)
                            .map_err(|e| crate::error::Error::other(format!("JSON parse error: {e}")))
                    },
                );

                let model_runnable =
                    ChatModelRunnable::new(std::sync::Arc::from(
                        Box::new(model) as Box<dyn BaseChatModel>
                    ));
                let chain = crate::runnables::base::pipe(model_runnable, parse_json_content);
                Ok(Box::new(chain))
            }
            _ => {
                // function_calling method
                let tool_name = extract_tool_name_from_schema(&schema);
                let tool_like = ToolLike::Schema(schema);
                let mut all_tools = vec![tool_like];
                if let Some(extra_tools) = tools {
                    all_tools.extend(extra_tools.iter().cloned());
                }
                let bound_model = if tools.is_some() {
                    self.bind_tools_with_options(
                        &all_tools,
                        Some(ToolChoice::any()),
                        strict,
                        None,
                        None,
                    )?
                } else {
                    self.bind_tools_with_options(
                        &all_tools,
                        Some(ToolChoice::any()),
                        strict,
                        None,
                        None,
                    )?
                };

                let output_parser =
                    crate::output_parsers::openai_tools::JsonOutputKeyToolsParser::new(&tool_name)
                        .with_first_tool_only(true);
                let model_runnable =
                    ChatModelRunnable::new(std::sync::Arc::from(bound_model));
                if include_raw {
                    Ok(Box::new(
                        crate::language_models::StructuredOutputWithRaw::new(
                            model_runnable,
                            output_parser,
                        ),
                    ))
                } else {
                    let chain = crate::runnables::base::pipe(model_runnable, output_parser);
                    Ok(Box::new(chain))
                }
            }
        }
    }

    /// Invoke the model with input and optional stop sequences.
    pub async fn invoke_with_stop(
        &self,
        input: LanguageModelInput,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessage> {
        let messages = input.to_messages();
        let result = self._generate_internal(messages, stop, None).await?;
        Self::extract_ai_message(result)
    }

    /// Internal generate implementation.
    async fn _generate_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        if self.should_use_responses_api(None) {
            return self.generate_responses_api(messages, stop).await;
        }

        let url = format!("{}/chat/completions", self.api_base);
        let payload = self.build_request_payload(&messages, stop, None, false);

        if self.include_response_headers {
            let (resp, headers): (OpenAIResponse, HashMap<String, String>) =
                (|| self.send_json_request_with_headers(&url, &payload))
                    .retry(self.retry_strategy())
                    .when(|e| e.is_retryable())
                    .await?;
            let mut result = self.parse_response(resp)?;
            self.inject_headers_into_result(&mut result, &headers);
            Ok(result)
        } else {
            let resp: OpenAIResponse = (|| self.send_json_request(&url, &payload))
                .retry(self.retry_strategy())
                .when(|e| e.is_retryable())
                .await?;
            self.parse_response(resp)
        }
    }

    /// Internal generate with tools implementation.
    async fn generate_with_tools_internal(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessage> {
        let openai_tools: Vec<serde_json::Value> = tools
            .iter()
            .map(|t| {
                let mut func = serde_json::json!({
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters
                });
                if self.bound_strict == Some(true) {
                    func["strict"] = serde_json::json!(true);
                    if let Some(params) = func.get_mut("parameters")
                        && let Some(obj) = params.as_object_mut()
                    {
                        obj.insert("additionalProperties".to_string(), serde_json::json!(false));
                        if let Some(props) = obj.get("properties")
                            && let Some(props_obj) = props.as_object()
                        {
                            let all_keys: Vec<String> = props_obj.keys().cloned().collect();
                            obj.insert("required".to_string(), serde_json::json!(all_keys));
                        }
                    }
                }
                serde_json::json!({
                    "type": "function",
                    "function": func
                })
            })
            .collect();

        if self.should_use_responses_api(None) {
            let url = format!("{}/responses", self.api_base);
            let payload =
                self.build_responses_api_payload(&messages, stop, Some(&openai_tools), false);

            let resp: ResponsesApiResponse = (|| self.send_json_request(&url, &payload))
                .retry(self.retry_strategy())
                .when(|e| e.is_retryable())
                .await?;

            let result = self.parse_responses_api_response(resp)?;
            return Self::extract_ai_message(result);
        }

        let url = format!("{}/chat/completions", self.api_base);
        let mut payload = self.build_request_payload(&messages, stop, Some(&openai_tools), false);

        if let Some(choice) = tool_choice {
            let choice_json = match choice {
                ToolChoice::String(s) => {
                    if s == "any" {
                        serde_json::json!("required")
                    } else if WELL_KNOWN_TOOLS.contains(&s.as_str()) {
                        serde_json::json!({"type": s})
                    } else {
                        serde_json::json!(s)
                    }
                }
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

        let resp: OpenAIResponse = (|| self.send_json_request(&url, &payload))
            .retry(self.retry_strategy())
            .when(|e| e.is_retryable())
            .await?;

        let result = self.parse_response(resp)?;
        Self::extract_ai_message(result)
    }
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    id: Option<String>,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
    system_fingerprint: Option<String>,
    service_tier: Option<String>,
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
    logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolCall {
    id: String,
    function: OpenAIFunction,
}

#[derive(Debug, Deserialize)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
    prompt_tokens_details: Option<TokenDetails>,
    completion_tokens_details: Option<TokenDetails>,
}

#[derive(Debug, Deserialize)]
struct TokenDetails {
    cached_tokens: Option<u32>,
    audio_tokens: Option<u32>,
    reasoning_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
    usage: Option<OpenAIUsage>,
    service_tier: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAIStreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamToolCall {
    index: u32,
    id: Option<String>,
    function: Option<OpenAIStreamFunction>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponsesApiResponse {
    id: Option<String>,
    model: String,
    output: Vec<ResponsesOutput>,
    usage: Option<ResponsesUsage>,
    status: Option<String>,
    service_tier: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponsesOutput {
    #[serde(rename = "message")]
    Message { content: Vec<ResponsesContent> },
    #[serde(rename = "function_call")]
    FunctionCall {
        name: String,
        arguments: String,
        call_id: String,
    },
    #[serde(rename = "web_search_call")]
    WebSearchCall {},
    #[serde(rename = "file_search_call")]
    FileSearchCall {},
    #[serde(rename = "code_interpreter_call")]
    CodeInterpreterCall {},
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponsesContent {
    #[serde(rename = "output_text")]
    OutputText {
        text: String,
        #[serde(default)]
        #[allow(dead_code)]
        annotations: Vec<TextAnnotation>,
    },
    #[serde(rename = "refusal")]
    #[allow(dead_code)]
    Refusal { refusal: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct ResponsesUsage {
    input_tokens: u32,
    output_tokens: u32,
    total_tokens: Option<u32>,
    output_tokens_details: Option<ResponsesTokenDetails>,
    input_tokens_details: Option<ResponsesTokenDetails>,
}

#[derive(Debug, Deserialize)]
struct ResponsesTokenDetails {
    reasoning_tokens: Option<u32>,
    cached_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ResponsesStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<String>,
    response: Option<ResponsesStreamResponse>,
    #[allow(dead_code)]
    annotation: Option<TextAnnotation>,
    call_id: Option<String>,
    item_id: Option<String>,
    item: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ResponsesStreamResponse {
    usage: Option<ResponsesUsage>,
    status: Option<String>,
    service_tier: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let model = ChatOpenAI::new("gpt-4o");
        assert_eq!(model.model, "gpt-4o");
        assert_eq!(model.max_retries, 2);
        assert!(!model.streaming);
    }

    #[test]
    fn test_builder_methods() {
        let model = ChatOpenAI::new("gpt-4o")
            .temperature(0.7)
            .max_tokens(1024)
            .api_key("test-key")
            .streaming(true)
            .seed(42);

        assert_eq!(model.temperature, Some(0.7));
        assert_eq!(model.max_tokens, Some(1024));
        assert_eq!(model.api_key, Some("test-key".to_string()));
        assert!(model.streaming);
        assert_eq!(model.seed, Some(42));
    }

    #[test]
    fn test_llm_type() {
        let model = ChatOpenAI::new("gpt-4o");
        assert_eq!(model.llm_type(), "openai-chat");
    }

    #[test]
    fn test_o1_temperature() {
        let model = ChatOpenAI::new("o1-preview");
        assert_eq!(model.temperature, Some(1.0));
    }

    #[test]
    fn test_should_use_responses_api_explicit() {
        let model = ChatOpenAI::new("gpt-4o").with_responses_api(true);
        assert!(model.should_use_responses_api(None));

        let model = ChatOpenAI::new("gpt-4o").with_responses_api(false);
        assert!(!model.should_use_responses_api(None));
    }

    #[test]
    fn test_should_use_responses_api_builtin_tools() {
        let model = ChatOpenAI::new("gpt-4o").with_builtin_tools(vec![BuiltinTool::WebSearch]);
        assert!(model.should_use_responses_api(None));
    }

    #[test]
    fn test_format_messages_ai_null_content_with_tool_calls() {
        let ai_msg = AIMessage::builder()
            .content("")
            .tool_calls(vec![
                ToolCall::builder()
                    .name("test")
                    .args(serde_json::json!({}))
                    .id("call_1".to_string())
                    .build(),
            ])
            .build();

        let model = ChatOpenAI::new("gpt-4o");
        let formatted = model.format_messages(&[BaseMessage::AI(ai_msg)]);
        assert_eq!(formatted.len(), 1);
        assert!(formatted[0]["content"].is_null());
    }

    #[test]
    fn test_build_request_payload_uses_max_completion_tokens() {
        let model = ChatOpenAI::new("gpt-4o").max_tokens(100);
        let payload = model.build_request_payload(&[], None, None, false);
        assert!(payload.get("max_tokens").is_none());
        assert_eq!(payload["max_completion_tokens"], 100);
    }

    #[test]
    fn test_build_request_payload_developer_role_for_o_series() {
        use crate::messages::SystemMessage;
        let sys = SystemMessage::builder().content("Be helpful").build();
        let model = ChatOpenAI::new("o3-mini");
        let payload = model.build_request_payload(&[BaseMessage::System(sys)], None, None, false);
        let messages = payload["messages"].as_array().expect("messages array");
        assert_eq!(messages[0]["role"], "developer");
    }

    #[test]
    fn test_build_request_payload_stream_options() {
        let model = ChatOpenAI::new("gpt-4o").stream_usage(true);
        let payload = model.build_request_payload(&[], None, None, true);
        assert_eq!(payload["stream"], true);
        assert_eq!(payload["stream_options"]["include_usage"], true);
    }

    #[test]
    fn test_filter_disabled_params_remove() {
        let mut disabled = HashMap::new();
        disabled.insert("temperature".to_string(), None);
        let model = ChatOpenAI::new("gpt-4o")
            .temperature(0.5)
            .disabled_params(disabled);
        let payload = model.build_request_payload(&[], None, None, false);
        assert!(payload.get("temperature").is_none());
    }

    #[test]
    fn test_filter_disabled_params_list_of_values() {
        let mut disabled = HashMap::new();
        disabled.insert(
            "temperature".to_string(),
            Some(serde_json::json!([0.5, 0.7])),
        );
        let model = ChatOpenAI::new("gpt-4o")
            .temperature(0.5)
            .disabled_params(disabled);
        let payload = model.build_request_payload(&[], None, None, false);
        assert!(payload.get("temperature").is_none());
    }

    #[test]
    fn test_gpt5_temperature_validation() {
        let model = ChatOpenAI::new("gpt-5").temperature(0.5);
        assert!(model.effective_temperature().is_none());

        let model = ChatOpenAI::new("gpt-5").temperature(1.0);
        assert_eq!(model.effective_temperature(), Some(1.0));

        let model = ChatOpenAI::new("gpt-5-chat").temperature(0.5);
        assert_eq!(model.effective_temperature(), Some(0.5));
    }

    #[test]
    fn test_is_o_series_model() {
        assert!(is_o_series_model("o1-preview"));
        assert!(is_o_series_model("o3-mini"));
        assert!(is_o_series_model("o4-mini"));
        assert!(!is_o_series_model("gpt-4o"));
        assert!(!is_o_series_model("gpt-4o-mini"));
    }

    #[test]
    fn test_payload_requires_responses_api() {
        let payload = serde_json::json!({"model": "gpt-4o", "messages": []});
        assert!(!payload_requires_responses_api(&payload));

        let payload = serde_json::json!({"model": "gpt-4o", "reasoning": {"effort": "medium"}});
        assert!(payload_requires_responses_api(&payload));

        let payload = serde_json::json!({
            "model": "gpt-4o",
            "tools": [{"type": "web_search"}]
        });
        assert!(payload_requires_responses_api(&payload));
    }
}
