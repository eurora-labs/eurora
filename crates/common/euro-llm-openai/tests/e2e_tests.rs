#[cfg(feature = "e2e-tests")]
mod e2e_tests {
    use dotenv::dotenv;
    use euro_llm_core::*;
    use euro_llm_openai::*;
    use std::env;

    // These tests require a real OpenAI API key and should only run when explicitly enabled
    // Run with: cargo test --features e2e-tests

    fn get_test_config() -> Option<OpenAIConfig> {
        dotenv().ok();
        env::var("OPENAI_API_KEY")
            .ok()
            .map(|key| OpenAIConfig::new(key, "gpt-3.5-turbo"))
    }

    #[tokio::test]
    async fn test_live_chat_completion() {
        let Some(config) = get_test_config() else {
            println!("Skipping live test - OPENAI_API_KEY not set");
            return;
        };

        let provider = OpenAIProvider::new(config).unwrap();
        let request = ChatRequest::builder()
            .message(Message::user("Say 'Hello, World!' and nothing else."))
            .temperature(0.0)
            .max_tokens(10)
            .build();

        let response = provider.chat(request).await;
        match response {
            Ok(resp) => {
                assert!(!resp.content().is_empty());
                println!("Live test response: {}", resp.content());
            }
            Err(e) => {
                panic!("Live test failed (this may be expected): {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_live_embeddings() {
        let Some(config) = get_test_config() else {
            println!("Skipping live test - OPENAI_API_KEY not set");
            return;
        };

        let provider = OpenAIProvider::new(config).unwrap();
        let texts = vec!["Hello, world!".to_string()];

        let response = provider.embed(&texts).await;
        match response {
            Ok(embeddings) => {
                assert_eq!(embeddings.len(), 1);
                assert!(!embeddings[0].embedding.is_empty());
                println!(
                    "Live embedding test passed, got {} dimensions",
                    embeddings[0].embedding.len()
                );
            }
            Err(e) => {
                panic!("Live embedding test failed (this may be expected): {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_live_streaming_chat() {
        let Some(config) = get_test_config() else {
            println!("Skipping live streaming test - OPENAI_API_KEY not set");
            return;
        };

        let provider = OpenAIProvider::new(config).unwrap();
        let request = ChatRequest::builder()
            .message(Message::user(
                "Count from 1 to 5, one number per response chunk.",
            ))
            .temperature(0.0)
            .max_tokens(50)
            .build();

        let stream_result = provider.chat_stream(request).await;
        match stream_result {
            Ok(mut stream) => {
                let mut content_parts = Vec::new();
                let mut chunk_count = 0;

                // Use tokio_stream::StreamExt for the stream operations
                use tokio_stream::StreamExt;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(content) => {
                            content_parts.push(content.clone());
                            chunk_count += 1;
                            println!("Streaming chunk {}: '{}'", chunk_count, content);

                            // Limit chunks to prevent infinite loops in tests
                            if chunk_count > 20 {
                                break;
                            }
                        }
                        Err(e) => {
                            panic!("Streaming error: {:?}", e);
                        }
                    }
                }

                assert!(chunk_count > 0, "Should receive at least one chunk");
                let full_content = content_parts.join("");
                assert!(
                    !full_content.is_empty(),
                    "Combined content should not be empty"
                );
                println!(
                    "Live streaming test passed with {} chunks, full content: '{}'",
                    chunk_count, full_content
                );
            }
            Err(e) => {
                panic!("Live streaming test failed (this may be expected): {:?}", e);
            }
        }
    }
}
