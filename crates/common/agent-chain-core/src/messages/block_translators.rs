//! Block translators for converting provider-specific content to standard format.
//!
//! This module provides translators that convert content blocks from various
//! AI providers (Anthropic, OpenAI, etc.) to a standardized LangChain format.
//!
//! This corresponds to `langchain_core/messages/block_translators/` in Python.

pub mod anthropic;
pub mod bedrock;
pub mod bedrock_converse;
pub mod google_genai;
pub mod google_vertexai;
pub mod groq;
pub mod langchain_v0;
pub mod openai;

use serde_json::Value;

/// A function type for translating content blocks.
pub type TranslatorFn = fn(&[Value], bool) -> Vec<Value>;

/// Get the translator function for a provider.
///
/// Returns None if no translator is registered for the provider,
/// in which case best-effort parsing in `BaseMessage` will be used.
pub fn get_translator(provider: &str) -> Option<TranslatorFn> {
    match provider {
        "anthropic" => Some(anthropic::convert_to_standard_blocks),
        "bedrock" => Some(bedrock::convert_to_standard_blocks),
        "bedrock_converse" => Some(bedrock_converse::convert_to_standard_blocks),
        "google_genai" => Some(google_genai::convert_to_standard_blocks),
        "google_vertexai" => Some(google_vertexai::convert_to_standard_blocks),
        "groq" => Some(groq::convert_to_standard_blocks),
        "openai" => Some(openai::convert_to_standard_blocks),
        _ => None,
    }
}
