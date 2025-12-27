//! Model profile types and utilities.
//!
//! This module provides information about chat model capabilities,
//! such as context window sizes and supported features.
//! Mirrors `langchain_core.language_models.model_profile`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Model profile providing information about chat model capabilities.
///
/// This is a beta feature and the format may be subject to change.
///
/// Provides information about:
/// - Input constraints (context window, supported modalities)
/// - Output constraints (max output tokens, supported outputs)
/// - Tool calling capabilities
/// - Structured output support
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelProfile {
    // --- Input constraints ---
    /// Maximum context window (tokens).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_input_tokens: Option<u32>,

    /// Whether image inputs are supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_inputs: Option<bool>,

    /// Whether image URL inputs are supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url_inputs: Option<bool>,

    /// Whether PDF inputs are supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_inputs: Option<bool>,

    /// Whether audio inputs are supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_inputs: Option<bool>,

    /// Whether video inputs are supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_inputs: Option<bool>,

    /// Whether images can be included in tool messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tool_message: Option<bool>,

    /// Whether PDFs can be included in tool messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_tool_message: Option<bool>,

    // --- Output constraints ---
    /// Maximum output tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    /// Whether the model supports reasoning / chain-of-thought.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_output: Option<bool>,

    /// Whether image outputs are supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_outputs: Option<bool>,

    /// Whether audio outputs are supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_outputs: Option<bool>,

    /// Whether video outputs are supported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_outputs: Option<bool>,

    // --- Tool calling ---
    /// Whether the model supports tool calling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calling: Option<bool>,

    /// Whether the model supports tool choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<bool>,

    // --- Structured output ---
    /// Whether the model supports a native structured output feature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<bool>,
}

impl ModelProfile {
    /// Create a new empty model profile.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum input tokens.
    pub fn with_max_input_tokens(mut self, tokens: u32) -> Self {
        self.max_input_tokens = Some(tokens);
        self
    }

    /// Set the maximum output tokens.
    pub fn with_max_output_tokens(mut self, tokens: u32) -> Self {
        self.max_output_tokens = Some(tokens);
        self
    }

    /// Set whether image inputs are supported.
    pub fn with_image_inputs(mut self, supported: bool) -> Self {
        self.image_inputs = Some(supported);
        self
    }

    /// Set whether image URL inputs are supported.
    pub fn with_image_url_inputs(mut self, supported: bool) -> Self {
        self.image_url_inputs = Some(supported);
        self
    }

    /// Set whether PDF inputs are supported.
    pub fn with_pdf_inputs(mut self, supported: bool) -> Self {
        self.pdf_inputs = Some(supported);
        self
    }

    /// Set whether audio inputs are supported.
    pub fn with_audio_inputs(mut self, supported: bool) -> Self {
        self.audio_inputs = Some(supported);
        self
    }

    /// Set whether video inputs are supported.
    pub fn with_video_inputs(mut self, supported: bool) -> Self {
        self.video_inputs = Some(supported);
        self
    }

    /// Set whether images can be included in tool messages.
    pub fn with_image_tool_message(mut self, supported: bool) -> Self {
        self.image_tool_message = Some(supported);
        self
    }

    /// Set whether PDFs can be included in tool messages.
    pub fn with_pdf_tool_message(mut self, supported: bool) -> Self {
        self.pdf_tool_message = Some(supported);
        self
    }

    /// Set whether reasoning output is supported.
    pub fn with_reasoning_output(mut self, supported: bool) -> Self {
        self.reasoning_output = Some(supported);
        self
    }

    /// Set whether image outputs are supported.
    pub fn with_image_outputs(mut self, supported: bool) -> Self {
        self.image_outputs = Some(supported);
        self
    }

    /// Set whether audio outputs are supported.
    pub fn with_audio_outputs(mut self, supported: bool) -> Self {
        self.audio_outputs = Some(supported);
        self
    }

    /// Set whether video outputs are supported.
    pub fn with_video_outputs(mut self, supported: bool) -> Self {
        self.video_outputs = Some(supported);
        self
    }

    /// Set whether tool calling is supported.
    pub fn with_tool_calling(mut self, supported: bool) -> Self {
        self.tool_calling = Some(supported);
        self
    }

    /// Set whether tool choice is supported.
    pub fn with_tool_choice(mut self, supported: bool) -> Self {
        self.tool_choice = Some(supported);
        self
    }

    /// Set whether structured output is supported.
    pub fn with_structured_output(mut self, supported: bool) -> Self {
        self.structured_output = Some(supported);
        self
    }

    /// Check if the model supports multimodal inputs.
    pub fn supports_multimodal_inputs(&self) -> bool {
        self.image_inputs.unwrap_or(false)
            || self.audio_inputs.unwrap_or(false)
            || self.video_inputs.unwrap_or(false)
            || self.pdf_inputs.unwrap_or(false)
    }

    /// Check if the model supports multimodal outputs.
    pub fn supports_multimodal_outputs(&self) -> bool {
        self.image_outputs.unwrap_or(false)
            || self.audio_outputs.unwrap_or(false)
            || self.video_outputs.unwrap_or(false)
    }

    /// Check if the model supports tool calling.
    pub fn supports_tool_calling(&self) -> bool {
        self.tool_calling.unwrap_or(false)
    }

    /// Check if the model supports structured output.
    pub fn supports_structured_output(&self) -> bool {
        self.structured_output.unwrap_or(false)
    }
}

/// Registry mapping model identifiers or names to their ModelProfile.
pub type ModelProfileRegistry = HashMap<String, ModelProfile>;

/// Create a new empty model profile registry.
#[allow(dead_code)]
pub fn new_registry() -> ModelProfileRegistry {
    HashMap::new()
}

/// Create a registry with some common model profiles.
#[allow(dead_code)]
pub fn default_registry() -> ModelProfileRegistry {
    let mut registry = HashMap::new();

    // GPT-4 Turbo
    registry.insert(
        "gpt-4-turbo".to_string(),
        ModelProfile::new()
            .with_max_input_tokens(128000)
            .with_max_output_tokens(4096)
            .with_image_inputs(true)
            .with_image_url_inputs(true)
            .with_tool_calling(true)
            .with_tool_choice(true)
            .with_structured_output(true),
    );

    // GPT-4o
    registry.insert(
        "gpt-4o".to_string(),
        ModelProfile::new()
            .with_max_input_tokens(128000)
            .with_max_output_tokens(16384)
            .with_image_inputs(true)
            .with_image_url_inputs(true)
            .with_audio_inputs(true)
            .with_audio_outputs(true)
            .with_tool_calling(true)
            .with_tool_choice(true)
            .with_structured_output(true),
    );

    // Claude 3.5 Sonnet
    registry.insert(
        "claude-3-5-sonnet-20241022".to_string(),
        ModelProfile::new()
            .with_max_input_tokens(200000)
            .with_max_output_tokens(8192)
            .with_image_inputs(true)
            .with_pdf_inputs(true)
            .with_tool_calling(true)
            .with_tool_choice(true)
            .with_structured_output(true),
    );

    // Claude 3 Opus
    registry.insert(
        "claude-3-opus-20240229".to_string(),
        ModelProfile::new()
            .with_max_input_tokens(200000)
            .with_max_output_tokens(4096)
            .with_image_inputs(true)
            .with_tool_calling(true)
            .with_tool_choice(true),
    );

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_profile_builder() {
        let profile = ModelProfile::new()
            .with_max_input_tokens(100000)
            .with_max_output_tokens(4096)
            .with_image_inputs(true)
            .with_tool_calling(true)
            .with_structured_output(true);

        assert_eq!(profile.max_input_tokens, Some(100000));
        assert_eq!(profile.max_output_tokens, Some(4096));
        assert_eq!(profile.image_inputs, Some(true));
        assert_eq!(profile.tool_calling, Some(true));
        assert_eq!(profile.structured_output, Some(true));
    }

    #[test]
    fn test_supports_multimodal_inputs() {
        let profile = ModelProfile::new().with_image_inputs(true);
        assert!(profile.supports_multimodal_inputs());

        let profile = ModelProfile::new().with_audio_inputs(true);
        assert!(profile.supports_multimodal_inputs());

        let profile = ModelProfile::new();
        assert!(!profile.supports_multimodal_inputs());
    }

    #[test]
    fn test_supports_multimodal_outputs() {
        let profile = ModelProfile::new().with_image_outputs(true);
        assert!(profile.supports_multimodal_outputs());

        let profile = ModelProfile::new();
        assert!(!profile.supports_multimodal_outputs());
    }

    #[test]
    fn test_default_registry() {
        let registry = default_registry();

        assert!(registry.contains_key("gpt-4-turbo"));
        assert!(registry.contains_key("gpt-4o"));
        assert!(registry.contains_key("claude-3-5-sonnet-20241022"));

        let gpt4o = registry.get("gpt-4o").unwrap();
        assert!(gpt4o.supports_multimodal_inputs());
        assert!(gpt4o.supports_tool_calling());
    }

    #[test]
    fn test_model_profile_serialization() {
        let profile = ModelProfile::new()
            .with_max_input_tokens(100000)
            .with_tool_calling(true);

        let json = serde_json::to_string(&profile).unwrap();
        let deserialized: ModelProfile = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.max_input_tokens, Some(100000));
        assert_eq!(deserialized.tool_calling, Some(true));
    }
}
