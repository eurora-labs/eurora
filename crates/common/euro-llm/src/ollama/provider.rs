//! Ollama provider implementation.

use super::{config::OllamaConfig, error::OllamaError, types::*};
use crate::core::{
    self, ChatProvider, ChatRequest, CompletionProvider, CompletionRequest, Embedding,
    EmbeddingProvider, ProviderResult, StreamingProvider,
};
use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client, RequestBuilder};
use serde_json::json;
use std::pin::Pin;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Ollama provider implementation.
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    config: OllamaConfig,
    client: Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider with the given configuration.
    pub fn new(config: OllamaConfig) -> Result<Self, OllamaError> {
        let mut headers = reqwest::header::HeaderMap::new();

        // Add content type header
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json"
                .parse()
                .map_err(|_| OllamaError::Config {
                    source: core::ConfigError::invalid_value("headers", "Invalid content type"),
                })?,
        );

        // Add user agent
        if let Some(ref user_agent) = config.http.user_agent {
            headers.insert(
                reqwest::header::USER_AGENT,
                user_agent.parse().map_err(|_| OllamaError::Config {
                    source: core::ConfigError::invalid_value(
                        "user_agent",
                        "Invalid user agent format",
                    ),
                })?,
            );
        }

        // Add custom headers
        for (key, value) in &config.http.headers {
            let header_name: reqwest::header::HeaderName =
                key.parse().map_err(|_| OllamaError::Config {
                    source: core::ConfigError::invalid_value("headers", "Invalid header name"),
                })?;
            let header_value: reqwest::header::HeaderValue =
                value.parse().map_err(|_| OllamaError::Config {
                    source: core::ConfigError::invalid_value("headers", "Invalid header value"),
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
            .map_err(|e| OllamaError::Network { source: e })?;

        Ok(Self { config, client })
    }

    /// Create a request builder with common settings.
    fn request_builder(&self, method: reqwest::Method, url: &str) -> RequestBuilder {
        self.client.request(method, url)
    }

    /// Handle HTTP response and convert to appropriate error.
    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T, OllamaError>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            response
                .json()
                .await
                .map_err(|e| OllamaError::Network { source: e })
        } else {
            let body = response.text().await.unwrap_or_default();
            Err(OllamaError::from_response(status.as_u16(), &body))
        }
    }

    /// Apply request parameters to options, handling both existing and new options.
    fn apply_parameters_to_options(
        parameters: &core::Parameters,
        existing_options: Option<serde_json::Value>,
    ) -> Option<serde_json::Value> {
        // Check if any parameters are set
        let has_parameters = parameters.temperature.is_some()
            || parameters.max_tokens.is_some()
            || parameters.top_p.is_some()
            || parameters.top_k.is_some()
            || !parameters.stop_sequences.is_empty();

        if !has_parameters {
            return existing_options;
        }

        let options = if let Some(mut opts) = existing_options {
            // Modify existing options
            if let Some(temp) = parameters.temperature {
                opts["temperature"] = json!(temp);
            }
            if let Some(max_tokens) = parameters.max_tokens {
                opts["num_predict"] = json!(max_tokens);
            }
            if let Some(top_p) = parameters.top_p {
                opts["top_p"] = json!(top_p);
            }
            if let Some(top_k) = parameters.top_k {
                opts["top_k"] = json!(top_k);
            }
            if !parameters.stop_sequences.is_empty() {
                opts["stop"] = json!(parameters.stop_sequences);
            }
            opts
        } else {
            // Create new options map
            let mut new_options = serde_json::Map::new();
            if let Some(temp) = parameters.temperature {
                new_options.insert("temperature".to_string(), json!(temp));
            }
            if let Some(max_tokens) = parameters.max_tokens {
                new_options.insert("num_predict".to_string(), json!(max_tokens));
            }
            if let Some(top_p) = parameters.top_p {
                new_options.insert("top_p".to_string(), json!(top_p));
            }
            if let Some(top_k) = parameters.top_k {
                new_options.insert("top_k".to_string(), json!(top_k));
            }
            if !parameters.stop_sequences.is_empty() {
                new_options.insert("stop".to_string(), json!(parameters.stop_sequences));
            }
            serde_json::Value::Object(new_options)
        };

        Some(options)
    }

    /// Convert core ChatRequest to Ollama format.
    fn convert_chat_request(&self, request: &ChatRequest) -> OllamaChatRequest {
        let mut ollama_request = OllamaChatRequest {
            model: self.config.model.clone(),
            messages: request.messages.iter().map(|m| m.into()).collect(),
            stream: Some(false),
            format: None,
            options: self.config.options.clone(),
            keep_alive: self.config.keep_alive.map(|ka| format!("{ka}s")),
        };

        // Apply parameters to options using helper function
        ollama_request.options =
            Self::apply_parameters_to_options(&request.parameters, ollama_request.options);

        ollama_request
    }

    /// Convert core CompletionRequest to Ollama format.
    fn convert_completion_request(&self, request: &CompletionRequest) -> OllamaCompletionRequest {
        let mut ollama_request = OllamaCompletionRequest {
            model: self.config.model.clone(),
            prompt: request.prompt.clone(),
            stream: Some(false),
            format: None,
            options: self.config.options.clone(),
            keep_alive: self.config.keep_alive.map(|ka| format!("{ka}s")),
            context: None,
        };

        // Apply parameters to options using helper function
        ollama_request.options =
            Self::apply_parameters_to_options(&request.parameters, ollama_request.options);

        ollama_request
    }
}

#[async_trait]
impl ChatProvider for OllamaProvider {
    type Config = OllamaConfig;
    type Response = OllamaChatResponse;
    type Error = OllamaError;

    async fn chat(&self, request: ChatRequest) -> ProviderResult<Self::Response, Self::Error> {
        let ollama_request = self.convert_chat_request(&request);

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.chat_url())
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| OllamaError::Network { source: e })?;

        self.handle_response(response).await
    }
}

#[async_trait]
impl CompletionProvider for OllamaProvider {
    type Config = OllamaConfig;
    type Response = OllamaCompletionResponse;
    type Error = OllamaError;

    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> ProviderResult<Self::Response, Self::Error> {
        let ollama_request = self.convert_completion_request(&request);

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.generate_url())
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| OllamaError::Network { source: e })?;

        self.handle_response(response).await
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaProvider {
    type Config = OllamaConfig;
    type Error = OllamaError;

    async fn embed(&self, texts: &[String]) -> ProviderResult<Vec<Embedding>, Self::Error> {
        let embedding_model = self
            .config
            .embedding_model
            .clone()
            .unwrap_or_else(|| "nomic-embed-text".to_string());

        let mut embeddings = Vec::new();

        // Ollama embeddings API processes one text at a time
        for (index, text) in texts.iter().enumerate() {
            let request = OllamaEmbeddingsRequest {
                model: embedding_model.clone(),
                prompt: text.clone(),
                options: self.config.options.clone(),
                keep_alive: self.config.keep_alive.map(|ka| format!("{ka}s")),
            };

            let response = self
                .request_builder(reqwest::Method::POST, &self.config.embeddings_url())
                .json(&request)
                .send()
                .await
                .map_err(|e| OllamaError::Network { source: e })?;

            let embeddings_response: OllamaEmbeddingsResponse =
                self.handle_response(response).await?;

            embeddings.push(Embedding {
                embedding: embeddings_response.embedding,
                index,
            });
        }

        Ok(embeddings)
    }
}

#[async_trait]
impl StreamingProvider for OllamaProvider {
    type StreamItem = String;
    type Stream = Pin<Box<dyn Stream<Item = Result<Self::StreamItem, Self::Error>> + Send>>;

    async fn chat_stream(&self, request: ChatRequest) -> ProviderResult<Self::Stream, Self::Error> {
        let mut ollama_request = self.convert_chat_request(&request);
        ollama_request.stream = Some(true);

        let response = self
            .request_builder(reqwest::Method::POST, &self.config.chat_url())
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| OllamaError::Network { source: e })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(OllamaError::from_response(status, &body));
        }

        // Create a tokio channel for streaming
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<String, OllamaError>>(100);

        // Spawn a task to process the streaming response
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            let mut byte_stream = response.bytes_stream();
            let mut buffer = Vec::new();

            while let Some(chunk_result) = byte_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.extend_from_slice(chunk.as_ref());

                        // Process complete lines (Ollama sends one JSON object per line)
                        let mut start = 0;
                        while let Some(pos) = buffer[start..].iter().position(|&b| b == b'\n') {
                            let line_end = start + pos;
                            let line = String::from_utf8_lossy(&buffer[start..line_end])
                                .trim()
                                .to_string();
                            start = line_end + 1;

                            if !line.is_empty() {
                                // Try to parse the JSON chunk
                                if let Ok(chunk) = serde_json::from_str::<OllamaStreamChunk>(&line)
                                {
                                    // Extract content from the chunk
                                    let content = if let Some(ref message) = chunk.message {
                                        message.content.as_str()
                                    } else {
                                        chunk.response.as_deref().unwrap_or_default()
                                    };

                                    if !content.is_empty()
                                        && tx_clone.send(Ok(content.to_string())).await.is_err()
                                    {
                                        // Receiver dropped
                                        return;
                                    }

                                    // Check if this is the final chunk
                                    if chunk.done {
                                        drop(tx_clone);
                                        return;
                                    }
                                }
                            }
                        }

                        // Keep remaining bytes in buffer
                        buffer.drain(0..start);
                    }
                    Err(e) => {
                        let _ = tx_clone.send(Err(OllamaError::Network { source: e })).await;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Message, Metadata, Parameters};

    fn create_test_config() -> OllamaConfig {
        OllamaConfig::new("llama2")
    }

    #[test]
    fn test_provider_creation() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_convert_chat_request() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config).unwrap();

        let request = ChatRequest {
            messages: vec![Message::user("Hello")],
            parameters: Parameters {
                temperature: Some(0.7),
                max_tokens: Some(100),
                ..Default::default()
            },
            metadata: Metadata::default(),
        };

        let ollama_request = provider.convert_chat_request(&request);
        assert_eq!(ollama_request.model, "llama2");
        assert_eq!(ollama_request.messages.len(), 1);
        assert_eq!(ollama_request.stream, Some(false));
    }

    #[test]
    fn test_convert_completion_request() {
        let config = create_test_config();
        let provider = OllamaProvider::new(config).unwrap();

        let request = CompletionRequest {
            prompt: "Complete this sentence".to_string(),
            parameters: Parameters {
                temperature: Some(0.8),
                max_tokens: Some(50),
                ..Default::default()
            },
            metadata: Metadata::default(),
        };

        let ollama_request = provider.convert_completion_request(&request);
        assert_eq!(ollama_request.model, "llama2");
        assert_eq!(ollama_request.prompt, "Complete this sentence");
        assert_eq!(ollama_request.stream, Some(false));
    }

    #[test]
    fn test_config_urls() {
        let config = create_test_config();
        assert_eq!(config.chat_url(), "http://localhost:11434/api/chat");
        assert_eq!(config.generate_url(), "http://localhost:11434/api/generate");
        assert_eq!(
            config.embeddings_url(),
            "http://localhost:11434/api/embeddings"
        );
    }

    #[test]
    fn test_apply_parameters_to_options_with_empty_parameters() {
        // Test that empty parameters return existing options unchanged
        let empty_params = Parameters::default();
        let existing_options = Some(json!({
            "temperature": 0.5,
            "existing_field": "value"
        }));

        let result =
            OllamaProvider::apply_parameters_to_options(&empty_params, existing_options.clone());
        assert_eq!(result, existing_options);

        // Test that empty parameters with no existing options return None
        let result = OllamaProvider::apply_parameters_to_options(&empty_params, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_apply_parameters_to_options_create_new_options() {
        // Test creating new options when none exist
        let params = Parameters {
            temperature: Some(0.8),
            max_tokens: Some(150),
            top_p: Some(0.9),
            top_k: Some(40),
            stop_sequences: vec!["STOP".to_string(), "END".to_string()],
            ..Default::default()
        };

        let result = OllamaProvider::apply_parameters_to_options(&params, None);
        assert!(result.is_some());

        let options = result.unwrap();
        assert!((options["temperature"].as_f64().unwrap() - 0.8).abs() < 1e-6);
        assert_eq!(options["num_predict"], json!(150));
        assert!((options["top_p"].as_f64().unwrap() - 0.9).abs() < 1e-6);
        assert_eq!(options["top_k"], json!(40));
        assert_eq!(options["stop"], json!(["STOP", "END"]));
    }

    #[test]
    fn test_apply_parameters_to_options_merge_with_existing() {
        // Test merging parameters into existing options
        let params = Parameters {
            temperature: Some(0.7),
            max_tokens: Some(200),
            top_p: Some(0.95),
            ..Default::default()
        };

        let existing_options = Some(json!({
            "temperature": 0.5,
            "existing_field": "preserved_value",
            "another_field": 42
        }));

        let result = OllamaProvider::apply_parameters_to_options(&params, existing_options);
        assert!(result.is_some());

        let options = result.unwrap();
        // Check that new parameters override existing ones
        assert!((options["temperature"].as_f64().unwrap() - 0.7).abs() < 1e-6);
        assert_eq!(options["num_predict"], json!(200));
        assert!((options["top_p"].as_f64().unwrap() - 0.95).abs() < 1e-6);

        // Check that existing fields are preserved
        assert_eq!(options["existing_field"], json!("preserved_value"));
        assert_eq!(options["another_field"], json!(42));
    }

    #[test]
    fn test_apply_parameters_to_options_partial_parameters() {
        // Test with only some parameters set
        let params = Parameters {
            temperature: Some(0.6),
            stop_sequences: vec!["HALT".to_string()],
            ..Default::default()
        };

        let result = OllamaProvider::apply_parameters_to_options(&params, None);
        assert!(result.is_some());

        let options = result.unwrap();
        assert!((options["temperature"].as_f64().unwrap() - 0.6).abs() < 1e-6);
        assert_eq!(options["stop"], json!(["HALT"]));

        // Check that unset parameters are not included
        assert!(!options.as_object().unwrap().contains_key("num_predict"));
        assert!(!options.as_object().unwrap().contains_key("top_p"));
        assert!(!options.as_object().unwrap().contains_key("top_k"));
    }

    #[test]
    fn test_apply_parameters_to_options_all_parameters() {
        // Test with all supported parameters set
        let params = Parameters {
            temperature: Some(1.2),
            max_tokens: Some(500),
            top_p: Some(0.85),
            top_k: Some(25),
            stop_sequences: vec!["STOP".to_string(), "END".to_string(), "FINISH".to_string()],
            frequency_penalty: Some(0.5), // This should be ignored as it's not supported by Ollama
            presence_penalty: Some(0.3),  // This should be ignored as it's not supported by Ollama
        };

        let result = OllamaProvider::apply_parameters_to_options(&params, None);
        assert!(result.is_some());

        let options = result.unwrap();
        assert!((options["temperature"].as_f64().unwrap() - 1.2).abs() < 1e-6);
        assert_eq!(options["num_predict"], json!(500));
        assert!((options["top_p"].as_f64().unwrap() - 0.85).abs() < 1e-6);
        assert_eq!(options["top_k"], json!(25));
        assert_eq!(options["stop"], json!(["STOP", "END", "FINISH"]));

        // Verify unsupported parameters are not included
        assert!(
            !options
                .as_object()
                .unwrap()
                .contains_key("frequency_penalty")
        );
        assert!(
            !options
                .as_object()
                .unwrap()
                .contains_key("presence_penalty")
        );
    }

    #[test]
    fn test_apply_parameters_to_options_overwrite_existing_array() {
        // Test that stop sequences completely replace existing ones
        let params = Parameters {
            stop_sequences: vec!["NEW_STOP".to_string()],
            ..Default::default()
        };

        let existing_options = Some(json!({
            "stop": ["OLD_STOP1", "OLD_STOP2"],
            "temperature": 0.5
        }));

        let result = OllamaProvider::apply_parameters_to_options(&params, existing_options);
        assert!(result.is_some());

        let options = result.unwrap();
        assert_eq!(options["stop"], json!(["NEW_STOP"]));
        assert_eq!(options["temperature"], json!(0.5)); // Preserved
    }

    #[test]
    fn test_apply_parameters_to_options_empty_stop_sequences() {
        // Test that empty stop sequences are not included
        let params = Parameters {
            temperature: Some(0.4),
            stop_sequences: vec![], // Empty vector
            ..Default::default()
        };

        let result = OllamaProvider::apply_parameters_to_options(&params, None);
        assert!(result.is_some());

        let options = result.unwrap();
        assert!((options["temperature"].as_f64().unwrap() - 0.4).abs() < 1e-6);
        assert!(!options.as_object().unwrap().contains_key("stop"));
    }

    #[test]
    fn test_apply_parameters_to_options_edge_values() {
        // Test with edge case values
        let params = Parameters {
            temperature: Some(0.0), // Minimum temperature
            max_tokens: Some(1),    // Minimum tokens
            top_p: Some(1.0),       // Maximum top_p
            top_k: Some(1),         // Minimum top_k
            ..Default::default()
        };

        let result = OllamaProvider::apply_parameters_to_options(&params, None);
        assert!(result.is_some());

        let options = result.unwrap();
        assert!((options["temperature"].as_f64().unwrap() - 0.0).abs() < 1e-6);
        assert_eq!(options["num_predict"], json!(1));
        assert!((options["top_p"].as_f64().unwrap() - 1.0).abs() < 1e-6);
        assert_eq!(options["top_k"], json!(1));
    }

    #[test]
    fn test_apply_parameters_to_options_preserve_non_conflicting_fields() {
        // Test that non-conflicting existing fields are preserved
        let params = Parameters {
            temperature: Some(0.9),
            ..Default::default()
        };

        let existing_options = Some(json!({
            "custom_field": "custom_value",
            "nested_object": {
                "inner_field": "inner_value"
            },
            "array_field": [1, 2, 3],
            "boolean_field": true,
            "number_field": 123.45
        }));

        let result = OllamaProvider::apply_parameters_to_options(&params, existing_options);
        assert!(result.is_some());

        let options = result.unwrap();
        assert!((options["temperature"].as_f64().unwrap() - 0.9).abs() < 1e-6);
        assert_eq!(options["custom_field"], json!("custom_value"));
        assert_eq!(
            options["nested_object"]["inner_field"],
            json!("inner_value")
        );
        assert_eq!(options["array_field"], json!([1, 2, 3]));
        assert_eq!(options["boolean_field"], json!(true));
        assert!((options["number_field"].as_f64().unwrap() - 123.45).abs() < 1e-6);
    }
}
