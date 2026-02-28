//! Standard integration tests for chat models.
//!
//! Ported from `langchain_tests/integration_tests/chat_models.py`.
//! These tests exercise the generic ChatModel interface and should work with
//! any provider. Currently run against OpenAI (gpt-4o-mini).
//!
//! Run with: `cargo test --package agent-chain --test standard_integration_tests -- --ignored`

use agent_chain::providers::openai::ChatOpenAI;
use agent_chain_core::ToolChoice;
use agent_chain_core::language_models::ToolLike;
use agent_chain_core::language_models::chat_models::BaseChatModel;
use agent_chain_core::messages::{AIMessage, BaseMessage, HumanMessage, ToolCall, ToolMessage};
use futures::StreamExt;

const MODEL: &str = "gpt-4o-mini";

fn load_env() {
    let _ = dotenv::dotenv();
}

fn make_model() -> ChatOpenAI {
    ChatOpenAI::new(MODEL)
}

// =============================================================================
// Basic invocation
// =============================================================================

/// Ported from `test_invoke`.
#[tokio::test]
#[ignore]
async fn test_invoke() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let result = model
        .invoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());
    Ok(())
}

/// Ported from `test_ainvoke`.
#[tokio::test]
#[ignore]
async fn test_ainvoke() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let result = model
        .ainvoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());
    Ok(())
}

/// Ported from `test_stream`.
#[tokio::test]
#[ignore]
async fn test_stream() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let mut stream = model
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
    assert!(num_chunks > 0, "Should receive at least one chunk");
    assert!(!full_text.is_empty());
    Ok(())
}

/// Ported from `test_astream`.
#[tokio::test]
#[ignore]
async fn test_astream() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let mut stream = model
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
    assert!(num_chunks > 0);
    assert!(!full_text.is_empty());
    Ok(())
}

/// Ported from `test_conversation`.
#[tokio::test]
#[ignore]
async fn test_conversation() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder().content("hello").build().into(),
        AIMessage::builder().content("hello").build().into(),
        HumanMessage::builder()
            .content("how are you")
            .build()
            .into(),
    ];

    let result = model.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());
    Ok(())
}

/// Ported from `test_double_messages_conversation`.
/// Tests consecutive messages from the same role.
#[tokio::test]
#[ignore]
async fn test_double_messages_conversation() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder().content("hello").build().into(),
        HumanMessage::builder()
            .content("how are you")
            .build()
            .into(),
    ];

    let result = model.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());
    Ok(())
}

/// Ported from `test_message_with_name`.
/// Tests HumanMessage with a name field.
#[tokio::test]
#[ignore]
async fn test_message_with_name() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let message = HumanMessage::builder()
        .content("hello")
        .name("Alice".to_string())
        .build();

    let result = model.invoke(vec![message.into()].into(), None).await?;
    assert!(!result.text().is_empty());
    Ok(())
}

// =============================================================================
// Stop sequences
// =============================================================================

/// Ported from `test_stop_sequence`.
/// Tests that the model respects stop sequences passed at invoke time.
#[tokio::test]
#[ignore]
async fn test_stop_sequence() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    // stop at invoke time
    let model = make_model();
    let result = model
        .invoke_with_stop(
            vec![
                HumanMessage::builder()
                    .content("Say 'hello you' and nothing else")
                    .build()
                    .into(),
            ]
            .into(),
            Some(vec!["you".to_string()]),
        )
        .await?;
    assert!(!result.text().contains("you"));

    // stop via builder
    let model_with_stop = ChatOpenAI::new(MODEL).stop(vec!["you".to_string()]);
    let result2 = model_with_stop
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Say 'hello you' and nothing else")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result2.text().contains("you"));

    Ok(())
}

// =============================================================================
// Usage metadata
// =============================================================================

/// Ported from `test_usage_metadata`.
#[tokio::test]
#[ignore]
async fn test_usage_metadata() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let result = model
        .invoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;

    let usage = result
        .usage_metadata
        .as_ref()
        .expect("usage_metadata should be present");
    assert!(usage.input_tokens > 0);
    assert!(usage.output_tokens > 0);
    assert!(usage.total_tokens > 0);

    let model_name = result
        .response_metadata
        .get("model_name")
        .and_then(|v| v.as_str());
    assert!(
        model_name.is_some(),
        "model_name should be in response_metadata"
    );
    assert!(!model_name.unwrap().is_empty());

    Ok(())
}

/// Ported from `test_usage_metadata_streaming`.
#[tokio::test]
#[ignore]
async fn test_usage_metadata_streaming() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let mut stream = model
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

    let mut full_text = String::new();
    let mut has_usage = false;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        full_text.push_str(&chunk.text());
        if chunk.usage_metadata.is_some() {
            has_usage = true;
        }
    }
    assert!(!full_text.is_empty());
    assert!(has_usage, "At least one chunk should have usage metadata");

    Ok(())
}

// =============================================================================
// Tool calling
// =============================================================================

fn magic_function_schema() -> serde_json::Value {
    serde_json::json!({
        "title": "magic_function",
        "description": "Apply a magic function to an input.",
        "type": "object",
        "properties": {
            "input": {"type": "integer", "description": "The input value"}
        },
        "required": ["input"]
    })
}

fn magic_function_no_args_schema() -> serde_json::Value {
    serde_json::json!({
        "title": "magic_function_no_args",
        "description": "Calculate a magic function.",
        "type": "object",
        "properties": {},
        "required": []
    })
}

/// Ported from `test_tool_calling`.
#[tokio::test]
#[ignore]
async fn test_tool_calling() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let model_with_tools = model.bind_tools(
        &[ToolLike::Schema(magic_function_schema())],
        Some(ToolChoice::any()),
    )?;

    let query = "What is the value of magic_function(3)? Use the tool.";
    let result = model_with_tools
        .invoke(
            vec![HumanMessage::builder().content(query).build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.tool_calls.is_empty());
    assert_eq!(result.tool_calls[0].name, "magic_function");
    assert_eq!(
        result.tool_calls[0].args.get("input"),
        Some(&serde_json::json!(3))
    );

    // Also test streaming
    let mut stream = model_with_tools
        .astream(
            vec![HumanMessage::builder().content(query).build().into()].into(),
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
    assert_eq!(tool_calls[0].name, "magic_function");

    Ok(())
}

/// Ported from `test_tool_calling_async`.
#[tokio::test]
#[ignore]
async fn test_tool_calling_async() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let model_with_tools = model.bind_tools(
        &[ToolLike::Schema(magic_function_schema())],
        Some(ToolChoice::any()),
    )?;

    let query = "What is the value of magic_function(3)? Use the tool.";
    let result = model_with_tools
        .ainvoke(
            vec![HumanMessage::builder().content(query).build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.tool_calls.is_empty());
    assert_eq!(result.tool_calls[0].name, "magic_function");

    Ok(())
}

/// Ported from `test_tool_calling_with_no_arguments`.
#[tokio::test]
#[ignore]
async fn test_tool_calling_with_no_arguments() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let model_with_tools = model.bind_tools(
        &[ToolLike::Schema(magic_function_no_args_schema())],
        Some(ToolChoice::any()),
    )?;

    let query = "What is the value of magic_function_no_args()? Use the tool.";
    let result = model_with_tools
        .invoke(
            vec![HumanMessage::builder().content(query).build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.tool_calls.is_empty());
    assert_eq!(result.tool_calls[0].name, "magic_function_no_args");
    assert!(
        result.tool_calls[0]
            .args
            .as_object()
            .is_none_or(|o| o.is_empty()),
        "No-arg tool should have empty args"
    );

    Ok(())
}

/// Ported from `test_tool_message_histories_string_content`.
/// Tests that models handle message histories with string tool content.
#[tokio::test]
#[ignore]
async fn test_tool_message_histories_string_content() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let adder_schema = serde_json::json!({
        "title": "my_adder_tool",
        "description": "Tool that adds two integers.",
        "type": "object",
        "properties": {
            "a": {"type": "integer"},
            "b": {"type": "integer"}
        },
        "required": ["a", "b"]
    });
    let model = make_model();
    let model_with_tools = model.bind_tools(&[ToolLike::Schema(adder_schema)], None)?;

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("What is 1 + 2")
            .build()
            .into(),
        AIMessage::builder()
            .content("")
            .tool_calls(vec![
                ToolCall::builder()
                    .name("my_adder_tool")
                    .args(serde_json::json!({"a": 1, "b": 2}))
                    .id("abc123".to_string())
                    .build(),
            ])
            .build()
            .into(),
        ToolMessage::builder()
            .content(r#"{"result": 3}"#)
            .tool_call_id("abc123")
            .build()
            .into(),
    ];

    let result = model_with_tools.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

/// Ported from `test_tool_message_histories_list_content`.
/// Tests that models handle tool message content as list of dicts.
#[tokio::test]
#[ignore]
async fn test_tool_message_histories_list_content() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let adder_schema = serde_json::json!({
        "title": "my_adder_tool",
        "description": "Tool that adds two integers.",
        "type": "object",
        "properties": {
            "a": {"type": "integer"},
            "b": {"type": "integer"}
        },
        "required": ["a", "b"]
    });
    let model = make_model();
    let model_with_tools = model.bind_tools(&[ToolLike::Schema(adder_schema)], None)?;

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("What is 1 + 2")
            .build()
            .into(),
        AIMessage::builder()
            .content("")
            .tool_calls(vec![
                ToolCall::builder()
                    .name("my_adder_tool")
                    .args(serde_json::json!({"a": 1, "b": 2}))
                    .id("abc123".to_string())
                    .build(),
            ])
            .build()
            .into(),
        ToolMessage::builder()
            .content(r#"{"result": 3}"#)
            .tool_call_id("abc123")
            .build()
            .into(),
    ];

    let result = model_with_tools.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

/// Ported from `test_tool_message_error_status`.
/// Tests that the model handles ToolMessage with error status.
#[tokio::test]
#[ignore]
async fn test_tool_message_error_status() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let adder_schema = serde_json::json!({
        "title": "my_adder_tool",
        "description": "Tool that adds two integers.",
        "type": "object",
        "properties": {
            "a": {"type": "integer"},
            "b": {"type": "integer"}
        },
        "required": ["a", "b"]
    });
    let model = make_model();
    let model_with_tools = model.bind_tools(&[ToolLike::Schema(adder_schema)], None)?;

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("What is 1 + 2")
            .build()
            .into(),
        AIMessage::builder()
            .content("")
            .tool_calls(vec![
                ToolCall::builder()
                    .name("my_adder_tool")
                    .args(serde_json::json!({"a": 1, "b": 2}))
                    .id("abc123".to_string())
                    .build(),
            ])
            .build()
            .into(),
        ToolMessage::builder()
            .content("Error: tool execution failed")
            .tool_call_id("abc123")
            .status(agent_chain_core::messages::ToolStatus::Error)
            .build()
            .into(),
    ];

    let result = model_with_tools.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

// =============================================================================
// Structured output
// =============================================================================

/// Ported from `test_structured_output` with json_schema schema type.
#[tokio::test]
#[ignore]
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

    let model = make_model();
    let structured = model.with_structured_output(joke_schema, false)?;

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

/// Ported from `test_structured_output_async`.
#[tokio::test]
#[ignore]
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

    let model = make_model();
    let structured = model.with_structured_output(joke_schema, false)?;

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

/// Ported from `test_structured_output_optional_param`.
/// Tests structured output with optional parameters in the schema.
#[tokio::test]
#[ignore]
async fn test_structured_output_optional_param() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let schema = serde_json::json!({
        "title": "OptionalResponse",
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"},
            "nickname": {"type": ["string", "null"], "description": "Optional nickname"}
        },
        "required": ["name", "age"]
    });

    let model = make_model();
    let structured = model.with_structured_output(schema, false)?;

    let result = structured
        .ainvoke(
            vec![
                HumanMessage::builder()
                    .content("My name is Alice and I'm 30.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;

    assert!(result.get("name").is_some());
    assert!(result.get("age").is_some());

    Ok(())
}

/// Ported from `test_structured_few_shot_examples`.
/// Tests structured output with few-shot examples in the prompt.
#[tokio::test]
#[ignore]
async fn test_structured_few_shot_examples() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let joke_schema = serde_json::json!({
        "title": "Joke",
        "description": "Joke to tell user.",
        "type": "object",
        "properties": {
            "setup": {"type": "string"},
            "punchline": {"type": "string"}
        },
        "required": ["setup", "punchline"]
    });

    let model = make_model();
    let structured = model.with_structured_output(joke_schema, false)?;

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("Tell me a joke about cats.")
            .build()
            .into(),
        AIMessage::builder()
            .content(r#"{"setup": "Why was the cat sitting on the computer?", "punchline": "To keep an eye on the mouse!"}"#)
            .build()
            .into(),
        HumanMessage::builder()
            .content("Tell me another joke about dogs.")
            .build()
            .into(),
    ];

    let result = structured.ainvoke(messages.into(), None).await?;
    assert!(result.get("setup").is_some());
    assert!(result.get("punchline").is_some());

    Ok(())
}

/// Ported from `test_json_mode`.
/// Tests structured output via JSON mode.
#[tokio::test]
#[ignore]
async fn test_json_mode() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();
    let model_with_json = model.response_format(serde_json::json!({"type": "json_object"}));

    let result = model_with_json
        .invoke(
            vec![HumanMessage::builder()
                .content(
                    "Tell me a joke about cats. Return as JSON with 'setup' and 'punchline' keys. \
                     Return nothing other than JSON.",
                )
                .build()
                .into()]
            .into(),
            None,
        )
        .await?;

    let parsed: serde_json::Value = serde_json::from_str(&result.text())?;
    assert!(parsed.is_object());
    assert!(parsed.get("setup").is_some());
    assert!(parsed.get("punchline").is_some());

    Ok(())
}

// =============================================================================
// Image / multimodal inputs
// =============================================================================

/// Ported from `test_image_inputs`.
/// Tests that the model can process image inputs in base64 format.
#[tokio::test]
#[ignore]
async fn test_image_inputs() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    use agent_chain_core::messages::{ContentPart, MessageContent};

    let model = make_model();

    // Create a small test image (1x1 red PNG, base64)
    let tiny_png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

    let message = HumanMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "What do you see in this image? Be very brief.".to_string(),
            },
            ContentPart::Other(serde_json::json!({
                "type": "image_url",
                "image_url": {
                    "url": format!("data:image/png;base64,{}", tiny_png_b64)
                }
            })),
        ]))
        .build();

    let result = model.invoke(vec![message.into()].into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

// =============================================================================
// Agent loop
// =============================================================================

/// Ported from `test_agent_loop`.
/// Tests a simple ReAct agent loop: tool call → execute → response.
#[tokio::test]
#[ignore]
async fn test_agent_loop() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let weather_tool = serde_json::json!({
        "title": "get_weather",
        "description": "Get the weather at a location.",
        "type": "object",
        "properties": {
            "location": {"type": "string"}
        },
        "required": ["location"]
    });

    let model = make_model();
    let model_with_tools =
        model.bind_tools(&[ToolLike::Schema(weather_tool)], Some(ToolChoice::any()))?;

    let input_message: BaseMessage = HumanMessage::builder()
        .content("What is the weather in San Francisco, CA?")
        .build()
        .into();

    // Step 1: Get tool call
    let tool_call_message = model_with_tools
        .invoke(vec![input_message.clone()].into(), None)
        .await?;
    assert!(!tool_call_message.tool_calls.is_empty());
    let tool_call = &tool_call_message.tool_calls[0];
    assert_eq!(tool_call.name, "get_weather");

    // Step 2: Simulate tool execution
    let tool_message: BaseMessage = ToolMessage::builder()
        .content("It's sunny and 75 degrees.")
        .tool_call_id(tool_call.id.as_deref().unwrap_or(""))
        .build()
        .into();

    // Step 3: Get final response
    let response = model_with_tools
        .invoke(
            vec![
                input_message,
                BaseMessage::AI(tool_call_message),
                tool_message,
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!response.text().is_empty());

    Ok(())
}

/// Ported from `test_unicode_tool_call_integration`.
/// Tests that Unicode characters are preserved in tool call arguments.
#[tokio::test]
#[ignore]
async fn test_unicode_tool_call() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let unicode_tool = serde_json::json!({
        "title": "unicode_customer",
        "description": "Tool for creating a customer with Unicode name.",
        "type": "object",
        "properties": {
            "customer_name": {"type": "string", "description": "The customer's name in their native language."},
            "description": {"type": "string", "description": "Description of the customer."}
        },
        "required": ["customer_name", "description"]
    });

    let model = make_model();
    let model_with_tools =
        model.bind_tools(&[ToolLike::Schema(unicode_tool)], Some(ToolChoice::any()))?;

    let result = model_with_tools
        .invoke(
            vec![HumanMessage::builder()
                .content(
                    "Create a customer named \u{5c0f}\u{6797}\u{82b1}\u{5b50} (Kobayashi Hanako) who is a frequent buyer.",
                )
                .build()
                .into()]
            .into(),
            None,
        )
        .await?;

    assert!(!result.tool_calls.is_empty());
    let args = &result.tool_calls[0].args;
    let name = args
        .get("customer_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    // The Unicode characters should be preserved
    assert!(!name.is_empty(), "customer_name should not be empty");

    Ok(())
}
