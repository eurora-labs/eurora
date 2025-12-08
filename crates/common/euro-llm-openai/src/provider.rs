//! OpenAI provider implementation.

use crate::{config::OpenAIConfig, error::OpenAIError, types::*};
use async_trait::async_trait;
use euro_llm_core::{
    ChatProvider, ChatRequest, CompletionProvider, CompletionRequest, Embedding, EmbeddingProvider,
    ProviderResult, StreamingProvider, Tool, ToolProvider,
};
use futures::Stream;
use reqwest::{Client, RequestBuilder};
use serde_json::json;
use std::pin::Pin;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// OpenAI provider implementation.
#[derive(Debug, Clone)]
pub struct OpenAIProvider {
    config: OpenAIConfig,
    client: Client,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider with the given configuration.
    pub fn new(config: OpenAIConfig) -> Result<Self, OpenAIError> {
        let mut headers = reqwest::header::HeaderMap::new();

        // Add authorization header
        let auth_value = format!("Bearer {}", config.api_key.expose_secret());
        headers.insert(
            reqwest::header::AUTHORIZATION,
            auth_value.parse().map_err(|_| OpenAIError::Config {
                source: euro_llm_core::ConfigError::invalid_value(
                    "api_key",
                    "Invalid API key format",
                ),
            })?,
        );

        // Add organization header if provided
        if let Some(ref org) = config.organization {
            headers.insert(
                "OpenAI-Organization",
                org.parse().map_err(|_| OpenAIError::Config {
                    source: euro_llm_core::ConfigError::invalid_value(
                        "organization",
                        "Invalid organization format",
                    ),
                })?,
            );
        }

        // Add project header if provided
        if let Some(ref project) = config.project {
            headers.insert(
                "OpenAI-Project",
                project.parse().map_err(|_| OpenAIError::Config {
                    source: euro_llm_core::ConfigError::invalid_value(
                        "project",
                        "Invalid project format",
                    ),
                })?,
            );
        }

        // Add user agent
        if let Some(ref user_agent) = config.http.user_agent {
            headers.insert(
                reqwest::header::USER_AGENT,
                user_agent.parse().map_err(|_| OpenAIError::Config {
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
                key.parse().map_err(|_| OpenAIError::Config {
                    source: euro_llm_core::ConfigError::invalid_value(
                        "headers",
                        "Invalid header name",
                    ),
                })?;
            let header_value: reqwest::header::HeaderValue =
                value.parse().map_err(|_| OpenAIError::Config {
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
            .map_err(|e| OpenAIError::Network { source: e })?;

        Ok(Self { config, client })
    }

    /// Create a request builder with common settings.
    fn request_builder(&self, method: reqwest::Method, url: &str) -> RequestBuilder {
        self.client.request(method, url)
    }

    /// Handle HTTP response and convert to appropriate error.
    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T, OpenAIError>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| OpenAIError::Network { source: e })
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(OpenAIError::from_response(status.as_u16(), &body))
        }
    }

    /// Convert core ChatRequest to OpenAI format.
    fn convert_chat_request(&self, request: &ChatRequest) -> OpenAIChatRequest {
        OpenAIChatRequest {
            model: self.config.model.clone(),
            messages: request.messages.iter().map(|m| m.into()).collect(),
            temperature: request.parameters.temperature,
            max_tokens: request.parameters.max_tokens,
            top_p: request.parameters.top_p,
            frequency_penalty: request.parameters.frequency_penalty,
            presence_penalty: request.parameters.presence_penalty,
            stop: request.parameters.stop_sequences.clone(),
            stream: Some(false),
            tools: None, // Will be set by chat_with_tools
            tool_choice: None,
            user: request.metadata.user_id.clone(),
        }
    }

    /// Convert core CompletionRequest to OpenAI format.
    fn convert_completion_request(&self, request: &CompletionRequest) -> OpenAICompletionRequest {
        OpenAICompletionRequest {
            model: self.config.model.clone(),
            prompt: request.prompt.clone(),
            max_tokens: request.parameters.max_tokens,
            temperature: request.parameters.temperature,
            top_p: request.parameters.top_p,
            frequency_penalty: request.parameters.frequency_penalty,
            presence_penalty: request.parameters.presence_penalty,
            stop: request.parameters.stop_sequences.clone(),
            stream: Some(false),
            user: request.metadata.user_id.clone(),
        }
    }
}

#[async_trait]
impl ChatProvider for OpenAIProvider {
    type Config = OpenAIConfig;
    type Response = OpenAIChatResponse;
    type Error = OpenAIError;

    async fn chat(&self, request: ChatRequest) -> ProviderResult<Self::Response, Self::Error> {
        let openai_request = self.convert_chat_request(&request);

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.chat_url())
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| OpenAIError::Network { source: e })?;

        self.handle_response(response).await
    }
}

#[async_trait]
impl CompletionProvider for OpenAIProvider {
    type Config = OpenAIConfig;
    type Response = OpenAICompletionResponse;
    type Error = OpenAIError;

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> ProviderResult<Self::Response, Self::Error> {
        let openai_request = self.convert_completion_request(&request);

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.completions_url())
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| OpenAIError::Network { source: e })?;

        self.handle_response(response).await
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIProvider {
    type Config = OpenAIConfig;
    type Error = OpenAIError;

    async fn embed(&self, texts: &[String]) -> ProviderResult<Vec<Embedding>, Self::Error> {
        let request = OpenAIEmbeddingsRequest {
            model: self
                .config
                .embedding_model
                .clone()
                .unwrap_or_else(|| "text-embedding-ada-002".to_string()),
            input: if texts.len() == 1 {
                json!(texts[0])
            } else {
                json!(texts)
            },
            encoding_format: Some("float".to_string()),
            dimensions: None,
            user: None,
        };

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.embeddings_url())
            .json(&request)
            .send()
            .await
            .map_err(|e| OpenAIError::Network { source: e })?;

        let embeddings_response: OpenAIEmbeddingsResponse = self.handle_response(response).await?;

        let embeddings = embeddings_response
            .data
            .into_iter()
            .map(|e| Embedding {
                embedding: e.embedding,
                index: e.index,
            })
            .collect();

        Ok(embeddings)
    }
}

#[async_trait]
impl StreamingProvider for OpenAIProvider {
    type StreamItem = String;
    type Stream = Pin<Box<dyn Stream<Item = Result<Self::StreamItem, Self::Error>> + Send>>;

    async fn chat_stream(&self, request: ChatRequest) -> ProviderResult<Self::Stream, Self::Error> {
        let mut openai_request = self.convert_chat_request(&request);
        openai_request.stream = Some(true);

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.chat_url())
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| OpenAIError::Network { source: e })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(OpenAIError::from_response(status, &body));
        }

        // Create a tokio channel for streaming
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, OpenAIError>>(100);

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

                            // Process SSE format: "data: {json}" or "data: [DONE]"
                            if let Some(data) = line.strip_prefix("data: ") {
                                if data == "[DONE]" {
                                    // End of stream
                                    drop(tx_clone);
                                    return;
                                }

                                // Try to parse the JSON chunk
                                if let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(data)
                                    && let Some(choice) = chunk.choices.first()
                                    && let Some(content) = &choice.delta.content
                                    && !content.is_empty()
                                    && tx_clone.send(Ok(content.clone())).await.is_err()
                                {
                                    // Receiver dropped
                                    return;
                                }
                            }
                        }

                        // Keep remaining bytes in buffer
                        buffer.drain(0..start);
                    }
                    Err(e) => {
                        let _ = tx_clone.send(Err(OpenAIError::Network { source: e })).await;
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
impl ToolProvider for OpenAIProvider {
    async fn chat_with_tools(
        &self,
        request: ChatRequest,
        tools: &[Tool],
    ) -> ProviderResult<Self::Response, Self::Error> {
        let mut openai_request = self.convert_chat_request(&request);

        if !tools.is_empty() {
            openai_request.tools = Some(tools.iter().map(|t| t.into()).collect());
            openai_request.tool_choice = Some(json!("auto"));
        }

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.chat_url())
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| OpenAIError::Network { source: e })?;

        self.handle_response(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use euro_llm_core::{Message, Metadata, Parameters};

    fn create_test_config() -> OpenAIConfig {
        OpenAIConfig::new("sk-test123456789", "gpt-3.5-turbo")
    }

    #[test]
    fn test_provider_creation() {
        let config = create_test_config();
        let provider = OpenAIProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_convert_chat_request() {
        let config = create_test_config();
        let provider = OpenAIProvider::new(config).unwrap();

        let request = ChatRequest {
            messages: vec![Message::user("Hello")],
            parameters: Parameters {
                temperature: Some(0.7),
                max_tokens: Some(100),
                ..Default::default()
            },
            metadata: Metadata::default(),
        };

        let openai_request = provider.convert_chat_request(&request);
        assert_eq!(openai_request.model, "gpt-3.5-turbo");
        assert_eq!(openai_request.temperature, Some(0.7));
        assert_eq!(openai_request.max_tokens, Some(100));
        assert_eq!(openai_request.messages.len(), 1);
    }
}
