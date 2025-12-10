//! Streaming chat example using the OpenAI provider.
//!
//! This example demonstrates how to use the euro-llm-openai crate to send
//! a chat request and receive a streaming response, displaying tokens as they arrive.
//!
//! To run this example:
//! ```bash
//! export OPENAI_API_KEY="your-api-key-here"
//! cargo run --example openai_chat_streaming
//! ```

#[cfg(feature = "openai")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use euro_llm::core::{ChatRequest, StreamingProvider};
    use euro_llm::openai::{OpenAIConfig, OpenAIProvider};
    use futures::StreamExt;
    use std::io::{self, Write};
    use tracing::{error, info};
    dotenv::dotenv().ok();
    // Initialize tracing for better error reporting
    tracing_subscriber::fmt::init();

    info!("🌊 OpenAI Streaming Chat Example");
    info!("=================================");

    // Load configuration from environment variables
    // Requires OPENAI_API_KEY to be set
    let config = OpenAIConfig::from_env()
        .map_err(|e| format!("Failed to load config from environment: {e}"))?;

    info!("✅ Configuration loaded successfully");
    info!("📝 Model: {}", config.model);

    // Create the OpenAI provider
    let provider = OpenAIProvider::new(config)
        .map_err(|e| format!("Failed to create OpenAI provider: {e}"))?;

    info!("🔗 Provider created successfully");

    // Create a chat request that will generate a longer response using the improved API
    let request = ChatRequest::builder()
        .system_message("You are a creative storyteller. Write engaging and descriptive stories.")
        .user_message("Tell me a short story about a robot who discovers they can dream. Make it about 200 words.")
        .temperature(0.8)
        .max_tokens(300)
        .top_p(1.0)
        .request_id("example-streaming-001".to_string())
        .user_id("example-user".to_string())
        .build();

    info!("📤 Starting streaming chat request...");

    // Send the streaming chat request
    let mut stream = provider
        .chat_stream(request)
        .await
        .map_err(|e| format!("Streaming chat request failed: {e}"))?;

    info!("🤖 Assistant Response (streaming):");
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

    info!("\n");
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

#[cfg(not(feature = "openai"))]
fn main() {
    println!("OpenAI provider is not enabled. Please enable the 'openai' feature.");
}
