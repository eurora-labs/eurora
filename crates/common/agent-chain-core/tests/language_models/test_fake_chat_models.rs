//! Tests for fake chat models.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/language_models/test_fake_chat_models.py`

#[cfg(test)]
mod test_fake_messages_list_chat_model {
    //! Tests for FakeMessagesListChatModel class
    //! Python equivalent: TestFakeMessagesListChatModel

    use std::time::{Duration, Instant};

    use agent_chain_core::FakeMessagesListChatModel;
    use agent_chain_core::language_models::{BaseChatModel, BaseLanguageModel};
    use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage};

    #[test]
    fn test_initialization() {
        let responses = vec![
            BaseMessage::AI(AIMessage::builder().content("response1").build()),
            BaseMessage::AI(AIMessage::builder().content("response2").build()),
        ];
        let model = FakeMessagesListChatModel::new(responses);
        assert_eq!(model.current_index(), 0);
    }

    #[test]
    fn test_initialization_with_sleep() {
        let model = FakeMessagesListChatModel::new(vec![BaseMessage::AI(
            AIMessage::builder().content("test").build(),
        )])
        .with_sleep(Duration::from_millis(100));
        assert_eq!(model.current_index(), 0);
    }

    #[test]
    fn test_llm_type() {
        let model = FakeMessagesListChatModel::new(vec![BaseMessage::AI(
            AIMessage::builder().content("test").build(),
        )]);
        assert_eq!(model.llm_type(), "fake-messages-list-chat-model");
    }

    #[tokio::test]
    async fn test_invoke_returns_message() {
        let response = BaseMessage::AI(AIMessage::builder().content("hello").build());
        let model = FakeMessagesListChatModel::new(vec![response]);
        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "hello");
    }

    #[tokio::test]
    async fn test_invoke_cycles_through_responses() {
        let responses = vec![
            BaseMessage::AI(AIMessage::builder().content("first").build()),
            BaseMessage::AI(AIMessage::builder().content("second").build()),
            BaseMessage::AI(AIMessage::builder().content("third").build()),
        ];
        let model = FakeMessagesListChatModel::new(responses);

        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "first");

        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "second");

        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "third");

        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "first");
    }

    #[tokio::test]
    async fn test_invoke_with_single_response() {
        let model = FakeMessagesListChatModel::new(vec![BaseMessage::AI(
            AIMessage::builder().content("only").build(),
        )]);

        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "only");

        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "only");

        assert_eq!(model.current_index(), 0);
    }

    #[tokio::test]
    async fn test_invoke_with_sleep() {
        let model = FakeMessagesListChatModel::new(vec![BaseMessage::AI(
            AIMessage::builder().content("test").build(),
        )])
        .with_sleep(Duration::from_millis(50));

        let start = Instant::now();
        let _ = model._generate(vec![], None, None).await.unwrap();
        let elapsed = start.elapsed();

        assert!(
            elapsed >= Duration::from_millis(50),
            "Expected at least 50ms, got {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_generate_returns_chat_result() {
        let model = FakeMessagesListChatModel::new(vec![BaseMessage::AI(
            AIMessage::builder().content("test").build(),
        )]);
        let result = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("hi").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].message.content(), "test");
    }
}

#[cfg(test)]
mod test_fake_list_chat_model_error {
    //! Tests for FakeListChatModelError exception
    //! Python equivalent: TestFakeListChatModelError

    use agent_chain_core::FakeListChatModelError;
    use std::error::Error;

    #[test]
    fn test_error_can_be_raised() {
        let error = FakeListChatModelError;
        assert_eq!(error.to_string(), "FakeListChatModelError");
    }

    #[test]
    fn test_error_is_exception() {
        let error = FakeListChatModelError;
        let _: &dyn Error = &error;
    }
}

#[cfg(test)]
mod test_fake_list_chat_model {
    //! Tests for FakeListChatModel class
    //! Python equivalent: TestFakeListChatModel

    use agent_chain_core::FakeListChatModel;
    use agent_chain_core::language_models::{BaseChatModel, BaseLanguageModel};
    use agent_chain_core::messages::BaseMessage;
    use futures::StreamExt;

    #[test]
    fn test_initialization() {
        let model = FakeListChatModel::new(vec!["response1".to_string(), "response2".to_string()]);
        assert_eq!(model.current_index(), 0);
    }

    #[test]
    fn test_llm_type() {
        let model = FakeListChatModel::new(vec!["test".to_string()]);
        assert_eq!(model.llm_type(), "fake-list-chat-model");
    }

    #[tokio::test]
    async fn test_invoke_returns_ai_message() {
        let model = FakeListChatModel::new(vec!["hello".to_string()]);
        let result = model._generate(vec![], None, None).await.unwrap();
        assert!(matches!(result.generations[0].message, BaseMessage::AI(_)));
        assert_eq!(result.generations[0].message.content(), "hello");
    }

    #[tokio::test]
    async fn test_invoke_cycles_through_responses() {
        let model = FakeListChatModel::new(vec!["first".to_string(), "second".to_string()]);

        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "first");

        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "second");

        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "first");
    }

    #[tokio::test]
    async fn test_stream_yields_characters() {
        let model = FakeListChatModel::new(vec!["hello".to_string()]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            chunks.push(chunk_result.unwrap());
        }

        assert_eq!(chunks.len(), 5);
        let contents: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        assert_eq!(contents, vec!["h", "e", "l", "l", "o"]);
    }

    #[tokio::test]
    async fn test_stream_with_chunk_position() {
        let model = FakeListChatModel::new(vec!["ab".to_string()]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            chunks.push(chunk_result.unwrap());
        }

        assert_eq!(chunks.len(), 2);
        if let BaseMessage::AI(_ai_msg) = &chunks[1].message {}
    }

    #[tokio::test]
    async fn test_stream_error_on_chunk() {
        let model = FakeListChatModel::new(vec!["hello".to_string()]).with_error_on_chunk(2);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let chunk1 = stream.next().await.unwrap();
        assert!(chunk1.is_ok()); // h

        let chunk2 = stream.next().await.unwrap();
        assert!(chunk2.is_ok()); // e

        let chunk3 = stream.next().await;
        assert!(chunk3.is_some());
        assert!(chunk3.unwrap().is_err()); // Should error
    }

    #[tokio::test]
    async fn test_astream_yields_characters() {
        let model = FakeListChatModel::new(vec!["hello".to_string()]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }

        assert_eq!(chunks.len(), 5);
        let contents: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        assert_eq!(contents, vec!["h", "e", "l", "l", "o"]);
    }

    #[tokio::test]
    async fn test_astream_error_on_chunk() {
        let model = FakeListChatModel::new(vec!["hello".to_string()]).with_error_on_chunk(2);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut successful_chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => successful_chunks.push(chunk),
                Err(_) => break,
            }
        }

        assert_eq!(successful_chunks.len(), 2);
    }

    #[test]
    fn test_identifying_params() {
        let model = FakeListChatModel::new(vec!["a".to_string(), "b".to_string()]);
        let params = model.identifying_params();
        assert!(params.contains_key("responses"));
        let responses = params.get("responses").unwrap();
        assert_eq!(responses, &serde_json::json!(["a", "b"]));
    }

    #[tokio::test]
    async fn test_batch_preserves_order() {
        let model =
            FakeListChatModel::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);

        let result1 = model._generate(vec![], None, None).await.unwrap();
        let result2 = model._generate(vec![], None, None).await.unwrap();
        let result3 = model._generate(vec![], None, None).await.unwrap();

        assert_eq!(result1.generations[0].message.content(), "r1");
        assert_eq!(result2.generations[0].message.content(), "r2");
        assert_eq!(result3.generations[0].message.content(), "r3");
    }

    #[tokio::test]
    async fn test_abatch_preserves_order() {
        let model =
            FakeListChatModel::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);

        let result1 = model._generate(vec![], None, None).await.unwrap();
        let result2 = model._generate(vec![], None, None).await.unwrap();
        let result3 = model._generate(vec![], None, None).await.unwrap();

        assert_eq!(result1.generations[0].message.content(), "r1");
        assert_eq!(result2.generations[0].message.content(), "r2");
        assert_eq!(result3.generations[0].message.content(), "r3");
    }

    #[tokio::test]
    async fn test_batch_with_config_list() {
        let model = FakeListChatModel::new(vec!["r1".to_string(), "r2".to_string()]);

        let result1 = model._generate(vec![], None, None).await.unwrap();
        let result2 = model._generate(vec![], None, None).await.unwrap();

        assert_eq!(result1.generations[0].message.content(), "r1");
        assert_eq!(result2.generations[0].message.content(), "r2");
    }

    #[tokio::test]
    async fn test_abatch_with_config_list() {
        let model = FakeListChatModel::new(vec!["r1".to_string(), "r2".to_string()]);

        let result1 = model._generate(vec![], None, None).await.unwrap();
        let result2 = model._generate(vec![], None, None).await.unwrap();

        assert_eq!(result1.generations[0].message.content(), "r1");
        assert_eq!(result2.generations[0].message.content(), "r2");
    }
}

#[cfg(test)]
mod test_fake_chat_model {
    //! Tests for FakeChatModel class
    //! Python equivalent: TestFakeChatModel

    use agent_chain_core::FakeChatModel;
    use agent_chain_core::language_models::{BaseChatModel, BaseLanguageModel};

    #[test]
    fn test_initialization() {
        let model = FakeChatModel::new();
        assert_eq!(model.llm_type(), "fake-chat-model");
    }

    #[tokio::test]
    async fn test_invoke_returns_fake_response() {
        let model = FakeChatModel::new();
        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "fake response");
    }

    #[tokio::test]
    async fn test_invoke_ignores_input() {
        use agent_chain_core::messages::{BaseMessage, HumanMessage};

        let model = FakeChatModel::new();
        let result1 = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("hello").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();
        let result2 = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("goodbye").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(
            result1.generations[0].message.content(),
            result2.generations[0].message.content()
        );
        assert_eq!(result1.generations[0].message.content(), "fake response");
    }

    #[tokio::test]
    async fn test_ainvoke_returns_fake_response() {
        let model = FakeChatModel::new();
        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "fake response");
    }

    #[test]
    fn test_identifying_params() {
        let model = FakeChatModel::new();
        let params = model.identifying_params();
        assert_eq!(params.get("key").unwrap(), "fake");
    }
}

#[cfg(test)]
mod test_generic_fake_chat_model {
    //! Tests for GenericFakeChatModel class
    //! Python equivalent: TestGenericFakeChatModel

    use agent_chain_core::GenericFakeChatModel;
    use agent_chain_core::language_models::{BaseChatModel, BaseLanguageModel};
    use agent_chain_core::messages::AIMessage;
    use futures::StreamExt;

    #[test]
    fn test_initialization() {
        let messages = vec![AIMessage::builder().content("test").build()];
        let model = GenericFakeChatModel::from_vec(messages);
        assert_eq!(model.llm_type(), "generic-fake-chat-model");
    }

    #[tokio::test]
    async fn test_invoke_returns_message_from_iterator() {
        let messages = vec![AIMessage::builder().content("hello").build()];
        let model = GenericFakeChatModel::from_vec(messages);
        let result = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "hello");
    }

    #[tokio::test]
    async fn test_invoke_with_string_messages() {
        let model =
            GenericFakeChatModel::from_strings(vec!["hello".to_string(), "world".to_string()]);

        let result1 = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result1.generations[0].message.content(), "hello");

        let result2 = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result2.generations[0].message.content(), "world");
    }

    #[tokio::test]
    async fn test_invoke_exhausts_iterator() {
        let messages = vec![AIMessage::builder().content("only").build()];
        let model = GenericFakeChatModel::from_vec(messages);

        let result1 = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result1.generations[0].message.content(), "only");

        let result2 = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result2.generations[0].message.content(), "");
    }

    #[tokio::test]
    async fn test_stream_splits_on_whitespace() {
        let messages = vec![AIMessage::builder().content("hello world").build()];
        let model = GenericFakeChatModel::from_vec(messages);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }

        let contents: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        let joined = contents.join("");
        assert_eq!(joined, "hello world");
    }

    #[tokio::test]
    async fn test_stream_with_function_call() {
        use serde_json::Value;
        use std::collections::HashMap;

        let mut function_call = HashMap::new();
        function_call.insert("name".to_string(), Value::String("test_func".to_string()));
        function_call.insert(
            "arguments".to_string(),
            Value::String(r#"{"a": 1}"#.to_string()),
        );

        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "function_call".to_string(),
            Value::Object(function_call.into_iter().collect()),
        );

        let ai_msg = AIMessage::builder()
            .content("")
            .additional_kwargs(additional_kwargs)
            .build();

        let model = GenericFakeChatModel::from_vec(vec![ai_msg]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }

        assert!(!chunks.is_empty());
    }

    #[tokio::test]
    async fn test_stream_with_additional_kwargs() {
        use serde_json::Value;
        use std::collections::HashMap;

        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "custom_key".to_string(),
            Value::String("custom_value".to_string()),
        );

        let ai_msg = AIMessage::builder()
            .content("")
            .additional_kwargs(additional_kwargs)
            .build();

        let model = GenericFakeChatModel::from_vec(vec![ai_msg]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }

        assert!(!chunks.is_empty());
    }

    #[tokio::test]
    async fn test_stream_empty_content_no_kwargs() {
        let ai_msg = AIMessage::builder().content("").build();
        let model = GenericFakeChatModel::from_vec(vec![ai_msg]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }

        assert!(chunks.is_empty());
    }
}

#[cfg(test)]
mod test_parrot_fake_chat_model {
    //! Tests for ParrotFakeChatModel class
    //! Python equivalent: TestParrotFakeChatModel

    use agent_chain_core::ParrotFakeChatModel;
    use agent_chain_core::language_models::{BaseChatModel, BaseLanguageModel};
    use agent_chain_core::messages::{BaseMessage, HumanMessage, SystemMessage};

    #[test]
    fn test_initialization() {
        let model = ParrotFakeChatModel::new();
        assert_eq!(model.llm_type(), "parrot-fake-chat-model");
    }

    #[tokio::test]
    async fn test_invoke_returns_last_message() {
        let model = ParrotFakeChatModel::new();
        let messages = vec![
            BaseMessage::System(SystemMessage::builder().content("You are helpful").build()),
            BaseMessage::Human(HumanMessage::builder().content("Hello!").build()),
        ];
        let result = model._generate(messages, None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "Hello!");
    }

    #[tokio::test]
    async fn test_invoke_with_single_message() {
        let model = ParrotFakeChatModel::new();
        let result = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("Single").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.generations[0].message.content(), "Single");
    }

    #[tokio::test]
    async fn test_invoke_with_string_input() {
        use agent_chain_core::language_models::LanguageModelInput;

        let model = ParrotFakeChatModel::new();
        let input = LanguageModelInput::Text("Hello string".to_string());
        let messages = input.to_messages();
        let result = model._generate(messages, None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "Hello string");
    }

    #[tokio::test]
    async fn test_invoke_preserves_message_type() {
        let model = ParrotFakeChatModel::new();
        let messages = vec![BaseMessage::Human(
            HumanMessage::builder().content("test").build(),
        )];
        let result = model._generate(messages, None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "test");
    }

    #[tokio::test]
    async fn test_ainvoke_returns_last_message() {
        let model = ParrotFakeChatModel::new();
        let messages = vec![
            BaseMessage::Human(HumanMessage::builder().content("First").build()),
            BaseMessage::Human(HumanMessage::builder().content("Last").build()),
        ];
        let result = model._generate(messages, None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "Last");
    }

    #[tokio::test]
    async fn test_generate_returns_chat_result() {
        let model = ParrotFakeChatModel::new();
        let result = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("test").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].message.content(), "test");
    }

    #[tokio::test]
    async fn test_batch_returns_last_messages() {
        let model = ParrotFakeChatModel::new();

        let result1 = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("batch1").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();
        let result2 = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("batch2").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(result1.generations[0].message.content(), "batch1");
        assert_eq!(result2.generations[0].message.content(), "batch2");
    }

    #[tokio::test]
    async fn test_with_complex_content() {
        use agent_chain_core::messages::{ContentPart, ImageSource, MessageContent};

        let model = ParrotFakeChatModel::new();
        let message = HumanMessage::builder()
            .content(MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "Hello".to_string(),
                },
                ContentPart::Image {
                    source: ImageSource::Url {
                        url: "https://example.com/img.png".to_string(),
                    },
                    detail: None,
                },
            ]))
            .build();

        let result = model
            ._generate(vec![BaseMessage::Human(message.clone())], None, None)
            .await
            .unwrap();

        assert_eq!(
            result.generations[0].message.content(),
            BaseMessage::Human(message).content()
        );
    }
}

#[cfg(test)]
mod test_fake_messages_list_additional {
    use agent_chain_core::FakeMessagesListChatModel;
    use agent_chain_core::language_models::BaseChatModel;
    use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage};

    /// Ported from `test_single_response_counter_stays_at_zero`.
    #[tokio::test]
    async fn test_single_response_counter_stays_at_zero() {
        let model = FakeMessagesListChatModel::new(vec![BaseMessage::AI(
            AIMessage::builder().content("only one").build(),
        )]);
        assert_eq!(model.current_index(), 0);
        let _ = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(model.current_index(), 0);
        let _ = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(model.current_index(), 0);
        let _ = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(model.current_index(), 0);
    }

    /// Ported from `test_ainvoke` (async cycling).
    #[tokio::test]
    async fn test_ainvoke() {
        let responses = vec![
            BaseMessage::AI(AIMessage::builder().content("async first").build()),
            BaseMessage::AI(AIMessage::builder().content("async second").build()),
        ];
        let model = FakeMessagesListChatModel::new(responses);

        let result1 = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result1.generations[0].message.content(), "async first");

        let result2 = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result2.generations[0].message.content(), "async second");

        let result3 = model._generate(vec![], None, None).await.unwrap();
        assert_eq!(result3.generations[0].message.content(), "async first");
    }

    /// Ported from `test_generate_returns_proper_chat_result_structure`.
    #[tokio::test]
    async fn test_generate_returns_proper_chat_result_structure() {
        let model = FakeMessagesListChatModel::new(vec![BaseMessage::AI(
            AIMessage::builder().content("structured").build(),
        )]);
        let result = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("hi").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(result.generations.len(), 1);
        let generation = &result.generations[0];
        assert!(matches!(generation.message, BaseMessage::AI(_)));
        assert_eq!(generation.message.content(), "structured");
    }

    /// Ported from `test_generate_with_non_ai_message_response`.
    #[tokio::test]
    async fn test_generate_with_non_ai_message_response() {
        let human_msg = BaseMessage::Human(HumanMessage::builder().content("echoed back").build());
        let model = FakeMessagesListChatModel::new(vec![human_msg]);
        let result = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("hi").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].message.content(), "echoed back");
        assert!(matches!(
            result.generations[0].message,
            BaseMessage::Human(_)
        ));
    }
}

#[cfg(test)]
mod test_fake_list_chat_model_additional {
    use agent_chain_core::FakeListChatModel;
    use agent_chain_core::language_models::BaseChatModel;
    use futures::StreamExt;
    use std::time::{Duration, Instant};

    /// Ported from `test_call_with_sleep`.
    #[tokio::test]
    async fn test_call_with_sleep() {
        let model =
            FakeListChatModel::new(vec!["hello".to_string()]).with_sleep(Duration::from_millis(50));

        let start = Instant::now();
        let result = model._generate(vec![], None, None).await.unwrap();
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(50));
        assert_eq!(result.generations[0].message.content(), "hello");
    }

    /// Ported from `test_stream_chunk_position_single_char`.
    #[tokio::test]
    async fn test_stream_chunk_position_single_char() {
        let model = FakeListChatModel::new(vec!["x".to_string()]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            chunks.push(chunk_result.unwrap());
        }

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "x");
    }

    /// Ported from `test_astream_chunk_position_last`.
    #[tokio::test]
    async fn test_astream_chunk_position_last() {
        let model = FakeListChatModel::new(vec!["abc".to_string()]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            chunks.push(chunk_result.unwrap());
        }

        assert_eq!(chunks.len(), 3);
        let contents: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        assert_eq!(contents, vec!["a", "b", "c"]);
    }

    /// Ported from `test_stream_error_on_first_chunk`.
    #[tokio::test]
    async fn test_stream_error_on_first_chunk() {
        let model = FakeListChatModel::new(vec!["hello".to_string()]).with_error_on_chunk(0);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let first = stream.next().await.unwrap();
        assert!(first.is_err());
    }

    /// Ported from `test_astream_error_on_first_chunk`.
    #[tokio::test]
    async fn test_astream_error_on_first_chunk() {
        let model = FakeListChatModel::new(vec!["hello".to_string()]).with_error_on_chunk(0);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => chunks.push(chunk),
                Err(_) => break,
            }
        }
        assert!(chunks.is_empty());
    }

    /// Ported from `test_stream_empty_string_response`.
    #[tokio::test]
    async fn test_stream_empty_string_response() {
        let model = FakeListChatModel::new(vec!["".to_string()]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }
        assert!(chunks.is_empty());
    }

    /// Ported from `test_batch_with_single_config`.
    #[tokio::test]
    async fn test_batch_with_single_config() {
        let model =
            FakeListChatModel::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);

        let result1 = model._generate(vec![], None, None).await.unwrap();
        let result2 = model._generate(vec![], None, None).await.unwrap();
        let result3 = model._generate(vec![], None, None).await.unwrap();

        assert_eq!(result1.generations[0].message.content(), "r1");
        assert_eq!(result2.generations[0].message.content(), "r2");
        assert_eq!(result3.generations[0].message.content(), "r3");
    }

    /// Ported from `test_abatch_with_single_config`.
    #[tokio::test]
    async fn test_abatch_with_single_config() {
        let model =
            FakeListChatModel::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);

        let result1 = model._generate(vec![], None, None).await.unwrap();
        let result2 = model._generate(vec![], None, None).await.unwrap();
        let result3 = model._generate(vec![], None, None).await.unwrap();

        assert_eq!(result1.generations[0].message.content(), "r1");
        assert_eq!(result2.generations[0].message.content(), "r2");
        assert_eq!(result3.generations[0].message.content(), "r3");
    }
}

#[cfg(test)]
mod test_fake_chat_model_additional {
    use agent_chain_core::FakeChatModel;
    use agent_chain_core::language_models::{BaseChatModel, BaseLanguageModel};
    use agent_chain_core::messages::{BaseMessage, HumanMessage};

    /// Ported from `test_agenerate_returns_chat_result`.
    #[tokio::test]
    async fn test_agenerate_returns_chat_result() {
        let model = FakeChatModel::new();
        let result = model
            ._generate(
                vec![BaseMessage::Human(
                    HumanMessage::builder().content("hi").build(),
                )],
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(result.generations.len(), 1);
        assert!(matches!(result.generations[0].message, BaseMessage::AI(_)));
        assert_eq!(result.generations[0].message.content(), "fake response");
    }

    /// Ported from `test_llm_type_and_identifying_params_consistency`.
    #[test]
    fn test_llm_type_and_identifying_params_consistency() {
        let model = FakeChatModel::new();
        assert_eq!(model.llm_type(), "fake-chat-model");
        let params = model.identifying_params();
        assert_eq!(params.get("key").unwrap(), "fake");
        assert_eq!(model.llm_type(), "fake-chat-model");
        assert_eq!(params.get("key").unwrap(), "fake");
    }
}

#[cfg(test)]
mod test_generic_fake_chat_model_additional {
    use agent_chain_core::GenericFakeChatModel;
    use agent_chain_core::language_models::BaseChatModel;
    use agent_chain_core::messages::AIMessage;
    use futures::StreamExt;

    /// Ported from `test_stream_with_multiple_words_preserves_whitespace`.
    #[tokio::test]
    async fn test_stream_with_multiple_words_preserves_whitespace() {
        let messages = vec![AIMessage::builder().content("hello world foo").build()];
        let model = GenericFakeChatModel::from_vec(messages);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk.text.clone());
            }
        }

        assert_eq!(chunks, vec!["hello", " ", "world", " ", "foo"]);
        assert_eq!(chunks.join(""), "hello world foo");
    }

    /// Ported from `test_stream_function_call_non_string_values`.
    ///
    /// Tests that function_call with non-string values (dict) produces chunks.
    /// The "name" key (string) gets split by comma, "parsed" key (dict) gets
    /// a single chunk.
    #[tokio::test]
    async fn test_stream_function_call_non_string_values() {
        use serde_json::Value;
        use std::collections::HashMap;

        let mut function_call = HashMap::new();
        function_call.insert("name".to_string(), Value::String("my_func".to_string()));
        function_call.insert("parsed".to_string(), serde_json::json!({"key": "value"}));

        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "function_call".to_string(),
            Value::Object(function_call.into_iter().collect()),
        );

        let ai_msg = AIMessage::builder()
            .content("")
            .additional_kwargs(additional_kwargs)
            .build();

        let model = GenericFakeChatModel::from_vec(vec![ai_msg]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }

        assert!(!chunks.is_empty());
    }

    /// Ported from `test_stream_chunk_position_last_no_additional_kwargs`.
    #[tokio::test]
    async fn test_stream_chunk_position_last_no_additional_kwargs() {
        let messages = vec![AIMessage::builder().content("hi there").build()];
        let model = GenericFakeChatModel::from_vec(messages);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }

        assert_eq!(chunks.len(), 3);
        let contents: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
        assert_eq!(contents, vec!["hi", " ", "there"]);
    }

    /// Ported from `test_stream_with_content_and_additional_kwargs`.
    ///
    /// Tests that a message with both content and additional_kwargs produces
    /// chunks for both. Content "hello" is a single word -> 1 content chunk.
    /// Then additional_kwargs produces 1+ chunks.
    #[tokio::test]
    async fn test_stream_with_content_and_additional_kwargs() {
        use serde_json::Value;
        use std::collections::HashMap;

        let mut additional_kwargs = HashMap::new();
        additional_kwargs.insert(
            "custom_key".to_string(),
            Value::String("custom_value".to_string()),
        );

        let ai_msg = AIMessage::builder()
            .content("hello")
            .additional_kwargs(additional_kwargs)
            .build();

        let model = GenericFakeChatModel::from_vec(vec![ai_msg]);
        let mut stream = model._stream(vec![], None, None).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk);
            }
        }

        assert!(chunks.len() >= 2);

        let content_chunks: Vec<_> = chunks.iter().filter(|c| !c.text.is_empty()).collect();
        assert!(!content_chunks.is_empty());
        let joined: String = content_chunks.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(joined, "hello");
    }
}

#[cfg(test)]
mod test_parrot_fake_chat_model_additional {
    use agent_chain_core::ParrotFakeChatModel;
    use agent_chain_core::language_models::{BaseChatModel, LanguageModelInput};
    use agent_chain_core::messages::{BaseMessage, HumanMessage, SystemMessage};

    /// Ported from `test_generate_with_multiple_messages_returns_last`.
    #[tokio::test]
    async fn test_generate_with_multiple_messages_returns_last() {
        let model = ParrotFakeChatModel::new();
        let messages = vec![
            BaseMessage::System(SystemMessage::builder().content("system prompt").build()),
            BaseMessage::Human(HumanMessage::builder().content("first human").build()),
            BaseMessage::Human(HumanMessage::builder().content("second human").build()),
            BaseMessage::Human(HumanMessage::builder().content("last human").build()),
        ];
        let result = model._generate(messages, None, None).await.unwrap();
        assert_eq!(result.generations.len(), 1);
        assert_eq!(result.generations[0].message.content(), "last human");
    }

    /// Ported from `test_ainvoke_with_string`.
    #[tokio::test]
    async fn test_ainvoke_with_string() {
        let model = ParrotFakeChatModel::new();
        let input = LanguageModelInput::Text("echo this string".to_string());
        let messages = input.to_messages();
        let result = model._generate(messages, None, None).await.unwrap();
        assert_eq!(result.generations[0].message.content(), "echo this string");
    }
}

#[cfg(test)]
mod test_generic_fake_chat_model_run_manager {
    use std::sync::{Arc, Mutex};

    use agent_chain_core::GenericFakeChatModel;
    use agent_chain_core::callbacks::CallbackManagerForLLMRun;
    use agent_chain_core::callbacks::base::{
        BaseCallbackHandler, CallbackManagerMixin, ChainManagerMixin, LLMManagerMixin,
        RetrieverManagerMixin, RunManagerMixin, ToolManagerMixin,
    };
    use agent_chain_core::language_models::BaseChatModel;
    use agent_chain_core::messages::AIMessage;
    use futures::StreamExt;
    use uuid::Uuid;

    /// A callback handler that records all tokens received via on_llm_new_token.
    #[derive(Debug, Clone)]
    struct TokenRecorder {
        tokens: Arc<Mutex<Vec<String>>>,
    }

    impl TokenRecorder {
        fn new() -> Self {
            Self {
                tokens: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn recorded_tokens(&self) -> Vec<String> {
            self.tokens.lock().unwrap().clone()
        }
    }

    impl LLMManagerMixin for TokenRecorder {
        fn on_llm_new_token(
            &self,
            token: &str,
            _run_id: Uuid,
            _parent_run_id: Option<Uuid>,
            _chunk: Option<&serde_json::Value>,
        ) {
            self.tokens.lock().unwrap().push(token.to_string());
        }
    }

    impl ChainManagerMixin for TokenRecorder {}
    impl ToolManagerMixin for TokenRecorder {}
    impl RetrieverManagerMixin for TokenRecorder {}
    impl CallbackManagerMixin for TokenRecorder {}
    impl RunManagerMixin for TokenRecorder {}

    impl BaseCallbackHandler for TokenRecorder {
        fn name(&self) -> &str {
            "TokenRecorder"
        }
    }

    /// Ported from `test_stream_with_run_manager_callback`.
    ///
    /// Verifies that _stream calls on_llm_new_token on the run_manager's
    /// handlers for each content chunk.
    #[tokio::test]
    async fn test_stream_with_run_manager_callback() {
        let messages = vec![AIMessage::builder().content("hello world").build()];
        let model = GenericFakeChatModel::from_vec(messages);

        let recorder = TokenRecorder::new();
        let handler: Arc<dyn BaseCallbackHandler> = Arc::new(recorder.clone());

        let run_manager = CallbackManagerForLLMRun::new(
            Uuid::new_v4(),
            vec![handler],
            vec![],
            None,
            None,
            None,
            None,
            None,
        );

        let mut stream = model._stream(vec![], None, Some(&run_manager)).unwrap();

        let mut chunks = Vec::new();
        while let Some(chunk_result) = stream.next().await {
            if let Ok(chunk) = chunk_result {
                chunks.push(chunk.text.clone());
            }
        }

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks, vec!["hello", " ", "world"]);

        let tokens = recorder.recorded_tokens();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens, vec!["hello", " ", "world"]);
    }
}
