//! Unit tests for the agent-chain crate.
//!
//! Ported from `langchain/libs/partners/openai/tests/unit_tests/chat_models/test_base.py`

use agent_chain::providers::openai::ChatOpenAI;
use agent_chain_core::language_models::BaseLanguageModel;
use agent_chain_core::messages::{
    AIMessage, BaseMessage, HumanMessage, SystemMessage, ToolCall, ToolMessage,
};
use std::collections::HashMap;

/// Ported from `test_openai_model_param`.
#[test]
fn test_openai_model_param() {
    let llm = ChatOpenAI::new("foo");
    assert_eq!(llm.model_name(), "foo");

    let llm = ChatOpenAI::new("foo").max_tokens(10);
    assert_eq!(llm.model_name(), "foo");
}

/// Ported from `test_openai_o1_temperature`.
#[test]
fn test_openai_o1_temperature() {
    let llm = ChatOpenAI::new("o1-preview");
    let params = llm.get_ls_params(None);
    assert_eq!(params.ls_temperature, Some(1.0));

    let llm = ChatOpenAI::new("o1-mini");
    let params = llm.get_ls_params(None);
    assert_eq!(params.ls_temperature, Some(1.0));
}

/// Ported from `test_init_o1`.
#[test]
fn test_init_o1() {
    let _llm = ChatOpenAI::new("o1").reasoning_effort("medium");
}

/// Ported from `test_init_minimal_reasoning_effort`.
#[test]
fn test_init_minimal_reasoning_effort() {
    let _llm = ChatOpenAI::new("gpt-5").reasoning_effort("minimal");
}

/// Ported from `test__get_request_payload` (basic case).
#[test]
fn test_get_request_payload_basic() {
    let llm = ChatOpenAI::new("gpt-4o-2024-08-06");
    let messages = vec![
        BaseMessage::System(SystemMessage::builder().content("hello").build()),
        BaseMessage::Human(HumanMessage::builder().content("how are you").build()),
    ];
    let payload = llm.build_request_payload(&messages, None, None, false);

    assert_eq!(payload["model"], "gpt-4o-2024-08-06");
    let msgs = payload["messages"].as_array().unwrap();
    assert_eq!(msgs[0]["role"], "system");
    assert_eq!(msgs[0]["content"], "hello");
    assert_eq!(msgs[1]["role"], "user");
    assert_eq!(msgs[1]["content"], "how are you");
}

/// Ported from `test_minimal_reasoning_effort_payload` (non-responses API).
#[test]
fn test_minimal_reasoning_effort_payload_chat_completions() {
    let llm = ChatOpenAI::new("gpt-5")
        .reasoning_effort("minimal")
        .max_tokens(100);

    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("hello").build(),
    )];
    let payload = llm.build_request_payload(&messages, None, None, false);

    assert_eq!(payload["reasoning_effort"], "minimal");
    assert_eq!(payload["max_completion_tokens"], 100);
}

/// Ported from `test_minimal_reasoning_effort_payload` (responses API).
#[test]
fn test_minimal_reasoning_effort_payload_responses_api() {
    let llm = ChatOpenAI::new("gpt-5")
        .reasoning_effort("minimal")
        .max_tokens(100)
        .with_responses_api(true);

    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("hello").build(),
    )];
    let payload = llm.build_responses_api_payload(&messages, None, None, false);

    assert!(payload.get("reasoning").is_some());
    assert_eq!(payload["reasoning"]["effort"], "minimal");
    assert_eq!(payload["max_output_tokens"], 100);
}

/// Ported from `test_output_version_compat`.
#[test]
fn test_output_version_compat() {
    let llm = ChatOpenAI::new("gpt-5").output_version("responses/v1");
    assert!(llm.should_use_responses_api(None));
}

/// Ported from `test_verbosity_parameter_payload`.
#[test]
fn test_verbosity_parameter_payload() {
    let llm = ChatOpenAI::new("gpt-5")
        .verbosity("high")
        .with_responses_api(true);

    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("hello").build(),
    )];
    let payload = llm.build_responses_api_payload(&messages, None, None, false);

    assert!(payload.get("text").is_some());
    assert_eq!(payload["text"]["verbosity"], "high");
}

/// Ported from `test_service_tier`.
#[test]
fn test_service_tier() {
    let llm = ChatOpenAI::new("o4-mini").service_tier("flex");
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("Hello").build(),
    )];
    let payload = llm.build_request_payload(&messages, None, None, false);
    assert_eq!(payload["service_tier"], "flex");
}

/// Ported from `test_extra_body_parameter`.
#[test]
fn test_extra_body_parameter() {
    let mut extra = HashMap::new();
    extra.insert("ttl".to_string(), serde_json::json!(300));
    extra.insert("custom_param".to_string(), serde_json::json!("test_value"));

    let llm = ChatOpenAI::new("gpt-4o-mini").extra_body(extra);

    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("Hello").build(),
    )];
    let payload = llm.build_request_payload(&messages, None, None, false);

    assert_eq!(payload["ttl"], 300);
    assert_eq!(payload["custom_param"], "test_value");
}

/// Ported from `test_extra_body_with_model_kwargs`.
#[test]
fn test_extra_body_with_model_kwargs() {
    let mut extra = HashMap::new();
    extra.insert("ttl".to_string(), serde_json::json!(600));

    let mut model_kwargs = HashMap::new();
    model_kwargs.insert(
        "custom_non_openai_param".to_string(),
        serde_json::json!("test_value"),
    );

    let llm = ChatOpenAI::new("gpt-4o-mini")
        .temperature(0.5)
        .extra_body(extra);
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("Hello").build(),
    )];
    let payload = llm.build_request_payload(&messages, None, None, false);

    assert_eq!(payload["ttl"], 600);
    assert_eq!(payload["temperature"], 0.5);
}

/// Ported from `test__create_usage_metadata`.
#[test]
fn test_create_usage_metadata_basic() {
    use agent_chain_core::messages::UsageMetadata;

    let usage = serde_json::from_str::<serde_json::Value>(
        r#"{"prompt_tokens": 11, "completion_tokens": 15, "total_tokens": 26}"#,
    )
    .unwrap();

    let metadata = UsageMetadata::new(
        usage["prompt_tokens"].as_i64().unwrap(),
        usage["completion_tokens"].as_i64().unwrap(),
    );

    assert_eq!(metadata.input_tokens, 11);
    assert_eq!(metadata.output_tokens, 15);
    assert_eq!(metadata.total_tokens, 26);
}

/// Ported from `test__create_usage_metadata_responses`.
#[test]
fn test_create_usage_metadata_responses() {
    use agent_chain_core::messages::UsageMetadata;

    let metadata = UsageMetadata::new(100, 50);
    assert_eq!(metadata.input_tokens, 100);
    assert_eq!(metadata.output_tokens, 50);
    assert_eq!(metadata.total_tokens, 150);
}

/// Ported from `test_model_prefers_responses_api`.
#[test]
fn test_model_prefers_responses_api() {
    let llm = ChatOpenAI::new("gpt-5.2-pro");
    assert!(llm.should_use_responses_api(None));

    let llm = ChatOpenAI::new("gpt-5.1");
    assert!(!llm.should_use_responses_api(None));
}

/// Tests that should_use_responses_api returns true for various conditions.
#[test]
fn test_should_use_responses_api_conditions() {
    let llm = ChatOpenAI::new("gpt-4o").with_responses_api(true);
    assert!(llm.should_use_responses_api(None));

    let llm = ChatOpenAI::new("gpt-4o").with_responses_api(false);
    assert!(!llm.should_use_responses_api(None));

    assert!(
        ChatOpenAI::new("gpt-4o")
            .with_builtin_tools(vec![agent_chain::providers::openai::BuiltinTool::WebSearch])
            .should_use_responses_api(None)
    );

    let mut reasoning = HashMap::new();
    reasoning.insert("effort".to_string(), serde_json::json!("high"));
    let llm = ChatOpenAI::new("gpt-4o").reasoning(reasoning);
    assert!(llm.should_use_responses_api(None));

    let llm = ChatOpenAI::new("gpt-4o").verbosity("high");
    assert!(llm.should_use_responses_api(None));

    let llm = ChatOpenAI::new("gpt-4o").truncation("auto");
    assert!(llm.should_use_responses_api(None));

    let llm = ChatOpenAI::new("gpt-4o").include(vec!["reasoning".to_string()]);
    assert!(llm.should_use_responses_api(None));

    let llm = ChatOpenAI::new("gpt-4o").output_version("responses/v1");
    assert!(llm.should_use_responses_api(None));

    let llm = ChatOpenAI::new("gpt-4o");
    assert!(!llm.should_use_responses_api(None));
}

/// Tests get_ls_params returns correct values.
#[test]
fn test_get_ls_params() {
    let llm = ChatOpenAI::new("gpt-4o").temperature(0.7).max_tokens(100);

    let params = llm.get_ls_params(Some(&["stop1".to_string()]));

    assert_eq!(params.ls_provider, Some("openai".to_string()));
    assert_eq!(params.ls_model_name, Some("gpt-4o".to_string()));
    assert_eq!(params.ls_model_type, Some("chat".to_string()));
    assert_eq!(params.ls_temperature, Some(0.7));
    assert_eq!(params.ls_max_tokens, Some(100));
    assert_eq!(params.ls_stop, Some(vec!["stop1".to_string()]));
}

/// Tests format_messages produces correct OpenAI API format.
#[test]
fn test_format_messages() {
    let llm = ChatOpenAI::new("gpt-4o");
    let messages = vec![
        BaseMessage::System(SystemMessage::builder().content("You are helpful.").build()),
        BaseMessage::Human(HumanMessage::builder().content("Hello!").build()),
        BaseMessage::AI(AIMessage::builder().content("Hi there!").build()),
    ];

    let formatted = llm.format_messages(&messages);

    assert_eq!(formatted.len(), 3);
    assert_eq!(formatted[0]["role"], "system");
    assert_eq!(formatted[0]["content"], "You are helpful.");
    assert_eq!(formatted[1]["role"], "user");
    assert_eq!(formatted[1]["content"], "Hello!");
    assert_eq!(formatted[2]["role"], "assistant");
}

/// Tests format_messages with tool calls on AI message.
#[test]
fn test_format_messages_with_tool_calls() {
    let llm = ChatOpenAI::new("gpt-4o");
    let ai_msg = AIMessage::builder()
        .content("")
        .tool_calls(vec![
            ToolCall::builder()
                .name("get_weather")
                .args(serde_json::json!({"location": "Boston"}))
                .id("call_123".to_string())
                .build(),
        ])
        .build();

    let messages = vec![BaseMessage::AI(ai_msg)];
    let formatted = llm.format_messages(&messages);

    assert_eq!(formatted[0]["role"], "assistant");
    let tool_calls = formatted[0]["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0]["id"], "call_123");
    assert_eq!(tool_calls[0]["type"], "function");
    assert_eq!(tool_calls[0]["function"]["name"], "get_weather");
}

/// Tests format_messages with tool message.
#[test]
fn test_format_messages_tool_message() {
    let llm = ChatOpenAI::new("gpt-4o");
    let messages = vec![BaseMessage::Tool(
        ToolMessage::builder()
            .content("sunny")
            .tool_call_id("call_123")
            .build(),
    )];

    let formatted = llm.format_messages(&messages);

    assert_eq!(formatted[0]["role"], "tool");
    assert_eq!(formatted[0]["tool_call_id"], "call_123");
    assert_eq!(formatted[0]["content"], "sunny");
}

/// Ported from `test__construct_responses_api_input_tool_message_conversion`.
#[test]
fn test_format_messages_for_responses_api_tool_message() {
    let llm = ChatOpenAI::new("gpt-4o");
    let messages = vec![BaseMessage::Tool(
        ToolMessage::builder()
            .content(r#"{"temperature": 72, "conditions": "sunny"}"#)
            .tool_call_id("call_123")
            .build(),
    )];

    let result = llm.format_messages_for_responses_api(&messages);

    assert_eq!(result.len(), 1);
    assert_eq!(result[0]["type"], "function_call_output");
    assert_eq!(result[0]["call_id"], "call_123");
    assert_eq!(
        result[0]["output"],
        r#"{"temperature": 72, "conditions": "sunny"}"#
    );
}

/// Ported from `test__construct_responses_api_input_ai_message_with_tool_calls`.
#[test]
fn test_format_messages_for_responses_api_ai_with_tool_calls() {
    let llm = ChatOpenAI::new("gpt-4o");
    let ai_msg = AIMessage::builder()
        .content("")
        .tool_calls(vec![
            ToolCall::builder()
                .name("get_weather")
                .args(serde_json::json!({"location": "San Francisco"}))
                .id("call_123".to_string())
                .build(),
        ])
        .build();

    let result = llm.format_messages_for_responses_api(&[BaseMessage::AI(ai_msg)]);

    assert!(!result.is_empty());
    let function_call = result.iter().find(|r| r["type"] == "function_call");
    assert!(function_call.is_some());
    let fc = function_call.unwrap();
    assert_eq!(fc["name"], "get_weather");
    assert_eq!(fc["call_id"], "call_123");
}

/// Ported from `test__construct_responses_api_input_multiple_message_types`.
#[test]
fn test_format_messages_for_responses_api_multiple_types() {
    let llm = ChatOpenAI::new("gpt-4o");
    let messages = vec![
        BaseMessage::System(
            SystemMessage::builder()
                .content("You are a helpful assistant.")
                .build(),
        ),
        BaseMessage::Human(
            HumanMessage::builder()
                .content("What's the weather?")
                .build(),
        ),
        BaseMessage::AI(
            AIMessage::builder()
                .content("")
                .tool_calls(vec![
                    ToolCall::builder()
                        .name("get_weather")
                        .args(serde_json::json!({"location": "SF"}))
                        .id("call_123".to_string())
                        .build(),
                ])
                .build(),
        ),
        BaseMessage::Tool(
            ToolMessage::builder()
                .content("Sunny, 72F")
                .tool_call_id("call_123")
                .build(),
        ),
    ];

    let result = llm.format_messages_for_responses_api(&messages);

    assert_eq!(result[0]["role"], "system");
    assert_eq!(result[1]["role"], "user");
    let has_function_call = result.iter().any(|r| r["type"] == "function_call");
    let has_function_output = result.iter().any(|r| r["type"] == "function_call_output");
    assert!(has_function_call);
    assert!(has_function_output);
}

/// Tests all builder methods work without panicking.
#[test]
fn test_builder_methods() {
    let _llm = ChatOpenAI::new("gpt-4o")
        .temperature(0.7)
        .max_tokens(1024)
        .api_base("https://custom.api.com/v1")
        .organization("org-123")
        .top_p(0.9)
        .frequency_penalty(0.5)
        .presence_penalty(0.3)
        .stop(vec!["STOP".to_string()])
        .timeout(30)
        .max_retries(3)
        .streaming(true)
        .seed(42)
        .logprobs(true)
        .top_logprobs(5)
        .n(2)
        .reasoning_effort("high")
        .stream_usage(true)
        .service_tier("flex")
        .store(true)
        .truncation("auto")
        .use_previous_response_id(true)
        .output_version("v1")
        .with_responses_api(true);
}

/// Tests llm_type returns correct value.
#[test]
fn test_llm_type() {
    let llm = ChatOpenAI::new("gpt-4o");
    assert_eq!(llm.llm_type(), "openai-chat");
}
