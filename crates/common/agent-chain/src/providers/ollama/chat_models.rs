//! Ollama chat model implementation.
//!
//! This module provides the `ChatOllama` struct which implements the
//! `ChatModel` trait for Ollama-hosted models.
//!
//! # Example
//!
//! ```ignore
//! use agent_chain::ollama::ChatOllama;
//! use agent_chain::tools::tool;
//!
//! #[tool]
//! fn multiply(a: i64, b: i64) -> i64 {
//!     a * b
//! }
//!
//! let model = ChatOllama::new("llama3.1")
//!     .temperature(0.7);
//!
//! let model_with_tools = model.bind_tools(vec![multiply::tool()]);
//! let result = model_with_tools.invoke("What is 6 times 7?");
//! ```

use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use async_stream::try_stream;
use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::io::StreamReader;

use crate::callbacks::{CallbackManagerForLLMRun, Callbacks};
use crate::chat_models::{
    BaseChatModel, ChatChunk, ChatModelConfig, ChatStream, LangSmithParams, ToolChoice,
    UsageMetadata,
};
use crate::error::{Error, Result};
use crate::language_models::{BaseLanguageModel, LanguageModelConfig, LanguageModelInput};
use crate::messages::{AIMessage, BaseMessage, ToolCall};
use crate::outputs::{ChatGeneration, ChatResult, LLMResult};
use crate::tools::{BaseTool, ToolDefinition};

/// Default API base URL for Ollama.
const DEFAULT_API_BASE: &str = "http://localhost:11434";

/// Ollama chat model.
///
/// This struct implements the `ChatModel` trait for Ollama-hosted models.
/// It follows the LangChain pattern of provider-specific implementations.
///
/// # Example
///
/// ```ignore
/// use agent_chain_core::ollama::ChatOllama;
///
/// let model = ChatOllama::new("llama3.1")
///     .temperature(0.7)
///     .num_ctx(4096);
///
/// let messages = vec![HumanMessage::new("Hello!").into()];
/// let response = model.generate(messages, None).await?;
/// ```
#[derive(Debug, Clone)]
pub struct ChatOllama {
    /// Model name/identifier.
    model: String,
    /// Temperature for generation (0.0 - 1.0).
    temperature: Option<f64>,
    /// Base URL for API requests.
    base_url: String,
    /// Whether to validate the model exists on initialization.
    validate_model_on_init: bool,
    /// Enable Mirostat sampling (0 = disabled, 1 = Mirostat, 2 = Mirostat 2.0).
    mirostat: Option<i32>,
    /// Mirostat learning rate (eta).
    mirostat_eta: Option<f64>,
    /// Mirostat target entropy (tau).
    mirostat_tau: Option<f64>,
    /// Context window size.
    num_ctx: Option<u32>,
    /// Number of GPUs to use.
    num_gpu: Option<i32>,
    /// Number of threads.
    num_thread: Option<i32>,
    /// Maximum tokens to predict.
    num_predict: Option<i32>,
    /// Repeat last n tokens for penalty.
    repeat_last_n: Option<i32>,
    /// Repeat penalty.
    repeat_penalty: Option<f64>,
    /// Random seed.
    seed: Option<i64>,
    /// Stop sequences.
    stop: Option<Vec<String>>,
    /// Tail free sampling parameter.
    tfs_z: Option<f64>,
    /// Top-k sampling.
    top_k: Option<i32>,
    /// Top-p (nucleus) sampling.
    top_p: Option<f64>,
    /// Output format (empty string, "json", or JSON schema).
    format: Option<OllamaFormat>,
    /// How long to keep model in memory.
    keep_alive: Option<String>,
    /// Controls reasoning/thinking mode for supported models.
    reasoning: Option<bool>,
    /// Additional client kwargs.
    #[allow(dead_code)]
    client_kwargs: HashMap<String, serde_json::Value>,
    /// Chat model configuration.
    chat_model_config: ChatModelConfig,
    /// Language model configuration.
    language_model_config: LanguageModelConfig,
    /// HTTP client.
    #[allow(dead_code)]
    client: reqwest::Client,
}

/// Ollama output format.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum OllamaFormat {
    /// Raw format (no special formatting).
    Raw,
    /// JSON mode.
    Json,
    /// JSON schema.
    JsonSchema(serde_json::Value),
}

impl ChatOllama {
    /// Create a new ChatOllama instance.
    ///
    /// # Arguments
    ///
    /// * `model` - The model name (e.g., "llama3.1", "mistral").
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            temperature: None,
            base_url: DEFAULT_API_BASE.to_string(),
            validate_model_on_init: false,
            mirostat: None,
            mirostat_eta: None,
            mirostat_tau: None,
            num_ctx: None,
            num_gpu: None,
            num_thread: None,
            num_predict: None,
            repeat_last_n: None,
            repeat_penalty: None,
            seed: None,
            stop: None,
            tfs_z: None,
            top_k: None,
            top_p: None,
            format: None,
            keep_alive: None,
            reasoning: None,
            client_kwargs: HashMap::new(),
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

    /// Set the base URL for Ollama API.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Set whether to validate the model on initialization.
    pub fn validate_model_on_init(mut self, validate: bool) -> Self {
        self.validate_model_on_init = validate;
        self
    }

    /// Set the Mirostat sampling mode.
    pub fn mirostat(mut self, mode: i32) -> Self {
        self.mirostat = Some(mode);
        self
    }

    /// Set the Mirostat learning rate.
    pub fn mirostat_eta(mut self, eta: f64) -> Self {
        self.mirostat_eta = Some(eta);
        self
    }

    /// Set the Mirostat target entropy.
    pub fn mirostat_tau(mut self, tau: f64) -> Self {
        self.mirostat_tau = Some(tau);
        self
    }

    /// Set the context window size.
    pub fn num_ctx(mut self, ctx: u32) -> Self {
        self.num_ctx = Some(ctx);
        self
    }

    /// Set the number of GPUs.
    pub fn num_gpu(mut self, gpu: i32) -> Self {
        self.num_gpu = Some(gpu);
        self
    }

    /// Set the number of threads.
    pub fn num_thread(mut self, thread: i32) -> Self {
        self.num_thread = Some(thread);
        self
    }

    /// Set the maximum tokens to predict.
    pub fn num_predict(mut self, predict: i32) -> Self {
        self.num_predict = Some(predict);
        self
    }

    /// Set the repeat last n tokens.
    pub fn repeat_last_n(mut self, n: i32) -> Self {
        self.repeat_last_n = Some(n);
        self
    }

    /// Set the repeat penalty.
    pub fn repeat_penalty(mut self, penalty: f64) -> Self {
        self.repeat_penalty = Some(penalty);
        self
    }

    /// Set the random seed.
    pub fn seed(mut self, seed: i64) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Set stop sequences.
    pub fn stop(mut self, sequences: Vec<String>) -> Self {
        self.stop = Some(sequences);
        self
    }

    /// Set the tail free sampling parameter.
    pub fn tfs_z(mut self, z: f64) -> Self {
        self.tfs_z = Some(z);
        self
    }

    /// Set top-k sampling.
    pub fn top_k(mut self, k: i32) -> Self {
        self.top_k = Some(k);
        self
    }

    /// Set top-p sampling.
    pub fn top_p(mut self, p: f64) -> Self {
        self.top_p = Some(p);
        self
    }

    /// Set the output format.
    pub fn format(mut self, format: OllamaFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set JSON mode.
    pub fn json_mode(mut self) -> Self {
        self.format = Some(OllamaFormat::Json);
        self
    }

    /// Set how long to keep the model in memory.
    pub fn keep_alive(mut self, duration: impl Into<String>) -> Self {
        self.keep_alive = Some(duration.into());
        self
    }

    /// Set reasoning/thinking mode.
    pub fn reasoning(mut self, enabled: bool) -> Self {
        self.reasoning = Some(enabled);
        self
    }

    /// Bind tools to this chat model.
    ///
    /// Returns a `BoundChatOllama` that includes the tools.
    pub fn bind_tools<T: BaseTool + 'static>(self, tools: Vec<T>) -> BoundChatOllama {
        let tools: Vec<Arc<dyn BaseTool + Send + Sync>> =
            tools.into_iter().map(|t| Arc::new(t) as _).collect();
        BoundChatOllama::new(self, tools)
    }

    /// Get the base URL, checking environment variable if not set directly.
    fn get_base_url(&self) -> String {
        if self.base_url != DEFAULT_API_BASE {
            self.base_url.clone()
        } else {
            env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_API_BASE.to_string())
        }
    }

    /// Build the HTTP client.
    fn build_client(&self) -> reqwest::Client {
        reqwest::Client::new()
    }

    /// Convert messages to Ollama API format.
    fn format_messages(&self, messages: &[BaseMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .filter_map(|msg| match msg {
                BaseMessage::System(m) => Some(serde_json::json!({
                    "role": "system",
                    "content": m.content()
                })),
                BaseMessage::Human(m) => Some(serde_json::json!({
                    "role": "user",
                    "content": m.content()
                })),
                BaseMessage::AI(m) => {
                    let mut message = serde_json::json!({
                        "role": "assistant",
                    });

                    if !m.content().is_empty() {
                        message["content"] = serde_json::json!(m.content());
                    }

                    if !m.tool_calls().is_empty() {
                        let tool_calls: Vec<serde_json::Value> = m
                            .tool_calls()
                            .iter()
                            .map(|tc| {
                                serde_json::json!({
                                    "type": "function",
                                    "id": tc.id(),
                                    "function": {
                                        "name": tc.name(),
                                        "arguments": tc.args()
                                    }
                                })
                            })
                            .collect();
                        message["tool_calls"] = serde_json::Value::Array(tool_calls);
                    }

                    Some(message)
                }
                BaseMessage::Tool(m) => Some(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": m.tool_call_id(),
                    "content": m.content()
                })),
                BaseMessage::Remove(_) => {
                    // RemoveMessage is used for message management, not sent to API
                    None
                }
                BaseMessage::Chat(m) => Some(serde_json::json!({
                    "role": m.role(),
                    "content": m.content()
                })),
                BaseMessage::Function(m) => Some(serde_json::json!({
                    "role": "function",
                    "name": m.name(),
                    "content": m.content()
                })),
            })
            .collect()
    }

    /// Build the options object for the request.
    fn build_options(&self, stop: Option<Vec<String>>) -> serde_json::Value {
        let mut options = serde_json::Map::new();

        if let Some(temp) = self.temperature {
            options.insert("temperature".to_string(), serde_json::json!(temp));
        }
        if let Some(mirostat) = self.mirostat {
            options.insert("mirostat".to_string(), serde_json::json!(mirostat));
        }
        if let Some(eta) = self.mirostat_eta {
            options.insert("mirostat_eta".to_string(), serde_json::json!(eta));
        }
        if let Some(tau) = self.mirostat_tau {
            options.insert("mirostat_tau".to_string(), serde_json::json!(tau));
        }
        if let Some(ctx) = self.num_ctx {
            options.insert("num_ctx".to_string(), serde_json::json!(ctx));
        }
        if let Some(gpu) = self.num_gpu {
            options.insert("num_gpu".to_string(), serde_json::json!(gpu));
        }
        if let Some(thread) = self.num_thread {
            options.insert("num_thread".to_string(), serde_json::json!(thread));
        }
        if let Some(predict) = self.num_predict {
            options.insert("num_predict".to_string(), serde_json::json!(predict));
        }
        if let Some(n) = self.repeat_last_n {
            options.insert("repeat_last_n".to_string(), serde_json::json!(n));
        }
        if let Some(penalty) = self.repeat_penalty {
            options.insert("repeat_penalty".to_string(), serde_json::json!(penalty));
        }
        if let Some(seed) = self.seed {
            options.insert("seed".to_string(), serde_json::json!(seed));
        }
        if let Some(z) = self.tfs_z {
            options.insert("tfs_z".to_string(), serde_json::json!(z));
        }
        if let Some(k) = self.top_k {
            options.insert("top_k".to_string(), serde_json::json!(k));
        }
        if let Some(p) = self.top_p {
            options.insert("top_p".to_string(), serde_json::json!(p));
        }

        let stop_sequences = stop.or_else(|| self.stop.clone());
        if let Some(stop) = stop_sequences {
            options.insert("stop".to_string(), serde_json::json!(stop));
        }

        serde_json::Value::Object(options)
    }

    /// Build the request payload.
    fn build_request_payload(
        &self,
        messages: &[BaseMessage],
        stop: Option<Vec<String>>,
        tools: Option<&[serde_json::Value]>,
        stream: bool,
    ) -> serde_json::Value {
        let formatted_messages = self.format_messages(messages);
        let options = self.build_options(stop);

        let mut payload = serde_json::json!({
            "model": self.model,
            "messages": formatted_messages,
            "stream": stream
        });

        // Only include options if not empty
        if let serde_json::Value::Object(ref opts) = options
            && !opts.is_empty()
        {
            payload["options"] = options;
        }

        if let Some(format) = &self.format {
            match format {
                OllamaFormat::Raw => {}
                OllamaFormat::Json => {
                    payload["format"] = serde_json::json!("json");
                }
                OllamaFormat::JsonSchema(schema) => {
                    payload["format"] = schema.clone();
                }
            }
        }

        if let Some(keep_alive) = &self.keep_alive {
            payload["keep_alive"] = serde_json::json!(keep_alive);
        }

        if let Some(reasoning) = self.reasoning {
            payload["think"] = serde_json::json!(reasoning);
        }

        if let Some(tools) = tools
            && !tools.is_empty()
        {
            payload["tools"] = serde_json::Value::Array(tools.to_vec());
        }

        payload
    }

    /// Parse the API response into an AIMessage.
    fn parse_response_to_ai_message(&self, response: OllamaResponse) -> AIMessage {
        let content = response
            .message
            .as_ref()
            .and_then(|m| m.content.clone())
            .unwrap_or_default();

        let tool_calls: Vec<ToolCall> = response
            .message
            .as_ref()
            .and_then(|m| m.tool_calls.as_ref())
            .map(|tcs| {
                tcs.iter()
                    .filter_map(|tc| {
                        tc.function.as_ref().map(|f| {
                            let args = if let Some(args) = &f.arguments {
                                match args {
                                    serde_json::Value::String(s) => {
                                        serde_json::from_str(s).unwrap_or_default()
                                    }
                                    other => other.clone(),
                                }
                            } else {
                                serde_json::json!({})
                            };
                            ToolCall::new(&f.name, args)
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let ai_message = if tool_calls.is_empty() {
            AIMessage::new(content)
        } else {
            AIMessage::with_tool_calls(content, tool_calls)
        };

        // Add usage metadata if available
        if let (Some(prompt_eval_count), Some(eval_count)) =
            (response.prompt_eval_count, response.eval_count)
        {
            ai_message.with_usage_metadata(UsageMetadata::new(
                prompt_eval_count as i64,
                eval_count as i64,
            ))
        } else {
            ai_message
        }
    }
}

#[async_trait]
impl BaseLanguageModel for ChatOllama {
    fn llm_type(&self) -> &str {
        "chat-ollama"
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
            ls_provider: Some("ollama".to_string()),
            ls_model_name: Some(self.model.clone()),
            ls_model_type: Some("chat".to_string()),
            ls_temperature: self.temperature,
            ls_max_tokens: self.num_predict.map(|n| n as u32),
            ls_stop: stop.map(|s| s.to_vec()),
        }
    }
}

#[async_trait]
impl BaseChatModel for ChatOllama {
    fn chat_config(&self) -> &ChatModelConfig {
        &self.chat_model_config
    }

    async fn _generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        self._generate_internal(messages, stop, None).await
    }

    async fn generate_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        _tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessage> {
        // Convert tool definitions to Ollama format
        let ollama_tools: Vec<serde_json::Value> = tools
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

        let client = self.build_client();
        let payload = self.build_request_payload(&messages, stop, Some(&ollama_tools), false);
        let base_url = self.get_base_url();

        let response = client
            .post(format!("{}/api/chat", base_url))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(Error::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::api(status, error_text));
        }

        let ollama_resp: OllamaResponse = response.json().await.map_err(|e| {
            Error::Json(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )))
        })?;

        Ok(self.parse_response_to_ai_message(ollama_resp))
    }
}

impl ChatOllama {
    /// Internal generate implementation.
    async fn _generate_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let client = self.build_client();
        let payload = self.build_request_payload(&messages, stop, None, false);
        let base_url = self.get_base_url();

        let response = client
            .post(format!("{}/api/chat", base_url))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(Error::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::api(status, error_text));
        }

        let ollama_resp: OllamaResponse = response.json().await.map_err(|e| {
            Error::Json(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )))
        })?;

        let ai_message = self.parse_response_to_ai_message(ollama_resp);
        let generation = ChatGeneration::new(ai_message.into());
        Ok(ChatResult::new(vec![generation]))
    }

    /// Internal stream implementation.
    #[allow(dead_code)]
    async fn stream_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatStream> {
        let client = self.build_client();
        let payload = self.build_request_payload(&messages, stop, None, true);
        let base_url = self.get_base_url();

        let response = client
            .post(format!("{}/api/chat", base_url))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(Error::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::api(status, error_text));
        }

        let byte_stream = response
            .bytes_stream()
            .map(|result| result.map_err(std::io::Error::other));
        let stream_reader = StreamReader::new(byte_stream);
        let buf_reader = BufReader::new(stream_reader);
        let mut lines = buf_reader.lines();

        let stream = try_stream! {
            while let Some(line) = lines.next_line().await.map_err(Error::Io)? {
                if line.trim().is_empty() {
                    continue;
                }

                let stream_resp: OllamaResponse = serde_json::from_str(&line)
                    .map_err(Error::Json)?;

                let content = stream_resp
                    .message
                    .as_ref()
                    .and_then(|m| m.content.clone())
                    .unwrap_or_default();

                let is_done = stream_resp.done.unwrap_or(false);

                // Skip responses with done_reason='load' and empty content
                if is_done
                    && stream_resp.done_reason.as_deref() == Some("load")
                    && content.trim().is_empty()
                {
                    continue;
                }

                yield ChatChunk {
                    content,
                    is_final: is_done,
                };
            }
        };

        Ok(Box::pin(stream))
    }
}

/// Ollama API response structure.
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    #[allow(dead_code)]
    model: Option<String>,
    message: Option<OllamaMessage>,
    #[allow(dead_code)]
    done: Option<bool>,
    done_reason: Option<String>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

/// Ollama message in response.
#[derive(Debug, Deserialize)]
struct OllamaMessage {
    #[allow(dead_code)]
    role: Option<String>,
    content: Option<String>,
    tool_calls: Option<Vec<OllamaToolCall>>,
}

/// Ollama tool call in response.
#[derive(Debug, Deserialize)]
struct OllamaToolCall {
    function: Option<OllamaFunction>,
}

/// Ollama function in tool call.
#[derive(Debug, Deserialize)]
struct OllamaFunction {
    name: String,
    arguments: Option<serde_json::Value>,
}

// ============================================================================
// BoundChatOllama - Chat model with bound tools
// ============================================================================

/// A ChatOllama instance with bound tools.
///
/// This wraps a ChatOllama model and includes tool definitions
/// that will be passed to the model on each invocation.
pub struct BoundChatOllama {
    /// The underlying chat model.
    model: ChatOllama,
    /// Tools bound to this model.
    tools: Vec<Arc<dyn BaseTool + Send + Sync>>,
    /// Tool choice configuration.
    tool_choice: Option<ToolChoice>,
}

impl BoundChatOllama {
    /// Create a new bound chat model.
    pub fn new(model: ChatOllama, tools: Vec<Arc<dyn BaseTool + Send + Sync>>) -> Self {
        Self {
            model,
            tools,
            tool_choice: None,
        }
    }

    /// Set the tool choice.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Get the tool definitions.
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.tools.iter().map(|t| t.definition()).collect()
    }

    /// Get a reference to the underlying model.
    pub fn model(&self) -> &ChatOllama {
        &self.model
    }

    /// Get the tools.
    pub fn tools(&self) -> &[Arc<dyn BaseTool + Send + Sync>] {
        &self.tools
    }

    /// Invoke the model with a prompt (synchronous).
    ///
    /// This method creates a tokio runtime to run the async operation.
    /// For better performance in async contexts, use `invoke_async` instead.
    pub fn invoke(&self, prompt: impl Into<String>) -> Box<dyn MessageWithAny> {
        let prompt = prompt.into();
        let messages = vec![crate::messages::HumanMessage::new(prompt).into()];

        // Try to use existing runtime, or create a new one
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                // We're already in a tokio context, use block_in_place
                tokio::task::block_in_place(|| {
                    handle.block_on(self.invoke_async_internal(messages))
                })
            }
            Err(_) => {
                // No runtime, create a new one
                let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
                rt.block_on(self.invoke_async_internal(messages))
            }
        }
    }

    /// Invoke the model with messages (async).
    pub async fn invoke_async(&self, messages: Vec<BaseMessage>) -> Box<dyn MessageWithAny> {
        self.invoke_async_internal(messages).await
    }

    /// Internal async implementation.
    async fn invoke_async_internal(&self, messages: Vec<BaseMessage>) -> Box<dyn MessageWithAny> {
        use crate::language_models::BaseChatModel;
        let tool_definitions = self.tool_definitions();
        match self
            .model
            .generate_with_tools(messages, &tool_definitions, self.tool_choice.as_ref(), None)
            .await
        {
            Ok(ai_message) => Box::new(ai_message),
            Err(e) => Box::new(AIMessage::new(format!("Error: {}", e))),
        }
    }
}

/// Trait for messages that can be downcast via Any.
///
/// This allows for type-safe downcasting of message results,
/// similar to Python's isinstance() checks.
pub trait MessageWithAny: Send + Sync {
    /// Get a reference to self as Any for downcasting.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Get the message content.
    fn content(&self) -> &str;
}

impl MessageWithAny for AIMessage {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn content(&self) -> &str {
        AIMessage::content(self)
    }
}

impl MessageWithAny for BaseMessage {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn content(&self) -> &str {
        BaseMessage::content(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let model = ChatOllama::new("llama3.1");
        assert_eq!(model.model, "llama3.1");
        assert!(model.temperature.is_none());
    }

    #[test]
    fn test_builder_methods() {
        let model = ChatOllama::new("llama3.1")
            .temperature(0.7)
            .num_ctx(4096)
            .top_p(0.9)
            .repeat_penalty(1.1)
            .validate_model_on_init(true);

        assert_eq!(model.temperature, Some(0.7));
        assert_eq!(model.num_ctx, Some(4096));
        assert_eq!(model.top_p, Some(0.9));
        assert_eq!(model.repeat_penalty, Some(1.1));
        assert!(model.validate_model_on_init);
    }

    #[test]
    fn test_llm_type() {
        use crate::language_models::BaseLanguageModel;
        let model = ChatOllama::new("llama3.1");
        assert_eq!(model.llm_type(), "chat-ollama");
    }
}
