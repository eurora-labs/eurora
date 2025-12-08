# euro-llm-anthropic

[![Crates.io](https://img.shields.io/crates/v/euro-llm-anthropic.svg)](https://crates.io/crates/euro-llm-anthropic)
[![Documentation](https://docs.rs/euro-llm-anthropic/badge.svg)](https://docs.rs/euro-llm-anthropic)

Anthropic provider implementation for the euro-llm ecosystem. This crate provides a complete implementation of Anthropic's Claude API, including chat completions, streaming responses, and tool calling capabilities.

## Features

-   **Claude Chat** - Full support for Anthropic's Messages API
-   **Streaming** - Real-time streaming responses for chat completions
-   **Tool Calling** - Function calling and tool use capabilities with Claude
-   **Multiple Models** - Support for Claude 3.5 Sonnet, Claude 3 Haiku, and Claude 3 Opus
-   **Flexible Configuration** - Environment-based and programmatic configuration
-   **Error Handling** - Comprehensive error types with retry logic
-   **Type Safety** - Full Rust type safety with serde serialization
-   **Vision Support** - Image analysis capabilities with Claude 3 models

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
euro-llm-anthropic = "0.2.0"
```

Or use the main euro-llm crate with the Anthropic feature:

```toml
[dependencies]
euro-llm = { version = "0.2.0", features = ["anthropic"] }
```

## Quick Start

### Basic Chat

```rust
use euro_llm_anthropic::{AnthropicConfig, AnthropicProvider};
use euro_llm_core::{ChatProvider, ChatRequest, Message, MessageContent, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let config = AnthropicConfig::from_env()?;
    let provider = AnthropicProvider::new(config)?;

    // Create a chat request
    let request = ChatRequest {
        messages: vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Explain the theory of relativity".to_string()),
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
    println!("Claude: {}", response.content());

    Ok(())
}
```

### Streaming Chat

```rust
use euro_llm_anthropic::{AnthropicConfig, AnthropicProvider};
use euro_llm_core::{StreamingProvider, ChatRequest};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AnthropicConfig::from_env()?;
    let provider = AnthropicProvider::new(config)?;

    let request = ChatRequest {
        // ... request setup
        ..Default::default()
    };

    let mut stream = provider.chat_stream(request).await?;

    print!("Claude: ");
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(data) => print!("{}", data.content()),
            Err(e) => eprintln!("Stream error: {}", e),
        }
    }
    println!();

    Ok(())
}
```

## Configuration

### Environment Variables

Set these environment variables for automatic configuration:

```bash
export ANTHROPIC_API_KEY="sk-ant-your-api-key-here"
export ANTHROPIC_MODEL="claude-3-5-sonnet-20241022"  # Optional, defaults to claude-3-5-sonnet-20241022
export ANTHROPIC_BASE_URL="https://api.anthropic.com"  # Optional
export ANTHROPIC_VERSION="2023-06-01"  # Optional, API version
```

### Programmatic Configuration

```rust
use euro_llm_anthropic::AnthropicConfig;
use std::time::Duration;

// Simple configuration
let config = AnthropicConfig::new("sk-ant-your-api-key", "claude-3-5-sonnet-20241022");

// Using the builder pattern
let config = AnthropicConfig::builder()
    .api_key("sk-ant-your-api-key")
    .model("claude-3-haiku-20240307")
    .version("2023-06-01")
    .timeout(Duration::from_secs(60))
    .max_retries(3)
    .header("Custom-Header", "value")
    .build();

// From environment with validation
let config = AnthropicConfig::from_env()?;
```

### Custom Base URL

For custom Anthropic-compatible endpoints:

```rust
let config = AnthropicConfig::builder()
    .api_key("your-api-key")
    .model("claude-3-5-sonnet-20241022")
    .base_url("https://your-custom-endpoint.com")?
    .build();
```

## Supported Models

### Claude 3.5 Models

-   `claude-3-5-sonnet-20241022` - Latest and most capable model (default)
-   `claude-3-5-haiku-20241022` - Fast and efficient for most tasks

### Claude 3 Models

-   `claude-3-opus-20240229` - Most capable model for complex tasks
-   `claude-3-sonnet-20240229` - Balanced performance and speed
-   `claude-3-haiku-20240307` - Fastest model for simple tasks

### Model Capabilities

| Model             | Context Length | Strengths                                  |
| ----------------- | -------------- | ------------------------------------------ |
| Claude 3.5 Sonnet | 200K tokens    | Best overall performance, coding, analysis |
| Claude 3.5 Haiku  | 200K tokens    | Fast responses, cost-effective             |
| Claude 3 Opus     | 200K tokens    | Most capable, complex reasoning            |
| Claude 3 Sonnet   | 200K tokens    | Balanced performance                       |
| Claude 3 Haiku    | 200K tokens    | Speed and efficiency                       |

## Advanced Usage

### Tool Calling

```rust
use euro_llm_anthropic::{AnthropicConfig, AnthropicProvider};
use euro_llm_core::{ToolProvider, ChatRequest, Tool, ToolFunction};

let provider = AnthropicProvider::new(config)?;

let tools = vec![
    Tool {
        r#type: "function".to_string(),
        function: ToolFunction {
            name: "calculate".to_string(),
            description: Some("Perform mathematical calculations".to_string()),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Mathematical expression to evaluate"
                    }
                },
                "required": ["expression"]
            }),
        },
    }
];

let response = provider.chat_with_tools(request, &tools).await?;

// Handle tool calls in the response
if let Some(tool_calls) = response.tool_calls() {
    for tool_call in tool_calls {
        println!("Tool called: {}", tool_call.function.name);
        println!("Arguments: {}", tool_call.function.arguments);
    }
}
```

### Vision (Image Analysis)

```rust
use euro_llm_anthropic::{AnthropicConfig, AnthropicProvider};
use euro_llm_core::{ChatProvider, ChatRequest, Message, MessageContent, Role};

let provider = AnthropicProvider::new(config)?;

let request = ChatRequest {
    messages: vec![
        Message {
            role: Role::User,
            content: MessageContent::Mixed(vec![
                ContentPart::Text("What do you see in this image?".to_string()),
                ContentPart::Image {
                    source: ImageSource::Base64 {
                        media_type: "image/jpeg".to_string(),
                        data: base64_image_data,
                    },
                },
            ]),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            created_at: chrono::Utc::now(),
        }
    ],
    parameters: Default::default(),
    metadata: Default::default(),
};

let response = provider.chat(request).await?;
println!("Claude's analysis: {}", response.content());
```

### System Messages

```rust
use euro_llm_core::{ChatRequest, Message, MessageContent, Role, Parameters};

let request = ChatRequest {
    messages: vec![
        Message {
            role: Role::System,
            content: MessageContent::Text(
                "You are a helpful assistant that explains complex topics simply.".to_string()
            ),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            created_at: chrono::Utc::now(),
        },
        Message {
            role: Role::User,
            content: MessageContent::Text("Explain quantum computing".to_string()),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            created_at: chrono::Utc::now(),
        }
    ],
    parameters: Parameters {
        temperature: Some(0.7),
        max_tokens: Some(1000),
        ..Default::default()
    },
    metadata: Default::default(),
};
```

## Error Handling

The crate provides comprehensive error handling:

```rust
use euro_llm_anthropic::{AnthropicError, AnthropicProvider};
use euro_llm_core::ErrorKind;

match provider.chat(request).await {
    Ok(response) => println!("Success: {}", response.content()),
    Err(e) => match e.kind() {
        ErrorKind::Authentication => eprintln!("Invalid API key"),
        ErrorKind::RateLimited => eprintln!("Rate limit exceeded"),
        ErrorKind::InvalidRequest => eprintln!("Invalid request: {}", e),
        ErrorKind::ServerError => eprintln!("Anthropic server error: {}", e),
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
ANTHROPIC_API_KEY=sk-ant-your-key cargo test --test integration_tests
```

## Examples

See the [examples directory](../../examples/) for complete working examples:

-   [`anthropic_chat.rs`](../../examples/anthropic_chat.rs) - Basic chat example
-   [`anthropic_chat_streaming.rs`](../../examples/anthropic_chat_streaming.rs) - Streaming chat example

Run examples:

```bash
export ANTHROPIC_API_KEY="sk-ant-your-key"
cargo run --example anthropic_chat --features anthropic
```

## Rate Limiting

The provider includes automatic retry logic with exponential backoff for rate-limited requests. Configure retry behavior:

```rust
let config = AnthropicConfig::builder()
    .api_key("sk-ant-your-key")
    .max_retries(5)  // Maximum retry attempts
    .timeout(Duration::from_secs(30))  // Request timeout
    .build();
```

## API Compatibility

This crate is compatible with:

-   Anthropic Messages API v2023-06-01
-   Claude 3 and Claude 3.5 model families
-   All current Anthropic API features including tool use and vision

## Best Practices

### Token Management

-   Claude models have a 200K token context window
-   Monitor token usage through response metadata
-   Use appropriate models based on task complexity

### Performance Optimization

-   Use Claude 3 Haiku for simple, fast responses
-   Use Claude 3.5 Sonnet for balanced performance
-   Use Claude 3 Opus for complex reasoning tasks

### Safety and Content Policy

-   Claude models have built-in safety measures
-   Review Anthropic's usage policies
-   Handle content policy violations gracefully

## Contributing

This crate is part of the euro-llm workspace. See the main [repository](../../README.md) for contribution guidelines.

## License

Licensed under the Apache License 2.0. See [LICENSE](../../LICENSE) for details.
