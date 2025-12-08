# euro-llm-openai

[![Crates.io](https://img.shields.io/crates/v/euro-llm-openai.svg)](https://crates.io/crates/euro-llm-openai)
[![Documentation](https://docs.rs/euro-llm-openai/badge.svg)](https://docs.rs/euro-llm-openai)

OpenAI provider implementation for the euro-llm ecosystem. This crate provides a complete implementation of OpenAI's API, including chat completions, text completions, embeddings, streaming, and tool calling capabilities.

## Features

-   **Chat Completions** - Full support for OpenAI's chat completions API
-   **Text Completions** - Legacy completions API support
-   **Streaming** - Real-time streaming responses for chat and completions
-   **Embeddings** - Text embedding generation using OpenAI's embedding models
-   **Tool Calling** - Function calling and tool use capabilities
-   **Flexible Configuration** - Environment-based and programmatic configuration
-   **Error Handling** - Comprehensive error types with retry logic
-   **Type Safety** - Full Rust type safety with serde serialization

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
euro-llm-openai = "0.2.0"
```

Or use the main euro-llm crate with the OpenAI feature:

```toml
[dependencies]
euro-llm = { version = "0.2.0", features = ["openai"] }
```

## Quick Start

### Basic Chat

```rust
use euro_llm_openai::{OpenAIConfig, OpenAIProvider};
use euro_llm_core::{ChatProvider, ChatRequest, Message, MessageContent, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let config = OpenAIConfig::from_env()?;
    let provider = OpenAIProvider::new(config)?;

    // Create a chat request
    let request = ChatRequest {
        messages: vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Explain quantum computing".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                created_at: chrono::Utc::now(),
            }
        ],
        parameters: Default::default(),
        metadata: Default::default(),
    };

    // Send the request
    let response = provider.chat(request).await?;
    println!("Response: {}", response.content());

    Ok(())
}
```

### Streaming Chat

```rust
use euro_llm_openai::{OpenAIConfig, OpenAIProvider};
use euro_llm_core::{StreamingProvider, ChatRequest};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = OpenAIConfig::from_env()?;
    let provider = OpenAIProvider::new(config)?;

    let request = ChatRequest {
        // ... request setup
        ..Default::default()
    };

    let mut stream = provider.chat_stream(request).await?;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(data) => print!("{}", data.content()),
            Err(e) => eprintln!("Stream error: {}", e),
        }
    }

    Ok(())
}
```

## Configuration

### Environment Variables

Set these environment variables for automatic configuration:

```bash
export OPENAI_API_KEY="sk-your-api-key-here"
export OPENAI_MODEL="gpt-4"                    # Optional, defaults to gpt-3.5-turbo
export OPENAI_BASE_URL="https://api.openai.com/v1"  # Optional
export OPENAI_ORGANIZATION="org-your-org-id"   # Optional
export OPENAI_PROJECT="proj-your-project-id"   # Optional
```

### Programmatic Configuration

```rust
use euro_llm_openai::OpenAIConfig;
use std::time::Duration;

// Simple configuration
let config = OpenAIConfig::new("sk-your-api-key", "gpt-4");

// Using the builder pattern
let config = OpenAIConfig::builder()
    .api_key("sk-your-api-key")
    .model("gpt-4")
    .organization("org-your-org-id")
    .timeout(Duration::from_secs(60))
    .max_retries(3)
    .header("Custom-Header", "value")
    .build();

// From environment with validation
let config = OpenAIConfig::from_env()?;
```

### Custom Base URL

For OpenAI-compatible APIs (like Azure OpenAI):

```rust
let config = OpenAIConfig::builder()
    .api_key("your-api-key")
    .model("gpt-4")
    .base_url("https://your-custom-endpoint.com/v1")?
    .build();
```

## Supported Models

### Chat Models

-   `gpt-4` - Most capable model
-   `gpt-4-turbo` - Latest GPT-4 with improved performance
-   `gpt-3.5-turbo` - Fast and efficient for most tasks
-   `gpt-3.5-turbo-16k` - Extended context length

### Embedding Models

-   `text-embedding-ada-002` - Most capable embedding model
-   `text-embedding-3-small` - Smaller, faster embedding model
-   `text-embedding-3-large` - Larger, more capable embedding model

### Image Models

-   `dall-e-3` - Latest image generation model
-   `dall-e-2` - Previous generation image model

### Audio Models

-   `whisper-1` - Speech-to-text transcription
-   `tts-1` - Text-to-speech synthesis
-   `tts-1-hd` - High-definition text-to-speech

## Advanced Usage

### Tool Calling

```rust
use euro_llm_openai::{OpenAIConfig, OpenAIProvider};
use euro_llm_core::{ToolProvider, ChatRequest, Tool, ToolFunction};

let provider = OpenAIProvider::new(config)?;

let tools = vec![
    Tool {
        r#type: "function".to_string(),
        function: ToolFunction {
            name: "get_weather".to_string(),
            description: Some("Get current weather".to_string()),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {"type": "string"}
                }
            }),
        },
    }
];

let response = provider.chat_with_tools(request, &tools).await?;
```

### Embeddings

```rust
use euro_llm_openai::{OpenAIConfig, OpenAIProvider};
use euro_llm_core::EmbeddingProvider;

let provider = OpenAIProvider::new(config)?;

let texts = vec![
    "The quick brown fox".to_string(),
    "jumps over the lazy dog".to_string(),
];

let embeddings = provider.embed(&texts).await?;
for embedding in embeddings {
    println!("Embedding dimension: {}", embedding.vector.len());
}
```

### Image Generation

```rust
use euro_llm_openai::{OpenAIConfig, OpenAIProvider};
use euro_llm_core::{ImageProvider, ImageRequest};

let provider = OpenAIProvider::new(config)?;

let request = ImageRequest {
    prompt: "A futuristic city at sunset".to_string(),
    n: Some(1),
    size: Some("1024x1024".to_string()),
    quality: Some("standard".to_string()),
    style: Some("vivid".to_string()),
};

let response = provider.generate_image(request).await?;
for image in response.images() {
    println!("Generated image URL: {}", image.url);
}
```

## Error Handling

The crate provides comprehensive error handling:

```rust
use euro_llm_openai::{OpenAIError, OpenAIProvider};
use euro_llm_core::ErrorKind;

match provider.chat(request).await {
    Ok(response) => println!("Success: {}", response.content()),
    Err(e) => match e.kind() {
        ErrorKind::Authentication => eprintln!("Invalid API key"),
        ErrorKind::RateLimited => eprintln!("Rate limit exceeded"),
        ErrorKind::InvalidRequest => eprintln!("Invalid request: {}", e),
        ErrorKind::ServerError => eprintln!("OpenAI server error: {}", e),
        ErrorKind::NetworkError => eprintln!("Network error: {}", e),
        ErrorKind::Timeout => eprintln!("Request timeout"),
        _ => eprintln!("Unknown error: {}", e),
    }
}
```

## Testing

Run the test suite:

```bash
# Unit tests
cargo test

# Integration tests (requires API key)
OPENAI_API_KEY=sk-your-key cargo test --test integration_tests

# End-to-end tests
OPENAI_API_KEY=sk-your-key cargo test --test e2e_tests
```

## Examples

See the [examples directory](../../examples/) for complete working examples:

-   [`openai_chat.rs`](../../examples/openai_chat.rs) - Basic chat example
-   [`openai_chat_streaming.rs`](../../examples/openai_chat_streaming.rs) - Streaming chat example

Run examples:

```bash
export OPENAI_API_KEY="sk-your-key"
cargo run --example openai_chat --features openai
```

## Rate Limiting

The provider includes automatic retry logic with exponential backoff for rate-limited requests. Configure retry behavior:

```rust
let config = OpenAIConfig::builder()
    .api_key("sk-your-key")
    .max_retries(5)  // Maximum retry attempts
    .timeout(Duration::from_secs(30))  // Request timeout
    .build();
```

## Compatibility

This crate is compatible with:

-   OpenAI API v1
-   Azure OpenAI Service
-   OpenAI-compatible APIs (with custom base URL)

## Contributing

This crate is part of the euro-llm workspace. See the main [repository](../../README.md) for contribution guidelines.

## License

Licensed under the Apache License 2.0. See [LICENSE](../../LICENSE) for details.
