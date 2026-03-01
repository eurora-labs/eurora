use std::collections::HashMap;

use agent_chain_core::callbacks::base::LLMManagerMixin;
use agent_chain_core::callbacks::usage::{
    UsageMetadataCallbackHandler, get_usage_metadata_callback,
};
use agent_chain_core::messages::{AIMessage, InputTokenDetails, OutputTokenDetails, UsageMetadata};
use agent_chain_core::outputs::{ChatGeneration, ChatResult};
use uuid::Uuid;

fn usage1() -> UsageMetadata {
    UsageMetadata {
        input_tokens: 1,
        output_tokens: 2,
        total_tokens: 3,
        input_token_details: None,
        output_token_details: None,
    }
}

fn usage2() -> UsageMetadata {
    UsageMetadata {
        input_tokens: 4,
        output_tokens: 5,
        total_tokens: 9,
        input_token_details: None,
        output_token_details: None,
    }
}

fn usage3() -> UsageMetadata {
    UsageMetadata {
        input_tokens: 10,
        output_tokens: 20,
        total_tokens: 30,
        input_token_details: Some(InputTokenDetails {
            audio: Some(5),
            cache_creation: None,
            cache_read: None,
            ..Default::default()
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: None,
            reasoning: Some(10),
            ..Default::default()
        }),
    }
}

fn usage4() -> UsageMetadata {
    UsageMetadata {
        input_tokens: 5,
        output_tokens: 10,
        total_tokens: 15,
        input_token_details: Some(InputTokenDetails {
            audio: Some(3),
            cache_creation: None,
            cache_read: None,
            ..Default::default()
        }),
        output_token_details: Some(OutputTokenDetails {
            audio: None,
            reasoning: Some(5),
            ..Default::default()
        }),
    }
}

fn create_chat_result(content: &str, model_name: &str, usage: &UsageMetadata) -> ChatResult {
    let mut response_metadata = HashMap::new();
    response_metadata.insert("model_name".to_string(), serde_json::json!(model_name));

    let ai_msg = AIMessage::builder()
        .content(content)
        .usage_metadata(usage.clone())
        .response_metadata(response_metadata)
        .build();

    ChatResult {
        generations: vec![ChatGeneration::builder().message(ai_msg.into()).build()],
        llm_output: None,
    }
}

#[test]
fn test_usage_callback_accumulation() {
    let callback = get_usage_metadata_callback();
    let handler = callback.handler();

    let result1 = create_chat_result("Response 1", "test_model", &usage1());
    handler.on_llm_end(&result1, Uuid::new_v4(), None);

    let result2 = create_chat_result("Response 2", "test_model", &usage2());
    handler.on_llm_end(&result2, Uuid::new_v4(), None);

    let total_1_2 = usage1().add(&usage2());
    let metadata = callback.usage_metadata();
    assert_eq!(metadata.len(), 1);
    assert_eq!(
        metadata.get("test_model").unwrap(),
        &total_1_2,
        "After 2 invocations, usage should be sum of usage1 and usage2"
    );

    let result3 = create_chat_result("Response 3", "test_model", &usage3());
    handler.on_llm_end(&result3, Uuid::new_v4(), None);

    let result4 = create_chat_result("Response 4", "test_model", &usage4());
    handler.on_llm_end(&result4, Uuid::new_v4(), None);

    let total_3_4 = usage3().add(&usage4());
    let expected = total_1_2.add(&total_3_4);
    let metadata = callback.usage_metadata();
    assert_eq!(
        metadata.get("test_model").unwrap(),
        &expected,
        "After 4 invocations, usage should be sum of all"
    );
}

#[test]
fn test_usage_callback_via_handler() {
    let callback = UsageMetadataCallbackHandler::new();

    let result1 = create_chat_result("Response 1", "test_model", &usage1());
    let result2 = create_chat_result("Response 2", "test_model", &usage2());
    callback.on_llm_end(&result1, Uuid::new_v4(), None);
    callback.on_llm_end(&result2, Uuid::new_v4(), None);

    let total_1_2 = usage1().add(&usage2());
    assert_eq!(
        callback.usage_metadata(),
        HashMap::from([("test_model".to_string(), total_1_2)])
    );
}

#[test]
fn test_usage_callback_multiple_models() {
    let callback = UsageMetadataCallbackHandler::new();

    let result1 = create_chat_result("Response 1", "test_model_1", &usage1());
    let result2 = create_chat_result("Response 2", "test_model_1", &usage2());
    callback.on_llm_end(&result1, Uuid::new_v4(), None);
    callback.on_llm_end(&result2, Uuid::new_v4(), None);

    let result3 = create_chat_result("Response 3", "test_model_2", &usage3());
    let result4 = create_chat_result("Response 4", "test_model_2", &usage4());
    callback.on_llm_end(&result3, Uuid::new_v4(), None);
    callback.on_llm_end(&result4, Uuid::new_v4(), None);

    let total_1_2 = usage1().add(&usage2());
    let total_3_4 = usage3().add(&usage4());

    let metadata = callback.usage_metadata();
    assert_eq!(metadata.len(), 2);
    assert_eq!(
        metadata.get("test_model_1").unwrap(),
        &total_1_2,
        "test_model_1 should have usage1 + usage2"
    );
    assert_eq!(
        metadata.get("test_model_2").unwrap(),
        &total_3_4,
        "test_model_2 should have usage3 + usage4"
    );
}

#[test]
fn test_usage_callback_token_details_accumulation() {
    let callback = UsageMetadataCallbackHandler::new();

    let result3 = create_chat_result("Response 3", "test_model", &usage3());
    let result4 = create_chat_result("Response 4", "test_model", &usage4());
    callback.on_llm_end(&result3, Uuid::new_v4(), None);
    callback.on_llm_end(&result4, Uuid::new_v4(), None);

    let metadata = callback.usage_metadata();
    let usage = metadata.get("test_model").unwrap();

    assert_eq!(usage.input_tokens, 15);
    assert_eq!(usage.output_tokens, 30);
    assert_eq!(usage.total_tokens, 45);

    let input_details = usage.input_token_details.as_ref().unwrap();
    assert_eq!(input_details.audio, Some(8));

    let output_details = usage.output_token_details.as_ref().unwrap();
    assert_eq!(output_details.reasoning, Some(15));
}

#[test]
fn test_get_usage_metadata_callback_guard() {
    let guard = get_usage_metadata_callback();

    assert!(guard.usage_metadata().is_empty());

    let arc_handler = guard.as_arc_handler();
    assert_eq!(arc_handler.name(), "UsageMetadataCallbackHandler");
}

#[test]
fn test_usage_callback_no_model_name() {
    let callback = UsageMetadataCallbackHandler::new();

    let ai_msg = AIMessage::builder()
        .content("Response")
        .usage_metadata(usage1())
        .build();

    let result = ChatResult {
        generations: vec![ChatGeneration::builder().message(ai_msg.into()).build()],
        llm_output: None,
    };

    callback.on_llm_end(&result, Uuid::new_v4(), None);

    assert!(callback.usage_metadata().is_empty());
}

#[test]
fn test_usage_callback_no_usage_metadata() {
    let callback = UsageMetadataCallbackHandler::new();

    let mut response_metadata = HashMap::new();
    response_metadata.insert("model_name".to_string(), serde_json::json!("test_model"));

    let ai_msg = AIMessage::builder()
        .content("Response")
        .response_metadata(response_metadata)
        .build();

    let result = ChatResult {
        generations: vec![ChatGeneration::builder().message(ai_msg.into()).build()],
        llm_output: None,
    };

    callback.on_llm_end(&result, Uuid::new_v4(), None);

    assert!(callback.usage_metadata().is_empty());
}
