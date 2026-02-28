use agent_chain::providers::ollama::{ChatOllama, OllamaFormat};
use agent_chain_core::language_models::ToolLike;
use agent_chain_core::language_models::chat_models::BaseChatModel;
use agent_chain_core::messages::{BaseMessage, HumanMessage};
use futures::StreamExt;

const DEFAULT_MODEL: &str = "llama3.1";
fn load_env() {
    let _ = dotenv::dotenv();
}

// =============================================================================

/// Ported from `test_init_model_not_found`.
/// Tests that validation fails when the model doesn't exist in Ollama.
#[tokio::test]

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
// test_chat_models.py — Core ChatOllama tests
// =============================================================================

/// Ported from `test_structured_output` with method="function_calling".
#[tokio::test]

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
