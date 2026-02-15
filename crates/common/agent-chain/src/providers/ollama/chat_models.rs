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
use base64::Engine;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::io::StreamReader;

use crate::callbacks::{CallbackManagerForLLMRun, Callbacks};
use crate::chat_models::{
    BaseChatModel, ChatChunk, ChatModelConfig, LangSmithParams, ToolChoice, UsageMetadata,
};
use crate::error::{Error, Result};
use crate::language_models::{BaseLanguageModel, LanguageModelConfig, LanguageModelInput};
use crate::messages::{AIMessage, BaseMessage, ContentPart, ImageSource, MessageContent, ToolCall};
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
/// let messages = vec![HumanMessage::builder().content("Hello!").build().into()];
/// let response = model.generate(messages, None).await?;
/// ```
#[derive(Debug)]
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
    /// Supports `true`/`false` or string intensities like `"low"`, `"medium"`, `"high"`.
    reasoning: Option<serde_json::Value>,
    /// Additional client kwargs.
    client_kwargs: HashMap<String, serde_json::Value>,
    /// Chat model configuration.
    chat_model_config: ChatModelConfig,
    /// Language model configuration.
    language_model_config: LanguageModelConfig,
    /// Whether the model has been validated (for lazy validation).
    model_validated: std::sync::atomic::AtomicBool,
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
            client_kwargs: self.client_kwargs.clone(),
            chat_model_config: self.chat_model_config.clone(),
            language_model_config: self.language_model_config.clone(),
            model_validated: std::sync::atomic::AtomicBool::new(
                self.model_validated
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
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
            model_validated: std::sync::atomic::AtomicBool::new(false),
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
    ///
    /// Accepts `true`/`false` to enable/disable, or a string intensity like
    /// `"low"`, `"medium"`, `"high"` for supported models.
    pub fn reasoning(mut self, value: impl Into<serde_json::Value>) -> Self {
        self.reasoning = Some(value.into());
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
        let raw_url = if self.base_url != DEFAULT_API_BASE {
            self.base_url.clone()
        } else {
            env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_API_BASE.to_string())
        };

        // Strip userinfo from URL if present (auth is handled by build_client)
        strip_userinfo_from_url(&raw_url)
    }

    /// Build the HTTP client, optionally with basic auth from the URL.
    fn build_client(&self) -> reqwest::Client {
        let raw_url = if self.base_url != DEFAULT_API_BASE {
            self.base_url.clone()
        } else {
            env::var("OLLAMA_HOST").unwrap_or_else(|_| DEFAULT_API_BASE.to_string())
        };

        let mut builder = reqwest::Client::builder();

        if let Some((username, password)) = extract_userinfo(&raw_url) {
            let credentials = format!("{}:{}", username, password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
            let mut headers = reqwest::header::HeaderMap::new();
            if let Ok(value) = reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded))
            {
                headers.insert(reqwest::header::AUTHORIZATION, value);
            }
            builder = builder.default_headers(headers);
        }

        builder.build().unwrap_or_else(|_| reqwest::Client::new())
    }

    /// Validate that the model exists in Ollama.
    pub async fn validate_model(&self) -> Result<()> {
        let client = self.build_client();
        let base_url = self.get_base_url();
        let response = client
            .post(format!("{}/api/show", base_url))
            .json(&serde_json::json!({ "name": self.model }))
            .send()
            .await
            .map_err(Error::Http)?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::Other(format!(
                "Model '{}' not found in Ollama: {}",
                self.model, error_text
            )));
        }
        Ok(())
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
    fn format_messages(&self, messages: &[BaseMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .filter_map(|msg| match msg {
                BaseMessage::System(m) => Some(serde_json::json!({
                    "role": "system",
                    "content": m.content.as_text()
                })),
                BaseMessage::Human(m) => {
                    let (content, images) = extract_content_and_images(&m.content);
                    let mut message = serde_json::json!({
                        "role": "user",
                        "content": content
                    });
                    if !images.is_empty() {
                        message["images"] = serde_json::Value::Array(
                            images.into_iter().map(serde_json::Value::String).collect(),
                        );
                    }
                    Some(message)
                }
                BaseMessage::AI(m) => {
                    let mut message = serde_json::json!({
                        "role": "assistant",
                    });

                    if !m.content().is_empty() {
                        message["content"] = serde_json::json!(m.content());
                    }

                    if !m.tool_calls.is_empty() {
                        let tool_calls: Vec<serde_json::Value> = m
                            .tool_calls
                            .iter()
                            .map(lc_tool_call_to_openai_tool_call)
                            .collect();
                        message["tool_calls"] = serde_json::Value::Array(tool_calls);
                    }

                    Some(message)
                }
                BaseMessage::Tool(m) => Some(serde_json::json!({
                    "role": "tool",
                    "tool_call_id": m.tool_call_id,
                    "content": m.content
                })),
                BaseMessage::Remove(_) => None,
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

    /// Build the options object for the request.
    fn build_options(&self, stop: Option<Vec<String>>) -> Result<serde_json::Value> {
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
    fn build_request_payload(
        &self,
        messages: &[BaseMessage],
        stop: Option<Vec<String>>,
        tools: Option<&[serde_json::Value]>,
        stream: bool,
    ) -> Result<serde_json::Value> {
        let formatted_messages = self.format_messages(messages);
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
            payload["keep_alive"] = serde_json::json!(keep_alive);
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

    /// Build generation_info from an Ollama response.
    fn build_generation_info(response: &OllamaResponse) -> HashMap<String, serde_json::Value> {
        let mut info = HashMap::new();
        if let Some(model) = &response.model {
            info.insert("model".to_string(), serde_json::json!(model));
            info.insert("model_name".to_string(), serde_json::json!(model));
        }
        info.insert("model_provider".to_string(), serde_json::json!("ollama"));
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

    async fn _astream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&crate::callbacks::AsyncCallbackManagerForLLMRun>,
    ) -> Result<crate::language_models::ChatGenerationStream> {
        use crate::outputs::ChatGenerationChunk;

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

                        let message = AIMessage::builder()
                            .content(&chat_chunk.chunk.content)
                            .tool_calls(chat_chunk.chunk.tool_calls.clone())
                            .maybe_usage_metadata(chat_chunk.chunk.usage_metadata.clone())
                            .additional_kwargs(additional_kwargs)
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
            let error_text = response.text().await.unwrap_or_default();
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
}

impl ChatOllama {
    /// Internal generate implementation.
    async fn _generate_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
        _run_manager: Option<&CallbackManagerForLLMRun>,
    ) -> Result<ChatResult> {
        self.ensure_model_validated().await?;

        let client = self.build_client();
        let payload = self.build_request_payload(&messages, stop, None, false)?;
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

        let generation_info = Self::build_generation_info(&ollama_resp);
        let ai_message = self.parse_response_to_ai_message(&ollama_resp);
        let generation = ChatGeneration::with_info(ai_message.into(), generation_info);
        Ok(ChatResult::new(vec![generation]))
    }

    /// Internal stream implementation.
    ///
    /// Returns a stream of `OllamaStreamChunk` which carries both the `ChatChunk`
    /// and Ollama-specific metadata (reasoning content, generation_info).
    async fn stream_internal(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<OllamaStreamChunk>> + Send>>>
    {
        self.ensure_model_validated().await?;

        let client = self.build_client();
        let payload = self.build_request_payload(&messages, stop, None, true)?;
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

        let reasoning_enabled = self.is_reasoning_enabled();

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

                let tool_calls = get_tool_calls_from_response(&stream_resp);

                let reasoning_content = if reasoning_enabled {
                    stream_resp
                        .message
                        .as_ref()
                        .and_then(|m| m.thinking.clone())
                } else {
                    None
                };

                if is_done {
                    let usage = if let (Some(prompt_eval_count), Some(eval_count)) =
                        (stream_resp.prompt_eval_count, stream_resp.eval_count)
                    {
                        Some(UsageMetadata::new(prompt_eval_count as i64, eval_count as i64))
                    } else {
                        None
                    };
                    let finish_reason = stream_resp.done_reason.clone();

                    let generation_info = Self::build_generation_info(&stream_resp);

                    let mut chunk = ChatChunk::final_chunk(usage, finish_reason);
                    chunk.tool_calls = tool_calls;

                    yield OllamaStreamChunk {
                        chunk,
                        reasoning_content,
                        generation_info: Some(generation_info),
                    };
                } else {
                    let mut chunk = ChatChunk::new(content);
                    chunk.tool_calls = tool_calls;

                    yield OllamaStreamChunk {
                        chunk,
                        reasoning_content,
                        generation_info: None,
                    };
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

// ============================================================================
// Ollama-specific stream chunk (carries metadata beyond ChatChunk)
// ============================================================================

/// A streaming chunk from Ollama that carries both the standard `ChatChunk`
/// and Ollama-specific metadata like reasoning content and generation info.
struct OllamaStreamChunk {
    chunk: ChatChunk,
    reasoning_content: Option<String>,
    generation_info: Option<HashMap<String, serde_json::Value>>,
}

// ============================================================================
// Ollama API response structures
// ============================================================================

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

// ============================================================================
// Helper functions
// ============================================================================

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
                    tc.function.as_ref().map(|f| {
                        let args = parse_tool_call_arguments(f.arguments.as_ref(), &f.name);
                        ToolCall::builder().name(&f.name).args(args).build()
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
fn parse_tool_call_arguments(
    raw_args: Option<&serde_json::Value>,
    function_name: &str,
) -> serde_json::Value {
    let Some(args) = raw_args else {
        return serde_json::json!({});
    };

    match args {
        serde_json::Value::String(s) => {
            serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!({}))
        }
        serde_json::Value::Object(map) => {
            let mut parsed = serde_json::Map::new();
            for (key, value) in map {
                // Filter out metadata fields like 'functionName' that echo function name
                if key == "functionName"
                    && let serde_json::Value::String(v) = value
                    && v == function_name
                {
                    continue;
                }
                match value {
                    serde_json::Value::String(s) => {
                        // Try to parse string values that might be JSON
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
            serde_json::Value::Object(parsed)
        }
        other => other.clone(),
    }
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
                            Some("tool_use") => {
                                // Skip tool_use blocks (matching Python)
                            }
                            _ => {
                                // Skip unknown types
                            }
                        }
                    }
                }
            }

            let combined_content = text_parts.join("\n");
            (combined_content, images)
        }
    }
}

/// Extract base64 image data from an ImageSource.
fn extract_image_data(source: &ImageSource) -> Option<String> {
    match source {
        ImageSource::Base64 { data, .. } => Some(data.clone()),
        ImageSource::Url { url } => {
            // Support data:image/jpeg;base64,<data> format and plain base64 strings
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

    // Strip data URI prefix
    if let Some((_prefix, data)) = url.split_once(',') {
        Some(data.to_string())
    } else {
        Some(url)
    }
}

/// Extract userinfo (username:password) from a URL string.
fn extract_userinfo(url: &str) -> Option<(String, String)> {
    // Parse: http://username:password@host:port/path
    let after_scheme = url.split_once("://").map(|(_, rest)| rest)?;
    let before_host = after_scheme.split_once('@').map(|(userinfo, _)| userinfo)?;
    let (username, password) = before_host.split_once(':')?;
    if username.is_empty() {
        return None;
    }
    Some((username.to_string(), password.to_string()))
}

/// Strip userinfo from a URL string, returning the URL without credentials.
fn strip_userinfo_from_url(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://")
        && let Some((_userinfo, after_at)) = rest.split_once('@')
    {
        return format!("{}://{}", scheme, after_at);
    }
    url.to_string()
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
    fn test_parse_tool_call_arguments_string() {
        let args = serde_json::json!(r#"{"a": 1, "b": 2}"#);
        let parsed = parse_tool_call_arguments(Some(&args), "test");
        assert_eq!(parsed, serde_json::json!({"a": 1, "b": 2}));
    }

    #[test]
    fn test_parse_tool_call_arguments_dict() {
        let args = serde_json::json!({"a": 1, "b": "hello"});
        let parsed = parse_tool_call_arguments(Some(&args), "test");
        assert_eq!(parsed, serde_json::json!({"a": 1, "b": "hello"}));
    }

    #[test]
    fn test_parse_tool_call_arguments_filters_function_name() {
        let args = serde_json::json!({"a": 1, "functionName": "test"});
        let parsed = parse_tool_call_arguments(Some(&args), "test");
        assert_eq!(parsed, serde_json::json!({"a": 1}));
    }

    #[test]
    fn test_parse_tool_call_arguments_nested_json_string() {
        let args = serde_json::json!({"a": r#"{"nested": true}"#});
        let parsed = parse_tool_call_arguments(Some(&args), "test");
        assert_eq!(parsed, serde_json::json!({"a": {"nested": true}}));
    }

    #[test]
    fn test_extract_userinfo() {
        assert_eq!(
            extract_userinfo("http://user:pass@localhost:11434"),
            Some(("user".to_string(), "pass".to_string()))
        );
        assert_eq!(extract_userinfo("http://localhost:11434"), None);
    }

    #[test]
    fn test_strip_userinfo() {
        assert_eq!(
            strip_userinfo_from_url("http://user:pass@localhost:11434"),
            "http://localhost:11434"
        );
        assert_eq!(
            strip_userinfo_from_url("http://localhost:11434"),
            "http://localhost:11434"
        );
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
        assert_eq!(text, "What's in this image?");
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
        let formatted = model.format_messages(&messages);
        assert_eq!(formatted.len(), 1);
        assert_eq!(formatted[0]["role"], "user");
        assert_eq!(formatted[0]["content"], "Describe this");
        assert_eq!(formatted[0]["images"][0], "base64data");
    }
}
