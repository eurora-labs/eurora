//! Block translators for converting provider-specific content to standard format.
//!
//! This module provides translators that convert content blocks from various
//! AI providers (Anthropic, OpenAI, etc.) to a standardized LangChain format.
//!
//! This corresponds to `langchain_core/messages/block_translators/` in Python.

pub mod anthropic;
pub mod langchain_v0;
pub mod openai;

use serde_json::Value;

/// A function type for translating content blocks.
pub type TranslatorFn = fn(&[Value], bool) -> Vec<Value>;

/// Registry for block translators.
/// Currently we use a simple match-based approach rather than a dynamic registry.
pub fn get_translator(provider: &str) -> Option<TranslatorFn> {
    match provider {
        "anthropic" => Some(anthropic::convert_to_standard_blocks),
        "openai" => Some(openai::convert_to_standard_blocks),
        _ => None,
    }
}
