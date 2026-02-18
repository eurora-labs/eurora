//! Tests for model_profile module.

use agent_chain_core::{ModelProfile, ModelProfileRegistry};
use std::collections::HashMap;

#[cfg(test)]
mod model_profile_tests {
    use super::*;

    #[test]
    fn test_model_profile_all_fields() {
        let profile = ModelProfile {
            max_input_tokens: Some(128000),
            image_inputs: Some(true),
            image_url_inputs: Some(true),
            pdf_inputs: Some(true),
            audio_inputs: Some(true),
            video_inputs: Some(false),
            image_tool_message: Some(true),
            pdf_tool_message: Some(false),

            max_output_tokens: Some(4096),
            reasoning_output: Some(true),
            image_outputs: Some(false),
            audio_outputs: Some(true),
            video_outputs: Some(false),

            tool_calling: Some(true),
            tool_choice: Some(true),

            structured_output: Some(true),
        };

        assert_eq!(profile.max_input_tokens, Some(128000));
        assert_eq!(profile.image_inputs, Some(true));
        assert_eq!(profile.image_url_inputs, Some(true));
        assert_eq!(profile.pdf_inputs, Some(true));
        assert_eq!(profile.audio_inputs, Some(true));
        assert_eq!(profile.video_inputs, Some(false));
        assert_eq!(profile.image_tool_message, Some(true));
        assert_eq!(profile.pdf_tool_message, Some(false));
        assert_eq!(profile.max_output_tokens, Some(4096));
        assert_eq!(profile.reasoning_output, Some(true));
        assert_eq!(profile.image_outputs, Some(false));
        assert_eq!(profile.audio_outputs, Some(true));
        assert_eq!(profile.video_outputs, Some(false));
        assert_eq!(profile.tool_calling, Some(true));
        assert_eq!(profile.tool_choice, Some(true));
        assert_eq!(profile.structured_output, Some(true));
    }

    #[test]
    fn test_model_profile_partial_input_constraints() {
        let profile = ModelProfile {
            max_input_tokens: Some(32000),
            image_inputs: Some(true),
            audio_inputs: Some(false),
            ..Default::default()
        };

        assert_eq!(profile.max_input_tokens, Some(32000));
        assert_eq!(profile.image_inputs, Some(true));
        assert_eq!(profile.audio_inputs, Some(false));
        assert_eq!(profile.max_output_tokens, None);
        assert_eq!(profile.tool_calling, None);
    }

    #[test]
    fn test_model_profile_partial_output_constraints() {
        let profile = ModelProfile {
            max_output_tokens: Some(8192),
            reasoning_output: Some(true),
            image_outputs: Some(true),
            ..Default::default()
        };

        assert_eq!(profile.max_output_tokens, Some(8192));
        assert_eq!(profile.reasoning_output, Some(true));
        assert_eq!(profile.image_outputs, Some(true));
        assert_eq!(profile.max_input_tokens, None);
    }

    #[test]
    fn test_model_profile_tool_calling_only() {
        let profile = ModelProfile {
            tool_calling: Some(true),
            tool_choice: Some(false),
            ..Default::default()
        };

        assert_eq!(profile.tool_calling, Some(true));
        assert_eq!(profile.tool_choice, Some(false));
        assert_eq!(profile.structured_output, None);
    }

    #[test]
    fn test_model_profile_structured_output_only() {
        let profile = ModelProfile {
            structured_output: Some(true),
            ..Default::default()
        };

        assert_eq!(profile.structured_output, Some(true));
        assert_eq!(profile.tool_calling, None);
    }

    #[test]
    fn test_model_profile_empty() {
        let profile = ModelProfile::default();

        assert_eq!(profile.max_input_tokens, None);
        assert_eq!(profile.tool_calling, None);
        assert_eq!(profile.structured_output, None);
    }

    #[test]
    fn test_model_profile_gpt4_like() {
        let profile = ModelProfile {
            max_input_tokens: Some(128000),
            max_output_tokens: Some(4096),
            image_inputs: Some(true),
            image_url_inputs: Some(true),
            pdf_inputs: Some(false),
            audio_inputs: Some(true),
            video_inputs: Some(false),
            tool_calling: Some(true),
            tool_choice: Some(true),
            structured_output: Some(true),
            reasoning_output: Some(false),
            ..Default::default()
        };

        assert_eq!(profile.max_input_tokens, Some(128000));
        assert_eq!(profile.tool_calling, Some(true));
        assert_eq!(profile.structured_output, Some(true));
    }

    #[test]
    fn test_model_profile_claude_like() {
        let profile = ModelProfile {
            max_input_tokens: Some(200000),
            max_output_tokens: Some(8192),
            image_inputs: Some(true),
            image_url_inputs: Some(true),
            pdf_inputs: Some(true),
            audio_inputs: Some(false),
            video_inputs: Some(false),
            tool_calling: Some(true),
            tool_choice: Some(true),
            structured_output: Some(true),
            reasoning_output: Some(true),
            ..Default::default()
        };

        assert_eq!(profile.max_input_tokens, Some(200000));
        assert_eq!(profile.pdf_inputs, Some(true));
        assert_eq!(profile.reasoning_output, Some(true));
    }

    #[test]
    fn test_model_profile_basic_llm() {
        let profile = ModelProfile {
            max_input_tokens: Some(4096),
            max_output_tokens: Some(2048),
            image_inputs: Some(false),
            audio_inputs: Some(false),
            video_inputs: Some(false),
            tool_calling: Some(false),
            structured_output: Some(false),
            ..Default::default()
        };

        assert_eq!(profile.max_input_tokens, Some(4096));
        assert_eq!(profile.image_inputs, Some(false));
        assert_eq!(profile.tool_calling, Some(false));
    }

    #[test]
    fn test_model_profile_multimodal_output() {
        let profile = ModelProfile {
            max_input_tokens: Some(32000),
            max_output_tokens: Some(4096),
            image_outputs: Some(true),
            audio_outputs: Some(true),
            video_outputs: Some(true),
            ..Default::default()
        };

        assert_eq!(profile.image_outputs, Some(true));
        assert_eq!(profile.audio_outputs, Some(true));
        assert_eq!(profile.video_outputs, Some(true));
    }
}

#[cfg(test)]
mod test_model_profile_registry {
    use super::*;

    #[test]
    fn test_model_profile_registry_empty() {
        let registry: ModelProfileRegistry = HashMap::new();
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_model_profile_registry_single_model() {
        let mut registry: ModelProfileRegistry = HashMap::new();
        registry.insert(
            "gpt-4".to_string(),
            ModelProfile {
                max_input_tokens: Some(128000),
                max_output_tokens: Some(4096),
                tool_calling: Some(true),
                ..Default::default()
            },
        );

        assert!(registry.contains_key("gpt-4"));
        assert_eq!(
            registry.get("gpt-4").unwrap().max_input_tokens,
            Some(128000)
        );
        assert_eq!(registry.get("gpt-4").unwrap().tool_calling, Some(true));
    }

    #[test]
    fn test_model_profile_registry_multiple_models() {
        let mut registry: ModelProfileRegistry = HashMap::new();

        registry.insert(
            "gpt-4".to_string(),
            ModelProfile {
                max_input_tokens: Some(128000),
                max_output_tokens: Some(4096),
                tool_calling: Some(true),
                ..Default::default()
            },
        );

        registry.insert(
            "gpt-3.5-turbo".to_string(),
            ModelProfile {
                max_input_tokens: Some(16385),
                max_output_tokens: Some(4096),
                tool_calling: Some(true),
                ..Default::default()
            },
        );

        registry.insert(
            "claude-3-opus".to_string(),
            ModelProfile {
                max_input_tokens: Some(200000),
                max_output_tokens: Some(4096),
                tool_calling: Some(true),
                pdf_inputs: Some(true),
                ..Default::default()
            },
        );

        assert_eq!(registry.len(), 3);
        assert_eq!(
            registry.get("gpt-4").unwrap().max_input_tokens,
            Some(128000)
        );
        assert_eq!(
            registry.get("gpt-3.5-turbo").unwrap().max_input_tokens,
            Some(16385)
        );
        assert_eq!(
            registry.get("claude-3-opus").unwrap().pdf_inputs,
            Some(true)
        );
    }

    #[test]
    fn test_model_profile_registry_lookup() {
        let mut registry: ModelProfileRegistry = HashMap::new();

        registry.insert(
            "model-a".to_string(),
            ModelProfile {
                max_input_tokens: Some(1000),
                ..Default::default()
            },
        );

        registry.insert(
            "model-b".to_string(),
            ModelProfile {
                max_input_tokens: Some(2000),
                ..Default::default()
            },
        );

        let profile = registry.get("model-a");
        assert!(profile.is_some());
        assert_eq!(profile.unwrap().max_input_tokens, Some(1000));

        let missing = registry.get("model-c");
        assert!(missing.is_none());

        let default_profile = ModelProfile {
            max_input_tokens: Some(0),
            ..Default::default()
        };
        let result = registry.get("model-c").unwrap_or(&default_profile);
        assert_eq!(result.max_input_tokens, Some(0));
    }

    #[test]
    fn test_model_profile_registry_iteration() {
        let mut registry: ModelProfileRegistry = HashMap::new();

        registry.insert(
            "model-1".to_string(),
            ModelProfile {
                max_input_tokens: Some(1000),
                ..Default::default()
            },
        );

        registry.insert(
            "model-2".to_string(),
            ModelProfile {
                max_input_tokens: Some(2000),
                ..Default::default()
            },
        );

        registry.insert(
            "model-3".to_string(),
            ModelProfile {
                max_input_tokens: Some(3000),
                ..Default::default()
            },
        );

        let model_names: Vec<_> = registry.keys().collect();
        assert_eq!(model_names.len(), 3);
        assert!(model_names.contains(&&"model-1".to_string()));
        assert!(model_names.contains(&&"model-2".to_string()));
        assert!(model_names.contains(&&"model-3".to_string()));

        let profiles: Vec<_> = registry.values().collect();
        assert_eq!(profiles.len(), 3);

        let items: Vec<_> = registry.iter().collect();
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_model_profile_registry_update() {
        let mut registry: ModelProfileRegistry = HashMap::new();

        registry.insert(
            "model-a".to_string(),
            ModelProfile {
                max_input_tokens: Some(1000),
                ..Default::default()
            },
        );

        registry.insert(
            "model-b".to_string(),
            ModelProfile {
                max_input_tokens: Some(2000),
                tool_calling: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(registry.len(), 2);
        assert_eq!(registry.get("model-b").unwrap().tool_calling, Some(true));

        registry.insert(
            "model-a".to_string(),
            ModelProfile {
                max_input_tokens: Some(1500),
                image_inputs: Some(true),
                ..Default::default()
            },
        );
        assert_eq!(
            registry.get("model-a").unwrap().max_input_tokens,
            Some(1500)
        );
        assert_eq!(registry.get("model-a").unwrap().image_inputs, Some(true));
    }

    #[test]
    fn test_model_profile_registry_delete() {
        let mut registry: ModelProfileRegistry = HashMap::new();

        registry.insert(
            "model-a".to_string(),
            ModelProfile {
                max_input_tokens: Some(1000),
                ..Default::default()
            },
        );

        registry.insert(
            "model-b".to_string(),
            ModelProfile {
                max_input_tokens: Some(2000),
                ..Default::default()
            },
        );

        registry.remove("model-a");
        assert_eq!(registry.len(), 1);
        assert!(!registry.contains_key("model-a"));
        assert!(registry.contains_key("model-b"));
    }

    #[test]
    fn test_model_profile_registry_with_version_suffixes() {
        let mut registry: ModelProfileRegistry = HashMap::new();

        registry.insert(
            "gpt-4-0613".to_string(),
            ModelProfile {
                max_input_tokens: Some(8192),
                ..Default::default()
            },
        );

        registry.insert(
            "gpt-4-1106-preview".to_string(),
            ModelProfile {
                max_input_tokens: Some(128000),
                ..Default::default()
            },
        );

        registry.insert(
            "gpt-4-turbo-2024-04-09".to_string(),
            ModelProfile {
                max_input_tokens: Some(128000),
                ..Default::default()
            },
        );

        assert_eq!(
            registry.get("gpt-4-0613").unwrap().max_input_tokens,
            Some(8192)
        );
        assert_eq!(
            registry.get("gpt-4-1106-preview").unwrap().max_input_tokens,
            Some(128000)
        );
        assert_eq!(
            registry
                .get("gpt-4-turbo-2024-04-09")
                .unwrap()
                .max_input_tokens,
            Some(128000)
        );
    }

    #[test]
    fn test_model_profile_registry_provider_namespaced() {
        let mut registry: ModelProfileRegistry = HashMap::new();

        registry.insert(
            "openai/gpt-4".to_string(),
            ModelProfile {
                max_input_tokens: Some(128000),
                tool_calling: Some(true),
                ..Default::default()
            },
        );

        registry.insert(
            "anthropic/claude-3-opus".to_string(),
            ModelProfile {
                max_input_tokens: Some(200000),
                tool_calling: Some(true),
                ..Default::default()
            },
        );

        registry.insert(
            "google/gemini-pro".to_string(),
            ModelProfile {
                max_input_tokens: Some(32000),
                tool_calling: Some(true),
                ..Default::default()
            },
        );

        assert!(registry.contains_key("openai/gpt-4"));
        assert!(registry.contains_key("anthropic/claude-3-opus"));
        assert!(registry.contains_key("google/gemini-pro"));
    }
}
