use agent_chain::providers::ollama::ChatOllama;
use agent_chain_core::language_models::chat_models::BaseChatModel;
use agent_chain_core::language_models::{ToolChoice, ToolLike};
use agent_chain_core::messages::{
    AIMessage, BaseMessage, HumanMessage, SystemMessage, ToolMessage, ToolStatus,
};
use futures::StreamExt;

const DEFAULT_MODEL: &str = "llama3.1";
const TOOL_MODEL: &str = "gpt-oss:20b";
fn load_env() {
    let _ = dotenv::dotenv();
}

// =============================================================================
// Basic invoke/stream tests
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_invoke`.
#[tokio::test]
async fn test_invoke() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let result = llm
        .invoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());

    Ok(())
}

/// Ported from `ChatModelIntegrationTests.test_ainvoke`.
#[tokio::test]
async fn test_ainvoke() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let result = llm
        .ainvoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());

    Ok(())
}

/// Ported from `ChatModelIntegrationTests.test_stream`.
#[tokio::test]
async fn test_stream() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let mut stream = llm
        .astream(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
            None,
        )
        .await?;

    let mut num_chunks = 0;
    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
        num_chunks += 1;
    }
    assert!(num_chunks > 0, "Stream should produce at least one chunk");
    assert!(
        !full_text.is_empty(),
        "Concatenated content should be non-empty"
    );

    Ok(())
}

/// Ported from `ChatModelIntegrationTests.test_astream`.
#[tokio::test]
async fn test_astream() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let mut stream = llm
        .astream(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
            None,
        )
        .await?;

    let mut num_chunks = 0;
    let mut full_text = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
        num_chunks += 1;
    }
    assert!(
        num_chunks > 0,
        "Async stream should produce at least one chunk"
    );
    assert!(
        !full_text.is_empty(),
        "Concatenated content should be non-empty"
    );

    Ok(())
}

// =============================================================================
// Conversation tests
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_conversation`.
#[tokio::test]
async fn test_conversation() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder().content("hello").build().into(),
        BaseMessage::AI(AIMessage::builder().content("hello").build()),
        HumanMessage::builder()
            .content("how are you")
            .build()
            .into(),
    ];

    let result = llm.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

/// Ported from `ChatModelIntegrationTests.test_message_with_name`.
#[tokio::test]
async fn test_message_with_name() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("hello")
            .name("alice".to_string())
            .build()
            .into(),
    ];

    let result = llm.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

// =============================================================================
// Usage metadata tests
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_usage_metadata`.
#[tokio::test]
async fn test_usage_metadata() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let result = llm
        .invoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;

    let usage = result
        .usage_metadata
        .as_ref()
        .expect("usage_metadata should be present");
    assert!(usage.input_tokens > 0, "input_tokens should be positive");
    assert!(usage.output_tokens > 0, "output_tokens should be positive");
    assert!(usage.total_tokens > 0, "total_tokens should be positive");

    let model_name = result
        .response_metadata
        .get("model_name")
        .and_then(|v| v.as_str());
    assert!(
        model_name.is_some(),
        "model_name should be in response_metadata"
    );
    assert!(
        !model_name.unwrap_or("").is_empty(),
        "model_name should not be empty"
    );

    Ok(())
}

/// Ported from `ChatModelIntegrationTests.test_usage_metadata_streaming`.
#[tokio::test]
async fn test_usage_metadata_streaming() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let mut stream = llm
        .astream(
            vec![
                HumanMessage::builder()
                    .content("Write me 2 haikus. Only include the haikus.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
            None,
        )
        .await?;

    let mut total_input = 0i64;
    let mut total_output = 0i64;
    let mut chunk_count = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        if let Some(usage) = &chunk.usage_metadata {
            total_input += usage.input_tokens;
            total_output += usage.output_tokens;
        }
        chunk_count += 1;
    }

    assert!(chunk_count > 0, "Stream should produce chunks");
    assert!(
        total_input > 0 || total_output > 0,
        "Usage metadata should accumulate token counts across stream chunks"
    );

    Ok(())
}

// =============================================================================
// Stop sequence test
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_stop_sequence`.
#[tokio::test]
async fn test_stop_sequence() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL)
        .temperature(0.0)
        .stop(vec!["you".to_string()]);

    let result = llm
        .invoke(
            vec![HumanMessage::builder().content("hi").build().into()].into(),
            None,
        )
        .await?;
    // Python just asserts isinstance(result, AIMessage) — the invoke succeeded.
    let _ = result;

    Ok(())
}

// =============================================================================
// Tool calling tests (standard base class versions)
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_tool_calling` (from base class).
#[tokio::test]
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
                    .content("What is the value of magic_function(3)? Use the tool.")
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

/// Ported from `ChatModelIntegrationTests.test_tool_calling_async` (from base class).
#[tokio::test]
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
                    .content("What is the value of magic_function(3)? Use the tool.")
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

/// Ported from `ChatModelIntegrationTests.test_tool_calling_with_no_arguments`.
#[tokio::test]
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

// =============================================================================
// Tool message history tests
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_tool_message_histories_string_content`.
#[tokio::test]
async fn test_tool_message_histories_string_content() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let adder_schema = serde_json::json!({
        "title": "my_adder_tool",
        "description": "Tool that adds two integers. Takes two integers, a and b, and returns their sum.",
        "type": "object",
        "properties": {
            "a": {"type": "integer", "description": "First integer"},
            "b": {"type": "integer", "description": "Second integer"}
        },
        "required": ["a", "b"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(adder_schema)], None)?;

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("What is 1 + 2")
            .build()
            .into(),
        BaseMessage::AI(
            AIMessage::builder()
                .content("")
                .tool_calls(vec![agent_chain_core::messages::ToolCall {
                    name: "my_adder_tool".to_string(),
                    args: serde_json::json!({"a": 1, "b": 2}),
                    id: Some("abc123".to_string()),
                    call_type: Some("tool_call".to_string()),
                }])
                .build(),
        ),
        agent_chain_core::messages::ToolMessage::builder()
            .content(r#"{"result": 3}"#)
            .name("my_adder_tool".to_string())
            .tool_call_id("abc123")
            .build()
            .into(),
    ];

    let result = llm_with_tools.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty() || !result.tool_calls.is_empty());

    Ok(())
}

/// Ported from `ChatModelIntegrationTests.test_tool_message_histories_list_content`.
#[tokio::test]
async fn test_tool_message_histories_list_content() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let adder_schema = serde_json::json!({
        "title": "my_adder_tool",
        "description": "Tool that adds two integers. Takes two integers, a and b, and returns their sum.",
        "type": "object",
        "properties": {
            "a": {"type": "integer", "description": "First integer"},
            "b": {"type": "integer", "description": "Second integer"}
        },
        "required": ["a", "b"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(adder_schema)], None)?;

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("What is 1 + 2")
            .build()
            .into(),
        BaseMessage::AI(
            AIMessage::builder()
                .content(vec![
                    serde_json::json!({"type": "text", "text": "some text"}),
                    serde_json::json!({
                        "type": "tool_use",
                        "id": "abc123",
                        "name": "my_adder_tool",
                        "input": {"a": 1, "b": 2}
                    }),
                ])
                .tool_calls(vec![agent_chain_core::messages::ToolCall {
                    name: "my_adder_tool".to_string(),
                    args: serde_json::json!({"a": 1, "b": 2}),
                    id: Some("abc123".to_string()),
                    call_type: Some("tool_call".to_string()),
                }])
                .build(),
        ),
        agent_chain_core::messages::ToolMessage::builder()
            .content(r#"{"result": 3}"#)
            .name("my_adder_tool".to_string())
            .tool_call_id("abc123")
            .build()
            .into(),
    ];

    let result = llm_with_tools.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty() || !result.tool_calls.is_empty());

    Ok(())
}

/// Ported from `ChatModelIntegrationTests.test_tool_message_error_status`.
#[tokio::test]
async fn test_tool_message_error_status() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let adder_schema = serde_json::json!({
        "title": "my_adder_tool",
        "description": "Tool that adds two integers. Takes two integers, a and b, and returns their sum.",
        "type": "object",
        "properties": {
            "a": {"type": "integer", "description": "First integer"},
            "b": {"type": "integer", "description": "Second integer"}
        },
        "required": ["a", "b"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(adder_schema)], None)?;

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("What is 1 + 2")
            .build()
            .into(),
        BaseMessage::AI(
            AIMessage::builder()
                .content("")
                .tool_calls(vec![agent_chain_core::messages::ToolCall {
                    name: "my_adder_tool".to_string(),
                    args: serde_json::json!({"a": 1}),
                    id: Some("abc123".to_string()),
                    call_type: Some("tool_call".to_string()),
                }])
                .build(),
        ),
        agent_chain_core::messages::ToolMessage::builder()
            .content("Error: Missing required argument 'b'.")
            .name("my_adder_tool".to_string())
            .tool_call_id("abc123")
            .status(ToolStatus::Error)
            .build()
            .into(),
    ];

    let result = llm_with_tools.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty() || !result.tool_calls.is_empty());

    Ok(())
}

// =============================================================================
// JSON mode test
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_json_mode`.
#[tokio::test]
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
// Structured output tests
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_structured_output` (json_schema variant).
#[tokio::test]
async fn test_structured_output() -> Result<(), Box<dyn std::error::Error>> {
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

    let llm = ChatOllama::new(TOOL_MODEL).temperature(0.0);
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
    assert!(
        result.get("setup").is_some(),
        "Result should have 'setup' key"
    );
    assert!(
        result.get("punchline").is_some(),
        "Result should have 'punchline' key"
    );

    Ok(())
}

/// Ported from `ChatModelIntegrationTests.test_structured_output_async`.
#[tokio::test]
async fn test_structured_output_async() -> Result<(), Box<dyn std::error::Error>> {
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

    let llm = ChatOllama::new(TOOL_MODEL).temperature(0.0);
    let structured = llm.with_structured_output(joke_schema, false)?;

    let result = structured
        .ainvoke(
            vec![
                HumanMessage::builder()
                    .content("Tell me a joke about dogs.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(
        result.get("setup").is_some(),
        "Result should have 'setup' key"
    );
    assert!(
        result.get("punchline").is_some(),
        "Result should have 'punchline' key"
    );

    Ok(())
}

// =============================================================================
// Image input tests
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_image_inputs`.
#[tokio::test]
async fn test_image_inputs() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let image_url = "https://raw.githubusercontent.com/langchain-ai/docs/4d11d08b6b0e210bd456943f7a22febbd168b543/src/images/agentic-rag-output.png";

    let response = reqwest::get(image_url).await?;
    let image_bytes = response.bytes().await?;
    use base64::Engine;
    let image_data = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

    let message: BaseMessage = HumanMessage::builder()
        .content(vec![
            serde_json::json!({"type": "text", "text": "Give a concise description of this image."}),
            serde_json::json!({
                "type": "image_url",
                "image_url": {"url": format!("data:image/png;base64,{image_data}")}
            }),
        ])
        .build()
        .into();

    let result = llm.invoke(vec![message].into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

// =============================================================================
// Batch tests
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_batch`.
#[tokio::test]
async fn test_batch() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let prompts = vec!["Hello", "Hey"];
    let mut results = Vec::new();
    for prompt in prompts {
        let result = llm
            .invoke(
                vec![HumanMessage::builder().content(prompt).build().into()].into(),
                None,
            )
            .await?;
        results.push(result);
    }
    assert_eq!(results.len(), 2);
    for msg in &results {
        assert!(!msg.text().is_empty());
    }

    Ok(())
}

/// Ported from `ChatModelIntegrationTests.test_abatch`.
#[tokio::test]
async fn test_abatch() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let futures: Vec<_> = vec!["Hello", "Hey"]
        .into_iter()
        .map(|prompt| {
            let llm = llm.clone();
            async move {
                llm.ainvoke(
                    vec![HumanMessage::builder().content(prompt).build().into()].into(),
                    None,
                )
                .await
            }
        })
        .collect();

    let results = futures::future::join_all(futures).await;
    assert_eq!(results.len(), 2);
    for result in results {
        let msg = result?;
        assert!(!msg.text().is_empty());
    }

    Ok(())
}

// =============================================================================
// Double messages conversation test
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_double_messages_conversation`.
#[tokio::test]
async fn test_double_messages_conversation() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = ChatOllama::new(DEFAULT_MODEL).temperature(0.0);

    let messages: Vec<BaseMessage> = vec![
        SystemMessage::builder().content("hello").build().into(),
        SystemMessage::builder().content("hello").build().into(),
        HumanMessage::builder().content("hello").build().into(),
        HumanMessage::builder().content("hello").build().into(),
        BaseMessage::AI(AIMessage::builder().content("hello").build()),
        BaseMessage::AI(AIMessage::builder().content("hello").build()),
        HumanMessage::builder()
            .content("how are you")
            .build()
            .into(),
    ];

    let result = llm.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

// =============================================================================
// Tool choice test
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_tool_choice`.
#[tokio::test]
async fn test_tool_choice() -> Result<(), Box<dyn std::error::Error>> {
    load_env();

    let magic_schema = serde_json::json!({
        "title": "magic_function",
        "description": "Applies a magic function to an input.",
        "type": "object",
        "properties": {
            "input": {"type": "integer", "description": "The input value"}
        },
        "required": ["input"]
    });

    let weather_schema = serde_json::json!({
        "title": "get_weather",
        "description": "Get weather at a location.",
        "type": "object",
        "properties": {
            "location": {"type": "string", "description": "The location"}
        },
        "required": ["location"]
    });

    let llm = ChatOllama::new(TOOL_MODEL);

    // Bind both tools, ask a question that should trigger one of them
    let llm_with_tools = BaseChatModel::bind_tools(
        &llm,
        &[
            ToolLike::Schema(magic_schema.clone()),
            ToolLike::Schema(weather_schema.clone()),
        ],
        None,
    )?;

    let result = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("What is the weather in Tokyo? Use the get_weather tool.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.tool_calls.is_empty(), "Should produce a tool call");

    // Ask specifically for magic_function
    let llm_with_tools = BaseChatModel::bind_tools(
        &llm,
        &[
            ToolLike::Schema(magic_schema),
            ToolLike::Schema(weather_schema),
        ],
        None,
    )?;

    let result = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("What is the value of magic_function(3)? Use the magic_function tool.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(
        !result.tool_calls.is_empty(),
        "Should produce a tool call for magic_function"
    );
    assert_eq!(result.tool_calls[0].name, "magic_function");

    Ok(())
}

// =============================================================================
// Structured few-shot examples test
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_structured_few_shot_examples`.
#[tokio::test]
async fn test_structured_few_shot_examples() -> Result<(), Box<dyn std::error::Error>> {
    load_env();

    let adder_schema = serde_json::json!({
        "title": "my_adder_tool",
        "description": "Tool that adds two integers.",
        "type": "object",
        "properties": {
            "a": {"type": "integer", "description": "First integer"},
            "b": {"type": "integer", "description": "Second integer"}
        },
        "required": ["a", "b"]
    });

    let llm = ChatOllama::new(TOOL_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(
        &llm,
        &[ToolLike::Schema(adder_schema)],
        Some(ToolChoice::any()),
    )?;

    let few_shot_messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("What is 1 + 2")
            .build()
            .into(),
        BaseMessage::AI(
            AIMessage::builder()
                .content("")
                .tool_calls(vec![agent_chain_core::messages::ToolCall {
                    name: "my_adder_tool".to_string(),
                    args: serde_json::json!({"a": 1, "b": 2}),
                    id: Some("example_1".to_string()),
                    call_type: Some("tool_call".to_string()),
                }])
                .build(),
        ),
        ToolMessage::builder()
            .content(r#"{"result": 3}"#)
            .name("my_adder_tool".to_string())
            .tool_call_id("example_1")
            .build()
            .into(),
        BaseMessage::AI(AIMessage::builder().content(r#"{"result": 3}"#).build()),
        HumanMessage::builder()
            .content("What is 3 + 4")
            .build()
            .into(),
    ];

    let result = llm_with_tools
        .invoke(few_shot_messages.into(), None)
        .await?;
    assert!(
        !result.tool_calls.is_empty(),
        "Model should produce a tool call after few-shot examples"
    );

    Ok(())
}

// =============================================================================
// Structured output with optional parameter test
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_structured_output_optional_param`.
#[tokio::test]
async fn test_structured_output_optional_param() -> Result<(), Box<dyn std::error::Error>> {
    load_env();

    let joke_schema = serde_json::json!({
        "title": "Joke",
        "description": "Joke to tell user.",
        "type": "object",
        "properties": {
            "setup": {"type": "string", "description": "question to set up a joke"},
            "punchline": {
                "anyOf": [{"type": "string"}, {"type": "null"}],
                "default": null,
                "description": "answer to resolve the joke"
            }
        },
        "required": ["setup"]
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
    assert!(
        result.get("setup").is_some(),
        "Result should have 'setup' key"
    );

    Ok(())
}

// =============================================================================
// Unicode tool call integration test
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_unicode_tool_call_integration`.
#[tokio::test]
async fn test_unicode_tool_call_integration() -> Result<(), Box<dyn std::error::Error>> {
    load_env();

    let unicode_customer_schema = serde_json::json!({
        "title": "unicode_customer",
        "description": "Tool for creating a customer with Unicode name.",
        "type": "object",
        "properties": {
            "customer_name": {
                "type": "string",
                "description": "The customer's name in their native language."
            },
            "description": {
                "type": "string",
                "description": "Description of the customer."
            }
        },
        "required": ["customer_name", "description"]
    });

    let llm = ChatOllama::new(TOOL_MODEL);
    let llm_with_tools = BaseChatModel::bind_tools(
        &llm,
        &[ToolLike::Schema(unicode_customer_schema)],
        Some(ToolChoice::any()),
    )?;

    // Test with Chinese characters
    let result = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content(
                        "Create a customer named '你好啊集团' (Hello Group) - a Chinese \
                         technology company",
                    )
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;

    assert!(
        !result.tool_calls.is_empty(),
        "Expected at least 1 tool call"
    );
    let tool_call = &result.tool_calls[0];
    assert_eq!(tool_call.name, "unicode_customer");
    let customer_name = tool_call.args["customer_name"]
        .as_str()
        .expect("customer_name should be a string");
    assert!(
        customer_name.contains('你')
            || customer_name.contains('好')
            || customer_name.contains("你好"),
        "Unicode characters not found in: {customer_name}"
    );

    // Test with Japanese characters
    let result = llm_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content(
                        "Create a customer named 'こんにちは株式会社' (Hello Corporation) - a \
                         Japanese company",
                    )
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;

    assert!(
        !result.tool_calls.is_empty(),
        "Expected at least 1 tool call"
    );
    let tool_call = &result.tool_calls[0];
    let customer_name = tool_call.args["customer_name"]
        .as_str()
        .expect("customer_name should be a string");
    assert!(
        customer_name.contains("こんにちは")
            || customer_name.contains("株式会社")
            || customer_name.contains('こ')
            || customer_name.contains('ん'),
        "Japanese Unicode characters not found in: {customer_name}"
    );

    Ok(())
}

// =============================================================================
// Agent loop test
// =============================================================================

/// Ported from `ChatModelIntegrationTests.test_agent_loop`.
#[tokio::test]
async fn test_agent_loop() -> Result<(), Box<dyn std::error::Error>> {
    load_env();

    let weather_schema = serde_json::json!({
        "title": "get_weather",
        "description": "Get the weather at a location.",
        "type": "object",
        "properties": {
            "location": {"type": "string", "description": "The location to check"}
        },
        "required": ["location"]
    });

    let llm = ChatOllama::new(DEFAULT_MODEL);
    let llm_with_tools =
        BaseChatModel::bind_tools(&llm, &[ToolLike::Schema(weather_schema)], None)?;

    let input_message: BaseMessage = HumanMessage::builder()
        .content("What is the weather in San Francisco, CA?")
        .build()
        .into();

    // Step 1: model should request a tool call
    let tool_call_message = llm_with_tools
        .invoke(vec![input_message.clone()].into(), None)
        .await?;
    assert!(
        !tool_call_message.tool_calls.is_empty(),
        "Model should produce a tool call"
    );

    let tool_call = &tool_call_message.tool_calls[0];

    // Step 2: simulate the tool response
    let tool_response: BaseMessage = ToolMessage::builder()
        .content("It's sunny.")
        .name("get_weather".to_string())
        .tool_call_id(tool_call.id.clone().unwrap_or_default())
        .build()
        .into();

    // Step 3: feed the tool result back to the model
    let response = llm_with_tools
        .invoke(
            vec![
                input_message,
                BaseMessage::AI(tool_call_message),
                tool_response,
            ]
            .into(),
            None,
        )
        .await?;
    assert!(
        !response.text().is_empty() || !response.tool_calls.is_empty(),
        "Model should produce a final response after tool execution"
    );

    Ok(())
}
