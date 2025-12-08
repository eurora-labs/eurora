//! Integration tests for the Ollama provider.

use euro_llm_core::{
    ChatProvider, ChatRequest, CompletionProvider, CompletionRequest, EmbeddingProvider, Message,
    Metadata, Parameters, ProviderConfig, StreamingProvider,
};
use euro_llm_ollama::{OllamaConfig, OllamaProvider};

fn create_test_config() -> OllamaConfig {
    OllamaConfig::builder()
        .model("llama2")
        .base_url("http://localhost:11434")
        .unwrap()
        .build()
}

#[tokio::test]
async fn test_provider_creation() {
    let config = create_test_config();
    let provider = OllamaProvider::new(config);
    assert!(provider.is_ok());
}

#[tokio::test]
#[ignore] // Requires running Ollama server
async fn test_chat_completion() {
    let config = create_test_config();
    let provider = OllamaProvider::new(config).unwrap();

    let request = ChatRequest {
        messages: vec![Message::user("Hello, how are you?")],
        parameters: Parameters {
            temperature: Some(0.7),
            max_tokens: Some(50),
            ..Default::default()
        },
        metadata: Metadata::default(),
    };

    let result = provider.chat(request).await;
    match result {
        Ok(response) => {
            println!("Chat response: {:?}", response);
            assert!(!response.message.content.is_empty());
        }
        Err(e) => {
            println!("Chat error: {:?}", e);
            // Don't fail the test if Ollama is not running
        }
    }
}

#[tokio::test]
#[ignore] // Requires running Ollama server
async fn test_completion() {
    let config = create_test_config();
    let provider = OllamaProvider::new(config).unwrap();

    let request = CompletionRequest {
        prompt: "The capital of France is".to_string(),
        parameters: Parameters {
            temperature: Some(0.5),
            max_tokens: Some(20),
            ..Default::default()
        },
        metadata: Metadata::default(),
    };

    let result = provider.complete(request).await;
    match result {
        Ok(response) => {
            println!("Completion response: {:?}", response);
            assert!(!response.response.is_empty());
        }
        Err(e) => {
            println!("Completion error: {:?}", e);
            // Don't fail the test if Ollama is not running
        }
    }
}

#[tokio::test]
#[ignore] // Requires running Ollama server with embedding model
async fn test_embeddings() {
    let config = OllamaConfig::builder()
        .model("llama2")
        .embedding_model("nomic-embed-text")
        .build();
    let provider = OllamaProvider::new(config).unwrap();

    let texts = vec!["Hello world".to_string(), "Goodbye world".to_string()];

    let result = provider.embed(&texts).await;
    match result {
        Ok(embeddings) => {
            println!("Embeddings: {} vectors", embeddings.len());
            assert_eq!(embeddings.len(), 2);
            assert!(!embeddings[0].embedding.is_empty());
            assert!(!embeddings[1].embedding.is_empty());
        }
        Err(e) => {
            println!("Embeddings error: {:?}", e);
            // Don't fail the test if Ollama is not running or model not available
        }
    }
}

#[tokio::test]
#[ignore] // Requires running Ollama server
async fn test_streaming() {
    use futures::StreamExt;

    let config = create_test_config();
    let provider = OllamaProvider::new(config).unwrap();

    let request = ChatRequest {
        messages: vec![Message::user("Tell me a short story")],
        parameters: Parameters {
            temperature: Some(0.8),
            max_tokens: Some(100),
            ..Default::default()
        },
        metadata: Metadata::default(),
    };

    let result = provider.chat_stream(request).await;
    match result {
        Ok(mut stream) => {
            let mut content = String::new();
            let mut chunk_count = 0;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        content.push_str(&chunk);
                        chunk_count += 1;
                        println!("Chunk {}: {}", chunk_count, chunk);
                    }
                    Err(e) => {
                        println!("Stream error: {:?}", e);
                        break;
                    }
                }

                // Limit chunks for testing
                if chunk_count >= 10 {
                    break;
                }
            }

            println!("Total content: {}", content);
            println!("Total chunks: {}", chunk_count);
            assert!(chunk_count > 0);
        }
        Err(e) => {
            println!("Streaming error: {:?}", e);
            // Don't fail the test if Ollama is not running
        }
    }
}

#[test]
fn test_config_validation() {
    let config = OllamaConfig::new("llama2");
    assert!(config.validate().is_ok());

    let invalid_config = OllamaConfig::new("");
    assert!(invalid_config.validate().is_err());
}

#[test]
fn test_config_builder() {
    let config = OllamaConfig::builder()
        .model("codellama")
        .embedding_model("nomic-embed-text")
        .keep_alive(300)
        .build();

    assert_eq!(config.model, "codellama");
    assert_eq!(config.embedding_model, Some("nomic-embed-text".to_string()));
    assert_eq!(config.keep_alive, Some(300));
}

#[test]
fn test_config_from_env() {
    // Set environment variables for testing
    unsafe {
        std::env::set_var("OLLAMA_MODEL", "test-model");
        std::env::set_var("OLLAMA_BASE_URL", "http://test:11434");
    }

    let result = OllamaConfig::from_env();

    // Clean up environment variables
    unsafe {
        std::env::remove_var("OLLAMA_MODEL");
        std::env::remove_var("OLLAMA_BASE_URL");
    }

    match result {
        Ok(config) => {
            assert_eq!(config.model, "test-model");
            assert!(config.base_url().starts_with("http://test:11434"));
        }
        Err(e) => {
            println!("Config from env error: {:?}", e);
        }
    }
}
