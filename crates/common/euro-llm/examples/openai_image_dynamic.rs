//! Image description example using the OpenAI provider.
//!
//! This example demonstrates how to use the euro-llm-openai crate to send
//! an image to OpenAI's vision-capable models and receive a description.
//!
//! To run this example:
//! ```bash
//! export OPENAI_API_KEY="your-api-key-here"
//! cargo run --example openai_image_description
//! ```

#[cfg(feature = "openai")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use euro_llm::core::{ChatProvider, ChatRequest, ChatResponse, ContentPart, ImageSource};
    use euro_llm::openai::{OpenAIConfig, OpenAIProvider};
    use std::path::Path;
    use tracing::info;

    dotenv::dotenv().ok();
    // Initialize tracing for better error reporting
    tracing_subscriber::fmt::init();

    info!("🖼️  OpenAI Image Description Example");
    info!("====================================");

    // Load configuration from environment variables
    // Requires OPENAI_API_KEY to be set
    let mut config = OpenAIConfig::from_env()
        .map_err(|e| format!("Failed to load config from environment: {e}"))?;

    // Use a vision-capable model for image analysis
    config.model = "gpt-4o".to_string();

    info!("✅ Configuration loaded successfully");
    info!("📝 Model: {}", config.model);

    // Create the OpenAI provider
    let provider = OpenAIProvider::new(config)
        .map_err(|e| format!("Failed to create OpenAI provider: {e}"))?;

    info!("🔗 Provider created successfully");

    // Get the path to the image in the assets folder
    let image_path = Path::new("assets/illustration.png");

    if !image_path.exists() {
        return Err(format!("Image not found at path: {}", image_path.display()).into());
    }

    info!("📸 Loading image from: {}", image_path.display());

    // Read the image file and convert to base64
    let image_data =
        std::fs::read(image_path).map_err(|e| format!("Failed to read image file: {e}"))?;

    let image =
        image::load_from_memory(&image_data).map_err(|e| format!("Failed to load image: {e}"))?;

    info!("🔄 Image converted to base64 data URL");

    // Create multimodal content with text and image
    let content_parts = vec![
        ContentPart::text("Please describe this image in detail. What do you see?"),
        ContentPart::image(ImageSource::DynamicImage(image)),
    ];

    // Create a chat request with multimodal content
    let request = ChatRequest::builder()
        .system_message(
            "You are a helpful assistant that provides detailed and accurate descriptions of images. \
             Focus on the main subjects, colors, composition, and any notable details or artistic elements.",
        )
        .user_multimodal(content_parts)
        .temperature(0.3)
        .max_tokens(500)
        .request_id("example-image-description-001".to_string())
        .user_id("example-user".to_string())
        .build();

    info!("📤 Sending image description request...");

    // Send the chat request
    let response = provider
        .chat(request)
        .await
        .map_err(|e| format!("Chat request failed: {e}"))?;

    info!("📥 Response received!");

    // Display the response
    info!("🤖 Image Description:");
    info!("────────────────────");
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

    info!("✅ Image description example completed successfully!");

    Ok(())
}

#[cfg(not(feature = "openai"))]
fn main() {
    println!("OpenAI provider is not enabled. Please enable the 'openai' feature.");
}
