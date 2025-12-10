//! Basic chat example using the Ollama provider.
//!
//! This example demonstrates how to use the euro-llm-ollama crate to send
//! a simple chat request to Ollama's API and receive a response.
//!
//! To run this example:
//! ```bash
//! # Make sure Ollama is running locally with a model installed
//! # For example: ollama pull llama2
//! cargo run --example ollama_chat
//! ```

#[cfg(feature = "ollama")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use euro_llm::core::{ChatProvider, ChatRequest, ChatResponse};
    use euro_llm::ollama::{OllamaConfig, OllamaProvider};
    use tracing::info;
    dotenv::dotenv().ok();
    // Initialize tracing for better error reporting
    tracing_subscriber::fmt::init();

    info!("🦙 Ollama Chat Example");
    info!("=====================");

    // Create Ollama configuration
    // Uses default localhost:11434 endpoint with llama2 model
    let config = OllamaConfig::builder()
        .model("llama3.2")
        .keep_alive(300) // Keep model loaded for 5 minutes
        .build();

    info!("✅ Configuration created successfully");
    info!("📝 Model: {}", config.model);
    info!("🌐 Base URL: {}", config.base_url());

    // Create the Ollama provider
    let provider = OllamaProvider::new(config)
        .map_err(|e| format!("Failed to create Ollama provider: {e}"))?;

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
        .request_id("example-ollama-chat-001".to_string())
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
    info!("🦙 Assistant Response:");
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

#[cfg(not(feature = "ollama"))]
fn main() {
    println!("Ollama provider is not enabled. Please enable the 'ollama' feature.");
}
