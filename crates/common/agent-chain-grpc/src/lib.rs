//! gRPC provider for agent-chain.
//!
//! This crate provides a gRPC-based implementation of the `ChatModel` trait
//! from agent-chain, allowing you to use Eurora's chat service with the
//! LangChain-compatible agent-chain interface.
//!
//! # Example
//!
//! ```ignore
//! use agent_chain_grpc::ChatEurora;
//! use agent_chain_core::{ChatModel, HumanMessage};
//! use url::Url;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create configuration
//!     let config = EuroraConfig::new(Url::parse("https://api.eurora.com")?);
//!
//!     // Create the chat model
//!     let model = ChatEurora::new(config).await?;
//!
//!     // Use the model
//!     let messages = vec![HumanMessage::new("Hello!").into()];
//!     let response = model.generate(messages, None).await?;
//!
//!     println!("Response: {}", response.message.content());
//!     Ok(())
//! }
//! ```
//!
//! # Streaming
//!
//! The provider also supports streaming responses:
//!
//! ```ignore
//! use agent_chain_grpc::ChatEurora;
//! use agent_chain_core::{ChatModel, HumanMessage};
//! use futures::StreamExt;
//! use url::Url;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = EuroraConfig::new(Url::parse("https://api.eurora.com")?);
//!     let model = ChatEurora::new(config).await?;
//!
//!     let messages = vec![HumanMessage::new("Tell me a story").into()];
//!     let mut stream = model.stream(messages, None).await?;
//!
//!     while let Some(chunk) = stream.next().await {
//!         match chunk {
//!             Ok(c) => print!("{}", c.content),
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

/// Proto-generated types for the Eurora chat service.
pub mod proto {
    tonic::include_proto!("chat_service");
}

pub mod types;

// Re-export agent-chain types that users will need
pub use agent_chain_core::{BaseChatModel, ChatResult, ChatStream};
