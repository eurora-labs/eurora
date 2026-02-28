//! Ollama integration tests.
//!
//! Ported from `langchain_ollama/tests/integration_tests/chat_models/test_chat_models.py`
//! and `test_chat_models_reasoning.py`.
//!
//! These tests require a running Ollama instance with the appropriate models pulled.
//! Run with: `cargo test --package agent-chain --test ollama_integration_tests -- --ignored`

use agent_chain::providers::ollama::{ChatOllama, OllamaFormat};
use agent_chain_core::language_models::ToolLike;
use agent_chain_core::language_models::chat_models::BaseChatModel;
use agent_chain_core::messages::{BaseMessage, HumanMessage};
use futures::StreamExt;

const DEFAULT_MODEL: &str = "llama3.1";
const REASONING_MODEL: &str = "deepseek-r1:1.5b";
const SAMPLE_PROMPT: &str = "What is 3^3?";

fn load_env() {
    let _ = dotenv::dotenv();
}

// =============================================================================
// test_chat_models.py — Core ChatOllama tests
// =============================================================================

/// Ported from `test_structured_output` with method="function_calling".
#[tokio::test]
#[ignore]
async fn test_structured_output_function_calling() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let joke_schema = serde_json::json!({
        "title": "Joke",
        "description": "Joke to tell user.",
        "type": "object",
        "properties": {
            "setup": {"type": "string", "description": "question to set up a joke"},
            "punchline": {"type": "string", "description": "answer to resolve the joke"}
        },
        "required": ["setup", "punchline"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);
    let structured = llm.with_structured_output(joke_schema, false)?;
    let result = structured
        .ainvoke(
            vec![
                HumanMessage::builder()
                    .content("Tell me a joke about cats.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(result.get("setup").is_some());
    assert!(result.get("punchline").is_some());

    Ok(())
}

/// Ported from `test_structured_output` with method="json_schema".
#[tokio::test]
#[ignore]
async fn test_structured_output_json_schema() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let joke_schema = serde_json::json!({
        "title": "Joke",
        "description": "Joke to tell user.",
        "type": "object",
        "properties": {
            "setup": {"type": "string", "description": "question to set up a joke"},
            "punchline": {"type": "string", "description": "answer to resolve the joke"}
        },
        "required": ["setup", "punchline"]
    });

    // json_schema method uses Ollama's format parameter instead of tool calling.
    // In Rust this requires with_structured_output_options with method="json_schema".
    // For now, use the format builder + JSON parsing to match the Python behavior.
    let llm_with_format = ChatOllama::new(DEFAULT_MODEL)
        .temperature(0.0)
        .format(OllamaFormat::JsonSchema(joke_schema.clone()));

    let response = llm_with_format
        .invoke(
            vec![HumanMessage::builder()
                .content("Tell me a joke about cats. Respond as JSON with 'setup' and 'punchline' keys.")
                .build()
                .into()]
            .into(),
            None,
        )
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&response.text())?;
    assert!(parsed.get("setup").is_some());
    assert!(parsed.get("punchline").is_some());

    Ok(())
}

/// Ported from `test_structured_output_deeply_nested`.
#[tokio::test]
#[ignore]
async fn test_structured_output_deeply_nested() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let data_schema = serde_json::json!({
        "title": "Data",
        "description": "Extracted data about people.",
        "type": "object",
        "properties": {
            "people": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "The name of the person"},
                        "hair_color": {"type": "string", "description": "The color of the person's hair if known"},
                        "height_in_meters": {"type": "string", "description": "Height measured in meters"}
                    }
                }
            }
        },
        "required": ["people"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);
    let structured = llm.with_structured_output(data_schema, false)?;

    let result = structured
        .ainvoke(
            vec![
                HumanMessage::builder()
                    .content(
                        "Alan Smith is 6 feet tall and has blond hair. \
                     Alan Poe is 3 feet tall and has grey hair.",
                    )
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;

    assert!(result.get("people").is_some());
    let people = result
        .get("people")
        .and_then(|p| p.as_array())
        .expect("people should be an array");
    assert!(people.len() >= 2);

    Ok(())
}

/// Ported from `test_tool_streaming`.
#[tokio::test]
#[ignore]
async fn test_tool_streaming() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let weather_tool = serde_json::json!({
        "title": "get_current_weather",
        "description": "Gets the current weather in a given location.",
        "type": "object",
        "properties": {
            "location": {"type": "string", "description": "The location to get weather for"}
        },
        "required": ["location"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(weather_tool)], None)?;

    let mut stream = llm_with_tools
        .astream(
            vec![
                HumanMessage::builder()
                    .content("What is the weather today in Boston?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut final_tool_calls = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        final_tool_calls.extend(chunk.tool_calls.clone());
    }

    assert!(
        !final_tool_calls.is_empty(),
        "Tool streaming should produce at least one tool call"
    );
    assert_eq!(final_tool_calls[0].name, "get_current_weather");

    Ok(())
}

/// Ported from `test_tool_astreaming`.
#[tokio::test]
#[ignore]
async fn test_tool_astreaming() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let weather_tool = serde_json::json!({
        "title": "get_current_weather",
        "description": "Gets the current weather in a given location.",
        "type": "object",
        "properties": {
            "location": {"type": "string", "description": "The location to get weather for"}
        },
        "required": ["location"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(weather_tool)], None)?;

    let mut stream = llm_with_tools
        .astream(
            vec![
                HumanMessage::builder()
                    .content("What is the weather today in Boston?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut final_tool_calls = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        final_tool_calls.extend(chunk.tool_calls.clone());
    }

    assert!(
        !final_tool_calls.is_empty(),
        "Async tool streaming should produce at least one tool call"
    );
    assert_eq!(final_tool_calls[0].name, "get_current_weather");

    Ok(())
}

/// Ported from `test_agent_loop` with output_version=None.
#[tokio::test]
#[ignore]
async fn test_agent_loop() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let weather_tool = serde_json::json!({
        "title": "get_weather",
        "description": "Get the weather for a location.",
        "type": "object",
        "properties": {
            "location": {"type": "string"}
        },
        "required": ["location"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL).reasoning(serde_json::json!("low"));
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(weather_tool)], None)?;

    let input_message: BaseMessage = HumanMessage::builder()
        .content("What is the weather in San Francisco, CA?")
        .build()
        .into();

    let tool_call_message = llm_with_tools
        .invoke(vec![input_message.clone()].into(), None)
        .await?;
    assert!(
        !tool_call_message.tool_calls.is_empty(),
        "Model should produce a tool call"
    );

    let tool_call = &tool_call_message.tool_calls[0];
    assert_eq!(tool_call.name, "get_weather");
    assert!(tool_call.args.get("location").is_some());

    // Simulate tool response
    let tool_message: BaseMessage = agent_chain_core::messages::ToolMessage::builder()
        .content("It's sunny and 75 degrees.")
        .tool_call_id(tool_call.id.as_deref().unwrap_or(""))
        .build()
        .into();

    let resp_message = llm_with_tools
        .invoke(
            vec![
                input_message.clone(),
                BaseMessage::AI(tool_call_message.clone()),
                tool_message.clone(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!resp_message.text().is_empty());

    Ok(())
}

/// Ported from `test_agent_loop` with output_version="v1".
#[tokio::test]
#[ignore]
async fn test_agent_loop_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let weather_tool = serde_json::json!({
        "title": "get_weather",
        "description": "Get the weather for a location.",
        "type": "object",
        "properties": {
            "location": {"type": "string"}
        },
        "required": ["location"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL)
        .output_version("v1")
        .reasoning(serde_json::json!("low"));
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(weather_tool)], None)?;

    let input_message: BaseMessage = HumanMessage::builder()
        .content("What is the weather in San Francisco, CA?")
        .build()
        .into();

    let tool_call_message = llm_with_tools
        .invoke(vec![input_message.clone()].into(), None)
        .await?;
    assert!(
        !tool_call_message.tool_calls.is_empty(),
        "Model should produce a tool call"
    );

    let tool_call = &tool_call_message.tool_calls[0];
    let tool_message: BaseMessage = agent_chain_core::messages::ToolMessage::builder()
        .content("It's sunny and 75 degrees.")
        .tool_call_id(tool_call.id.as_deref().unwrap_or(""))
        .build()
        .into();

    let resp_message = llm_with_tools
        .invoke(
            vec![
                input_message.clone(),
                BaseMessage::AI(tool_call_message.clone()),
                tool_message.clone(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!resp_message.text().is_empty());

    // Follow-up with reasoning
    let follow_up: BaseMessage = HumanMessage::builder()
        .content("Explain why that might be using a reasoning step.")
        .build()
        .into();

    let response = llm_with_tools
        .invoke(
            vec![
                input_message,
                BaseMessage::AI(tool_call_message),
                tool_message,
                BaseMessage::AI(resp_message),
                follow_up,
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty());

    Ok(())
}

// =============================================================================
// test_chat_models_reasoning.py — Reasoning mode tests
// =============================================================================

/// Ported from `test_stream_no_reasoning` (sync).
#[tokio::test]
#[ignore]
async fn test_reasoning_stream_no_reasoning_sync() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(false));

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());
    assert!(
        !full_text.contains("<think>"),
        "Content should not contain <think> tags"
    );
    assert!(
        !full_text.contains("</think>"),
        "Content should not contain </think> tags"
    );

    Ok(())
}

/// Ported from `test_stream_no_reasoning` (async).
#[tokio::test]
#[ignore]
async fn test_reasoning_stream_no_reasoning_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(false));

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());
    assert!(!full_text.contains("<think>"));
    assert!(!full_text.contains("</think>"));

    Ok(())
}

/// Ported from `test_stream_reasoning_none` (sync).
#[tokio::test]
#[ignore]
async fn test_reasoning_stream_none_sync() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL).num_ctx(4096);
    // reasoning=None is the default (not set)

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());

    Ok(())
}

/// Ported from `test_stream_reasoning_none` (async).
#[tokio::test]
#[ignore]
async fn test_reasoning_stream_none_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL).num_ctx(4096);

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());

    Ok(())
}

/// Ported from `test_reasoning_stream` (sync) — reasoning=True.
#[tokio::test]
#[ignore]
async fn test_reasoning_stream_enabled_sync() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(true));

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());
    assert!(!full_text.contains("<think>"));
    assert!(!full_text.contains("</think>"));

    Ok(())
}

/// Ported from `test_reasoning_stream` (async) — reasoning=True.
#[tokio::test]
#[ignore]
async fn test_reasoning_stream_enabled_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(true));

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());
    assert!(!full_text.contains("<think>"));
    assert!(!full_text.contains("</think>"));

    Ok(())
}

/// Ported from `test_invoke_no_reasoning` (sync).
#[tokio::test]
#[ignore]
async fn test_reasoning_invoke_no_reasoning_sync() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(false));

    let result = llm
        .invoke(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());
    assert!(!result.text().contains("<think>"));
    assert!(!result.text().contains("</think>"));
    assert!(
        !result.additional_kwargs.contains_key("reasoning_content"),
        "reasoning_content should not be present when reasoning=false"
    );

    Ok(())
}

/// Ported from `test_invoke_no_reasoning` (async).
#[tokio::test]
#[ignore]
async fn test_reasoning_invoke_no_reasoning_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(false));

    let result = llm
        .ainvoke(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());
    assert!(!result.text().contains("<think>"));
    assert!(
        !result.additional_kwargs.contains_key("reasoning_content"),
        "reasoning_content should not be present when reasoning=false"
    );

    Ok(())
}

/// Ported from `test_invoke_reasoning_none` (sync).
#[tokio::test]
#[ignore]
async fn test_reasoning_invoke_none_sync() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL).num_ctx(4096);
    // reasoning=None (default, not set)

    let result = llm
        .invoke(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());
    assert!(
        !result.additional_kwargs.contains_key("reasoning_content"),
        "reasoning_content should not be captured when reasoning is unset"
    );

    Ok(())
}

/// Ported from `test_invoke_reasoning_none` (async).
#[tokio::test]
#[ignore]
async fn test_reasoning_invoke_none_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL).num_ctx(4096);

    let result = llm
        .ainvoke(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());
    assert!(
        !result.additional_kwargs.contains_key("reasoning_content"),
        "reasoning_content should not be captured when reasoning is unset"
    );

    Ok(())
}

/// Ported from `test_reasoning_invoke` (sync) — reasoning=True.
#[tokio::test]
#[ignore]
async fn test_reasoning_invoke_enabled_sync() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(true));

    let result = llm
        .invoke(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());
    assert!(!result.text().contains("<think>"));
    assert!(!result.text().contains("</think>"));
    assert!(
        result.additional_kwargs.contains_key("reasoning_content"),
        "reasoning_content should be present when reasoning=true"
    );
    let reasoning = result.additional_kwargs["reasoning_content"]
        .as_str()
        .expect("reasoning_content should be a string");
    assert!(!reasoning.is_empty());
    assert!(!reasoning.contains("<think>"));
    assert!(!reasoning.contains("</think>"));

    Ok(())
}

/// Ported from `test_reasoning_invoke` (async) — reasoning=True.
#[tokio::test]
#[ignore]
async fn test_reasoning_invoke_enabled_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(true));

    let result = llm
        .ainvoke(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());
    assert!(!result.text().contains("<think>"));
    assert!(
        result.additional_kwargs.contains_key("reasoning_content"),
        "reasoning_content should be present when reasoning=true"
    );
    let reasoning = result.additional_kwargs["reasoning_content"]
        .as_str()
        .expect("reasoning_content should be a string");
    assert!(!reasoning.is_empty());
    assert!(!reasoning.contains("<think>"));

    Ok(())
}

/// Ported from `test_reasoning_modes_behavior`.
/// Documents behavior differences between reasoning=None, reasoning=false, reasoning=true.
#[tokio::test]
#[ignore]
async fn test_reasoning_modes_behavior() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let message: BaseMessage = HumanMessage::builder()
        .content(SAMPLE_PROMPT)
        .build()
        .into();

    // reasoning=None (default)
    let llm_default = ChatOllama::new(REASONING_MODEL).num_ctx(4096);
    let result_default = llm_default
        .invoke(vec![message.clone()].into(), None)
        .await?;
    assert!(!result_default.text().is_empty());
    assert!(!result_default.text().contains("<think>"));
    assert!(
        !result_default
            .additional_kwargs
            .contains_key("reasoning_content")
    );

    // reasoning=false
    let llm_disabled = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(false));
    let result_disabled = llm_disabled
        .invoke(vec![message.clone()].into(), None)
        .await?;
    assert!(!result_disabled.text().is_empty());
    assert!(!result_disabled.text().contains("<think>"));
    assert!(
        !result_disabled
            .additional_kwargs
            .contains_key("reasoning_content")
    );

    // reasoning=true
    let llm_enabled = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(true));
    let result_enabled = llm_enabled
        .invoke(vec![message.clone()].into(), None)
        .await?;
    assert!(!result_enabled.text().is_empty());
    assert!(!result_enabled.text().contains("<think>"));
    assert!(
        result_enabled
            .additional_kwargs
            .contains_key("reasoning_content")
    );
    let reasoning = result_enabled.additional_kwargs["reasoning_content"]
        .as_str()
        .expect("reasoning_content should be a string");
    assert!(!reasoning.is_empty());
    assert!(!reasoning.contains("<think>"));
    assert!(!reasoning.contains("</think>"));

    Ok(())
}

// =============================================================================
// test_chat_models.py — Init validation tests
// =============================================================================

/// Ported from `test_init_model_not_found`.
/// Tests that validation fails when the model doesn't exist in Ollama.
#[tokio::test]
#[ignore]
async fn test_init_model_not_found() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new("non-existent-model-xyz-12345").validate_model_on_init(true);

    let result = llm.validate_model().await;
    assert!(
        result.is_err(),
        "Validation should fail for non-existent model"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("not found") || err_msg.contains("not available"),
        "Error should mention model not found, got: {err_msg}"
    );

    Ok(())
}

/// Ported from `test_init_connection_error`.
/// Tests that validation fails when Ollama is unreachable.
#[tokio::test]
#[ignore]
async fn test_init_connection_error() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new("any-model")
        .base_url("http://localhost:1")
        .validate_model_on_init(true);

    let result = llm.validate_model().await;
    assert!(
        result.is_err(),
        "Validation should fail when Ollama is unreachable"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("connect") || err_msg.contains("Failed to connect"),
        "Error should mention connection failure, got: {err_msg}"
    );

    Ok(())
}

/// Ported from `test_init_response_error`.
/// Tests that validation fails when the Ollama API returns an error.
/// Uses a valid HTTP server that isn't Ollama to trigger a response error.
#[tokio::test]
#[ignore]
async fn test_init_response_error() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    // Point to a URL that returns HTTP errors (not an Ollama server)
    let llm = ChatOllama::new("any-model")
        .base_url("http://httpbin.org/status/500")
        .validate_model_on_init(true);

    let result = llm.validate_model().await;
    assert!(
        result.is_err(),
        "Validation should fail when API returns an error"
    );

    Ok(())
}

// =============================================================================
// test_chat_models_standard.py — Standard integration tests
// =============================================================================

/// Ported from `TestChatOllama.test_tool_calling` (from ChatModelIntegrationTests).
/// Tests basic tool calling with ChatOllama.
#[tokio::test]
#[ignore]
async fn test_standard_tool_calling() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let tool_schema = serde_json::json!({
        "title": "magic_function",
        "description": "Applies a magic function to an input.",
        "type": "object",
        "properties": {
            "input": {"type": "integer", "description": "The input value"}
        },
        "required": ["input"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(tool_schema)], None)?;

    let result = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Apply the magic function to 3.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(
        !result.tool_calls.is_empty(),
        "Model should produce a tool call"
    );
    assert_eq!(result.tool_calls[0].name, "magic_function");

    Ok(())
}

/// Ported from `TestChatOllama.test_tool_calling_async` (from ChatModelIntegrationTests).
#[tokio::test]
#[ignore]
async fn test_standard_tool_calling_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let tool_schema = serde_json::json!({
        "title": "magic_function",
        "description": "Applies a magic function to an input.",
        "type": "object",
        "properties": {
            "input": {"type": "integer", "description": "The input value"}
        },
        "required": ["input"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(tool_schema)], None)?;

    let result = llm_with_tools
        .ainvoke(
            vec![
                HumanMessage::builder()
                    .content("Apply the magic function to 3.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(
        !result.tool_calls.is_empty(),
        "Model should produce a tool call"
    );
    assert_eq!(result.tool_calls[0].name, "magic_function");

    Ok(())
}

/// Ported from `TestChatOllama.test_tool_calling_with_no_arguments`.
#[tokio::test]
#[ignore]
async fn test_standard_tool_calling_no_arguments() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let tool_schema = serde_json::json!({
        "title": "magic_function_no_args",
        "description": "A magic function that takes no arguments.",
        "type": "object",
        "properties": {},
        "required": []
    });

    let llm = ChatOllama::new(DEFAULT_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(tool_schema)], None)?;

    let result = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Call the magic function with no arguments.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(
        !result.tool_calls.is_empty(),
        "Model should produce a tool call"
    );

    Ok(())
}

/// Ported from `TestChatOllama.supports_json_mode`.
/// Tests that ChatOllama supports JSON mode output.
#[tokio::test]
#[ignore]
async fn test_standard_json_mode() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).json_mode();

    let result = llm
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Return a JSON object with a 'name' key set to 'Alice'.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&result.text())?;
    assert!(parsed.is_object());

    Ok(())
}

// =============================================================================
// test_llms.py — OllamaLLM tests (ported using ChatOllama since OllamaLLM
// does not exist in Rust yet — these tests expose that gap)
// =============================================================================

/// Ported from `test_invoke` (OllamaLLM).
/// Uses ChatOllama since OllamaLLM is not implemented in Rust.
#[tokio::test]
#[ignore]
async fn test_llm_invoke() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL);

    let result = llm
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("I'm Pickle Rick")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());

    Ok(())
}

/// Ported from `test_ainvoke` (OllamaLLM).
#[tokio::test]
#[ignore]
async fn test_llm_ainvoke() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL);

    let result = llm
        .ainvoke(
            vec![
                HumanMessage::builder()
                    .content("I'm Pickle Rick")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());

    Ok(())
}

/// Ported from `test_stream_text_tokens` (OllamaLLM).
#[tokio::test]
#[ignore]
async fn test_llm_stream() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL);

    let mut stream = llm
        .astream(
            vec![HumanMessage::builder().content("Hi.").build().into()].into(),
            None,
            None,
        )
        .await?;

    let mut got_chunk = false;
    while let Some(chunk) = stream.next().await {
        let _chunk = chunk?;
        got_chunk = true;
    }
    assert!(got_chunk, "Stream should produce at least one chunk");

    Ok(())
}

/// Ported from `test_astream_text_tokens` (OllamaLLM).
#[tokio::test]
#[ignore]
async fn test_llm_astream() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL);

    let mut stream = llm
        .astream(
            vec![HumanMessage::builder().content("Hi.").build().into()].into(),
            None,
            None,
        )
        .await?;

    let mut got_chunk = false;
    while let Some(chunk) = stream.next().await {
        let _chunk = chunk?;
        got_chunk = true;
    }
    assert!(got_chunk, "Async stream should produce at least one chunk");

    Ok(())
}

/// Ported from `test__stream_no_reasoning` (OllamaLLM).
/// Tests streaming with a reasoning model but reasoning disabled.
#[tokio::test]
#[ignore]
async fn test_llm_stream_no_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL).num_ctx(4096);

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());

    Ok(())
}

/// Ported from `test__astream_no_reasoning` (OllamaLLM).
#[tokio::test]
#[ignore]
async fn test_llm_astream_no_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL).num_ctx(4096);

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());

    Ok(())
}

/// Ported from `test__stream_with_reasoning` (OllamaLLM).
/// Tests streaming with reasoning=True on a reasoning model.
#[tokio::test]
#[ignore]
async fn test_llm_stream_with_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(true));

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());
    assert!(!full_text.contains("<think>"));
    assert!(!full_text.contains("</think>"));

    Ok(())
}

/// Ported from `test__astream_with_reasoning` (OllamaLLM).
#[tokio::test]
#[ignore]
async fn test_llm_astream_with_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(serde_json::json!(true));

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content(SAMPLE_PROMPT)
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
    }
    assert!(!full_text.is_empty());
    assert!(!full_text.contains("<think>"));
    assert!(!full_text.contains("</think>"));

    Ok(())
}
