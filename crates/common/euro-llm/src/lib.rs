//! Core traits and types for the LLM library ecosystem.
//!
//! This crate provides the foundational abstractions that all LLM providers
//! implement, including traits for chat, completion, streaming, and tool calling,
//! as well as standardized request/response types and error handling.
pub mod core;

#[cfg(feature = "ollama")]
pub mod ollama;

#[cfg(feature = "openai")]
pub mod openai;

#[cfg(feature = "anthropic")]
pub mod anthropic;
