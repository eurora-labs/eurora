# euro-llm-ollama

[![Crates.io](https://img.shields.io/crates/v/euro-llm-ollama.svg)](https://crates.io/crates/euro-llm-ollama)
[![Documentation](https://docs.rs/euro-llm-ollama/badge.svg)](https://docs.rs/euro-llm-ollama)

Ollama provider implementation for the euro-llm ecosystem. This crate provides a complete implementation of Ollama's local API, including chat completions, text generation, streaming responses, and embeddings for locally-hosted language models.

## Features

-   **Local Model Support** - Run models locally with Ollama
-   **Chat Completions** - Full support for Ollama's chat API
-   **Text Generation** - Legacy generate API support
-   **Streaming** - Real-time streaming responses for chat and generation
-   **Embeddings** - Text embedding generation using local embedding models
-   **Model Management** - Integration with Ollama's model management
-   **Flexible Configuration** - Environment-based and programmatic configuration
-   **No API Keys** - Works entirely with local models, no external API keys required
-   **Custom Models** - Support for custom and fine-tuned models
-   **Performance Control** - Configure model parameters like temperature, top-p, etc.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
euro-llm-ollama = "0.2.0"
```

Or use the main euro-llm crate with the Ollama feature:

```toml
[dependencies]
euro-llm = { version = "0.2.0", features = ["ollama"] }
```

## Prerequisites

You need to have Ollama installed and running on your system:

1. **Install Ollama**: Visit [ollama.ai](https://ollama.ai) and follow the installation instructions for your platform.

2. **Start Ollama**: Run the Ollama service:

    ```bash
    ollama serve
    ```

3. **Pull a model**: Download a model to use:
    ```bash
    ollama pull llama2
    ollama pull codellama
    ollama pull mistral
    ```

## Quick Start

### Basic Chat

```rust
use euro_llm_ollama::{OllamaConfig, OllamaProvider};
use euro_llm_core::{ChatProvider, ChatRequest, Message, MessageContent, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration (no API key needed!)
    let config = OllamaConfig::new("llama2");
    let provider = OllamaProvider::new(config)?;

    // Create a chat request
    let request = ChatRequest {
        messages: vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Explain machine learning in simple terms".to_string()),
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
    println!("Llama2: {}", response.content());

    Ok(())
}
```

### Streaming Chat

```rust
use euro_llm_ollama::{OllamaConfig, OllamaProvider};
use euro_llm_core::{StreamingProvider, ChatRequest};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = OllamaConfig::new("codellama");
    let provider = OllamaProvider::new(config)?;

    let request = ChatRequest {
        // ... request setup
        ..Default::default()
    };

    let mut stream = provider.chat_stream(request).await?;

    print!("CodeLlama: ");
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
export OLLAMA_MODEL="llama2"                    # Optional, defaults to llama2
export OLLAMA_BASE_URL="http://localhost:11434" # Optional, defaults to localhost:11434
export OLLAMA_EMBEDDING_MODEL="nomic-embed-text" # Optional, for embeddings
export OLLAMA_KEEP_ALIVE="300"                  # Optional, model keep-alive in seconds
```

### Programmatic Configuration

```rust
use euro_llm_ollama::OllamaConfig;
use std::time::Duration;

// Simple configuration
let config = OllamaConfig::new("mistral");

// Using the builder pattern
let config = OllamaConfig::builder()
    .model("codellama")
    .embedding_model("nomic-embed-text")
    .keep_alive(600)  // Keep model loaded for 10 minutes
    .timeout(Duration::from_secs(120))
    .max_retries(3)
    .build();

// From environment with validation
let config = OllamaConfig::from_env()?;
```

### Remote Ollama Instance

Connect to a remote Ollama server:

```rust
let config = OllamaConfig::builder()
    .model("llama2")
    .base_url("http://ollama-server:11434")?
    .build();
```

## Supported Models

Ollama supports a wide variety of models. Here are some popular ones:

### General Purpose Models

-   `llama2` - Meta's Llama 2 (7B, 13B, 70B variants)
-   `llama2:13b` - Llama 2 13B parameter model
-   `llama2:70b` - Llama 2 70B parameter model
-   `mistral` - Mistral 7B model
-   `mixtral` - Mixtral 8x7B mixture of experts
-   `neural-chat` - Intel's neural chat model

### Code-Specialized Models

-   `codellama` - Code Llama for programming tasks
-   `codellama:python` - Code Llama specialized for Python
-   `phind-codellama` - Phind's fine-tuned Code Llama
-   `wizard-coder` - WizardCoder model

### Embedding Models

-   `nomic-embed-text` - Nomic's text embedding model
-   `all-minilm` - Sentence transformer embedding model

### Check Available Models

```bash
# List downloaded models
ollama list

# Pull a new model
ollama pull mistral:7b

# Remove a model
ollama rm old-model
```

## Advanced Usage

### Text Generation (Legacy API)

```rust
use euro_llm_ollama::{OllamaConfig, OllamaProvider};
use euro_llm_core::{CompletionProvider, CompletionRequest};

let provider = OllamaProvider::new(config)?;

let request = CompletionRequest {
    prompt: "Write a Python function to calculate fibonacci numbers:".to_string(),
    parameters: Default::default(),
    metadata: Default::default(),
};

let response = provider.complete(request).await?;
println!("Generated code:\n{}", response.content());
```

### Embeddings

```rust
use euro_llm_ollama::{OllamaConfig, OllamaProvider};
use euro_llm_core::EmbeddingProvider;

let config = OllamaConfig::builder()
    .model("llama2")
    .embedding_model("nomic-embed-text")
    .build();

let provider = OllamaProvider::new(config)?;

let texts = vec![
    "The quick brown fox".to_string(),
    "jumps over the lazy dog".to_string(),
];

let embeddings = provider.embed(&texts).await?;
for embedding in embeddings {
    println!("Embedding dimension: {}", embedding.vector.len());
}
```

### Model Parameters

Configure model behavior with custom parameters:

```rust
use euro_llm_core::{ChatRequest, Parameters};

let request = ChatRequest {
    messages: vec![/* ... */],
    parameters: Parameters {
        temperature: Some(0.8),    // Creativity level (0.0 - 2.0)
        top_p: Some(0.9),         // Nucleus sampling
        top_k: Some(40),          // Top-k sampling
        max_tokens: Some(500),    // Maximum response length
        stop_sequences: vec!["Human:".to_string()], // Stop generation at these sequences
        ..Default::default()
    },
    metadata: Default::default(),
};
```

### Custom Model Options

Pass Ollama-specific options:

```rust
let config = OllamaConfig::builder()
    .model("llama2")
    .options(serde_json::json!({
        "num_ctx": 4096,      // Context window size
        "num_predict": 256,   // Number of tokens to predict
        "repeat_penalty": 1.1, // Repetition penalty
        "temperature": 0.7,   // Temperature
        "top_k": 40,         // Top-k sampling
        "top_p": 0.9         // Top-p sampling
    }))
    .build();
```

## Error Handling

The crate provides comprehensive error handling:

```rust
use euro_llm_ollama::{OllamaError, OllamaProvider};
use euro_llm_core::ErrorKind;

match provider.chat(request).await {
    Ok(response) => println!("Success: {}", response.content()),
    Err(e) => match e.kind() {
        ErrorKind::InvalidRequest => eprintln!("Invalid request: {}", e),
        ErrorKind::ServerError => eprintln!("Ollama server error: {}", e),
        ErrorKind::NetworkError => eprintln!("Network error - is Ollama running?"),
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

# Integration tests (requires Ollama running)
cargo test --test integration_tests
```

**Note**: Integration tests require Ollama to be running with at least the `llama2` model available.

## Examples

See the [examples directory](../../examples/) for complete working examples:

-   [`ollama_chat.rs`](../../examples/ollama_chat.rs) - Basic chat example
-   [`ollama_chat_streaming.rs`](../../examples/ollama_chat_streaming.rs) - Streaming chat example

Run examples:

```bash
# Make sure Ollama is running and has the model
ollama pull llama2
cargo run --example ollama_chat --features ollama
```

## Performance Tips

### Model Loading

-   Use `keep_alive` to keep models in memory between requests
-   Larger models provide better quality but are slower
-   Consider using smaller models (7B) for faster responses

### Hardware Optimization

-   Ollama automatically uses GPU acceleration when available
-   More RAM allows for larger models and longer contexts
-   SSD storage improves model loading times

### Configuration

```rust
let config = OllamaConfig::builder()
    .model("llama2:7b")        // Use smaller model for speed
    .keep_alive(1800)          // Keep model loaded for 30 minutes
    .timeout(Duration::from_secs(60)) // Reasonable timeout
    .build();
```

## Troubleshooting

### Common Issues

1. **Connection Refused**

    ```
    Error: Network error - is Ollama running?
    ```

    - Ensure Ollama is running: `ollama serve`
    - Check the base URL in your configuration

2. **Model Not Found**

    ```
    Error: model 'llama2' not found
    ```

    - Pull the model: `ollama pull llama2`
    - Check available models: `ollama list`

3. **Out of Memory**
    ```
    Error: not enough memory to load model
    ```
    - Use a smaller model variant (e.g., `llama2:7b` instead of `llama2:70b`)
    - Close other applications to free memory

### Debugging

Enable debug logging to troubleshoot issues:

```rust
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    // Your code here
}
```

## Model Comparison

| Model      | Size  | Use Case                     | Speed  | Quality       |
| ---------- | ----- | ---------------------------- | ------ | ------------- |
| llama2:7b  | ~4GB  | General chat, fast responses | Fast   | Good          |
| llama2:13b | ~7GB  | Better reasoning             | Medium | Better        |
| llama2:70b | ~40GB | Complex tasks                | Slow   | Best          |
| codellama  | ~4GB  | Code generation              | Fast   | Good for code |
| mistral    | ~4GB  | Efficient general purpose    | Fast   | Good          |
| mixtral    | ~26GB | High-quality responses       | Medium | Excellent     |

## Contributing

This crate is part of the euro-llm workspace. See the main [repository](../../README.md) for contribution guidelines.

## License

Licensed under the Apache License 2.0. See [LICENSE](../../LICENSE) for details.
