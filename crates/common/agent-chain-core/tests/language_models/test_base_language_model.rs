//! Tests for base language model.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/language_models/test_base_language_model.py`

use std::collections::HashMap;

use agent_chain_core::language_models::{
    BaseLanguageModel, FakeListLLM, LangSmithParams, LanguageModelConfig, LanguageModelInput,
    LanguageModelOutput, get_token_ids_default,
};
use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage};
use agent_chain_core::prompt_values::StringPromptValue;

#[cfg(test)]
mod test_lang_smith_params {
    use super::*;

    #[test]
    fn test_langsmith_params_all_fields() {
        let params = LangSmithParams::new()
            .with_provider("openai")
            .with_model_name("gpt-4")
            .with_model_type("chat")
            .with_temperature(0.7)
            .with_max_tokens(1000)
            .with_stop(vec!["stop1".to_string(), "stop2".to_string()]);

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
        let params = LangSmithParams::new()
            .with_provider("anthropic")
            .with_model_type("chat");

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
        let chat_params = LangSmithParams::new().with_model_type("chat");
        let llm_params = LangSmithParams::new().with_model_type("llm");

        assert_eq!(chat_params.ls_model_type, Some("chat".to_string()));
        assert_eq!(llm_params.ls_model_type, Some("llm".to_string()));
    }

    #[test]
    fn test_langsmith_params_builder_pattern() {
        let params = LangSmithParams::new()
            .with_provider("openai")
            .with_model_name("gpt-4")
            .with_temperature(0.5);

        assert_eq!(params.ls_provider, Some("openai".to_string()));
        assert_eq!(params.ls_model_name, Some("gpt-4".to_string()));
        assert_eq!(params.ls_temperature, Some(0.5));
    }

    #[test]
    fn test_langsmith_params_serialization() {
        let params = LangSmithParams::new()
            .with_provider("openai")
            .with_model_name("gpt-4");

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
        let config = LanguageModelConfig::new().with_cache(true);
        assert_eq!(config.cache, Some(true));
    }

    #[test]
    fn test_config_with_cache_false() {
        let config = LanguageModelConfig::new().with_cache(false);
        assert_eq!(config.cache, Some(false));
    }

    #[test]
    fn test_config_with_tags() {
        let config =
            LanguageModelConfig::new().with_tags(vec!["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(
            config.tags,
            Some(vec!["tag1".to_string(), "tag2".to_string()])
        );
    }

    #[test]
    fn test_config_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("key".to_string(), serde_json::json!("value"));

        let config = LanguageModelConfig::new().with_metadata(metadata.clone());
        assert_eq!(config.metadata, Some(metadata));
    }

    #[test]
    fn test_config_builder_chain() {
        let config = LanguageModelConfig::new()
            .with_cache(true)
            .with_tags(vec!["test".to_string()]);

        assert_eq!(config.cache, Some(true));
        assert_eq!(config.tags, Some(vec!["test".to_string()]));
    }

    #[test]
    fn test_config_serialization() {
        let config = LanguageModelConfig::new().with_cache(true);

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("cache"));
    }
}

#[cfg(test)]
mod test_language_model_input {
    use super::*;

    #[test]
    fn test_language_model_input_accepts_string() {
        let input: LanguageModelInput = "test string".into();
        match input {
            LanguageModelInput::Text(s) => assert_eq!(s, "test string"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_language_model_input_accepts_owned_string() {
        let input: LanguageModelInput = String::from("test string").into();
        match input {
            LanguageModelInput::Text(s) => assert_eq!(s, "test string"),
            _ => panic!("Expected Text variant"),
        }
    }

    #[test]
    fn test_language_model_input_accepts_prompt_value() {
        let prompt = StringPromptValue::new("test prompt");
        let input: LanguageModelInput = prompt.into();

        match input {
            LanguageModelInput::StringPrompt(p) => {
                assert_eq!(p.text, "test prompt");
            }
            _ => panic!("Expected StringPrompt variant"),
        }
    }

    #[test]
    fn test_language_model_input_accepts_message_sequence() {
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder().content("Hello").build(),
        )];
        let input: LanguageModelInput = messages.into();

        match input {
            LanguageModelInput::Messages(m) => {
                assert_eq!(m.len(), 1);
            }
            _ => panic!("Expected Messages variant"),
        }
    }

    #[test]
    fn test_language_model_input_to_messages() {
        let input: LanguageModelInput = "hello".into();
        let messages = input.to_messages();

        assert_eq!(messages.len(), 1);
        match &messages[0] {
            BaseMessage::Human(m) => {
                assert_eq!(m.content.as_text(), "hello");
            }
            _ => panic!("Expected Human message"),
        }
    }

    #[test]
    fn test_language_model_input_display() {
        let input: LanguageModelInput = "test display".into();
        let display = format!("{}", input);
        assert_eq!(display, "test display");
    }
}

#[cfg(test)]
mod test_language_model_output {
    use super::*;

    #[test]
    fn test_language_model_output_accepts_string() {
        let output: LanguageModelOutput = "test output".to_string().into();
        assert_eq!(output.text(), "test output");
    }

    #[test]
    fn test_language_model_output_accepts_ai_message() {
        let message = AIMessage::builder().content("test message").build();
        let output: LanguageModelOutput = message.into();
        assert_eq!(output.text(), "test message");
    }

    #[test]
    fn test_language_model_output_into_text() {
        let output: LanguageModelOutput = "hello".to_string().into();
        let text = output.into_text();
        assert_eq!(text, "hello");
    }

    #[test]
    fn test_language_model_output_message_variant() {
        let ai_message = AIMessage::builder().content("direct message").build();
        let output = LanguageModelOutput::message(ai_message);
        assert_eq!(output.text(), "direct message");
    }
}

#[cfg(test)]
mod test_language_model_config_serialization {
    use super::*;

    #[test]
    fn test_cache_serialization() {
        let config = LanguageModelConfig::new().with_cache(true);
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("cache"));
        assert!(json.contains("true"));
    }

    #[test]
    fn test_tags_excluded_when_none() {
        let config = LanguageModelConfig::new();
        let json = serde_json::to_string(&config).unwrap();

        assert!(!json.contains("tags"));
    }

    #[test]
    fn test_metadata_excluded_when_none() {
        let config = LanguageModelConfig::new();
        let json = serde_json::to_string(&config).unwrap();

        assert!(!json.contains("metadata"));
    }

    #[test]
    fn test_deserialization_roundtrip() {
        let config = LanguageModelConfig::new()
            .with_cache(true)
            .with_tags(vec!["test".to_string()]);

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
        let params = LangSmithParams::new()
            .with_provider("openai")
            .with_model_name("gpt-4");

        let cloned = params.clone();
        assert_eq!(cloned.ls_provider, params.ls_provider);
        assert_eq!(cloned.ls_model_name, params.ls_model_name);
    }

    #[test]
    fn test_langsmith_params_debug() {
        let params = LangSmithParams::new().with_provider("openai");
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
        let config = LanguageModelConfig::new().with_custom_get_token_ids(simple_char_tokenizer);

        assert!(config.custom_get_token_ids.is_some());
    }

    #[test]
    fn test_custom_get_token_ids_none_by_default() {
        let config = LanguageModelConfig::default();

        assert!(config.custom_get_token_ids.is_none());
    }

    #[test]
    fn test_custom_tokenizer_function_execution() {
        let config = LanguageModelConfig::new().with_custom_get_token_ids(fixed_tokenizer);

        let tokenizer = config.custom_get_token_ids.unwrap();
        let result = tokenizer("any text");
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_custom_tokenizer_with_char_encoding() {
        let config = LanguageModelConfig::new().with_custom_get_token_ids(simple_char_tokenizer);

        let tokenizer = config.custom_get_token_ids.unwrap();
        let result = tokenizer("Hi");

        assert_eq!(result, vec![72, 105]);
    }

    #[test]
    fn test_custom_get_token_ids_excluded_from_serialization() {
        let config = LanguageModelConfig::new().with_custom_get_token_ids(simple_char_tokenizer);

        let json = serde_json::to_string(&config).unwrap();

        assert!(!json.contains("custom_get_token_ids"));
    }
}

#[cfg(test)]
mod test_base_language_model_trait {
    use super::*;

    #[test]
    fn test_get_num_tokens() {
        let model = FakeListLLM::new(vec!["response".to_string()]);

        let result = model.get_num_tokens("hello world test foo bar");
        assert_eq!(result, 5); // 5 words
    }

    #[test]
    fn test_get_num_tokens_empty_string() {
        let model = FakeListLLM::new(vec!["response".to_string()]);

        let result = model.get_num_tokens("");
        assert_eq!(result, 0);
    }

    #[test]
    fn test_get_num_tokens_single_word() {
        let model = FakeListLLM::new(vec!["response".to_string()]);

        let result = model.get_num_tokens("hello");
        assert_eq!(result, 1);
    }

    #[test]
    fn test_get_num_tokens_from_messages() {
        let model = FakeListLLM::new(vec!["response".to_string()]);

        let messages = vec![
            BaseMessage::Human(HumanMessage::builder().content("Hi").build()),
            BaseMessage::AI(AIMessage::builder().content("Hello").build()),
        ];

        let result = model.get_num_tokens_from_messages(&messages, None);

        assert!(result > 0);
        assert_eq!(result, 10);
    }

    #[test]
    fn test_get_num_tokens_from_messages_empty() {
        let model = FakeListLLM::new(vec!["response".to_string()]);

        let messages: Vec<BaseMessage> = vec![];

        let result = model.get_num_tokens_from_messages(&messages, None);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_identifying_params() {
        let model = FakeListLLM::new(vec!["response".to_string()]);

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
        let model = FakeListLLM::new(vec!["response".to_string()]);

        assert_eq!(model.llm_type(), "fake-list");
    }

    #[test]
    fn test_model_name() {
        let model = FakeListLLM::new(vec!["response".to_string()]);

        assert_eq!(model.model_name(), "fake-list-llm");
    }

    #[test]
    fn test_get_token_ids() {
        let model = FakeListLLM::new(vec!["response".to_string()]);

        let result = model.get_token_ids("hello world");

        assert_eq!(result.len(), 2);
        assert_eq!(result, vec![0, 1]);
    }

    #[test]
    fn test_get_ls_params() {
        let model = FakeListLLM::new(vec!["response".to_string()]);

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
        let model = FakeListLLM::new(vec!["response".to_string()]);

        let params = model.get_ls_params(None);

        assert!(params.ls_provider.is_some());
        assert!(params.ls_model_name.is_some());
        assert_eq!(params.ls_stop, None);
    }
}

#[cfg(test)]
mod test_get_num_tokens_edge_cases {
    use super::*;

    /// Ported from `test_get_num_tokens_whitespace_only`.
    #[test]
    fn test_get_num_tokens_whitespace_only() {
        let model = FakeListLLM::new(vec!["response".to_string()]);
        let result = model.get_num_tokens("   ");
        assert_eq!(result, 0);
    }

    /// Ported from `test_get_num_tokens_single_token`.
    #[test]
    fn test_get_num_tokens_single_token() {
        let model = FakeListLLM::new(vec!["response".to_string()]);
        let result = model.get_num_tokens("a");
        assert_eq!(result, 1);
    }
}

#[cfg(test)]
mod test_get_num_tokens_from_messages_edge_cases {
    use super::*;

    /// Ported from `test_single_message_returns_correct_count`.
    #[test]
    fn test_single_message_returns_correct_count() {
        let model = FakeListLLM::new(vec!["response".to_string()]);
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder().content("Hello world").build(),
        )];
        let result = model.get_num_tokens_from_messages(&messages, None);
        assert!(result > 0);
    }
}

#[cfg(test)]
mod test_generate_prompt {
    use super::*;

    /// Ported from `test_generate_prompt_single_prompt`.
    #[tokio::test]
    async fn test_generate_prompt_single_prompt() {
        let model = FakeListLLM::new(vec!["test response".to_string()]);
        let prompts = vec![LanguageModelInput::from("Hello")];
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

    /// Ported from `test_generate_prompt_multiple_prompts`.
    #[tokio::test]
    async fn test_generate_prompt_multiple_prompts() {
        let model = FakeListLLM::new(vec![
            "Response 1".to_string(),
            "Response 2".to_string(),
            "Response 3".to_string(),
        ]);
        let prompts = vec![
            LanguageModelInput::from("Prompt 1"),
            LanguageModelInput::from("Prompt 2"),
            LanguageModelInput::from("Prompt 3"),
        ];
        let result = model.generate_prompt(prompts, None, None).await.unwrap();

        assert_eq!(result.generations.len(), 3);
        for gen_list in &result.generations {
            assert_eq!(gen_list.len(), 1);
        }
    }

    /// Ported from `test_generate_prompt_empty_prompts`.
    #[tokio::test]
    async fn test_generate_prompt_empty_prompts() {
        let model = FakeListLLM::new(vec!["response".to_string()]);
        let result = model.generate_prompt(vec![], None, None).await.unwrap();

        assert_eq!(result.generations.len(), 0);
    }
}

#[cfg(test)]
mod test_agenerate_prompt {
    use super::*;

    /// Ported from `test_agenerate_prompt_single_prompt`.
    ///
    /// In Rust, generate_prompt is already async, so this tests the same
    /// code path as the sync test but explicitly exercises the async nature.
    #[tokio::test]
    async fn test_agenerate_prompt_single_prompt() {
        let model = FakeListLLM::new(vec!["test response".to_string()]);
        let prompts = vec![LanguageModelInput::from("Hello")];
        let result = model.generate_prompt(prompts, None, None).await.unwrap();

        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].len(), 1);
    }

    /// Ported from `test_agenerate_prompt_multiple_prompts`.
    #[tokio::test]
    async fn test_agenerate_prompt_multiple_prompts() {
        let model = FakeListLLM::new(vec!["Response 1".to_string(), "Response 2".to_string()]);
        let prompts = vec![
            LanguageModelInput::from("Prompt 1"),
            LanguageModelInput::from("Prompt 2"),
        ];
        let result = model.generate_prompt(prompts, None, None).await.unwrap();

        assert_eq!(result.generations.len(), 2);
    }

    /// Ported from `test_agenerate_prompt_empty_prompts`.
    #[tokio::test]
    async fn test_agenerate_prompt_empty_prompts() {
        let model = FakeListLLM::new(vec!["response".to_string()]);
        let result = model.generate_prompt(vec![], None, None).await.unwrap();

        assert_eq!(result.generations.len(), 0);
    }
}

#[cfg(test)]
mod test_callbacks_config {
    use agent_chain_core::callbacks::Callbacks;
    use agent_chain_core::callbacks::base::{
        BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
        RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
    };
    use agent_chain_core::language_models::LanguageModelConfig;
    use std::sync::Arc;

    #[derive(Debug)]
    struct TestHandler;
    impl LLMManagerMixin for TestHandler {}
    impl ChainManagerMixin for TestHandler {}
    impl ToolManagerMixin for TestHandler {}
    impl RetrieverManagerMixin for TestHandler {}
    impl CallbackManagerMixin for TestHandler {}
    impl RunManagerMixin for TestHandler {}
    impl BaseCallbackHandler for TestHandler {
        fn name(&self) -> &str {
            "TestHandler"
        }
    }

    /// Ported from `test_initialization_with_callbacks`.
    #[test]
    fn test_initialization_with_callbacks() {
        let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
        let callbacks = Callbacks::from_handlers(vec![handler]);
        let config = LanguageModelConfig::new().with_callbacks(callbacks);
        assert!(config.callbacks.is_some());
    }

    /// Ported from `test_callbacks_excluded_from_serialization`.
    #[test]
    fn test_callbacks_excluded_from_serialization() {
        let handler: Arc<dyn BaseCallbackHandler> = Arc::new(TestHandler);
        let callbacks = Callbacks::from_handlers(vec![handler]);
        let config = LanguageModelConfig::new().with_callbacks(callbacks);

        let json = serde_json::to_string(&config).unwrap();
        assert!(
            !json.contains("callbacks"),
            "callbacks should be excluded from serialization"
        );
    }
}
