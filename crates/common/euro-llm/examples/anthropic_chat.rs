//! Basic chat example using the Anthropic provider.
//!
//! This example demonstrates how to use the euro-llm-anthropic crate to send
//! a simple chat request to Anthropic's Claude API and receive a response.
//!
//! To run this example:
//! ```bash
//! export ANTHROPIC_API_KEY="your-api-key-here"
//! cargo run --example anthropic_chat
//! ```

#[cfg(feature = "anthropic")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use euro_llm::anthropic::{AnthropicConfig, AnthropicProvider};
    use euro_llm::{ChatProvider, ChatRequest, ChatResponse};
    use tracing::info;
    dotenv::dotenv().ok();
    // Initialize tracing for better error reporting
    tracing_subscriber::fmt::init();

    info!("🤖 Anthropic Chat Example");
    info!("=========================");

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

    // Create a simple chat request using the improved API
    let request = ChatRequest::builder()
        .system_message(
            "You are a helpful assistant that provides concise and informative responses.",
        )
        .user_message("Hello! Can you explain what Rust is in one paragraph?")
        .temperature(0.7)
        .max_tokens(150)
        .top_p(1.0)
        .request_id("example-anthropic-chat-001".to_string())
        .user_id("example-user".to_string())
        .build();

    info!("📤 Sending chat request...");

    // Send the chat request
    let response = provider
        .chat(request)
        .await
        .map_err(|e| format!("Chat request failed: {e}"))?;

    info!("📥 Response received!");

    // Display the response
    info!("🤖 Claude Response:");
    info!("──────────────────");
    info!("{}", response.content());

    // Display usage information if available
    if let Some(usage) = response.usage() {
        info!("📊 Usage Statistics:");
        info!("   • Prompt tokens: {}", usage.prompt_tokens);
        info!("   • Completion tokens: {}", usage.completion_tokens);
        info!("   • Total tokens: {}", usage.total_tokens);
    }

    // Display finish reason
    if let Some(finish_reason) = response.finish_reason() {
        info!("🏁 Finish reason: {:?}", finish_reason);
    }

    // Display metadata
    let metadata = response.metadata();
    if let Some(request_id) = metadata.request_id {
        info!("🔍 Request ID: {}", request_id);
    }

    info!("✅ Example completed successfully!");

    Ok(())
}

#[cfg(not(feature = "anthropic"))]
fn main() {
    println!("Anthropic provider is not enabled. Please enable the 'anthropic' feature.");
}
