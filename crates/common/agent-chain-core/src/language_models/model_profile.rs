use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_input_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_inputs: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url_inputs: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_inputs: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_inputs: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_inputs: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tool_message: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_tool_message: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_output: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_outputs: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_outputs: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_outputs: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calling: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_output: Option<bool>,
}

impl ModelProfile {
    pub fn new() -> Self {
        Self::default()
    }
}

pub type ModelProfileRegistry = HashMap<String, ModelProfile>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_profile_creation() {
        let profile = ModelProfile::new();
        assert_eq!(profile.max_input_tokens, None);
        assert_eq!(profile.image_inputs, None);
        assert_eq!(profile.tool_calling, None);
        assert_eq!(profile.structured_output, None);

        let profile = ModelProfile {
            max_input_tokens: Some(100000),
            max_output_tokens: Some(4096),
            image_inputs: Some(true),
            tool_calling: Some(true),
            structured_output: Some(true),
            ..Default::default()
        };

        assert_eq!(profile.max_input_tokens, Some(100000));
        assert_eq!(profile.max_output_tokens, Some(4096));
        assert_eq!(profile.image_inputs, Some(true));
        assert_eq!(profile.tool_calling, Some(true));
        assert_eq!(profile.structured_output, Some(true));
        assert_eq!(profile.audio_inputs, None);
    }

    #[test]
    fn test_model_profile_serialization() {
        let profile = ModelProfile {
            max_input_tokens: Some(100000),
            tool_calling: Some(true),
            ..Default::default()
        };

        let json = serde_json::to_string(&profile).expect("serialization should succeed");
        let deserialized: ModelProfile =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.max_input_tokens, Some(100000));
        assert_eq!(deserialized.tool_calling, Some(true));
        assert_eq!(deserialized.image_inputs, None);
    }

    #[test]
    fn test_model_profile_skips_none_in_json() {
        let profile = ModelProfile {
            max_input_tokens: Some(100000),
            ..Default::default()
        };

        let json = serde_json::to_string(&profile).expect("serialization should succeed");
        assert!(json.contains("max_input_tokens"));
        assert!(!json.contains("image_inputs"));
    }

    #[test]
    fn test_model_profile_registry() {
        let mut registry: ModelProfileRegistry = HashMap::new();
        registry.insert(
            "test-model".to_string(),
            ModelProfile {
                max_input_tokens: Some(128000),
                tool_calling: Some(true),
                ..Default::default()
            },
        );

        assert!(registry.contains_key("test-model"));
        let profile = registry.get("test-model").expect("profile should exist");
        assert_eq!(profile.max_input_tokens, Some(128000));
    }
}
