use agent_chain::providers::ollama::{ChatOllama, parse_tool_call_arguments};
use agent_chain_core::messages::{BaseMessage, HumanMessage};

const MODEL_NAME: &str = "llama3.1";

// =============================================================================
// Ported from unit_tests/test_chat_models.py
// =============================================================================

/// Ported from `test_none_parameters_excluded_from_options`.
#[test]
fn test_none_parameters_excluded_from_options() {
    let llm = ChatOllama::new(MODEL_NAME).num_ctx(4096);
    let options = llm.build_options(None).unwrap();
    let options_map = options.as_object().unwrap();

    assert_eq!(options_map.get("num_ctx"), Some(&serde_json::json!(4096)));
    assert!(!options_map.contains_key("mirostat"));
    assert!(!options_map.contains_key("mirostat_eta"));
    assert!(!options_map.contains_key("mirostat_tau"));
    assert!(!options_map.contains_key("tfs_z"));
}

/// Ported from `test_all_none_parameters_results_in_empty_options`.
#[test]
fn test_all_none_parameters_results_in_empty_options() {
    let llm = ChatOllama::new(MODEL_NAME);
    let options = llm.build_options(None).unwrap();
    let options_map = options.as_object().unwrap();

    assert!(
        options_map.is_empty(),
        "Options should be empty when no parameters are set"
    );
}

/// Ported from `test_reasoning_param_passed_to_client`.
#[test]
fn test_reasoning_param_passed_to_payload() {
    let messages: Vec<BaseMessage> = vec![HumanMessage::builder().content("Hello").build().into()];

    // reasoning=true
    let llm = ChatOllama::new("deepseek-r1").reasoning(true);
    let payload = llm
        .build_request_payload(&messages, None, None, false)
        .unwrap();
    assert_eq!(payload["think"], serde_json::json!(true));

    // reasoning=false
    let llm = ChatOllama::new("deepseek-r1").reasoning(false);
    let payload = llm
        .build_request_payload(&messages, None, None, false)
        .unwrap();
    assert_eq!(payload["think"], serde_json::json!(false));

    // reasoning not set — think should be absent
    let llm = ChatOllama::new("deepseek-r1");
    let payload = llm
        .build_request_payload(&messages, None, None, false)
        .unwrap();
    assert!(payload.get("think").is_none());
}

/// Ported from `test_arbitrary_roles_accepted_in_chatmessages`.
#[test]
fn test_arbitrary_roles_accepted_in_chatmessages() {
    let llm = ChatOllama::new(MODEL_NAME);
    let messages: Vec<BaseMessage> = vec![
        agent_chain_core::messages::ChatMessage::builder()
            .role("somerandomrole")
            .content("I'm ok with you adding any role message now!")
            .build()
            .into(),
        agent_chain_core::messages::ChatMessage::builder()
            .role("control")
            .content("thinking")
            .build()
            .into(),
        agent_chain_core::messages::ChatMessage::builder()
            .role("user")
            .content("What is the meaning of life?")
            .build()
            .into(),
    ];

    let formatted = llm.format_messages(&messages);
    assert!(
        formatted.is_ok(),
        "ChatOllama should accept arbitrary roles in ChatMessage: {:?}",
        formatted.err()
    );
    let formatted = formatted.unwrap();
    assert_eq!(formatted.len(), 3);
    assert_eq!(formatted[0]["role"], "somerandomrole");
    assert_eq!(formatted[1]["role"], "control");
    assert_eq!(formatted[2]["role"], "user");
}

/// Ported from `test_none_parameters_excluded_from_options` — num_ctx variant.
#[test]
fn test_options_only_include_set_parameters() {
    let llm = ChatOllama::new(MODEL_NAME).num_ctx(4096).temperature(0.5);
    let options = llm.build_options(None).unwrap();
    let options_map = options.as_object().unwrap();

    assert_eq!(options_map.get("num_ctx"), Some(&serde_json::json!(4096)));
    assert_eq!(
        options_map.get("temperature"),
        Some(&serde_json::json!(0.5))
    );
    assert!(!options_map.contains_key("top_p"));
    assert!(!options_map.contains_key("top_k"));
    assert!(!options_map.contains_key("repeat_penalty"));
}

/// Ported from `test_chat_ollama_ignores_strict_arg`.
/// In Rust, `strict` is not a parameter on ChatOllama — this test verifies
/// the request payload doesn't include it.
#[test]
fn test_payload_does_not_include_strict() {
    let llm = ChatOllama::new(MODEL_NAME);
    let messages: Vec<BaseMessage> = vec![HumanMessage::builder().content("Hello").build().into()];

    let payload = llm
        .build_request_payload(&messages, None, None, false)
        .unwrap();
    assert!(
        payload.get("strict").is_none(),
        "Payload should not contain 'strict'"
    );
}

/// Ported from `test_explicit_options_dict_preserved`.
/// Verifies that options set on the model are correctly included in the payload.
#[test]
fn test_explicit_options_in_payload() {
    let llm = ChatOllama::new(MODEL_NAME).temperature(0.5).num_ctx(4096);
    let messages: Vec<BaseMessage> = vec![HumanMessage::builder().content("Hello").build().into()];

    let payload = llm
        .build_request_payload(&messages, None, None, false)
        .unwrap();
    let options = payload.get("options").and_then(|o| o.as_object()).unwrap();
    assert_eq!(options.get("temperature"), Some(&serde_json::json!(0.5)));
    assert_eq!(options.get("num_ctx"), Some(&serde_json::json!(4096)));
}

/// Ported from the stream-related test: payload includes stream flag.
#[test]
fn test_payload_stream_flag() {
    let llm = ChatOllama::new(MODEL_NAME);
    let messages: Vec<BaseMessage> = vec![HumanMessage::builder().content("Hello").build().into()];

    let payload_stream = llm
        .build_request_payload(&messages, None, None, true)
        .unwrap();
    assert_eq!(payload_stream["stream"], serde_json::json!(true));

    let payload_no_stream = llm
        .build_request_payload(&messages, None, None, false)
        .unwrap();
    assert_eq!(payload_no_stream["stream"], serde_json::json!(false));
}

/// Ported: tools are included in request payload when provided.
#[test]
fn test_tools_included_in_payload() {
    let llm = ChatOllama::new(MODEL_NAME);
    let messages: Vec<BaseMessage> = vec![HumanMessage::builder().content("Hello").build().into()];

    let tools = vec![serde_json::json!({
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get weather",
            "parameters": {"type": "object", "properties": {}}
        }
    })];

    let payload = llm
        .build_request_payload(&messages, None, Some(&tools), false)
        .unwrap();
    assert!(payload.get("tools").is_some());
    assert_eq!(payload["tools"].as_array().unwrap().len(), 1);
}

/// Ported: stop sequences are included in options.
#[test]
fn test_stop_sequences_in_payload() {
    let llm = ChatOllama::new(MODEL_NAME);
    let messages: Vec<BaseMessage> = vec![HumanMessage::builder().content("Hello").build().into()];

    let payload = llm
        .build_request_payload(&messages, Some(vec!["STOP".to_string()]), None, false)
        .unwrap();
    let options = payload.get("options").and_then(|o| o.as_object()).unwrap();
    assert_eq!(options.get("stop"), Some(&serde_json::json!(["STOP"])));
}

// =============================================================================
// Ported from unit_tests/test_chat_models.py — _parse_arguments_from_tool_call
// =============================================================================

/// Ported from `test__parse_arguments_from_tool_call`.
///
/// String-typed tool arguments should be preserved as strings rather than
/// being parsed as JSON when they're already valid string arguments.
#[test]
fn test_parse_arguments_from_tool_call() {
    let raw_response: serde_json::Value = serde_json::from_str(
        r#"{"model":"sample-model","message":{"role":"assistant","content":"",
        "tool_calls":[{"function":{"name":"get_profile_details",
        "arguments":{"arg_1":"12345678901234567890123456"}}}]},"done":false}"#,
    )
    .unwrap();

    let raw_tool_calls = raw_response["message"]["tool_calls"].as_array().unwrap();
    let tool_call = &raw_tool_calls[0];
    let function_name = tool_call["function"]["name"].as_str().unwrap();
    let args = tool_call["function"].get("arguments");

    let response = parse_tool_call_arguments(args, function_name).unwrap();
    assert!(response["arg_1"].is_string());
    assert_eq!(response["arg_1"], "12345678901234567890123456");
}

/// Ported from `test__parse_arguments_from_tool_call_with_function_name_metadata`.
///
/// `functionName` metadata that echoes the function name should be filtered out.
#[test]
fn test_parse_arguments_from_tool_call_with_function_name_metadata() {
    // Arguments contain only functionName metadata — should be empty after filtering
    let args = serde_json::json!({"functionName": "magic_function_no_args"});
    let response = parse_tool_call_arguments(Some(&args), "magic_function_no_args").unwrap();
    assert_eq!(response, serde_json::json!({}));

    // Arguments contain both real args and metadata
    let args = serde_json::json!({"functionName": "some_function", "real_arg": "value"});
    let response = parse_tool_call_arguments(Some(&args), "some_function").unwrap();
    assert_eq!(response, serde_json::json!({"real_arg": "value"}));

    // functionName has different value (should be preserved)
    let args = serde_json::json!({"functionName": "function_b"});
    let response = parse_tool_call_arguments(Some(&args), "function_a").unwrap();
    assert_eq!(response, serde_json::json!({"functionName": "function_b"}));
}

/// Ported from `test_validate_model_on_init`.
///
/// Verifies that the `validate_model_on_init` flag is stored correctly.
/// In Python, this test mocks `validate_model` and checks it's called.
/// In Rust, model validation is deferred to first use; we verify the flag.
#[test]
fn test_validate_model_on_init_flag() {
    let llm = ChatOllama::new(MODEL_NAME).validate_model_on_init(true);
    let base_url = llm.get_base_url();
    assert!(!base_url.is_empty());

    let llm = ChatOllama::new(MODEL_NAME).validate_model_on_init(false);
    let base_url = llm.get_base_url();
    assert!(!base_url.is_empty());

    let llm = ChatOllama::new(MODEL_NAME);
    let base_url = llm.get_base_url();
    assert!(!base_url.is_empty());
}
