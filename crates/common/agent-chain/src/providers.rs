
//! Provider implementations for different LLM services.
//!
//! This module contains provider-specific implementations of the `ChatModel` trait.
//! Each provider is implemented in its own submodule.

mod anthropic;
mod ollama;
mod openai;

pub use anthropic::ChatAnthropic;
pub use ollama::{BoundChatOllama, ChatOllama, MessageWithAny, OllamaFormat};
pub use openai::{BuiltinTool, ChatOpenAI, ContentBlock, TextAnnotation};

use crate::error::{Error, Result};

/// Supported provider names.
pub const SUPPORTED_PROVIDERS: &[&str] = &[
    "anthropic",
    "openai",
    // "azure_openai",
    // "google_vertexai",
    // "google_genai",
    // "cohere",
    // "fireworks",
    "ollama",
    // "mistralai",
    // "groq",
    // "together",
    // "bedrock",
    // "deepseek",
    // "xai",
    // "perplexity",
];

/// Attempt to infer the provider from a model name.
///
/// # Arguments
///
/// * `model` - The model name/identifier.
///
/// # Returns
///
/// The inferred provider name, or `None` if inference failed.
///
/// # Examples
///
/// ```
/// use agent_chain::providers::infer_provider;
///
/// assert_eq!(infer_provider("gpt-4"), Some("openai"));
/// assert_eq!(infer_provider("claude-3-opus"), Some("anthropic"));
//  assert_eq!(infer_provider("gemini-pro"), Some("google_vertexai"));
/// assert_eq!(infer_provider("unknown-model"), None);
/// ```
pub fn infer_provider(model: &str) -> Option<&'static str> {
    // OpenAI models
    if model.starts_with("gpt-") || model.starts_with("o1") || model.starts_with("o3") {
        return Some("openai");
    }

    // Anthropic models
    if model.starts_with("claude") {
        return Some("anthropic");
    }

    // Google models
    // if model.starts_with("gemini") {
    //     return Some("google_vertexai");
    // }

    // Cohere models
    // if model.starts_with("command") {
    //     return Some("cohere");
    // }

    // Fireworks models
    // if model.starts_with("accounts/fireworks") {
    //     return Some("fireworks");
    // }

    // AWS Bedrock models
    // if model.starts_with("amazon.") {
    //     return Some("bedrock");
    // }

    // Mistral models
    // if model.starts_with("mistral") {
    //     return Some("mistralai");
    // }

    // DeepSeek models
    // if model.starts_with("deepseek") {
    //     return Some("deepseek");
    // }

    // xAI models
    // if model.starts_with("grok") {
    //     return Some("xai");
    // }

    // Perplexity models
    // if model.starts_with("sonar") {
    //     return Some("perplexity");
    // }

    None
}

/// Parse a model string that may contain a provider prefix.
///
/// Supports formats like "openai:gpt-4" or just "gpt-4".
///
/// # Arguments
///
/// * `model` - The model string, optionally with provider prefix.
/// * `model_provider` - Optional explicit provider override.
///
/// # Returns
///
/// A tuple of (model_name, provider_name).
pub fn parse_model(model: &str, model_provider: Option<&str>) -> Result<(String, String)> {
    let (model, provider) = if model_provider.is_none() && model.contains(':') {
        let parts: Vec<&str> = model.splitn(2, ':').collect();
        let prefix = parts[0];
        let suffix = parts[1];

        // Check if prefix is a known provider
        if SUPPORTED_PROVIDERS.contains(&prefix) {
            (suffix.to_string(), prefix.to_string())
        } else if let Some(inferred) = infer_provider(prefix) {
            (suffix.to_string(), inferred.to_string())
        } else {
            // Not a provider prefix, use the whole string as model
            let inferred =
                infer_provider(model).ok_or_else(|| Error::unable_to_infer_provider(model))?;
            (model.to_string(), inferred.to_string())
        }
    } else if let Some(provider) = model_provider {
        (model.to_string(), provider.to_string())
    } else {
        let inferred =
            infer_provider(model).ok_or_else(|| Error::unable_to_infer_provider(model))?;
        (model.to_string(), inferred.to_string())
    };

    Ok((model, provider.replace('-', "_").to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_provider() {
        assert_eq!(infer_provider("gpt-4"), Some("openai"));
        assert_eq!(infer_provider("gpt-4o"), Some("openai"));
        assert_eq!(infer_provider("o1-preview"), Some("openai"));
        assert_eq!(infer_provider("o3-mini"), Some("openai"));
        assert_eq!(infer_provider("claude-3-opus"), Some("anthropic"));
        assert_eq!(infer_provider("claude-sonnet-4-5"), Some("anthropic"));
        // assert_eq!(infer_provider("gemini-pro"), Some("google_vertexai"));
        // assert_eq!(infer_provider("command-r"), Some("cohere"));
        // assert_eq!(infer_provider("mistral-large"), Some("mistralai"));
        // assert_eq!(infer_provider("deepseek-chat"), Some("deepseek"));
        // assert_eq!(infer_provider("grok-1"), Some("xai"));
        // assert_eq!(infer_provider("sonar-small"), Some("perplexity"));
        assert_eq!(infer_provider("unknown"), None);
    }

    #[test]
    fn test_parse_model_with_prefix() {
        let (model, provider) = parse_model("openai:gpt-4", None).unwrap();
        assert_eq!(model, "gpt-4");
        assert_eq!(provider, "openai");
    }

    #[test]
    fn test_parse_model_without_prefix() {
        let (model, provider) = parse_model("claude-3-opus", None).unwrap();
        assert_eq!(model, "claude-3-opus");
        assert_eq!(provider, "anthropic");
    }

    #[test]
    fn test_parse_model_with_explicit_provider() {
        let (model, provider) = parse_model("my-model", Some("openai")).unwrap();
        assert_eq!(model, "my-model");
        assert_eq!(provider, "openai");
    }
}
