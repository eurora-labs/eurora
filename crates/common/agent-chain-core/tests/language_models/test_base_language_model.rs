use std::collections::HashMap;

use agent_chain_core::language_models::{
    BaseLanguageModel, FakeListLLM, LangSmithParams, LanguageModelConfig, get_token_ids_default,
};
use agent_chain_core::messages::{AIMessage, AnyMessage, HumanMessage};

#[cfg(test)]
mod test_lang_smith_params {
    use super::*;

    #[test]
    fn test_langsmith_params_all_fields() {
        let params = LangSmithParams::builder()
            .provider("openai")
            .model_name("gpt-4")
            .model_type("chat")
            .temperature(0.7)
            .max_tokens(1000)
            .stop(vec!["stop1".to_string(), "stop2".to_string()])
            .build();

        assert_eq!(params.ls_provider, Some("openai".to_string()));
        assert_eq!(params.ls_model_name, Some("gpt-4".to_string()));
        assert_eq!(params.ls_model_type, Some("chat".to_string()));
        assert_eq!(params.ls_temperature, Some(0.7));
        assert_eq!(params.ls_max_tokens, Some(1000));
        assert_eq!(
            params.ls_stop,
            Some(vec!["stop1".to_string(), "stop2".to_string()])
        );
    }

    #[test]
    fn test_langsmith_params_partial() {
        let params = LangSmithParams::builder()
            .provider("anthropic")
            .model_type("chat")
            .build();

        assert_eq!(params.ls_provider, Some("anthropic".to_string()));
        assert_eq!(params.ls_model_type, Some("chat".to_string()));
        assert_eq!(params.ls_model_name, None);
    }

    #[test]
    fn test_langsmith_params_empty() {
        let params = LangSmithParams::default();

        assert_eq!(params.ls_provider, None);
        assert_eq!(params.ls_model_name, None);
        assert_eq!(params.ls_model_type, None);
        assert_eq!(params.ls_temperature, None);
        assert_eq!(params.ls_max_tokens, None);
        assert_eq!(params.ls_stop, None);
    }

    #[test]
    fn test_langsmith_params_model_type_values() {
        let chat_params = LangSmithParams::builder().model_type("chat").build();
        let llm_params = LangSmithParams::builder().model_type("llm").build();

        assert_eq!(chat_params.ls_model_type, Some("chat".to_string()));
        assert_eq!(llm_params.ls_model_type, Some("llm".to_string()));
    }

    #[test]
    fn test_langsmith_params_builder_pattern() {
        let params = LangSmithParams::builder()
            .provider("openai")
            .model_name("gpt-4")
            .temperature(0.5)
            .build();

        assert_eq!(params.ls_provider, Some("openai".to_string()));
        assert_eq!(params.ls_model_name, Some("gpt-4".to_string()));
        assert_eq!(params.ls_temperature, Some(0.5));
    }

    #[test]
    fn test_langsmith_params_serialization() {
        let params = LangSmithParams::builder()
            .provider("openai")
            .model_name("gpt-4")
            .build();

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("openai"));
        assert!(json.contains("gpt-4"));

        assert!(!json.contains("ls_temperature"));
    }

    #[test]
    fn test_langsmith_params_deserialization() {
        let json = r#"{"ls_provider":"anthropic","ls_model_name":"claude-3"}"#;
        let params: LangSmithParams = serde_json::from_str(json).unwrap();

        assert_eq!(params.ls_provider, Some("anthropic".to_string()));
        assert_eq!(params.ls_model_name, Some("claude-3".to_string()));
        assert_eq!(params.ls_model_type, None);
    }
}

#[cfg(test)]
mod test_get_token_ids_default_method {
    use super::*;

    #[test]
    fn test_get_token_ids_default_method() {
        let text = "hello world test";
        let token_ids = get_token_ids_default(text);

        assert_eq!(token_ids.len(), 3); // 3 words
        assert_eq!(token_ids, vec![0, 1, 2]);
    }

    #[test]
    fn test_get_token_ids_empty_string() {
        let token_ids = get_token_ids_default("");
        assert!(token_ids.is_empty());
    }

    #[test]
    fn test_get_token_ids_single_word() {
        let token_ids = get_token_ids_default("hello");
        assert_eq!(token_ids.len(), 1);
        assert_eq!(token_ids, vec![0]);
    }

    #[test]
    fn test_get_token_ids_multiple_spaces() {
        let token_ids = get_token_ids_default("hello    world");
        assert_eq!(token_ids.len(), 2); // split_whitespace handles multiple spaces
    }
}

#[cfg(test)]
mod test_language_model_config {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = LanguageModelConfig::default();

        assert_eq!(config.cache, None);
        assert_eq!(config.tags, None);
        assert_eq!(config.metadata, None);
    }

    #[test]
    fn test_config_with_cache_true() {
        let config = LanguageModelConfig::builder().cache(true).build();
        assert_eq!(config.cache, Some(true));
    }

    #[test]
    fn test_config_with_cache_false() {
        let config = LanguageModelConfig::builder().cache(false).build();
        assert_eq!(config.cache, Some(false));
    }

    #[test]
    fn test_config_with_tags() {
        let config = LanguageModelConfig::builder()
            .tags(vec!["tag1".to_string(), "tag2".to_string()])
            .build();
        assert_eq!(
            config.tags,
            Some(vec!["tag1".to_string(), "tag2".to_string()])
        );
    }

    #[test]
    fn test_config_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));

        let config = LanguageModelConfig::builder()
            .metadata(metadata.clone())
            .build();
        assert_eq!(config.metadata, Some(metadata));
    }

    #[test]
    fn test_config_builder_chain() {
        let config = LanguageModelConfig::builder()
            .cache(true)
            .tags(vec!["test".to_string()])
            .build();

        assert_eq!(config.cache, Some(true));
        assert_eq!(config.tags, Some(vec!["test".to_string()]));
    }

    #[test]
    fn test_config_serialization() {
        let config = LanguageModelConfig::builder().cache(true).build();

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("cache"));
    }
}

#[cfg(test)]
mod test_ai_message_output {
    use super::*;

    #[test]
    fn test_ai_message_text() {
        let message = AIMessage::builder().content("test message").build();
        assert_eq!(message.text(), "test message");
    }

    #[test]
    fn test_ai_message_content() {
        let message = AIMessage::builder().content("direct message").build();
        assert_eq!(message.text(), "direct message");
    }
}

#[cfg(test)]
mod test_language_model_config_serialization {
    use super::*;

    #[test]
    fn test_cache_serialization() {
        let config = LanguageModelConfig::builder().cache(true).build();
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("cache"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_tags_excluded_when_none() {
        let config = LanguageModelConfig::builder().build();
        let json = serde_json::to_string(&config).unwrap();

        assert!(!json.contains("tags"));
    }

    #[test]
    fn test_metadata_excluded_when_none() {
        let config = LanguageModelConfig::builder().build();
        let json = serde_json::to_string(&config).unwrap();

        assert!(!json.contains("metadata"));
    }

    #[test]
    fn test_deserialization_roundtrip() {
        let config = LanguageModelConfig::builder()
            .cache(true)
            .tags(vec!["test".to_string()])
            .build();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: LanguageModelConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.cache, config.cache);
        assert_eq!(deserialized.tags, config.tags);
    }
}

#[cfg(test)]
mod test_langsmith_params_additional {
    use super::*;

    #[test]
    fn test_langsmith_params_clone() {
        let params = LangSmithParams::builder()
            .provider("openai")
            .model_name("gpt-4")
            .build();

        let cloned = params.clone();
        assert_eq!(cloned.ls_provider, params.ls_provider);
        assert_eq!(cloned.ls_model_name, params.ls_model_name);
    }

    #[test]
    fn test_langsmith_params_debug() {
        let params = LangSmithParams::builder().provider("openai").build();
        let debug_str = format!("{:?}", params);
        assert!(debug_str.contains("openai"));
    }

    #[test]
    fn test_langsmith_params_default() {
        let params = LangSmithParams::default();
        assert_eq!(params.ls_provider, None);
        assert_eq!(params.ls_model_name, None);
        assert_eq!(params.ls_model_type, None);
        assert_eq!(params.ls_temperature, None);
        assert_eq!(params.ls_max_tokens, None);
        assert_eq!(params.ls_stop, None);
    }
}

#[cfg(test)]
mod test_custom_get_token_ids {
    use super::*;

    fn simple_char_tokenizer(text: &str) -> Vec<u32> {
        text.chars().map(|c| c as u32).collect()
    }

    fn fixed_tokenizer(_text: &str) -> Vec<u32> {
        vec![1, 2, 3]
    }

    #[test]
    fn test_custom_get_token_ids_field() {
        let config = LanguageModelConfig::builder()
            .custom_get_token_ids(simple_char_tokenizer)
            .build();

        assert!(config.custom_get_token_ids.is_some());
    }

    #[test]
    fn test_custom_get_token_ids_none_by_default() {
        let config = LanguageModelConfig::default();

        assert!(config.custom_get_token_ids.is_none());
    }

    #[test]
    fn test_custom_tokenizer_function_execution() {
        let config = LanguageModelConfig::builder()
            .custom_get_token_ids(fixed_tokenizer)
            .build();

        let tokenizer = config.custom_get_token_ids.unwrap();
        let result = tokenizer("any text");
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_custom_tokenizer_with_char_encoding() {
        let config = LanguageModelConfig::builder()
            .custom_get_token_ids(simple_char_tokenizer)
            .build();

        let tokenizer = config.custom_get_token_ids.unwrap();
        let result = tokenizer("Hi");

        assert_eq!(result, vec![72, 105]);
    }

    #[test]
    fn test_custom_get_token_ids_excluded_from_serialization() {
        let config = LanguageModelConfig::builder()
            .custom_get_token_ids(simple_char_tokenizer)
            .build();

        let json = serde_json::to_string(&config).unwrap();

        assert!(!json.contains("custom_get_token_ids"));
    }
}

#[cfg(test)]
mod test_base_language_model_trait {
    use super::*;

    #[test]
    fn test_get_num_tokens() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        let result = model.get_num_tokens("hello world test foo bar");
        assert_eq!(result, 5); // 5 words
    }

    #[test]
    fn test_get_num_tokens_empty_string() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        let result = model.get_num_tokens("");
        assert_eq!(result, 0);
    }

    #[test]
    fn test_get_num_tokens_single_word() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        let result = model.get_num_tokens("hello");
        assert_eq!(result, 1);
    }

    #[test]
    fn test_get_num_tokens_from_messages() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        let messages = vec![
            AnyMessage::HumanMessage(HumanMessage::builder().content("Hi").build()),
            AnyMessage::AIMessage(AIMessage::builder().content("Hello").build()),
        ];

        let result = model.get_num_tokens_from_messages(&messages, None);

        assert!(result > 0);
        assert_eq!(result, 10);
    }

    #[test]
    fn test_get_num_tokens_from_messages_empty() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        let messages: Vec<AnyMessage> = vec![];

        let result = model.get_num_tokens_from_messages(&messages, None);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_identifying_params() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        let params = model.identifying_params();

        assert!(params.contains_key("_type"));

        assert!(params.contains_key("responses"));

        if let Some(serde_json::Value::String(type_val)) = params.get("_type") {
            assert_eq!(type_val, "fake-list");
        } else {
            panic!("Expected _type to be a string");
        }
    }

    #[test]
    fn test_llm_type() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        assert_eq!(model.llm_type(), "fake-list");
    }

    #[test]
    fn test_model_name() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        assert_eq!(model.model_name(), "fake-list-llm");
    }

    #[test]
    fn test_get_token_ids() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        let result = model.get_token_ids("hello world");

        assert_eq!(result.len(), 2);
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn test_get_ls_params() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        let params = model.get_ls_params(Some(&["stop1".to_string(), "stop2".to_string()]));

        assert!(params.ls_provider.is_some());
        assert!(params.ls_model_name.is_some());
        assert_eq!(
            params.ls_stop,
            Some(vec!["stop1".to_string(), "stop2".to_string()])
        );
    }

    #[test]
    fn test_get_ls_params_without_stop() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();

        let params = model.get_ls_params(None);

        assert!(params.ls_provider.is_some());
        assert!(params.ls_model_name.is_some());
        assert_eq!(params.ls_stop, None);
    }
}

#[cfg(test)]
mod test_get_num_tokens_edge_cases {
    use super::*;

    #[test]
    fn test_get_num_tokens_whitespace_only() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();
        let result = model.get_num_tokens("   ");
        assert_eq!(result, 0);
    }

    #[test]
    fn test_get_num_tokens_single_token() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();
        let result = model.get_num_tokens("a");
        assert_eq!(result, 1);
    }
}

#[cfg(test)]
mod test_get_num_tokens_from_messages_edge_cases {
    use super::*;

    #[test]
    fn test_single_message_returns_correct_count() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();
        let messages = vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content("Hello world").build(),
        )];
        let result = model.get_num_tokens_from_messages(&messages, None);
        assert!(result > 0);
    }
}

#[cfg(test)]
mod test_generate_prompt {
    use super::*;

    #[tokio::test]
    async fn test_generate_prompt_single_prompt() {
        let model = FakeListLLM::builder()
            .responses(vec!["test response".to_string()])
            .build();
        let prompts = vec![vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content("Hello").build(),
        )]];
        let result = model.generate_prompt(prompts, None, None).await.unwrap();

        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].len(), 1);
        match &result.generations[0][0] {
            agent_chain_core::outputs::GenerationType::Generation(generation) => {
                assert_eq!(generation.text, "test response");
            }
            _ => panic!("Expected Generation variant"),
        }
    }

    #[tokio::test]
    async fn test_generate_prompt_multiple_prompts() {
        let model = FakeListLLM::builder()
            .responses(vec![
                "Response 1".to_string(),
                "Response 2".to_string(),
                "Response 3".to_string(),
            ])
            .build();
        let prompts = vec![
            vec![AnyMessage::HumanMessage(
                HumanMessage::builder().content("Prompt 1").build(),
            )],
            vec![AnyMessage::HumanMessage(
                HumanMessage::builder().content("Prompt 2").build(),
            )],
            vec![AnyMessage::HumanMessage(
                HumanMessage::builder().content("Prompt 3").build(),
            )],
        ];
        let result = model.generate_prompt(prompts, None, None).await.unwrap();

        assert_eq!(result.generations.len(), 3);
        for gen_list in &result.generations {
            assert_eq!(gen_list.len(), 1);
        }
    }

    #[tokio::test]
    async fn test_generate_prompt_empty_prompts() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();
        let result = model.generate_prompt(vec![], None, None).await.unwrap();

        assert_eq!(result.generations.len(), 0);
    }
}

#[cfg(test)]
mod test_agenerate_prompt {
    use super::*;

    #[tokio::test]
    async fn test_agenerate_prompt_single_prompt() {
        let model = FakeListLLM::builder()
            .responses(vec!["test response".to_string()])
            .build();
        let prompts = vec![vec![AnyMessage::HumanMessage(
            HumanMessage::builder().content("Hello").build(),
        )]];
        let result = model.generate_prompt(prompts, None, None).await.unwrap();

        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].len(), 1);
    }

    #[tokio::test]
    async fn test_agenerate_prompt_multiple_prompts() {
        let model = FakeListLLM::builder()
            .responses(vec!["Response 1".to_string(), "Response 2".to_string()])
            .build();
        let prompts = vec![
            vec![AnyMessage::HumanMessage(
                HumanMessage::builder().content("Prompt 1").build(),
            )],
            vec![AnyMessage::HumanMessage(
                HumanMessage::builder().content("Prompt 2").build(),
            )],
        ];
        let result = model.generate_prompt(prompts, None, None).await.unwrap();

        assert_eq!(result.generations.len(), 2);
    }

    #[tokio::test]
    async fn test_agenerate_prompt_empty_prompts() {
        let model = FakeListLLM::builder()
            .responses(vec!["response".to_string()])
            .build();
        let result = model.generate_prompt(vec![], None, None).await.unwrap();

        assert_eq!(result.generations.len(), 0);
    }
}

#[cfg(test)]
mod test_callbacks_config {
    use agent_chain_core::callbacks::BaseCallbackHandler;
    use agent_chain_core::callbacks::Callbacks;
    use agent_chain_core::language_models::LanguageModelConfig;
    use std::sync::Arc;

    #[derive(Debug)]
    struct TestHandler;
    impl BaseCallbackHandler for TestHandler {
        fn name(&self) -> &str {
            "TestHandler"
        }
    }

    #[test]
    fn test_initialization_with_callbacks() {
        let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
        let callbacks = Callbacks::from(vec![handler]);
        let config = LanguageModelConfig::builder().callbacks(callbacks).build();
        assert!(config.callbacks.is_some());
    }

    #[test]
    fn test_callbacks_excluded_from_serialization() {
        let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
        let callbacks = Callbacks::from(vec![handler]);
        let config = LanguageModelConfig::builder().callbacks(callbacks).build();

        let json = serde_json::to_string(&config).unwrap();
        assert!(
            !json.contains("callbacks"),
            "callbacks should be excluded from serialization"
        );
    }
}
