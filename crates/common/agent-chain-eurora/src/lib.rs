//! Eurora gRPC provider for agent-chain.
//!
//! This crate provides a gRPC-based implementation of the `ChatModel` trait
//! from agent-chain, allowing you to use Eurora's chat service with the
//! LangChain-compatible agent-chain interface.
//!
//! # Example
//!
//! ```ignore
//! use agent_chain_eurora::{ChatEurora, EuroraConfig};
//! use agent_chain::{ChatModel, HumanMessage};
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
//! use agent_chain_eurora::{ChatEurora, EuroraConfig};
//! use agent_chain::{ChatModel, HumanMessage};
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
    pub mod chat {
        tonic::include_proto!("eurora.chat");
    }
}

pub mod config;
pub mod error;
pub mod provider;
pub mod types;

// Re-export main types for convenience
pub use config::EuroraConfig;
pub use error::EuroraError;
pub use provider::ChatEurora;

// Re-export agent-chain types that users will need
pub use agent_chain::{ChatModel, ChatResult, ChatStream};

#[cfg(test)]
mod tests {
    use url::Url;

    use super::*;

    #[test]
    fn test_config_creation() {
        let config = EuroraConfig::new(Url::parse("http://localhost:50051").unwrap());
        assert_eq!(config.endpoint.to_string(), "http://localhost:50051/");
        assert!(!config.use_tls);
    }

    #[test]
    fn test_config_with_tls() {
        let config = EuroraConfig::new(Url::parse("https://api.example.com").unwrap())
            .with_tls(Some("api.example.com".to_string()))
            .with_auth_token("test-token".to_string());

        assert!(config.use_tls);
        assert_eq!(config.tls_domain, Some("api.example.com".to_string()));
        assert_eq!(config.auth_token, Some("test-token".to_string()));
    }

    #[tokio::test]
    async fn test_chat_provider_creation_fails_with_invalid_endpoint() {
        let config = EuroraConfig::new(Url::parse("http://invalid-endpoint:8080").unwrap());

        // This should fail because the endpoint is not reachable
        let result = ChatEurora::new(config).await;
        assert!(result.is_err());
    }
}
