//! Basic chat example using the OpenAI provider.
//!
//! This example demonstrates how to use the euro-llm-openai crate to send
//! a simple chat request to OpenAI's API and receive a response.
//!
//! To run this example:
//! ```bash
//! export OPENAI_API_KEY="your-api-key-here"
//! cargo run --example openai_chat
//! ```

#[cfg(feature = "openai")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use euro_llm::core::{ChatProvider, ChatRequest, ChatResponse};
    use euro_llm::openai::{OpenAIConfig, OpenAIProvider};
    use tracing::info;

    dotenv::dotenv().ok();
    // Initialize tracing for better error reporting
    tracing_subscriber::fmt::init();

    info!("🤖 OpenAI Chat Example");
    info!("======================");

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

    // Create a simple chat request using the improved API
    let request = ChatRequest::builder()
        .system_message(
            "You are a helpful assistant that provides concise and informative responses.",
        )
        .user_message("Hello! Can you explain what Rust is in one paragraph?")
        .temperature(0.7)
        .max_tokens(150)
        .top_p(1.0)
        .request_id("example-chat-001".to_string())
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
    info!("🤖 Assistant Response:");
    info!("─────────────────────");
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

#[cfg(not(feature = "openai"))]
fn main() {
    println!("OpenAI provider is not enabled. Please enable the 'openai' feature.");
}
