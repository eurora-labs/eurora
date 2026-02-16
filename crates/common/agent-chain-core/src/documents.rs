//! Documents module for data retrieval and processing workflows.
//!
//! This module provides core abstractions for handling data in retrieval-augmented
//! generation (RAG) pipelines, vector stores, and document processing workflows.
//!
//! # Documents vs. message content
//!
//! This module is distinct from the `messages::content` module, which provides
//! multimodal content blocks for **LLM chat I/O** (text, images, audio, etc. within
//! messages).
//!
//! **Key distinction:**
//!
//! - **Documents** (this module): For **data retrieval and processing workflows**
//!     - Vector stores, retrievers, RAG pipelines
//!     - Text chunking, embedding, and semantic search
//!     - Example: Chunks of a PDF stored in a vector database
//!
//! - **Content Blocks** (`messages::content`): For **LLM conversational I/O**
//!     - Multimodal message content sent to/from models
//!     - Tool calls, reasoning, citations within chat
//!     - Example: An image sent to a vision model in a chat message
//!
//! While both can represent similar data types (text, files), they serve different
//! architectural purposes in agent-chain applications.
//!
//! # Example
//!
//! ```
//! use agent_chain_core::documents::{Document, Blob};
//! use std::collections::HashMap;
//!
//! // Create a simple document
//! let document = Document::new("Hello, world!")
//!     .with_metadata(HashMap::from([
//!         ("source".to_string(), serde_json::json!("https://example.com"))
//!     ]));
//!
//! // Create a blob from in-memory data
//! let blob = Blob::from_data("Some raw data");
//! ```

pub mod base;
pub mod compressor;
pub mod transformers;

// Re-export base types
pub use base::{BaseMedia, Blob, BlobBuilder, BlobData, Document};

// Re-export compressor types
pub use compressor::BaseDocumentCompressor;

// Re-export transformer types
pub use transformers::BaseDocumentTransformer;
