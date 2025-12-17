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
//! use agent_chain::providers::ChatOpenAI;
//! use agent_chain::providers::openai::BuiltinTool;
//!
//! let model = ChatOpenAI::new("gpt-4o")
//!     .with_responses_api(true)
//!     .with_builtin_tools(vec![BuiltinTool::WebSearch]);
//!
//! let messages = vec![HumanMessage::new("What is the latest news?").into()];
//! let response = model.generate(messages, None).await?;
//! ```

use std::collections::HashMap;
use std::env;
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::chat_models::{
    ChatChunk, ChatModel, ChatResult, ChatResultMetadata, ChatStream, LangSmithParams, ToolChoice,
    UsageMetadata,
};
use crate::error::{Error, Result};
use crate::messages::{AIMessage, BaseMessage, ToolCall};
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
/// use agent_chain::providers::ChatOpenAI;
///
/// let model = ChatOpenAI::new("gpt-4o")
///     .temperature(0.7)
///     .max_tokens(1024);
///
/// let messages = vec![HumanMessage::new("Hello!").into()];
/// let response = model.generate(messages, None).await?;
/// ```
///
/// # Using Built-in Tools (Responses API)
///
/// ```ignore
/// use agent_chain::providers::ChatOpenAI;
/// use agent_chain::providers::openai::BuiltinTool;
///
/// let model = ChatOpenAI::new("gpt-4o")
///     .with_responses_api(true)
///     .with_builtin_tools(vec![BuiltinTool::WebSearch]);
///
/// let messages = vec![HumanMessage::new("What happened today?").into()];
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
    /// Whether to use the Responses API instead of Chat Completions API.
    use_responses_api: bool,
    /// Built-in tools to use with the Responses API.
    builtin_tools: Vec<BuiltinTool>,
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
        Self {
            model: model.into(),
            temperature: None,
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
            use_responses_api: false,
            builtin_tools: Vec::new(),
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

    /// Enable or disable the Responses API.
    ///
    /// The Responses API is required for built-in tools like web search.
    pub fn with_responses_api(mut self, enabled: bool) -> Self {
        self.use_responses_api = enabled;
        self
    }

    /// Set built-in tools to use with the Responses API.
    ///
    /// This automatically enables the Responses API.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use agent_chain::providers::ChatOpenAI;
    /// use agent_chain::providers::openai::BuiltinTool;
    ///
    /// let model = ChatOpenAI::new("gpt-4o")
    ///     .with_builtin_tools(vec![BuiltinTool::WebSearch]);
    /// ```
    pub fn with_builtin_tools(mut self, tools: Vec<BuiltinTool>) -> Self {
        self.builtin_tools = tools;
        if !self.builtin_tools.is_empty() {
            self.use_responses_api = true;
        }
        self
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
    fn format_messages(&self, messages: &[BaseMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|msg| match msg {
                BaseMessage::System(m) => {
                    serde_json::json!({
                        "role": "system",
                        "content": m.content()
                    })
                }
                BaseMessage::Human(m) => {
                    serde_json::json!({
                        "role": "user",
                        "content": m.content()
                    })
                }
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
                                    "id": tc.id(),
                                    "type": "function",
                                    "function": {
                                        "name": tc.name(),
                                        "arguments": tc.args().to_string()
                                    }
                                })
                            })
                            .collect();
                        message["tool_calls"] = serde_json::Value::Array(tool_calls);
                    }

                    message
                }
                BaseMessage::Tool(m) => {
                    serde_json::json!({
                        "role": "tool",
                        "tool_call_id": m.tool_call_id(),
                        "content": m.content()
                    })
                }
            })
            .collect()
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

        // Add any additional model kwargs
        if let serde_json::Value::Object(ref mut obj) = payload {
            for (k, v) in &self.model_kwargs {
                obj.insert(k.clone(), v.clone());
            }
        }

        payload
    }

    /// Build the request payload for the Responses API.
    fn build_responses_api_payload(
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
        if let serde_json::Value::Object(ref mut obj) = payload {
            for (k, v) in &self.model_kwargs {
                obj.insert(k.clone(), v.clone());
            }
        }

        payload
    }

    /// Format messages for the Responses API.
    fn format_messages_for_responses_api(
        &self,
        messages: &[BaseMessage],
    ) -> Vec<serde_json::Value> {
        let mut input = Vec::new();

        for msg in messages {
            match msg {
                BaseMessage::System(m) => {
                    input.push(serde_json::json!({
                        "role": "system",
                        "content": m.content()
                    }));
                }
                BaseMessage::Human(m) => {
                    input.push(serde_json::json!({
                        "role": "user",
                        "content": m.content()
                    }));
                }
                BaseMessage::AI(m) => {
                    // Add message content
                    if !m.content().is_empty() || m.tool_calls().is_empty() {
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
                    for tc in m.tool_calls() {
                        input.push(serde_json::json!({
                            "type": "function_call",
                            "name": tc.name(),
                            "arguments": tc.args().to_string(),
                            "call_id": tc.id()
                        }));
                    }
                }
                BaseMessage::Tool(m) => {
                    input.push(serde_json::json!({
                        "type": "function_call_output",
                        "call_id": m.tool_call_id(),
                        "output": m.content()
                    }));
                }
            }
        }

        input
    }

    /// Parse the API response into a ChatResult.
    fn parse_response(&self, response: OpenAIResponse) -> ChatResult {
        // Extract finish_reason before consuming choices
        let finish_reason = response
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
                        ToolCall::with_id(tc.id, tc.function.name, args)
                    })
                    .collect();
                (content, tool_calls)
            }
            None => (String::new(), Vec::new()),
        };

        let message = if tool_calls.is_empty() {
            AIMessage::new(content)
        } else {
            AIMessage::with_tool_calls(content, tool_calls)
        };

        let usage = response
            .usage
            .map(|u| UsageMetadata::new(u.prompt_tokens, u.completion_tokens));

        ChatResult {
            message,
            metadata: ChatResultMetadata {
                model: Some(response.model),
                stop_reason: finish_reason,
                usage,
            },
        }
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
                    tool_calls.push(ToolCall::with_id(call_id.clone(), name.clone(), args));
                }
                ResponsesOutput::WebSearchCall { .. } => {
                    // Web search is handled internally by OpenAI
                    // Results are included in the text content with annotations
                }
                ResponsesOutput::Other(_) => {}
            }
        }

        let message = if tool_calls.is_empty() {
            let mut msg = AIMessage::new(text_content);
            if !annotations.is_empty() {
                // Store annotations in additional_kwargs for access
                msg = msg.with_annotations(annotations);
            }
            msg
        } else {
            AIMessage::with_tool_calls(text_content, tool_calls)
        };

        let usage = response
            .usage
            .as_ref()
            .map(|u| UsageMetadata::new(u.input_tokens, u.output_tokens));

        ChatResult {
            message,
            metadata: ChatResultMetadata {
                model: Some(response.model.clone()),
                stop_reason: response.status.clone(),
                usage,
            },
        }
    }

    /// Generate using the Responses API.
    async fn generate_responses_api(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatResult> {
        let api_key = self.get_api_key()?;
        let client = self.build_client();
        let payload = self.build_responses_api_payload(&messages, stop, None);

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
impl ChatModel for ChatOpenAI {
    fn llm_type(&self) -> &str {
        "openai-chat"
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    async fn generate(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatResult> {
        // Use Responses API if enabled or if using built-in tools
        if self.use_responses_api || !self.builtin_tools.is_empty() {
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

    async fn generate_with_tools(
        &self,
        messages: Vec<BaseMessage>,
        tools: &[ToolDefinition],
        tool_choice: Option<&ToolChoice>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatResult> {
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
        if self.use_responses_api || !self.builtin_tools.is_empty() {
            let api_key = self.get_api_key()?;
            let client = self.build_client();
            let payload = self.build_responses_api_payload(&messages, stop, Some(&openai_tools));

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
            match choice {
                ToolChoice::Auto => {
                    payload["tool_choice"] = serde_json::json!("auto");
                }
                ToolChoice::Any => {
                    payload["tool_choice"] = serde_json::json!("required");
                }
                ToolChoice::Tool { name } => {
                    payload["tool_choice"] = serde_json::json!({
                        "type": "function",
                        "function": {"name": name}
                    });
                }
                ToolChoice::None => {
                    payload["tool_choice"] = serde_json::json!("none");
                }
            }
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

    async fn stream(
        &self,
        messages: Vec<BaseMessage>,
        stop: Option<Vec<String>>,
    ) -> Result<ChatStream> {
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
        let model = self.model.clone();
        let stream = async_stream::stream! {
            let mut bytes_stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut usage: Option<UsageMetadata> = None;
            let mut finish_reason: Option<String> = None;

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
                                        yield Ok(ChatChunk {
                                            content: String::new(),
                                            is_final: true,
                                            metadata: Some(ChatResultMetadata {
                                                model: Some(model.clone()),
                                                stop_reason: finish_reason.clone(),
                                                usage: usage.clone(),
                                            }),
                                        });
                                        continue;
                                    }

                                    if let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(data) {
                                        if let Some(choice) = chunk.choices.first() {
                                            if let Some(ref content) = choice.delta.content {
                                                yield Ok(ChatChunk {
                                                    content: content.clone(),
                                                    is_final: false,
                                                    metadata: None,
                                                });
                                            }
                                            if let Some(ref reason) = choice.finish_reason {
                                                finish_reason = Some(reason.clone());
                                            }
                                        }
                                        if let Some(ref u) = chunk.usage {
                                            usage = Some(UsageMetadata::new(
                                                u.prompt_tokens,
                                                u.completion_tokens,
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

    fn get_ls_params(&self, stop: Option<&[String]>) -> LangSmithParams {
        LangSmithParams {
            ls_provider: Some("openai".to_string()),
            ls_model_name: Some(self.model.clone()),
            ls_model_type: Some("chat".to_string()),
            ls_temperature: self.temperature,
            ls_max_tokens: self.max_tokens,
            ls_stop: stop.map(|s| s.to_vec()),
        }
    }

    fn identifying_params(&self) -> serde_json::Value {
        serde_json::json!({
            "_type": self.llm_type(),
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": self.temperature,
            "top_p": self.top_p,
            "frequency_penalty": self.frequency_penalty,
            "presence_penalty": self.presence_penalty,
        })
    }
}

/// OpenAI API response structure.
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
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
}

// ============================================================================
// Responses API types
// ============================================================================

/// OpenAI Responses API response structure.
#[derive(Debug, Deserialize)]
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
        let model = ChatOpenAI::new("gpt-4o");
        assert_eq!(model.llm_type(), "openai-chat");
    }
}
