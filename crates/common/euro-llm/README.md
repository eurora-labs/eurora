# euro-llm

[![Crates.io](https://img.shields.io/crates/v/euro-llm.svg)](https://crates.io/crates/euro-llm)
[![Documentation](https://docs.rs/euro-llm/badge.svg)](https://docs.rs/euro-llm)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%2B-orange.svg)](https://www.rust-lang.org)

A unified cross-platform Rust library for interacting with multiple Large Language Model providers. euro-LLM provides a modular, type-safe, and performant abstraction layer that allows developers to easily switch between different LLM providers while maintaining consistent APIs.

## 📦 Installation
Add euro-llm to your `Cargo.toml`:

```toml
[dependencies]
euro-llm = "*"
```

### Feature Flags

By default, no providers are enabled. Enable the providers you need:

```toml
[dependencies]
euro-llm = { version = "*", features = ["openai", "anthropic", "ollama", "specta"] }
```

Available features:

-   `default` - All providers (equivalent to enabling all individual features)
-   `openai` - OpenAI provider support
-   `anthropic` - Anthropic Claude provider support
-   `ollama` - Ollama local model provider support
-   `specta` - Specta types generator support
-   `dynamic-image` - Support for the image crate in order to send the image data via bytes

## 🔧 Quick Start

### Basic Chat Example

```rust
use euro_llm::{
    ChatProvider, ChatRequest, Message, MessageContent,
    Parameters, Role, Metadata,
    openai::{OpenAIConfig, OpenAIProvider},
};

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
                content: MessageContent::Text("Hello! Explain Rust in one sentence.".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                created_at: chrono::Utc::now(),
            }
        ],
        parameters: Parameters {
            temperature: Some(0.7),
            max_tokens: Some(100),
            ..Default::default()
        },
        metadata: Metadata::default(),
    };

    // Send the request
    let response = provider.chat(request).await?;
    println!("Response: {}", response.content());

    Ok(())
}
```

### Streaming Chat Example

```rust
use euro_llm::{
    StreamingProvider, ChatRequest, Message, MessageContent, Role,
    anthropic::{AnthropicConfig, AnthropicProvider},
};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AnthropicConfig::from_env()?;
    let provider = AnthropicProvider::new(config)?;

    let request = ChatRequest {
        messages: vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Tell me a story".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                created_at: chrono::Utc::now(),
            }
        ],
        parameters: Default::default(),
        metadata: Default::default(),
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

## 🔌 Supported Providers

### OpenAI

```rust
use euro_llm::openai::{OpenAIConfig, OpenAIProvider};

// From environment variables
let config = OpenAIConfig::from_env()?;

// Or configure manually
let config = OpenAIConfig {
    api_key: "your-api-key".to_string(),
    model: "gpt-4".to_string(),
    base_url: "https://api.openai.com/v1".to_string(),
    timeout: std::time::Duration::from_secs(30),
};

let provider = OpenAIProvider::new(config)?;
```

**Environment Variables:**

-   `OPENAI_API_KEY` - Your OpenAI API key (required)
-   `OPENAI_MODEL` - Model to use (default: "gpt-3.5-turbo")
-   `OPENAI_BASE_URL` - API base URL (default: "https://api.openai.com/v1")

### Anthropic

```rust
use euro_llm::anthropic::{AnthropicConfig, AnthropicProvider};

let config = AnthropicConfig::from_env()?;
let provider = AnthropicProvider::new(config)?;
```

**Environment Variables:**

-   `ANTHROPIC_API_KEY` - Your Anthropic API key (required)
-   `ANTHROPIC_MODEL` - Model to use (default: "claude-3-sonnet-20240229")
-   `ANTHROPIC_BASE_URL` - API base URL (default: "https://api.anthropic.com")

### Ollama

```rust
use euro_llm::ollama::{OllamaConfig, OllamaProvider};

let config = OllamaConfig::from_env()?;
let provider = OllamaProvider::new(config)?;
```

**Environment Variables:**

-   `OLLAMA_MODEL` - Model to use (default: "llama2")
-   `OLLAMA_BASE_URL` - Ollama server URL (default: "http://localhost:11434")

## 🎯 Core Traits

euro-LLM follows the Interface Segregation Principle with focused traits:

### [`ChatProvider`](crates/euro-llm-core/src/traits.rs)

Core chat functionality that most LLM providers support.

```rust
#[async_trait]
pub trait ChatProvider: Send + Sync {
    type Config: ProviderConfig;
    type Response: ChatResponse;
    type Error: ProviderError;

    async fn chat(&self, request: ChatRequest) -> Result<Self::Response, Self::Error>;
}
```

### [`StreamingProvider`](crates/euro-llm-core/src/traits.rs)

Extends ChatProvider with streaming capabilities.

```rust
#[async_trait]
pub trait StreamingProvider: ChatProvider {
    type StreamItem: Send + 'static;
    type Stream: Stream<Item = Result<Self::StreamItem, Self::Error>> + Send + 'static;

    async fn chat_stream(&self, request: ChatRequest) -> Result<Self::Stream, Self::Error>;
}
```

### Additional Traits

-   [`CompletionProvider`](crates/euro-llm-core/src/traits.rs) - Text completion (non-chat)
-   [`ToolProvider`](crates/euro-llm-core/src/traits.rs) - Function/tool calling
-   [`EmbeddingProvider`](crates/euro-llm-core/src/traits.rs) - Text embeddings
-   [`ImageProvider`](crates/euro-llm-core/src/traits.rs) - Image generation
-   [`SpeechToTextProvider`](crates/euro-llm-core/src/traits.rs) - Speech transcription
-   [`TextToSpeechProvider`](crates/euro-llm-core/src/traits.rs) - Speech synthesis

## 📚 Examples

The [`examples/`](examples/) directory contains comprehensive examples:

-   [`openai_chat.rs`](examples/openai_chat.rs) - Basic OpenAI chat
-   [`openai_chat_streaming.rs`](examples/openai_chat_streaming.rs) - OpenAI streaming chat
-   [`anthropic_chat.rs`](examples/anthropic_chat.rs) - Basic Anthropic chat
-   [`anthropic_chat_streaming.rs`](examples/anthropic_chat_streaming.rs) - Anthropic streaming chat
-   [`ollama_chat.rs`](examples/ollama_chat.rs) - Basic Ollama chat
-   [`ollama_chat_streaming.rs`](examples/ollama_chat_streaming.rs) - Ollama streaming chat

Run examples with:

```bash
# Set up environment variables
export OPENAI_API_KEY="your-key"
export ANTHROPIC_API_KEY="your-key"

# Run specific examples
cargo run --example openai_chat --features openai
cargo run --example anthropic_chat_streaming --features anthropic
cargo run --example ollama_chat --features ollama
```

## 🧪 Testing

Run tests for all crates:

```bash
# Run all tests
cargo test --workspace

# Run tests for specific provider
cargo test -p euro-llm-openai
cargo test -p euro-llm-anthropic
cargo test -p euro-llm-ollama

# Run integration tests
cargo test --test integration_tests
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Setup

1. Clone the repository:

    ```bash
    git clone https://github.com/your-username/euro-llm.git
    cd euro-llm
    ```

2. Install Rust (if not already installed):

    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

3. Set up environment variables:

    ```bash
    cp .env.example .env
    # Edit .env with your API keys
    ```

4. Run tests:
    ```bash
    cargo test --workspace
    ```

### Adding a New Provider

1. Create a new crate in `crates/euro-llm-{provider}/`
2. Implement the required traits from `euro-llm-core`
3. Add integration tests
4. Update the main crate's feature flags
5. Add examples demonstrating usage

## 📄 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

-   [Documentation](https://docs.rs/euro-llm)
-   [Crates.io](https://crates.io/crates/euro-llm)
-   [Repository](https://github.com/your-username/euro-llm)
-   [Issues](https://github.com/your-username/euro-llm/issues)

## 🙏 Acknowledgments

-   The Rust community for excellent async and HTTP libraries
-   OpenAI, Anthropic, and Ollama for their APIs and documentation
-   Contributors and users of this library

---

**Note**: This library is in active development. APIs may change before 1.0 release.
