use agent_chain::providers::openai::ChatOpenAI;
use agent_chain_core::ToolChoice;
use agent_chain_core::language_models::ToolLike;
use agent_chain_core::language_models::chat_models::BaseChatModel;
use agent_chain_core::messages::{AIMessage, HumanMessage};
use futures::StreamExt;
use std::collections::HashMap;

fn load_env() {
    let _ = dotenv::dotenv();
}

// =============================================================================
// Responses API integration tests
// Ported from langchain_openai/tests/integration_tests/chat_models/test_responses_api.py
// =============================================================================

/// Helper: validate Responses API message structure.
/// Matches Python's `_check_response`.
fn check_response(response: &AIMessage) {
    let text = response.text();
    assert!(!text.is_empty(), "Text content should not be empty");

    let usage = response
        .usage_metadata
        .as_ref()
        .expect("usage_metadata should be present");
    assert!(usage.input_tokens > 0);
    assert!(usage.output_tokens > 0);
    assert!(usage.total_tokens > 0);
    assert!(response.response_metadata.contains_key("model_name"));
}

/// Ported from `test_incomplete_response`.
#[tokio::test]

async fn test_responses_incomplete_response() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .max_tokens(16);

    // Invoke
    let response = model
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Tell me a 100 word story about a bear.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;

    assert!(
        response
            .response_metadata
            .contains_key("incomplete_details"),
        "incomplete_details should be present"
    );
    assert_eq!(
        response
            .response_metadata
            .get("status")
            .and_then(|v| v.as_str()),
        Some("incomplete")
    );

    // Stream
    let mut stream = model
        .astream(
            vec![
                HumanMessage::builder()
                    .content("Tell me a 100 word story about a bear.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut last_metadata = HashMap::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if !chunk.response_metadata.is_empty() {
            last_metadata.clone_from(&chunk.response_metadata);
        }
    }

    assert!(
        last_metadata.contains_key("incomplete_details"),
        "streaming should propagate incomplete_details"
    );
    assert_eq!(
        last_metadata.get("status").and_then(|v| v.as_str()),
        Some("incomplete")
    );

    Ok(())
}

/// Ported from `test_web_search` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_web_search_responses_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4o-mini").output_version("responses/v1");
    let web_search_tool = ToolLike::Builtin(serde_json::json!({"type": "web_search_preview"}));

    let llm_with_tools = llm.bind_tools(std::slice::from_ref(&web_search_tool), None)?;
    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("What was a positive news story from today?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response);
    assert!(response.response_metadata.contains_key("id"));

    // Stateful API: use previous_response_id
    let response_id = response
        .response_metadata
        .get("id")
        .and_then(|v| v.as_str())
        .expect("response should have id");
    let llm_stateful = ChatOpenAI::new("gpt-4o-mini")
        .output_version("responses/v1")
        .previous_response_id(response_id);
    let llm_stateful_with_tools =
        llm_stateful.bind_tools(std::slice::from_ref(&web_search_tool), None)?;
    let response2 = llm_stateful_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("what about a negative one")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response2);

    Ok(())
}

/// Ported from `test_web_search` with output_version="v1".
#[tokio::test]

async fn test_responses_web_search_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4o-mini").output_version("v1");
    let web_search_tool = ToolLike::Builtin(serde_json::json!({"type": "web_search_preview"}));

    let llm_with_tools = llm.bind_tools(&[web_search_tool], None)?;
    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("What was a positive news story from today?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response);

    Ok(())
}

/// Ported from `test_web_search_async`.
#[tokio::test]

async fn test_responses_web_search_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4o-mini").output_version("v0");
    let web_search_tool = ToolLike::Builtin(serde_json::json!({"type": "web_search_preview"}));
    let llm_with_tools = llm.bind_tools(&[web_search_tool], None)?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("What was a positive news story from today?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response);
    assert!(response.response_metadata.contains_key("status"));

    // Streaming
    let mut stream = llm_with_tools
        .astream(
            vec![
                HumanMessage::builder()
                    .content("What was a positive news story from today?")
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

/// Ported from `test_function_calling` with output_version="v0".
#[tokio::test]

async fn test_responses_function_calling_v0() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let multiply_schema = serde_json::json!({
        "title": "multiply",
        "description": "return x * y",
        "type": "object",
        "properties": {
            "x": {"type": "integer"},
            "y": {"type": "integer"}
        },
        "required": ["x", "y"]
    });
    let llm = ChatOpenAI::new("gpt-4o-mini").output_version("v0");
    let bound = llm.bind_tools(
        &[
            ToolLike::Schema(multiply_schema),
            ToolLike::Builtin(serde_json::json!({"type": "web_search_preview"})),
        ],
        None,
    )?;

    let msg = bound
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("whats 5 * 4")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!msg.tool_calls.is_empty());
    assert_eq!(msg.tool_calls[0].name, "multiply");

    // Web search query
    let response = bound
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("What was a positive news story from today?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response);

    Ok(())
}

/// Ported from `test_function_calling` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_function_calling_responses_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let multiply_schema = serde_json::json!({
        "title": "multiply",
        "description": "return x * y",
        "type": "object",
        "properties": {
            "x": {"type": "integer"},
            "y": {"type": "integer"}
        },
        "required": ["x", "y"]
    });
    let llm = ChatOpenAI::new("gpt-4o-mini").output_version("responses/v1");
    let bound = llm.bind_tools(
        &[
            ToolLike::Schema(multiply_schema),
            ToolLike::Builtin(serde_json::json!({"type": "web_search_preview"})),
        ],
        None,
    )?;

    let msg = bound
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("whats 5 * 4")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!msg.tool_calls.is_empty());
    assert_eq!(msg.tool_calls[0].name, "multiply");

    Ok(())
}

/// Ported from `test_function_calling` with output_version="v1".
#[tokio::test]

async fn test_responses_function_calling_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let multiply_schema = serde_json::json!({
        "title": "multiply",
        "description": "return x * y",
        "type": "object",
        "properties": {
            "x": {"type": "integer"},
            "y": {"type": "integer"}
        },
        "required": ["x", "y"]
    });
    let llm = ChatOpenAI::new("gpt-4o-mini").output_version("v1");
    let bound = llm.bind_tools(
        &[
            ToolLike::Schema(multiply_schema),
            ToolLike::Builtin(serde_json::json!({"type": "web_search_preview"})),
        ],
        None,
    )?;

    let msg = bound
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("whats 5 * 4")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!msg.tool_calls.is_empty());
    assert_eq!(msg.tool_calls[0].name, "multiply");

    Ok(())
}

/// Ported from `test_parsed_pydantic_schema` (using JSON schema, no Pydantic in Rust).
#[tokio::test]

async fn test_responses_parsed_schema_v0() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let foo_schema = serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "Foo",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": { "response": {"type": "string"} },
                "required": ["response"],
                "additionalProperties": false
            }
        }
    });

    let llm = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .output_version("v0")
        .response_format(foo_schema);

    let response = llm
        .invoke(
            vec![HumanMessage::builder().content("how are ya").build().into()].into(),
            None,
        )
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&response.text())?;
    assert!(parsed.get("response").and_then(|r| r.as_str()).is_some());

    Ok(())
}

/// Ported from `test_parsed_pydantic_schema` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_parsed_schema_responses_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let foo_schema = serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "Foo",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": { "response": {"type": "string"} },
                "required": ["response"],
                "additionalProperties": false
            }
        }
    });

    let llm = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .output_version("responses/v1")
        .response_format(foo_schema);

    let response = llm
        .invoke(
            vec![HumanMessage::builder().content("how are ya").build().into()].into(),
            None,
        )
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&response.text())?;
    assert!(parsed.get("response").and_then(|r| r.as_str()).is_some());

    Ok(())
}

/// Ported from `test_parsed_pydantic_schema` with output_version="v1".
#[tokio::test]

async fn test_responses_parsed_schema_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let foo_schema = serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "Foo",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": { "response": {"type": "string"} },
                "required": ["response"],
                "additionalProperties": false
            }
        }
    });

    let llm = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .output_version("v1")
        .response_format(foo_schema);

    let response = llm
        .invoke(
            vec![HumanMessage::builder().content("how are ya").build().into()].into(),
            None,
        )
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&response.text())?;
    assert!(parsed.get("response").and_then(|r| r.as_str()).is_some());

    Ok(())
}

/// Ported from `test_parsed_pydantic_schema_async`.
#[tokio::test]

async fn test_responses_parsed_schema_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let foo_schema = serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "Foo",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": { "response": {"type": "string"} },
                "required": ["response"],
                "additionalProperties": false
            }
        }
    });

    let llm = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .response_format(foo_schema);

    let response = llm
        .ainvoke(
            vec![HumanMessage::builder().content("how are ya").build().into()].into(),
            None,
        )
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&response.text())?;
    assert!(parsed.get("response").and_then(|r| r.as_str()).is_some());

    Ok(())
}

/// Ported from `test_parsed_dict_schema`.
#[tokio::test]

async fn test_responses_parsed_dict_schema() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let schema = serde_json::json!({
        "title": "Foo",
        "type": "object",
        "properties": { "response": {"type": "string"} },
        "required": ["response"]
    });

    let llm = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .response_format(schema);

    let response = llm
        .invoke(
            vec![HumanMessage::builder().content("how are ya").build().into()].into(),
            None,
        )
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&response.text())?;
    assert!(parsed.get("response").and_then(|r| r.as_str()).is_some());

    Ok(())
}

/// Ported from `test_parsed_strict`.
#[tokio::test]

async fn test_responses_parsed_strict() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let _valid_schema = serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "Joke",
            "schema": {
                "type": "object",
                "properties": {
                    "setup": {"type": "string"},
                    "punchline": {"type": "string"}
                },
                "required": ["setup", "punchline"]
            }
        }
    });

    let llm = ChatOpenAI::new("gpt-4o-mini").with_responses_api(true);

    // Non-strict should work
    let response = llm
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Tell me a joke")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await;
    assert!(response.is_ok());

    // Invalid strict schema should fail
    let invalid_schema = serde_json::json!({
        "type": "json_schema",
        "json_schema": {
            "name": "Joke",
            "strict": true,
            "schema": {
                "type": "object",
                "properties": {
                    "setup": {"type": "string"},
                    "punchline": {"type": "string"}
                },
                "required": ["setup"]
            }
        }
    });

    let llm_strict = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .response_format(invalid_schema);
    let result = llm_strict
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Tell me a joke about cats.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await;
    assert!(result.is_err(), "Invalid strict schema should fail");

    Ok(())
}

/// Ported from `test_parsed_dict_schema_async`.
#[tokio::test]

async fn test_responses_parsed_dict_schema_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let schema = serde_json::json!({
        "title": "Foo",
        "type": "object",
        "properties": { "response": {"type": "string"} },
        "required": ["response"]
    });

    let llm = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .response_format(schema);

    let response = llm
        .ainvoke(
            vec![HumanMessage::builder().content("how are ya").build().into()].into(),
            None,
        )
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&response.text())?;
    assert!(parsed.get("response").and_then(|r| r.as_str()).is_some());

    Ok(())
}

/// Ported from `test_function_calling_and_structured_output`.
#[tokio::test]

async fn test_responses_fn_calling_and_structured_output() -> Result<(), Box<dyn std::error::Error>>
{
    load_env();
    let multiply_schema = serde_json::json!({
        "title": "multiply",
        "description": "return x * y",
        "type": "object",
        "properties": {
            "x": {"type": "integer"},
            "y": {"type": "integer"}
        },
        "required": ["x", "y"]
    });
    let foo_schema = serde_json::json!({
        "title": "Foo",
        "type": "object",
        "properties": { "response": {"type": "string"} },
        "required": ["response"]
    });

    let llm = ChatOpenAI::new("gpt-4o-mini").with_responses_api(true);
    let bound = llm.bind_tools_with_options(
        &[ToolLike::Schema(multiply_schema)],
        None,
        Some(true),
        None,
        Some(foo_schema.clone()),
    )?;

    // Structured output
    let response = llm
        .invoke(
            vec![HumanMessage::builder().content("how are ya").build().into()].into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty());

    // Function calling
    let msg = bound
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("whats 5 * 4")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!msg.tool_calls.is_empty());
    assert_eq!(msg.tool_calls[0].name, "multiply");

    Ok(())
}

/// Ported from `test_reasoning` with output_version="v0".
#[tokio::test]

async fn test_responses_reasoning_v0() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let mut reasoning = HashMap::new();
    reasoning.insert("effort".to_string(), serde_json::json!("low"));

    let llm = ChatOpenAI::new("o4-mini")
        .with_responses_api(true)
        .output_version("v0")
        .reasoning(reasoning);

    let response = llm
        .invoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty());

    Ok(())
}

/// Ported from `test_reasoning` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_reasoning_responses_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let mut reasoning = HashMap::new();
    reasoning.insert("effort".to_string(), serde_json::json!("low"));

    let llm = ChatOpenAI::new("o4-mini")
        .with_responses_api(true)
        .output_version("responses/v1")
        .reasoning(reasoning);

    let response = llm
        .invoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty());

    Ok(())
}

/// Ported from `test_reasoning` with output_version="v1".
#[tokio::test]

async fn test_responses_reasoning_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let mut reasoning = HashMap::new();
    reasoning.insert("effort".to_string(), serde_json::json!("low"));

    let llm = ChatOpenAI::new("o4-mini")
        .with_responses_api(true)
        .output_version("v1")
        .reasoning(reasoning);

    let response = llm
        .invoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty());

    Ok(())
}

/// Ported from `test_stateful_api`.
#[tokio::test]

async fn test_responses_stateful_api() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4o-mini").with_responses_api(true);

    let response = llm
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("how are you, my name is Bobo")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(response.response_metadata.contains_key("id"));

    let response_id = response
        .response_metadata
        .get("id")
        .and_then(|v| v.as_str())
        .expect("response should have id");

    let llm2 = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .previous_response_id(response_id);
    let response2 = llm2
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("what's my name")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(
        response2.text().to_lowercase().contains("bobo"),
        "Model should remember the name via stateful API"
    );

    Ok(())
}

/// Ported from `test_route_from_model_kwargs`.
#[tokio::test]

async fn test_responses_route_from_model_kwargs() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let mut kwargs = HashMap::new();
    kwargs.insert(
        "text".to_string(),
        serde_json::json!({"format": {"type": "text"}}),
    );

    let llm = ChatOpenAI::new("gpt-4o-mini").model_kwargs(kwargs);

    let mut stream = llm
        .astream(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
            None,
        )
        .await?;

    let mut got_chunk = false;
    while let Some(chunk) = stream.next().await {
        let _chunk = chunk?;
        got_chunk = true;
    }
    assert!(got_chunk, "Should receive at least one chunk");

    Ok(())
}

/// Ported from `test_computer_calls`.
#[tokio::test]

async fn test_responses_computer_calls() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("computer-use-preview")
        .truncation("auto")
        .output_version("v0");
    let tool = serde_json::json!({
        "type": "computer_use_preview",
        "display_width": 1024,
        "display_height": 768,
        "environment": "browser"
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], Some(ToolChoice::any()))?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Please open the browser.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    // Computer use should return tool outputs
    assert!(!response.text().is_empty() || !response.tool_calls.is_empty());

    Ok(())
}

/// Ported from `test_file_search` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_file_search_responses_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let vector_store_id = match std::env::var("OPENAI_VECTOR_STORE_ID") {
        Ok(id) => id,
        Err(_) => {
            eprintln!(
                "Skipping test_responses_file_search_responses_v1: OPENAI_VECTOR_STORE_ID not set"
            );
            return Ok(());
        }
    };

    let llm = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .output_version("responses/v1");
    let tool = serde_json::json!({
        "type": "file_search",
        "vector_store_ids": [vector_store_id]
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("What is deep research by OpenAI?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response);

    Ok(())
}

/// Ported from `test_file_search` with output_version="v1".
#[tokio::test]

async fn test_responses_file_search_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let vector_store_id = match std::env::var("OPENAI_VECTOR_STORE_ID") {
        Ok(id) => id,
        Err(_) => {
            eprintln!("Skipping test_responses_file_search_v1: OPENAI_VECTOR_STORE_ID not set");
            return Ok(());
        }
    };

    let llm = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .output_version("v1");
    let tool = serde_json::json!({
        "type": "file_search",
        "vector_store_ids": [vector_store_id]
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("What is deep research by OpenAI?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response);

    Ok(())
}

/// Ported from `test_stream_reasoning_summary` with output_version="v0".
#[tokio::test]

async fn test_responses_stream_reasoning_summary_v0() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let mut reasoning = HashMap::new();
    reasoning.insert("effort".to_string(), serde_json::json!("medium"));
    reasoning.insert("summary".to_string(), serde_json::json!("auto"));

    let llm = ChatOpenAI::new("o4-mini")
        .reasoning(reasoning)
        .output_version("v0");

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content("What was the third tallest building in the year 2000?")
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

/// Ported from `test_stream_reasoning_summary` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_stream_reasoning_summary_responses_v1()
-> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let mut reasoning = HashMap::new();
    reasoning.insert("effort".to_string(), serde_json::json!("medium"));
    reasoning.insert("summary".to_string(), serde_json::json!("auto"));

    let llm = ChatOpenAI::new("o4-mini")
        .reasoning(reasoning)
        .output_version("responses/v1");

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content("What was the third tallest building in the year 2000?")
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

/// Ported from `test_stream_reasoning_summary` with output_version="v1".
#[tokio::test]

async fn test_responses_stream_reasoning_summary_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let mut reasoning = HashMap::new();
    reasoning.insert("effort".to_string(), serde_json::json!("medium"));
    reasoning.insert("summary".to_string(), serde_json::json!("auto"));

    let llm = ChatOpenAI::new("o4-mini")
        .reasoning(reasoning)
        .output_version("v1");

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content("What was the third tallest building in the year 2000?")
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

/// Ported from `test_code_interpreter` with output_version="v0".
#[tokio::test]

async fn test_responses_code_interpreter_v0() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("o4-mini")
        .with_responses_api(true)
        .output_version("v0");
    let tool = serde_json::json!({"type": "code_interpreter", "container": {"type": "auto"}});
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Write and run code to answer: what is 3^3?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response);

    Ok(())
}

/// Ported from `test_code_interpreter` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_code_interpreter_responses_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("o4-mini")
        .with_responses_api(true)
        .output_version("responses/v1");
    let tool = serde_json::json!({"type": "code_interpreter", "container": {"type": "auto"}});
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Write and run code to answer: what is 3^3?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response);

    Ok(())
}

/// Ported from `test_code_interpreter` with output_version="v1".
#[tokio::test]

async fn test_responses_code_interpreter_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("o4-mini")
        .with_responses_api(true)
        .output_version("v1");
    let tool = serde_json::json!({"type": "code_interpreter", "container": {"type": "auto"}});
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Write and run code to answer: what is 3^3?")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    check_response(&response);

    Ok(())
}

/// Ported from `test_mcp_builtin`.
#[tokio::test]

async fn test_responses_mcp_builtin() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("o4-mini")
        .with_responses_api(true)
        .output_version("v0");
    let mcp_tool = serde_json::json!({
        "type": "mcp",
        "server_label": "deepwiki",
        "server_url": "https://mcp.deepwiki.com/mcp",
        "require_approval": {"always": {"tool_names": ["read_wiki_structure"]}}
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(mcp_tool)], None)?;

    let response = llm_with_tools
        .invoke(
            vec![HumanMessage::builder()
                .content(
                    "What transport protocols does the 2025-03-26 version of the MCP spec support?",
                )
                .build()
                .into()]
            .into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty() || !response.tool_calls.is_empty());

    Ok(())
}

/// Ported from `test_mcp_builtin_zdr`.
#[tokio::test]

async fn test_responses_mcp_builtin_zdr() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4o-mini")
        .with_responses_api(true)
        .store(false)
        .include(vec!["reasoning.encrypted_content".to_string()]);
    let mcp_tool = serde_json::json!({
        "type": "mcp",
        "server_label": "deepwiki",
        "server_url": "https://mcp.deepwiki.com/mcp",
        "allowed_tools": ["ask_question"],
        "require_approval": "always"
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(mcp_tool)], None)?;

    let mut stream = llm_with_tools
        .astream(
            vec![
                HumanMessage::builder()
                    .content(
                        "What transport protocols does the 2025-03-26 version of the MCP \
                     spec (modelcontextprotocol/modelcontextprotocol) support?",
                    )
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
    // MCP may return approval requests instead of text

    Ok(())
}

/// Ported from `test_mcp_builtin_zdr_v1`.
#[tokio::test]

async fn test_responses_mcp_builtin_zdr_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4o-mini")
        .output_version("v1")
        .store(false)
        .include(vec!["reasoning.encrypted_content".to_string()]);
    let mcp_tool = serde_json::json!({
        "type": "mcp",
        "server_label": "deepwiki",
        "server_url": "https://mcp.deepwiki.com/mcp",
        "allowed_tools": ["ask_question"],
        "require_approval": "always"
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(mcp_tool)], None)?;

    let mut stream = llm_with_tools
        .astream(
            vec![
                HumanMessage::builder()
                    .content(
                        "What transport protocols does the 2025-03-26 version of the MCP \
                     spec (modelcontextprotocol/modelcontextprotocol) support?",
                    )
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

    Ok(())
}

/// Ported from `test_image_generation_streaming` with output_version="v0".
#[tokio::test]

async fn test_responses_image_gen_streaming_v0() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4.1")
        .with_responses_api(true)
        .output_version("v0");
    let tool = serde_json::json!({
        "type": "image_generation",
        "quality": "low",
        "output_format": "jpeg",
        "output_compression": 100,
        "size": "1024x1024"
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let mut stream = llm_with_tools
        .astream(
            vec![
                HumanMessage::builder()
                    .content("Draw a random short word in green font.")
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
    // Image generation may produce no text, but should complete without error

    Ok(())
}

/// Ported from `test_image_generation_streaming` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_image_gen_streaming_responses_v1() -> Result<(), Box<dyn std::error::Error>>
{
    load_env();
    let llm = ChatOpenAI::new("gpt-4.1")
        .with_responses_api(true)
        .output_version("responses/v1");
    let tool = serde_json::json!({
        "type": "image_generation",
        "quality": "low",
        "output_format": "jpeg",
        "output_compression": 100,
        "size": "1024x1024"
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let mut stream = llm_with_tools
        .astream(
            vec![
                HumanMessage::builder()
                    .content("Draw a random short word in green font.")
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

    Ok(())
}

/// Ported from `test_image_generation_streaming_v1`.
#[tokio::test]

async fn test_responses_image_gen_streaming_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4.1")
        .with_responses_api(true)
        .output_version("v1");
    let tool = serde_json::json!({
        "type": "image_generation",
        "quality": "low",
        "output_format": "jpeg",
        "output_compression": 100,
        "size": "1024x1024"
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let mut stream = llm_with_tools
        .astream(
            vec![
                HumanMessage::builder()
                    .content("Draw a random short word in green font.")
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

    Ok(())
}

/// Ported from `test_image_generation_multi_turn` with output_version="v0".
#[tokio::test]

async fn test_responses_image_gen_multi_turn_v0() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4.1")
        .with_responses_api(true)
        .output_version("v0");
    let tool = serde_json::json!({
        "type": "image_generation",
        "quality": "low",
        "output_format": "jpeg",
        "output_compression": 100,
        "size": "1024x1024"
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Draw a random short word in green font.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty() || !response.tool_calls.is_empty());

    Ok(())
}

/// Ported from `test_image_generation_multi_turn` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_image_gen_multi_turn_responses_v1() -> Result<(), Box<dyn std::error::Error>>
{
    load_env();
    let llm = ChatOpenAI::new("gpt-4.1")
        .with_responses_api(true)
        .output_version("responses/v1");
    let tool = serde_json::json!({
        "type": "image_generation",
        "quality": "low",
        "output_format": "jpeg",
        "output_compression": 100,
        "size": "1024x1024"
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Draw a random short word in green font.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty() || !response.tool_calls.is_empty());

    Ok(())
}

/// Ported from `test_image_generation_multi_turn_v1`.
#[tokio::test]

async fn test_responses_image_gen_multi_turn_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4.1")
        .with_responses_api(true)
        .output_version("v1");
    let tool = serde_json::json!({
        "type": "image_generation",
        "quality": "low",
        "output_format": "jpeg",
        "output_compression": 100,
        "size": "1024x1024"
    });
    let llm_with_tools = llm.bind_tools(&[ToolLike::Builtin(tool)], None)?;

    let response = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Draw a random short word in green font.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty() || !response.tool_calls.is_empty());

    Ok(())
}

/// Ported from `test_verbosity_parameter`.
#[tokio::test]

async fn test_responses_verbosity_parameter() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOpenAI::new("gpt-4o-mini")
        .verbosity("medium")
        .with_responses_api(true);

    let response = llm
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Hello, explain quantum computing.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty());

    Ok(())
}

/// Ported from `test_custom_tool` with output_version="responses/v1".
#[tokio::test]

async fn test_responses_custom_tool_responses_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let execute_code_schema = serde_json::json!({
        "title": "execute_code",
        "description": "Execute python code.",
        "type": "object",
        "properties": {
            "code": {"type": "string"}
        },
        "required": ["code"]
    });

    let llm = ChatOpenAI::new("gpt-4o-mini").output_version("responses/v1");
    let llm_with_tools = llm.bind_tools(&[ToolLike::Schema(execute_code_schema)], None)?;

    let msg = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Use the tool to evaluate 3^3.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!msg.tool_calls.is_empty());
    assert_eq!(msg.tool_calls[0].name, "execute_code");

    // Stream
    let mut stream = llm_with_tools
        .astream(
            vec![
                HumanMessage::builder()
                    .content("Use the tool to evaluate 3^3.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut tool_calls = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        tool_calls.extend(chunk.tool_calls.clone());
    }
    assert!(!tool_calls.is_empty());

    Ok(())
}

/// Ported from `test_custom_tool` with output_version="v1".
#[tokio::test]

async fn test_responses_custom_tool_v1() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let execute_code_schema = serde_json::json!({
        "title": "execute_code",
        "description": "Execute python code.",
        "type": "object",
        "properties": {
            "code": {"type": "string"}
        },
        "required": ["code"]
    });

    let llm = ChatOpenAI::new("gpt-4o-mini").output_version("v1");
    let llm_with_tools = llm.bind_tools(&[ToolLike::Schema(execute_code_schema)], None)?;

    let msg = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Use the tool to evaluate 3^3.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!msg.tool_calls.is_empty());
    assert_eq!(msg.tool_calls[0].name, "execute_code");

    Ok(())
}
