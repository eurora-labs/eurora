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

use async_trait::async_trait;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;
use tokio_util::io::StreamReader;

use super::compat::convert_from_v1_to_ollama;
use super::utils::{merge_auth_headers, parse_url_with_auth, validate_model};
use crate::callbacks::{CallbackManagerForLLMRun, Callbacks};
use crate::chat_models::{
    BaseChatModel, ChatChunk, ChatModelConfig, LangSmithParams, ToolChoice, UsageMetadata,
};
use crate::error::{Error, Result};
use crate::language_models::ToolLike;
use crate::language_models::{BaseLanguageModel, LanguageModelConfig, LanguageModelInput};
use crate::messages::{AIMessage, BaseMessage, ContentPart, ImageSource, MessageContent, ToolCall};
use crate::outputs::{ChatGeneration, ChatGenerationChunk, ChatResult, LLMResult};
use crate::runnables::base::Runnable;
use crate::tools::{BaseTool, ToolDefinition};

/// Default API base URL for Ollama.
const DEFAULT_API_BASE: &str = "http://localhost:11434";

/// Value for `keep_alive` parameter, matching Python's `int | str | None`.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum KeepAlive {
    /// Duration as seconds (integer).
    Seconds(i64),
    /// Duration as a string (e.g. `"5m"`).
    Duration(String),
}

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
/// let messages = vec![HumanMessage::builder().content("Hello!").build().into()];
/// let response = model.generate(messages, GenerateConfig::default()).await?;
/// ```
#[derive(Debug)]
pub struct ChatOllama {
    /// Model name/identifier.
    model: String,
    /// Temperature for generation (0.0 - 1.0).
    temperature: Option<f64>,
    /// Base URL for API requests. `None` means use `OLLAMA_HOST` env var or default.
    base_url: Option<String>,
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
    keep_alive: Option<KeepAlive>,
    /// Controls reasoning/thinking mode for supported models.
    /// Supports `true`/`false` or string intensities like `"low"`, `"medium"`, `"high"`.
    reasoning: Option<serde_json::Value>,
    output_version: Option<String>,
    /// Additional kwargs to pass to the HTTP clients. Pass headers in here.
    /// These arguments are passed to both synchronous and async clients.
    client_kwargs: HashMap<String, serde_json::Value>,
    /// Additional kwargs for async client only.
    async_client_kwargs: HashMap<String, serde_json::Value>,
    /// Additional kwargs for sync client only.
    sync_client_kwargs: HashMap<String, serde_json::Value>,
    /// Chat model configuration.
    chat_model_config: ChatModelConfig,
    /// Language model configuration.
    language_model_config: LanguageModelConfig,
    /// Whether the model has been validated (for lazy validation).
    model_validated: std::sync::atomic::AtomicBool,
    /// Tools bound to this model via `bind_tools()`.
    bound_tools: Vec<ToolDefinition>,
    /// Tool choice for bound tools.
    bound_tool_choice: Option<ToolChoice>,
}

impl Clone for ChatOllama {
    fn clone(&self) -> Self {
        Self {
            model: self.model.clone(),
            temperature: self.temperature,
            base_url: self.base_url.clone(),
            validate_model_on_init: self.validate_model_on_init,
            mirostat: self.mirostat,
            mirostat_eta: self.mirostat_eta,
            mirostat_tau: self.mirostat_tau,
            num_ctx: self.num_ctx,
            num_gpu: self.num_gpu,
            num_thread: self.num_thread,
            num_predict: self.num_predict,
            repeat_last_n: self.repeat_last_n,
            repeat_penalty: self.repeat_penalty,
            seed: self.seed,
            stop: self.stop.clone(),
            tfs_z: self.tfs_z,
            top_k: self.top_k,
            top_p: self.top_p,
            format: self.format.clone(),
            keep_alive: self.keep_alive.clone(),
            reasoning: self.reasoning.clone(),
            output_version: self.output_version.clone(),
            client_kwargs: self.client_kwargs.clone(),
            async_client_kwargs: self.async_client_kwargs.clone(),
            sync_client_kwargs: self.sync_client_kwargs.clone(),
            chat_model_config: self.chat_model_config.clone(),
            language_model_config: self.language_model_config.clone(),
            model_validated: std::sync::atomic::AtomicBool::new(
                self.model_validated
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            bound_tools: self.bound_tools.clone(),
            bound_tool_choice: self.bound_tool_choice.clone(),
        }
    }
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
            base_url: None,
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
            output_version: None,
            client_kwargs: HashMap::new(),
            async_client_kwargs: HashMap::new(),
            sync_client_kwargs: HashMap::new(),
            chat_model_config: ChatModelConfig::new(),
            language_model_config: LanguageModelConfig::new(),
            model_validated: std::sync::atomic::AtomicBool::new(false),
            bound_tools: Vec::new(),
            bound_tool_choice: None,
        }
    }

    /// Set the temperature.
    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set the base URL for Ollama API.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
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

    /// Set how long to keep the model in memory (string duration like `"5m"`).
    pub fn keep_alive(mut self, duration: impl Into<String>) -> Self {
        self.keep_alive = Some(KeepAlive::Duration(duration.into()));
        self
    }

    /// Set how long to keep the model in memory (seconds).
    pub fn keep_alive_seconds(mut self, seconds: i64) -> Self {
        self.keep_alive = Some(KeepAlive::Seconds(seconds));
        self
    }

    /// Set reasoning/thinking mode.
    ///
    /// Accepts `true`/`false` to enable/disable, or a string intensity like
    /// `"low"`, `"medium"`, `"high"` for supported models.
    pub fn reasoning(mut self, value: impl Into<serde_json::Value>) -> Self {
        self.reasoning = Some(value.into());
        self
    }

    pub fn output_version(mut self, version: impl Into<String>) -> Self {
        self.output_version = Some(version.into());
        self
    }

    /// Set additional client kwargs.
    pub fn client_kwargs(mut self, kwargs: HashMap<String, serde_json::Value>) -> Self {
        self.client_kwargs = kwargs;
        self
    }

    /// Set additional async client kwargs.
    pub fn async_client_kwargs(mut self, kwargs: HashMap<String, serde_json::Value>) -> Self {
        self.async_client_kwargs = kwargs;
        self
    }

    /// Set additional sync client kwargs.
    pub fn sync_client_kwargs(mut self, kwargs: HashMap<String, serde_json::Value>) -> Self {
        self.sync_client_kwargs = kwargs;
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

    /// Get the resolved base URL. Checks env var `OLLAMA_HOST` if `base_url` is `None`.
    ///
    /// Uses `parse_url_with_auth` to strip userinfo credentials from the URL.
    pub fn get_base_url(&self) -> String {
        let raw_url = match &self.base_url {
            Some(url) => url.clone(),
            None => env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_API_BASE.to_string()),
        };

        let (cleaned_url, _) = parse_url_with_auth(Some(&raw_url));
        let url = cleaned_url.unwrap_or(raw_url);
        url.trim_end_matches('/').to_string()
    }

    /// Build the HTTP client, optionally with basic auth from the URL.
    ///
    /// Uses `parse_url_with_auth` and `merge_auth_headers` from the utils module,
    /// matching Python's `_set_clients` validator pattern.
    fn build_client(&self) -> reqwest::Client {
        let raw_url = match &self.base_url {
            Some(url) => url.clone(),
            None => env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_API_BASE.to_string()),
        };

        let (_, auth_headers) = parse_url_with_auth(Some(&raw_url));

        let mut all_headers = HashMap::new();
        merge_auth_headers(&mut all_headers, auth_headers);

        let mut builder = reqwest::Client::builder();

        if !all_headers.is_empty() {
            let mut header_map = reqwest::header::HeaderMap::new();
            for (key, value) in &all_headers {
                if let (Ok(name), Ok(val)) = (
                    reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                    reqwest::header::HeaderValue::from_str(value),
                ) {
                    header_map.insert(name, val);
                }
            }
            builder = builder.default_headers(header_map);
        }

        builder.build().unwrap_or_else(|_| reqwest::Client::new())
    }

    /// Validate that the model exists in Ollama by listing local models.
    ///
    /// Delegates to `utils::validate_model()`, matching Python's pattern where
    /// `validate_model` is a standalone utility function.
    pub async fn validate_model(&self) -> Result<()> {
        let client = self.build_client();
        let base_url = self.get_base_url();
        validate_model(&client, &base_url, &self.model).await
    }

    /// Lazily validate the model if `validate_model_on_init` is set.
    async fn ensure_model_validated(&self) -> Result<()> {
        if self.validate_model_on_init
            && !self
                .model_validated
                .load(std::sync::atomic::Ordering::Relaxed)
        {
            self.validate_model().await?;
            self.model_validated
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(())
    }

    /// Returns true if reasoning mode is enabled (truthy value).
    fn is_reasoning_enabled(&self) -> bool {
        match &self.reasoning {
            Some(serde_json::Value::Bool(b)) => *b,
            Some(serde_json::Value::String(s)) => !s.is_empty(),
            Some(serde_json::Value::Null) | None => false,
            Some(_) => true,
        }
    }

    /// Convert messages to Ollama API format.
    ///
    /// Matches Python's `_convert_messages_to_ollama_messages`.
    /// Pre-processes v1-format AIMessages by converting their content blocks
    /// via `convert_from_v1_to_ollama`.
    pub fn format_messages(&self, messages: &[BaseMessage]) -> Result<Vec<serde_json::Value>> {
        let messages: Vec<std::borrow::Cow<'_, BaseMessage>> = messages
            .iter()
            .map(|msg| {
                if let BaseMessage::AI(ai) = msg {
                    let is_v1 = ai
                        .response_metadata
                        .get("output_version")
                        .and_then(|v| v.as_str())
                        == Some("v1");
                    if is_v1 {
                        let content_values = ai.content.as_json_values();
                        let converted = convert_from_v1_to_ollama(&content_values);
                        let new_content = MessageContent::Parts(
                            converted.into_iter().map(ContentPart::Other).collect(),
                        );
                        let mut new_ai = ai.clone();
                        new_ai.content = new_content;
                        return std::borrow::Cow::Owned(BaseMessage::AI(new_ai));
                    }
                }
                std::borrow::Cow::Borrowed(msg)
            })
            .collect();

        let mut ollama_messages = Vec::new();

        for msg in messages.iter() {
            let formatted = match msg.as_ref() {
                BaseMessage::System(m) => serde_json::json!({
                    "role": "system",
                    "content": m.content.as_text(),
                    "images": [],
                }),
                BaseMessage::Human(m) => {
                    let (content, images) = extract_content_and_images(&m.content);
                    serde_json::json!({
                        "role": "user",
                        "content": content,
                        "images": images,
                    })
                }
                BaseMessage::AI(m) => {
                    let mut message = serde_json::json!({
                        "role": "assistant",
                        "content": m.content.as_text(),
                        "images": [],
                    });

                    if !m.tool_calls.is_empty() {
                        let tool_calls: Vec<serde_json::Value> = m
                            .tool_calls
                            .iter()
                            .map(lc_tool_call_to_openai_tool_call)
                            .collect();
                        message["tool_calls"] = serde_json::Value::Array(tool_calls);
                    }

                    message
                }
                BaseMessage::Tool(m) => serde_json::json!({
                    "role": "tool",
                    "content": m.content,
                    "images": [],
                    "tool_call_id": m.tool_call_id,
                }),
                BaseMessage::Chat(m) => serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                    "images": [],
                }),
                BaseMessage::Remove(_) => continue,
                _ => {
                    return Err(Error::Other(
                        "Received unsupported message type for Ollama.".to_string(),
                    ));
                }
            };
            ollama_messages.push(formatted);
        }

        Ok(ollama_messages)
    }

    /// Build the options object for the request.
    pub fn build_options(&self, stop: Option<Vec<String>>) -> Result<serde_json::Value> {
        if self.stop.is_some() && stop.is_some() {
            return Err(Error::Other(
                "`stop` found in both the input and default params.".into(),
            ));
        }

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

        Ok(serde_json::Value::Object(options))
    }

    /// Build the request payload.
    ///
    /// Matches Python's `_chat_params`.
    pub fn build_request_payload(
        &self,
        messages: &[BaseMessage],
        stop: Option<Vec<String>>,
        tools: Option<&[serde_json::Value]>,
        stream: bool,
    ) -> Result<serde_json::Value> {
        let formatted_messages = self.format_messages(messages)?;
        let options = self.build_options(stop)?;

        let mut payload = serde_json::json!({
            "model": self.model,
            "messages": formatted_messages,
            "stream": stream
        });

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
            match keep_alive {
                KeepAlive::Seconds(s) => payload["keep_alive"] = serde_json::json!(s),
                KeepAlive::Duration(d) => payload["keep_alive"] = serde_json::json!(d),
            }
        }

        if let Some(reasoning) = &self.reasoning {
            payload["think"] = reasoning.clone();
        }

        if let Some(tools) = tools
            && !tools.is_empty()
        {
            payload["tools"] = serde_json::Value::Array(tools.to_vec());
        }

        Ok(payload)
    }

    /// Parse the API response into an AIMessage.
    fn parse_response_to_ai_message(&self, response: &OllamaResponse) -> AIMessage {
        let content = response
            .message
            .as_ref()
            .and_then(|m| m.content.clone())
            .unwrap_or_default();

        let tool_calls = get_tool_calls_from_response(response);

        let mut additional_kwargs = HashMap::new();
        if self.is_reasoning_enabled()
            && let Some(thinking) = response.message.as_ref().and_then(|m| m.thinking.as_ref())
        {
            additional_kwargs.insert(
                "reasoning_content".to_string(),
                serde_json::Value::String(thinking.clone()),
            );
        }

        let mut response_metadata = HashMap::new();
        if let Some(model) = &response.model {
            response_metadata.insert(
                "model_name".to_string(),
                serde_json::Value::String(model.clone()),
            );
        }
        response_metadata.insert(
            "model_provider".to_string(),
            serde_json::Value::String("ollama".to_string()),
        );
        if let Some(done_reason) = &response.done_reason {
            response_metadata.insert(
                "done_reason".to_string(),
                serde_json::Value::String(done_reason.clone()),
            );
        }

        let usage_metadata = if let (Some(prompt_eval_count), Some(eval_count)) =
            (response.prompt_eval_count, response.eval_count)
        {
            Some(UsageMetadata::new(
                prompt_eval_count as i64,
                eval_count as i64,
            ))
        } else {
            None
        };

        AIMessage::builder()
            .content(content)
            .tool_calls(tool_calls)
            .additional_kwargs(additional_kwargs)
            .response_metadata(response_metadata)
            .maybe_usage_metadata(usage_metadata)
            .build()
    }

    /// Build generation_info from a stream response, including all fields.
    ///
    /// Matches Python's `generation_info = dict(stream_resp)` which captures the
    /// entire response dict (timing data, token counts, etc.) plus `model_name`
    /// and `model_provider`, minus the `message` field.
    fn build_generation_info(response: &OllamaResponse) -> HashMap<String, serde_json::Value> {
        let mut info = HashMap::new();
        if let Some(model) = &response.model {
            info.insert("model".to_string(), serde_json::json!(model));
            info.insert("model_name".to_string(), serde_json::json!(model));
        }
        info.insert("model_provider".to_string(), serde_json::json!("ollama"));
        if let Some(done) = response.done {
            info.insert("done".to_string(), serde_json::json!(done));
        }
        if let Some(done_reason) = &response.done_reason {
            info.insert("done_reason".to_string(), serde_json::json!(done_reason));
        }
        if let Some(prompt_eval_count) = response.prompt_eval_count {
            info.insert(
                "prompt_eval_count".to_string(),
                serde_json::json!(prompt_eval_count),
            );
        }
        if let Some(eval_count) = response.eval_count {
            info.insert("eval_count".to_string(), serde_json::json!(eval_count));
        }
        if let Some(total_duration) = response.total_duration {
            info.insert(
                "total_duration".to_string(),
                serde_json::json!(total_duration),
            );
        }
        if let Some(load_duration) = response.load_duration {
            info.insert(
                "load_duration".to_string(),
                serde_json::json!(load_duration),
            );
        }
        if let Some(prompt_eval_duration) = response.prompt_eval_duration {
            info.insert(
                "prompt_eval_duration".to_string(),
                serde_json::json!(prompt_eval_duration),
            );
        }
        if let Some(eval_duration) = response.eval_duration {
            info.insert(
                "eval_duration".to_string(),
                serde_json::json!(eval_duration),
            );
        }
        if let Some(created_at) = &response.created_at {
            info.insert("created_at".to_string(), serde_json::json!(created_at));
        }
        info
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
            ls_max_tokens: None,
            ls_stop: stop.map(|s| s.to_vec()).or_else(|| self.stop.clone()),
        }
    }
}

#[async_trait]
impl BaseChatModel for ChatOllama {
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
                .generate_with_tools(
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
        _run_manager: Option<&crate::callbacks::AsyncCallbackManagerForLLMRun>,
    ) -> Result<crate::language_models::ChatGenerationStream> {
        let reasoning_enabled = self.is_reasoning_enabled();
        let chat_stream = self.stream_internal(messages, stop).await?;

        let generation_stream = async_stream::stream! {
            use futures::StreamExt;

            let mut pinned_stream = chat_stream;

            while let Some(result) = pinned_stream.next().await {
                match result {
                    Ok(chat_chunk) => {
                        let mut additional_kwargs = HashMap::new();
                        if reasoning_enabled
                            && let Some(reasoning) = &chat_chunk.reasoning_content {
                                additional_kwargs.insert(
                                    "reasoning_content".to_string(),
                                    serde_json::Value::String(reasoning.clone()),
                                );
                            }

                        let mut response_metadata = HashMap::new();
                        if let Some(ref info) = chat_chunk.generation_info {
                            if let Some(model) = info.get("model_name") {
                                response_metadata.insert("model_name".to_string(), model.clone());
                            }
                            if let Some(provider) = info.get("model_provider") {
                                response_metadata.insert("model_provider".to_string(), provider.clone());
                            }
                            if let Some(reason) = info.get("done_reason") {
                                response_metadata.insert("done_reason".to_string(), reason.clone());
                            }
                        }

                        let message = AIMessage::builder()
                            .content(&chat_chunk.chunk.content)
                            .tool_calls(chat_chunk.chunk.tool_calls.clone())
                            .maybe_usage_metadata(chat_chunk.chunk.usage_metadata.clone())
                            .additional_kwargs(additional_kwargs)
                            .response_metadata(response_metadata)
                            .build();

                        let chunk = if let Some(info) = chat_chunk.generation_info {
                            ChatGenerationChunk::with_info(message.into(), info)
                        } else {
                            ChatGenerationChunk::new(message.into())
                        };
                        yield Ok(chunk);
                    }
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                }
            }
        };

        Ok(Box::pin(generation_stream) as crate::language_models::ChatGenerationStream)
    }

    async fn generate_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        _tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<AIMessage> {
        self.ensure_model_validated().await?;

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
        let payload = self.build_request_payload(&messages, stop, Some(&ollama_tools), false)?;
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
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            return Err(Error::api(status, error_text));
        }

        let ollama_resp: OllamaResponse = response.json().await.map_err(|e| {
            Error::Json(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                e.to_string(),
            )))
        })?;

        Ok(self.parse_response_to_ai_message(&ollama_resp))
    }

    fn bind_tools(
        &self,
        tools: &[ToolLike],
        tool_choice: Option<ToolChoice>,
    ) -> Result<Box<dyn BaseChatModel>> {
        let mut bound = self.clone();
        bound.bound_tools = tools
            .iter()
            .map(|t| t.to_definition())
            .collect::<std::result::Result<Vec<_>, _>>()?;
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
        let tool_name = crate::language_models::extract_tool_name_from_schema(&schema)?;
        let tool_like = ToolLike::Schema(schema);
        let bound_model = self.bind_tools(&[tool_like], Some(ToolChoice::any()))?;

        let output_parser =
            crate::output_parsers::openai_tools::JsonOutputKeyToolsParser::new(&tool_name)
                .with_first_tool_only(true);

        let model_runnable =
            crate::language_models::ChatModelRunnable::new(std::sync::Arc::from(bound_model));

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

impl ChatOllama {
    /// Internal generate implementation.
    ///
    /// Aggregates the streaming response into a single result, matching
    /// Python's `_generate` which calls `_chat_stream_with_aggregation`.
    /// Uses `ChatGenerationChunk` addition to aggregate, matching Python's
    /// `final_chunk += chunk` semantics.
    async fn _generate_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        let stream = self.stream_internal(messages, stop).await?;
        futures::pin_mut!(stream);

        let mut final_chunk: Option<ChatGenerationChunk> = None;

        while let Some(result) = stream.next().await {
            let ollama_chunk = result?;

            let mut additional_kwargs = HashMap::new();
            if let Some(reasoning) = &ollama_chunk.reasoning_content {
                additional_kwargs.insert(
                    "reasoning_content".to_string(),
                    serde_json::Value::String(reasoning.clone()),
                );
            }

            let mut response_metadata = HashMap::new();
            if let Some(ref info) = ollama_chunk.generation_info {
                if let Some(model) = info.get("model_name") {
                    response_metadata.insert("model_name".to_string(), model.clone());
                }
                if let Some(provider) = info.get("model_provider") {
                    response_metadata.insert("model_provider".to_string(), provider.clone());
                }
                if let Some(reason) = info.get("done_reason") {
                    response_metadata.insert("done_reason".to_string(), reason.clone());
                }
            }

            let message = AIMessage::builder()
                .content(&ollama_chunk.chunk.content)
                .tool_calls(ollama_chunk.chunk.tool_calls)
                .maybe_usage_metadata(ollama_chunk.chunk.usage_metadata)
                .additional_kwargs(additional_kwargs)
                .response_metadata(response_metadata)
                .build();

            let gen_chunk = if let Some(info) = ollama_chunk.generation_info {
                ChatGenerationChunk::with_info(message.into(), info)
            } else {
                ChatGenerationChunk::new(message.into())
            };

            final_chunk = Some(match final_chunk {
                Some(existing) => existing + gen_chunk,
                None => gen_chunk,
            });
        }

        let final_chunk = final_chunk
            .ok_or_else(|| Error::Other("No data received from Ollama stream.".to_string()))?;

        let generation_info = final_chunk.generation_info.clone();

        let (content, usage_metadata, tool_calls, chunk_additional_kwargs) =
            match &final_chunk.message {
                BaseMessage::AI(ai) => (
                    ai.text(),
                    ai.usage_metadata.clone(),
                    ai.tool_calls.clone(),
                    ai.additional_kwargs.clone(),
                ),
                other => (other.text(), None, vec![], HashMap::new()),
            };

        let ai_message = AIMessage::builder()
            .content(content)
            .tool_calls(tool_calls)
            .additional_kwargs(chunk_additional_kwargs)
            .maybe_usage_metadata(usage_metadata)
            .build();

        let generation = if let Some(info) = generation_info {
            ChatGeneration::with_info(ai_message.into(), info)
        } else {
            ChatGeneration::new(ai_message.into())
        };

        Ok(ChatResult::new(vec![generation]))
    }

    /// Internal stream implementation.
    ///
    /// Returns a stream of `OllamaStreamChunk` which carries both the `ChatChunk`
    /// and Ollama-specific metadata (reasoning content, generation_info).
    ///
    /// Matches Python's `_aiterate_over_stream` / `_iterate_over_stream`.
    async fn stream_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<OllamaStreamChunk>> + Send>>>
    {
        self.ensure_model_validated().await?;

        let client = self.build_client();
        let tools = if !self.bound_tools.is_empty() {
            let ollama_tools: Vec<serde_json::Value> = self
                .bound_tools
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
            Some(ollama_tools)
        } else {
            None
        };
        let payload = self.build_request_payload(&messages, stop, tools.as_deref(), true)?;
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
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read response body: {e}>"));
            return Err(Error::api(status, error_text));
        }

        let byte_stream = response
            .bytes_stream()
            .map(|r| r.map_err(std::io::Error::other));
        let reader = tokio::io::BufReader::new(StreamReader::new(byte_stream));
        let mut lines = reader.lines();

        let reasoning_enabled = self.is_reasoning_enabled();

        let stream = async_stream::try_stream! {
            while let Some(line) = lines.next_line().await.map_err(|e| Error::Other(e.to_string()))? {
                if line.is_empty() {
                    continue;
                }

                let stream_resp: OllamaResponse = serde_json::from_str(&line)
                    .map_err(|e| Error::Other(format!("Failed to parse Ollama response: {}", e)))?;

                let content = stream_resp
                    .message
                    .as_ref()
                    .and_then(|m| m.content.as_deref())
                    .unwrap_or("");

                let is_done = stream_resp.done.unwrap_or(false);

                if is_done
                    && stream_resp.done_reason.as_deref() == Some("load")
                    && content.trim().is_empty()
                {
                    tracing::warn!(
                        "Ollama returned empty response with done_reason='load'. \
                         This typically indicates the model was loaded but no content \
                         was generated. Skipping this response."
                    );
                    continue;
                }

                let tool_calls = get_tool_calls_from_response(&stream_resp);

                let reasoning_content = if reasoning_enabled {
                    stream_resp
                        .message
                        .as_ref()
                        .and_then(|m| m.thinking.clone())
                } else {
                    None
                };

                let usage = if let (Some(prompt_eval_count), Some(eval_count)) =
                    (stream_resp.prompt_eval_count, stream_resp.eval_count)
                {
                    Some(UsageMetadata::new(prompt_eval_count as i64, eval_count as i64))
                } else {
                    None
                };

                let generation_info = if is_done {
                    Some(Self::build_generation_info(&stream_resp))
                } else {
                    None
                };

                let mut chunk = ChatChunk::new(content);
                chunk.tool_calls = tool_calls;
                chunk.usage_metadata = usage;
                if is_done {
                    chunk.is_final = true;
                    chunk.finish_reason = stream_resp.done_reason.clone();
                }

                yield OllamaStreamChunk {
                    chunk,
                    reasoning_content,
                    generation_info,
                };
            }
        };

        Ok(Box::pin(stream))
    }
}

/// A streaming chunk from Ollama that carries both the standard `ChatChunk`
/// and Ollama-specific metadata like reasoning content and generation info.
struct OllamaStreamChunk {
    chunk: ChatChunk,
    reasoning_content: Option<String>,
    generation_info: Option<HashMap<String, serde_json::Value>>,
}

/// Ollama API response structure.
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    model: Option<String>,
    message: Option<OllamaMessage>,
    done: Option<bool>,
    done_reason: Option<String>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    load_duration: Option<u64>,
    #[serde(default)]
    prompt_eval_duration: Option<u64>,
    #[serde(default)]
    eval_duration: Option<u64>,
    #[serde(default)]
    created_at: Option<String>,
}

/// Ollama message in response.
#[derive(Debug, Deserialize)]
struct OllamaMessage {
    #[allow(dead_code)]
    role: Option<String>,
    content: Option<String>,
    tool_calls: Option<Vec<OllamaToolCall>>,
    thinking: Option<String>,
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

/// Convert a LangChain ToolCall to OpenAI tool call format.
fn lc_tool_call_to_openai_tool_call(tc: &ToolCall) -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "id": tc.id,
        "function": {
            "name": tc.name,
            "arguments": tc.args
        }
    })
}

/// Extract tool calls from an Ollama response.
fn get_tool_calls_from_response(response: &OllamaResponse) -> Vec<ToolCall> {
    response
        .message
        .as_ref()
        .and_then(|m| m.tool_calls.as_ref())
        .map(|tcs| {
            tcs.iter()
                .filter_map(|tc| {
                    tc.function.as_ref().and_then(|f| {
                        let args = parse_tool_call_arguments(f.arguments.as_ref(), &f.name).ok()?;
                        Some(ToolCall::builder().name(&f.name).args(args).build())
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse tool call arguments with robust handling of Ollama's inconsistent formats.
///
/// Handles: dict arguments (with nested string-encoded JSON), string arguments
/// (tries JSON parse), and filters out `functionName` metadata.
pub fn parse_tool_call_arguments(
    raw_args: Option<&serde_json::Value>,
    function_name: &str,
) -> Result<serde_json::Value> {
    let Some(args) = raw_args else {
        return Ok(serde_json::json!({}));
    };

    match args {
        serde_json::Value::String(s) => match serde_json::from_str::<serde_json::Value>(s) {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::Other(format!(
                "Function {} arguments:\n\n{}\n\nare not valid JSON. Received error: {}",
                function_name, s, e
            ))),
        },
        serde_json::Value::Object(map) => {
            let mut parsed = serde_json::Map::new();
            for (key, value) in map {
                if key == "functionName"
                    && let serde_json::Value::String(v) = value
                    && v == function_name
                {
                    continue;
                }
                match value {
                    serde_json::Value::String(s) => {
                        if let Ok(parsed_value) = serde_json::from_str::<serde_json::Value>(s)
                            && (parsed_value.is_object() || parsed_value.is_array())
                        {
                            parsed.insert(key.clone(), parsed_value);
                            continue;
                        }
                        parsed.insert(key.clone(), value.clone());
                    }
                    _ => {
                        parsed.insert(key.clone(), value.clone());
                    }
                }
            }
            Ok(serde_json::Value::Object(parsed))
        }
        other => Ok(other.clone()),
    }
}

/// Format standard data content block to format expected by Ollama.
///
/// Matches Python `_get_image_from_data_content_block()`.
fn get_image_from_data_content_block(block: &serde_json::Value) -> Result<String> {
    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

    if block_type == "image" {
        if block.get("source_type").and_then(|s| s.as_str()) == Some("base64")
            && let Some(data) = block.get("data").and_then(|d| d.as_str())
        {
            return Ok(data.to_string());
        }
        if let Some(base64_data) = block.get("base64").and_then(|b| b.as_str()) {
            return Ok(base64_data.to_string());
        }
        return Err(Error::Other(
            "Image data only supported through in-line base64 format.".to_string(),
        ));
    }

    Err(Error::Other(format!(
        "Blocks of type {} not supported.",
        block_type
    )))
}

/// Extract content text and base64 images from a MessageContent.
fn extract_content_and_images(content: &MessageContent) -> (String, Vec<String>) {
    match content {
        MessageContent::Text(s) => (s.clone(), vec![]),
        MessageContent::Parts(parts) => {
            let mut text_parts = Vec::new();
            let mut images = Vec::new();

            for part in parts {
                match part {
                    ContentPart::Text { text } => {
                        text_parts.push(text.as_str());
                    }
                    ContentPart::Image { source, .. } => {
                        if let Some(image_data) = extract_image_data(source) {
                            images.push(image_data);
                        }
                    }
                    ContentPart::Other(value) => {
                        let part_type = value.get("type").and_then(|t| t.as_str());
                        match part_type {
                            Some("text") => {
                                if let Some(text) = value.get("text").and_then(|t| t.as_str()) {
                                    text_parts.push(text);
                                }
                            }
                            Some("image_url") => {
                                if let Some(image_data) = extract_image_url_data(value) {
                                    images.push(image_data);
                                }
                            }
                            Some("image") => {
                                if let Ok(image_data) = get_image_from_data_content_block(value) {
                                    images.push(image_data);
                                }
                            }
                            Some("tool_use") => {}
                            _ => {}
                        }
                    }
                }
            }

            let combined_content = if text_parts.is_empty() {
                String::new()
            } else {
                let mut result = String::new();
                for part in &text_parts {
                    result.push('\n');
                    result.push_str(part);
                }
                result
            };
            (combined_content, images)
        }
    }
}

/// Extract base64 image data from an ImageSource.
fn extract_image_data(source: &ImageSource) -> Option<String> {
    match source {
        ImageSource::Base64 { data, .. } => Some(data.clone()),
        ImageSource::Url { url } => {
            if let Some((_prefix, data)) = url.split_once(',') {
                Some(data.to_string())
            } else {
                Some(url.clone())
            }
        }
        ImageSource::FileId { .. } => None,
    }
}

/// Extract image data from an image_url content part (Other variant).
fn extract_image_url_data(value: &serde_json::Value) -> Option<String> {
    let image_url = value.get("image_url")?;
    let url = if let Some(s) = image_url.as_str() {
        s.to_string()
    } else if let Some(obj) = image_url.as_object() {
        obj.get("url")?.as_str()?.to_string()
    } else {
        return None;
    };

    if let Some((_prefix, data)) = url.split_once(',') {
        Some(data.to_string())
    } else {
        Some(url)
    }
}

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

    /// Invoke the model with messages.
    pub async fn invoke(&self, messages: Vec<BaseMessage>) -> Result<AIMessage> {
        use crate::language_models::BaseChatModel;
        let tool_definitions = self.tool_definitions();
        self.model
            .generate_with_tools(messages, &tool_definitions, self.tool_choice.as_ref(), None)
            .await
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
        assert!(model.base_url.is_none());
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
    fn test_base_url_option() {
        let model = ChatOllama::new("llama3.1");
        assert!(model.base_url.is_none());

        let model = ChatOllama::new("llama3.1").base_url("http://custom:8080");
        assert_eq!(model.base_url, Some("http://custom:8080".to_string()));
    }

    #[test]
    fn test_llm_type() {
        use crate::language_models::BaseLanguageModel;
        let model = ChatOllama::new("llama3.1");
        assert_eq!(model.llm_type(), "chat-ollama");
    }

    #[test]
    fn test_reasoning_bool() {
        let model = ChatOllama::new("deepseek-r1").reasoning(true);
        assert!(model.is_reasoning_enabled());

        let model = ChatOllama::new("deepseek-r1").reasoning(false);
        assert!(!model.is_reasoning_enabled());
    }

    #[test]
    fn test_reasoning_string() {
        let model = ChatOllama::new("deepseek-r1").reasoning("high");
        assert!(model.is_reasoning_enabled());
        assert_eq!(
            model.reasoning,
            Some(serde_json::Value::String("high".to_string()))
        );
    }

    #[test]
    fn test_stop_both_set_errors() {
        let model = ChatOllama::new("llama3.1").stop(vec!["foo".to_string()]);
        let result = model.build_options(Some(vec!["bar".to_string()]));
        assert!(result.is_err());
    }

    #[test]
    fn test_keep_alive_string() {
        let model = ChatOllama::new("llama3.1").keep_alive("5m");
        assert!(matches!(model.keep_alive, Some(KeepAlive::Duration(ref s)) if s == "5m"));
    }

    #[test]
    fn test_keep_alive_seconds() {
        let model = ChatOllama::new("llama3.1").keep_alive_seconds(300);
        assert!(matches!(model.keep_alive, Some(KeepAlive::Seconds(300))));
    }

    #[test]
    fn test_parse_tool_call_arguments_string() {
        let args = serde_json::json!(r#"{"a": 1, "b": 2}"#);
        let parsed = parse_tool_call_arguments(Some(&args), "test").unwrap();
        assert_eq!(parsed, serde_json::json!({"a": 1, "b": 2}));
    }

    #[test]
    fn test_parse_tool_call_arguments_dict() {
        let args = serde_json::json!({"a": 1, "b": "hello"});
        let parsed = parse_tool_call_arguments(Some(&args), "test").unwrap();
        assert_eq!(parsed, serde_json::json!({"a": 1, "b": "hello"}));
    }

    #[test]
    fn test_parse_tool_call_arguments_filters_function_name() {
        let args = serde_json::json!({"a": 1, "functionName": "test"});
        let parsed = parse_tool_call_arguments(Some(&args), "test").unwrap();
        assert_eq!(parsed, serde_json::json!({"a": 1}));
    }

    #[test]
    fn test_parse_tool_call_arguments_nested_json_string() {
        let args = serde_json::json!({"a": r#"{"nested": true}"#});
        let parsed = parse_tool_call_arguments(Some(&args), "test").unwrap();
        assert_eq!(parsed, serde_json::json!({"a": {"nested": true}}));
    }

    #[test]
    fn test_parse_url_with_auth_credentials() {
        let (cleaned, headers) = parse_url_with_auth(Some("http://user:pass@localhost:11434"));
        assert_eq!(cleaned.as_deref(), Some("http://localhost:11434"));
        assert!(headers.is_some());
        assert!(headers.unwrap().contains_key("Authorization"));
    }

    #[test]
    fn test_parse_url_with_auth_no_credentials() {
        let (cleaned, headers) = parse_url_with_auth(Some("http://localhost:11434"));
        assert_eq!(cleaned.as_deref(), Some("http://localhost:11434"));
        assert!(headers.is_none());
    }

    #[test]
    fn test_extract_content_and_images_text() {
        let content = MessageContent::Text("hello".to_string());
        let (text, images) = extract_content_and_images(&content);
        assert_eq!(text, "hello");
        assert!(images.is_empty());
    }

    #[test]
    fn test_extract_content_and_images_multipart() {
        let content = MessageContent::Parts(vec![
            ContentPart::Text {
                text: "What's in this image?".to_string(),
            },
            ContentPart::Image {
                source: ImageSource::Base64 {
                    media_type: "image/jpeg".to_string(),
                    data: "abc123".to_string(),
                },
                detail: None,
            },
        ]);
        let (text, images) = extract_content_and_images(&content);
        assert_eq!(text, "\nWhat's in this image?");
        assert_eq!(images, vec!["abc123"]);
    }

    #[test]
    fn test_extract_image_data_uri() {
        let source = ImageSource::Url {
            url: "data:image/jpeg;base64,abc123".to_string(),
        };
        assert_eq!(extract_image_data(&source), Some("abc123".to_string()));
    }

    #[test]
    fn test_format_messages_with_images() {
        let model = ChatOllama::new("llama3.2-vision");
        let messages = vec![BaseMessage::Human(
            crate::messages::HumanMessage::builder()
                .content(MessageContent::Parts(vec![
                    ContentPart::Text {
                        text: "Describe this".to_string(),
                    },
                    ContentPart::Image {
                        source: ImageSource::Base64 {
                            media_type: "image/png".to_string(),
                            data: "base64data".to_string(),
                        },
                        detail: None,
                    },
                ]))
                .build(),
        )];
        let formatted = model.format_messages(&messages).unwrap();
        assert_eq!(formatted.len(), 1);
        assert_eq!(formatted[0]["role"], "user");
        assert_eq!(formatted[0]["content"], "\nDescribe this");
        assert_eq!(formatted[0]["images"][0], "base64data");
    }

    #[test]
    fn test_format_messages_rejects_unsupported_types() {
        let model = ChatOllama::new("llama3.1");
        let messages = vec![BaseMessage::Function(
            crate::messages::FunctionMessage::builder()
                .name("fn_name")
                .content("content")
                .build(),
        )];
        let result = model.format_messages(&messages);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_base_url_default() {
        let model = ChatOllama::new("llama3.1");
        let url = model.get_base_url();
        assert!(url.starts_with("http"));
    }

    #[test]
    fn test_get_base_url_explicit() {
        let model = ChatOllama::new("llama3.1").base_url("http://custom:8080");
        let url = model.get_base_url();
        assert_eq!(url, "http://custom:8080");
    }

    #[test]
    fn test_build_generation_info_includes_timing() {
        let response = OllamaResponse {
            model: Some("llama3.1".to_string()),
            message: None,
            done: Some(true),
            done_reason: Some("stop".to_string()),
            prompt_eval_count: Some(10),
            eval_count: Some(20),
            total_duration: Some(1000000),
            load_duration: Some(200000),
            prompt_eval_duration: Some(300000),
            eval_duration: Some(500000),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
        };
        let info = ChatOllama::build_generation_info(&response);
        assert_eq!(info.get("model"), Some(&serde_json::json!("llama3.1")));
        assert_eq!(info.get("model_name"), Some(&serde_json::json!("llama3.1")));
        assert_eq!(
            info.get("model_provider"),
            Some(&serde_json::json!("ollama"))
        );
        assert_eq!(info.get("done"), Some(&serde_json::json!(true)));
        assert_eq!(info.get("done_reason"), Some(&serde_json::json!("stop")));
        assert_eq!(
            info.get("total_duration"),
            Some(&serde_json::json!(1000000u64))
        );
        assert_eq!(
            info.get("load_duration"),
            Some(&serde_json::json!(200000u64))
        );
        assert_eq!(
            info.get("prompt_eval_duration"),
            Some(&serde_json::json!(300000u64))
        );
        assert_eq!(
            info.get("eval_duration"),
            Some(&serde_json::json!(500000u64))
        );
        assert_eq!(
            info.get("created_at"),
            Some(&serde_json::json!("2024-01-01T00:00:00Z"))
        );
    }
}
