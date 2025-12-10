//! Streaming chat example using the Ollama provider.
//!
//! This example demonstrates how to use the euro-llm-ollama crate to send
//! a chat request and receive a streaming response, displaying tokens as they arrive.
//!
//! To run this example:
//! ```bash
//! # Make sure Ollama is running locally with a model installed
//! # For example: ollama pull llama2
//! cargo run --example ollama_chat_streaming
//! ```

#[cfg(feature = "ollama")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use euro_llm::ollama::{OllamaConfig, OllamaProvider};
    use euro_llm::{ChatRequest, StreamingProvider};
    use futures::StreamExt;
    use std::io::{self, Write};
    use tracing::{error, info};
    dotenv::dotenv().ok();
    // Initialize tracing for better error reporting
    tracing_subscriber::fmt::init();

    info!("🌊 Ollama Streaming Chat Example");
    info!("=================================");

    // Create Ollama configuration
    // Uses default localhost:11434 endpoint with llama2 model
    let config = OllamaConfig::builder()
        .model("llama3.2")
        .keep_alive(300) // Keep model loaded for 5 minutes
        .build();
    //
    info!("✅ Configuration created successfully");
    info!("📝 Model: {}", config.model);
    info!("🌐 Base URL: {}", config.base_url());

    // Create the Ollama provider
    let provider = OllamaProvider::new(config)
        .map_err(|e| format!("Failed to create Ollama provider: {e}"))?;

    info!("🔗 Provider created successfully");

    // Create a chat request that will generate a longer response using the improved API
    let request = ChatRequest::builder()
        .system_message("You are a creative storyteller. Write engaging and descriptive stories.")
        .user_message("Tell me a short story about a robot who discovers they can dream. Make it about 200 words.")
        .temperature(0.8)
        .max_tokens(300)
        .top_p(1.0)
        .request_id("example-ollama-streaming-001".to_string())
        .user_id("example-user".to_string())
        .build();

    info!("📤 Starting streaming chat request...");

    // Send the streaming chat request
    let mut stream = provider
        .chat_stream(request)
        .await
        .map_err(|e| format!("Streaming chat request failed: {e}"))?;

    info!("🦙 Assistant Response (streaming):");
    info!("─────────────────────────────────");

    let mut token_count = 0;
    let mut full_response = String::new();

    // Process the stream
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                // Print the chunk immediately (streaming effect)
                print!("{chunk}");
                io::stdout().flush().unwrap(); // Ensure immediate output

                // Accumulate the full response
                full_response.push_str(&chunk);
                token_count += 1;
            }
            Err(e) => {
                error!("\n❌ Error in stream: {e}");
                break;
            }
        }
    }

    println!("\n");
    info!("─────────────────────────────────");

    // Display statistics
    info!("📊 Streaming Statistics:");
    info!("   • Total chunks received: {}", token_count);
    info!("   • Total characters: {}", full_response.len());
    info!(
        "   • Total words (approx): {}",
        full_response.split_whitespace().count()
    );

    info!("✅ Streaming example completed successfully!");

    Ok(())
}

#[cfg(not(feature = "ollama"))]
fn main() {
    println!("Ollama provider is not enabled. Please enable the 'ollama' feature.");
}
