//! Anthropic provider implementation.

use crate::{config::AnthropicConfig, error::AnthropicError, types::*};
use async_trait::async_trait;
use euro_llm_core::{
    ChatProvider, ChatRequest, ProviderResult, StreamingProvider, Tool, ToolProvider,
};
use futures::Stream;
use reqwest::{Client, RequestBuilder};
use std::pin::Pin;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Anthropic provider implementation.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    config: AnthropicConfig,
    client: Client,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with the given configuration.
    pub fn new(config: AnthropicConfig) -> Result<Self, AnthropicError> {
        let mut headers = reqwest::header::HeaderMap::new();

        // Add authorization header
        let auth_value = config.api_key.expose_secret();
        headers.insert(
            "x-api-key",
            auth_value.parse().map_err(|_| AnthropicError::Config {
                source: euro_llm_core::ConfigError::invalid_value(
                    "api_key",
                    "Invalid API key format",
                ),
            })?,
        );

        // Add anthropic version header
        headers.insert(
            "anthropic-version",
            config.version.parse().map_err(|_| AnthropicError::Config {
                source: euro_llm_core::ConfigError::invalid_value(
                    "version",
                    "Invalid version format",
                ),
            })?,
        );

        // Add content type
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().unwrap(),
        );

        // Add user agent
        if let Some(ref user_agent) = config.http.user_agent {
            headers.insert(
                reqwest::header::USER_AGENT,
                user_agent.parse().map_err(|_| AnthropicError::Config {
                    source: euro_llm_core::ConfigError::invalid_value(
                        "user_agent",
                        "Invalid user agent format",
                    ),
                })?,
            );
        }

        // Add custom headers
        for (key, value) in &config.http.headers {
            let header_name: reqwest::header::HeaderName =
                key.parse().map_err(|_| AnthropicError::Config {
                    source: euro_llm_core::ConfigError::invalid_value(
                        "headers",
                        "Invalid header name",
                    ),
                })?;
            let header_value: reqwest::header::HeaderValue =
                value.parse().map_err(|_| AnthropicError::Config {
                    source: euro_llm_core::ConfigError::invalid_value(
                        "headers",
                        "Invalid header value",
                    ),
                })?;
            headers.insert(header_name, header_value);
        }

        let mut client_builder = Client::builder()
            .timeout(config.http.timeout)
            .default_headers(headers);

        // Configure compression
        if !config.http.compression {
            client_builder = client_builder.no_gzip();
        }

        // Configure connection pool
        client_builder = client_builder
            .pool_max_idle_per_host(config.http.pool.max_idle_connections)
            .pool_idle_timeout(config.http.pool.idle_timeout)
            .connect_timeout(config.http.pool.connect_timeout);

        let client = client_builder
            .build()
            .map_err(|e| AnthropicError::Network { source: e })?;

        Ok(Self { config, client })
    }

    /// Create a request builder with common settings.
    fn request_builder(&self, method: reqwest::Method, url: &str) -> RequestBuilder {
        self.client.request(method, url)
    }

    /// Handle HTTP response and convert to appropriate error.
    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T, AnthropicError>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| AnthropicError::Network { source: e })
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(AnthropicError::from_response(status.as_u16(), &body))
        }
    }

    /// Convert core ChatRequest to Anthropic format.
    fn convert_chat_request(&self, request: &ChatRequest) -> AnthropicMessagesRequest {
        let mut system_message = None;
        let mut messages = Vec::new();

        // Separate system messages from other messages
        for message in &request.messages {
            if message.role == euro_llm_core::Role::System {
                if let euro_llm_core::MessageContent::Text(text) = &message.content {
                    system_message = Some(text.clone());
                }
            } else {
                messages.push(message.into());
            }
        }

        AnthropicMessagesRequest {
            model: self.config.model.clone(),
            max_tokens: request.parameters.max_tokens.unwrap_or(4096), // Anthropic requires max_tokens
            messages,
            system: system_message,
            temperature: request.parameters.temperature,
            top_p: request.parameters.top_p,
            top_k: None, // Anthropic-specific parameter, not in core
            stop_sequences: request.parameters.stop_sequences.clone(),
            stream: Some(false),
            tools: None, // Will be set by chat_with_tools
            tool_choice: None,
        }
    }
}

#[async_trait]
impl ChatProvider for AnthropicProvider {
    type Config = AnthropicConfig;
    type Response = AnthropicMessagesResponse;
    type Error = AnthropicError;

    async fn chat(&self, request: ChatRequest) -> ProviderResult<Self::Response, Self::Error> {
        let anthropic_request = self.convert_chat_request(&request);

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.messages_url())
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| AnthropicError::Network { source: e })?;

        self.handle_response(response).await
    }
}

#[async_trait]
impl StreamingProvider for AnthropicProvider {
    type StreamItem = String;
    type Stream = Pin<Box<dyn Stream<Item = Result<Self::StreamItem, Self::Error>> + Send>>;

    async fn chat_stream(&self, request: ChatRequest) -> ProviderResult<Self::Stream, Self::Error> {
        let mut anthropic_request = self.convert_chat_request(&request);
        anthropic_request.stream = Some(true);

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.messages_url())
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| AnthropicError::Network { source: e })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(AnthropicError::from_response(status, &body));
        }

        // Create a tokio channel for streaming
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, AnthropicError>>(100);

        // Spawn a task to process the SSE stream
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let mut byte_stream = response.bytes_stream();
            let mut buffer = Vec::new();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.extend_from_slice(chunk.as_ref());

                        // Process complete lines
                        let mut start = 0;
                        while let Some(pos) = buffer[start..].iter().position(|&b| b == b'\n') {
                            let line_end = start + pos;
                            let line = String::from_utf8_lossy(&buffer[start..line_end])
                                .trim()
                                .to_string();
                            start = line_end + 1;

                            // Process SSE format: "data: {json}" or "event: message_stop"
                            if let Some(data) = line.strip_prefix("data: ") {
                                // Try to parse the JSON chunk
                                if let Ok(chunk) =
                                    serde_json::from_str::<AnthropicStreamChunk>(data)
                                {
                                    match chunk {
                                        AnthropicStreamChunk::ContentBlockDelta {
                                            delta, ..
                                        } => {
                                            match delta {
                                                AnthropicContentDelta::TextDelta { text } => {
                                                    if !text.is_empty()
                                                        && tx_clone.send(Ok(text)).await.is_err()
                                                    {
                                                        // Receiver dropped
                                                        return;
                                                    }
                                                }
                                                AnthropicContentDelta::InputJsonDelta {
                                                    ..
                                                } => {
                                                    // TODO: Handle input json delta
                                                }
                                            }
                                        }
                                        AnthropicStreamChunk::MessageStop => {
                                            // End of stream
                                            drop(tx_clone);
                                            return;
                                        }
                                        AnthropicStreamChunk::Error { error } => {
                                            let _ = tx_clone
                                                .send(Err(AnthropicError::Other {
                                                    message: error.message,
                                                }))
                                                .await;
                                            return;
                                        }
                                        _ => {} // Handle other chunk types if needed
                                    }
                                }
                            } else if line.starts_with("event: message_stop") {
                                // Alternative way to detect end of stream
                                drop(tx_clone);
                                return;
                            }
                        }

                        // Keep remaining bytes in buffer
                        buffer.drain(0..start);
                    }
                    Err(e) => {
                        let _ = tx_clone
                            .send(Err(AnthropicError::Network { source: e }))
                            .await;
                        return;
                    }
                }
            }

            // Close the channel when done
            drop(tx_clone);
        });

        // Convert the receiver to a stream
        let content_stream = ReceiverStream::new(rx);

        Ok(Box::pin(content_stream))
    }
}

#[async_trait]
impl ToolProvider for AnthropicProvider {
    async fn chat_with_tools(
        &self,
        request: ChatRequest,
        tools: &[Tool],
    ) -> ProviderResult<Self::Response, Self::Error> {
        let mut anthropic_request = self.convert_chat_request(&request);

        if !tools.is_empty() {
            anthropic_request.tools = Some(tools.iter().map(|t| t.into()).collect());
            anthropic_request.tool_choice = Some(AnthropicToolChoice::Auto);
        }

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.messages_url())
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| AnthropicError::Network { source: e })?;

        self.handle_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use euro_llm_core::{Message, Metadata, Parameters};

    fn create_test_config() -> AnthropicConfig {
        AnthropicConfig::new("sk-ant-test123456789", "claude-3-5-sonnet-20241022")
    }

    #[test]
    fn test_provider_creation() {
        let config = create_test_config();
        let provider = AnthropicProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_convert_chat_request() {
        let config = create_test_config();
        let provider = AnthropicProvider::new(config).unwrap();

        let request = ChatRequest {
            messages: vec![
                Message::system("You are a helpful assistant"),
                Message::user("Hello"),
            ],
            parameters: Parameters {
                temperature: Some(0.7),
                max_tokens: Some(100),
                ..Default::default()
            },
            metadata: Metadata::default(),
        };

        let anthropic_request = provider.convert_chat_request(&request);
        assert_eq!(anthropic_request.model, "claude-3-5-sonnet-20241022");
        assert_eq!(anthropic_request.temperature, Some(0.7));
        assert_eq!(anthropic_request.max_tokens, 100);
        assert_eq!(anthropic_request.messages.len(), 1); // System message separated
        assert_eq!(
            anthropic_request.system,
            Some("You are a helpful assistant".to_string())
        );
    }
}
