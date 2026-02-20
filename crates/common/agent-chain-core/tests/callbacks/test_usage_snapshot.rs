use std::collections::HashMap;

use agent_chain_core::callbacks::base::{BaseCallbackHandler, LLMManagerMixin};
use agent_chain_core::callbacks::usage::{
    UsageMetadataCallbackHandler, get_usage_metadata_callback,
};
use agent_chain_core::messages::{AIMessage, InputTokenDetails, OutputTokenDetails, UsageMetadata};
use agent_chain_core::outputs::{ChatGeneration, ChatResult};
use uuid::Uuid;

fn make_chat_result(content: &str, usage: &UsageMetadata, model_name: &str) -> ChatResult {
    let mut response_metadata = HashMap::new();
    response_metadata.insert("model_name".to_string(), serde_json::json!(model_name));

    let ai_msg = AIMessage::builder()
        .content(content)
        .usage_metadata(usage.clone())
        .response_metadata(response_metadata)
        .build();

    ChatResult {
        generations: vec![ChatGeneration::new(ai_msg.into())],
        llm_output: None,
    }
}

#[test]
fn test_empty_usage_metadata_on_init() {
    let handler = UsageMetadataCallbackHandler::new();
    assert!(handler.usage_metadata().is_empty());
}

#[test]
fn test_handler_is_thread_safe() {
    let handler1 = UsageMetadataCallbackHandler::new();
    let handler2 = handler1.clone();

    let usage = UsageMetadata::new(1, 1);
    let result = make_chat_result("x", &usage, "m");
    handler1.on_llm_end(&result, Uuid::new_v4(), None);

    assert_eq!(handler1.usage_metadata(), handler2.usage_metadata());
}

#[test]
fn test_display_empty() {
    let handler = UsageMetadataCallbackHandler::new();
    let repr = format!("{}", handler);
    assert_eq!(repr, "{}");
}

#[test]
fn test_collects_single_response() {
    let usage = UsageMetadata::new(10, 5);
    let result = make_chat_result("hi", &usage, "model-a");
    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&result, Uuid::new_v4(), None);

    let metadata = handler.usage_metadata();
    assert_eq!(metadata.len(), 1);
    assert_eq!(metadata.get("model-a").unwrap(), &usage);
}

#[test]
fn test_accumulates_multiple_responses_same_model() {
    let u1 = UsageMetadata::new(10, 5);
    let u2 = UsageMetadata::new(20, 10);
    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&make_chat_result("a", &u1, "model-a"), Uuid::new_v4(), None);
    handler.on_llm_end(&make_chat_result("b", &u2, "model-a"), Uuid::new_v4(), None);

    let expected = u1.add(&u2);
    assert_eq!(handler.usage_metadata().get("model-a").unwrap(), &expected);
}

#[test]
fn test_tracks_multiple_models() {
    let u1 = UsageMetadata::new(10, 5);
    let u2 = UsageMetadata::new(20, 10);
    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&make_chat_result("a", &u1, "model-a"), Uuid::new_v4(), None);
    handler.on_llm_end(&make_chat_result("b", &u2, "model-b"), Uuid::new_v4(), None);

    let metadata = handler.usage_metadata();
    assert_eq!(metadata.len(), 2);
    assert_eq!(metadata.get("model-a").unwrap(), &u1);
    assert_eq!(metadata.get("model-b").unwrap(), &u2);
}

#[test]
fn test_with_token_details() {
    let usage = UsageMetadata {
        input_tokens: 10,
        output_tokens: 5,
        total_tokens: 15,
        input_token_details: Some(InputTokenDetails {
            audio: Some(3),
            cache_creation: None,
            cache_read: Some(2),
            ..Default::default()
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: None,
            reasoning: Some(4),
            ..Default::default()
        }),
    };
    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&make_chat_result("a", &usage, "m"), Uuid::new_v4(), None);

    let stored = handler.usage_metadata();
    let stored = stored.get("m").unwrap();
    let input_details = stored.input_token_details.as_ref().unwrap();
    assert_eq!(input_details.audio, Some(3));
    assert_eq!(input_details.cache_read, Some(2));
    let output_details = stored.output_token_details.as_ref().unwrap();
    assert_eq!(output_details.reasoning, Some(4));
}

#[test]
fn test_empty_generations_ignored() {
    let result = ChatResult {
        generations: vec![],
        llm_output: None,
    };
    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&result, Uuid::new_v4(), None);
    assert!(handler.usage_metadata().is_empty());
}

#[test]
fn test_no_generations_at_all() {
    let result = ChatResult {
        generations: Vec::new(),
        llm_output: None,
    };
    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&result, Uuid::new_v4(), None);
    assert!(handler.usage_metadata().is_empty());
}

#[test]
fn test_non_ai_message_ignored() {
    use agent_chain_core::messages::HumanMessage;

    let human_msg = HumanMessage::builder().content("hello").build();
    let generation = ChatGeneration::new(human_msg.into());

    let result = ChatResult {
        generations: vec![generation],
        llm_output: None,
    };

    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&result, Uuid::new_v4(), None);
    assert!(handler.usage_metadata().is_empty());
}

#[test]
fn test_missing_model_name_ignored() {
    let ai_msg = AIMessage::builder()
        .content("hi")
        .usage_metadata(UsageMetadata::new(1, 1))
        .build();

    let result = ChatResult {
        generations: vec![ChatGeneration::new(ai_msg.into())],
        llm_output: None,
    };

    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&result, Uuid::new_v4(), None);
    assert!(handler.usage_metadata().is_empty());
}

#[test]
fn test_missing_usage_metadata_ignored() {
    let mut response_metadata = HashMap::new();
    response_metadata.insert("model_name".to_string(), serde_json::json!("m"));

    let ai_msg = AIMessage::builder()
        .content("hi")
        .response_metadata(response_metadata)
        .build();

    let result = ChatResult {
        generations: vec![ChatGeneration::new(ai_msg.into())],
        llm_output: None,
    };

    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&result, Uuid::new_v4(), None);
    assert!(handler.usage_metadata().is_empty());
}

#[test]
fn test_display_with_data() {
    let usage = UsageMetadata::new(1, 2);
    let handler = UsageMetadataCallbackHandler::new();
    handler.on_llm_end(&make_chat_result("a", &usage, "m"), Uuid::new_v4(), None);

    let repr = format!("{}", handler);
    assert!(
        repr.contains("m"),
        "Display should contain model name, got: {repr}"
    );
}

#[test]
fn test_concurrent_on_llm_end_calls() {
    let handler = UsageMetadataCallbackHandler::new();
    let usage = UsageMetadata::new(1, 1);
    let num_threads = 20;
    let iterations_per_thread = 50;

    let mut handles = Vec::new();
    for _ in 0..num_threads {
        let handler_clone = handler.clone();
        let usage_clone = usage.clone();
        let handle = std::thread::spawn(move || {
            for _ in 0..iterations_per_thread {
                let result = make_chat_result("x", &usage_clone, "m");
                handler_clone.on_llm_end(&result, Uuid::new_v4(), None);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let stored = handler.usage_metadata();
    let model_usage = stored.get("m").unwrap();
    let expected_count = (num_threads * iterations_per_thread) as i64;
    assert_eq!(model_usage.input_tokens, expected_count);
    assert_eq!(model_usage.output_tokens, expected_count);
    assert_eq!(model_usage.total_tokens, expected_count * 2);
}

#[test]
fn test_guard_yields_handler() {
    let guard = get_usage_metadata_callback();
    assert_eq!(guard.handler().name(), "UsageMetadataCallbackHandler");
}

#[test]
fn test_guard_handler_starts_empty() {
    let guard = get_usage_metadata_callback();
    assert!(guard.usage_metadata().is_empty());
}

#[test]
fn test_guard_returns_valid_handler() {
    let guard = get_usage_metadata_callback();
    let arc = guard.as_arc_handler();
    assert_eq!(arc.name(), "UsageMetadataCallbackHandler");
}

#[test]
fn test_multiple_guards_independent() {
    let guard1 = get_usage_metadata_callback();
    let guard2 = get_usage_metadata_callback();

    let usage = UsageMetadata::new(5, 5);
    guard1
        .handler()
        .on_llm_end(&make_chat_result("a", &usage, "m"), Uuid::new_v4(), None);

    assert!(guard2.usage_metadata().is_empty());
    assert_eq!(guard1.usage_metadata().len(), 1);
}
