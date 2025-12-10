//! Streaming chat example using the Anthropic provider.
//!
//! This example demonstrates how to use the euro-llm-anthropic crate to send
//! a chat request and receive a streaming response, displaying tokens as they arrive.
//!
//! To run this example:
//! ```bash
//! export ANTHROPIC_API_KEY="your-api-key-here"
//! cargo run --example anthropic_chat_streaming
//! ```

#[cfg(feature = "anthropic")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use euro_llm::anthropic::{AnthropicConfig, AnthropicProvider};
    use euro_llm::core::{ChatRequest, StreamingProvider};
    use futures::StreamExt;
    use std::io::{self, Write};
    use tracing::{error, info};

    dotenv::dotenv().ok();
    // Initialize tracing for better error reporting
    tracing_subscriber::fmt::init();

    info!("🌊 Anthropic Streaming Chat Example");
    info!("===================================");

    // Load configuration from environment variables
    // Requires ANTHROPIC_API_KEY to be set
    let config = AnthropicConfig::from_env()
        .map_err(|e| format!("Failed to load config from environment: {e}"))?;

    info!("✅ Configuration loaded successfully");
    info!("📝 Model: {}", config.model);
    info!("🌐 Base URL: {}", config.base_url());

    // Create the Anthropic provider
    let provider = AnthropicProvider::new(config)
        .map_err(|e| format!("Failed to create Anthropic provider: {e}"))?;

    info!("🔗 Provider created successfully");

    // Create a chat request that will generate a longer response using the improved API
    let request = ChatRequest::builder()
        .system_message("You are a creative storyteller. Write engaging and descriptive stories.")
        .user_message("Tell me a short story about a robot who discovers they can dream. Make it about 200 words.")
        .temperature(0.8)
        .max_tokens(300)
        .top_p(1.0)
        .request_id("example-anthropic-streaming-001".to_string())
        .user_id("example-user".to_string())
        .build();

    info!("📤 Starting streaming chat request...");

    // Send the streaming chat request
    let mut stream = provider
        .chat_stream(request)
        .await
        .map_err(|e| format!("Streaming chat request failed: {e}"))?;

    info!("🤖 Claude Response (streaming):");
    info!("──────────────────────────────");

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
    info!("──────────────────────────────");

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

#[cfg(not(feature = "anthropic"))]
fn main() {
    println!("Anthropic provider is not enabled. Please enable the 'anthropic' feature.");
}
