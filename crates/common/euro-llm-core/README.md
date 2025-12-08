# euro-llm-core

[![Crates.io](https://img.shields.io/crates/v/euro-llm-core.svg)](https://crates.io/crates/euro-llm-core)
[![Documentation](https://docs.rs/euro-llm-core/badge.svg)](https://docs.rs/euro-llm-core)

Core traits and types for the euro-llm ecosystem. This crate provides the foundational abstractions that all LLM providers implement, including traits for chat, completion, streaming, and tool calling, as well as standardized request/response types and error handling.

## Overview

`euro-llm-core` is the foundation crate that defines the common interface for all LLM providers in the euro-llm ecosystem. It follows the Interface Segregation Principle, allowing providers to implement only the capabilities they support.

## Features

-   **Core Traits**: Foundational traits for chat, completion, streaming, and specialized capabilities
-   **Standardized Types**: Common request/response types across all providers
-   **Error Handling**: Unified error types and handling patterns
-   **Configuration**: Base configuration traits and utilities
-   **Type Safety**: Leverages Rust's type system for safe LLM interactions
-   **Async Support**: Full async/await support with proper trait bounds

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
euro-llm-core = "0.2.0"
```

## Core Traits

### ChatProvider

The primary trait for chat-based LLM interactions:

```rust
use euro_llm_core::{ChatProvider, ChatRequest};

#[async_trait]
pub trait ChatProvider: Send + Sync {
    type Config: ProviderConfig;
    type Response: ChatResponse;
    type Error: ProviderError;

    async fn chat(&self, request: ChatRequest) -> Result<Self::Response, Self::Error>;
}
```

### StreamingProvider

Extends `ChatProvider` with streaming capabilities:

```rust
use euro_llm_core::{StreamingProvider, ChatRequest};
use futures::Stream;

#[async_trait]
pub trait StreamingProvider: ChatProvider {
    type StreamItem: Send + 'static;
    type Stream: Stream<Item = Result<Self::StreamItem, Self::Error>> + Send + 'static;

    async fn chat_stream(&self, request: ChatRequest) -> Result<Self::Stream, Self::Error>;
}
```

### Specialized Traits

-   **`CompletionProvider`** - Text completion (non-chat) capabilities
-   **`ToolProvider`** - Function/tool calling support
-   **`EmbeddingProvider`** - Text embedding generation
-   **`ImageProvider`** - Image generation capabilities
-   **`SpeechToTextProvider`** - Speech transcription
-   **`TextToSpeechProvider`** - Speech synthesis

## Core Types

### ChatRequest

Standard request structure for chat interactions:

```rust
use euro_llm_core::{ChatRequest, Message, Parameters, Metadata};

let request = ChatRequest {
    messages: vec![
        Message {
            role: Role::User,
            content: MessageContent::Text("Hello!".to_string()),
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
```

### Message Types

-   **`Role`** - System, User, Assistant, Tool
-   **`MessageContent`** - Text, Image, or Mixed content
-   **`Parameters`** - Generation parameters (temperature, max_tokens, etc.)
-   **`Metadata`** - Request metadata and extensions

### Response Types

All providers implement standardized response traits:

```rust
use euro_llm_core::ChatResponse;

// Common response interface
pub trait ChatResponse {
    fn content(&self) -> &str;
    fn usage(&self) -> Option<&Usage>;
    fn finish_reason(&self) -> Option<&FinishReason>;
    fn metadata(&self) -> &Metadata;
}
```

## Error Handling

Unified error handling across all providers:

```rust
use euro_llm_core::{ProviderError, ErrorKind};

pub trait ProviderError: std::error::Error + Send + Sync {
    fn kind(&self) -> ErrorKind;
    fn is_retryable(&self) -> bool;
    fn status_code(&self) -> Option<u16>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    Authentication,
    RateLimited,
    InvalidRequest,
    ServerError,
    NetworkError,
    Timeout,
    Unknown,
}
```

## Configuration

Base configuration trait for all providers:

```rust
use euro_llm_core::ProviderConfig;

pub trait ProviderConfig: Clone + Send + Sync {
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn timeout(&self) -> std::time::Duration;
    fn base_url(&self) -> &str;
}
```

## Usage Example

```rust
use euro_llm_core::{
    ChatProvider, ChatRequest, Message, MessageContent,
    Role, Parameters, Metadata
};

// This example shows how to use the core types
// Actual provider implementations are in separate crates

async fn example_usage<P>(provider: P) -> Result<(), P::Error>
where
    P: ChatProvider,
{
    let request = ChatRequest {
        messages: vec![
            Message {
                role: Role::User,
                content: MessageContent::Text("Explain Rust".to_string()),
                name: None,
                tool_calls: None,
                tool_call_id: None,
                created_at: chrono::Utc::now(),
            }
        ],
        parameters: Parameters {
            temperature: Some(0.7),
            max_tokens: Some(150),
            ..Default::default()
        },
        metadata: Metadata::default(),
    };

    let response = provider.chat(request).await?;
    println!("Response: {}", response.content());

    Ok(())
}
```

## Provider Implementation

To implement a new provider, create a struct that implements the relevant traits:

```rust
use euro_llm_core::{ChatProvider, ChatRequest, ChatResponse};
use async_trait::async_trait;

pub struct MyProvider {
    config: MyConfig,
}

#[async_trait]
impl ChatProvider for MyProvider {
    type Config = MyConfig;
    type Response = MyResponse;
    type Error = MyError;

    async fn chat(&self, request: ChatRequest) -> Result<Self::Response, Self::Error> {
        // Implementation here
        todo!()
    }
}
```

## Testing

The core crate includes utilities for testing provider implementations:

```rust
cargo test
```

## Contributing

This crate is part of the euro-llm workspace. See the main [repository](../../README.md) for contribution guidelines.

## License

Licensed under the Apache License 2.0. See [LICENSE](../../LICENSE) for details.
