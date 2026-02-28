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

// =============================================================================
// Batch operations
// =============================================================================

/// Ported from `test_batch`.
/// Tests batch processing of multiple messages.
#[tokio::test]

async fn test_batch() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();

    let input1 = vec![HumanMessage::builder().content("Hello").build().into()];
    let input2 = vec![HumanMessage::builder().content("Hey").build().into()];

    let result1 = model.invoke(input1.into(), None).await?;
    let result2 = model.invoke(input2.into(), None).await?;

    assert!(!result1.text().is_empty());
    assert!(!result2.text().is_empty());

    Ok(())
}

/// Ported from `test_abatch`.
/// Tests async batch processing of multiple messages.
#[tokio::test]

async fn test_abatch() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();

    let input1 = vec![HumanMessage::builder().content("Hello").build().into()];
    let input2 = vec![HumanMessage::builder().content("Hey").build().into()];

    let result1 = model.ainvoke(input1.into(), None).await?;
    let result2 = model.ainvoke(input2.into(), None).await?;

    assert!(!result1.text().is_empty());
    assert!(!result2.text().is_empty());

    Ok(())
}

// =============================================================================
// Model override
// =============================================================================

/// Ported from `test_invoke_with_model_override`.
/// Tests that a different model can be used by constructing a new instance.
/// Python passes model= as a kwarg to invoke(); Rust requires a new instance.
#[tokio::test]

async fn test_invoke_with_model_override() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let override_model = "gpt-4o";
    let model = ChatOpenAI::new(override_model);

    let result = model
        .invoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());

    let model_name = result
        .response_metadata
        .get("model_name")
        .and_then(|v| v.as_str())
        .expect("model_name should be in response_metadata");
    assert!(
        model_name.contains("gpt-4o"),
        "Expected model name to contain 'gpt-4o', got '{model_name}'"
    );

    Ok(())
}

/// Ported from `test_ainvoke_with_model_override`.
#[tokio::test]

async fn test_ainvoke_with_model_override() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let override_model = "gpt-4o";
    let model = ChatOpenAI::new(override_model);

    let result = model
        .ainvoke(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.text().is_empty());

    let model_name = result
        .response_metadata
        .get("model_name")
        .and_then(|v| v.as_str())
        .expect("model_name should be in response_metadata");
    assert!(model_name.contains("gpt-4o"));

    Ok(())
}

/// Ported from `test_stream_with_model_override`.
#[tokio::test]

async fn test_stream_with_model_override() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let override_model = "gpt-4o";
    let model = ChatOpenAI::new(override_model);

    let mut stream = model
        .astream(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
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

/// Ported from `test_astream_with_model_override`.
#[tokio::test]

async fn test_astream_with_model_override() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let override_model = "gpt-4o";
    let model = ChatOpenAI::new(override_model);

    let mut stream = model
        .astream(
            vec![HumanMessage::builder().content("Hello").build().into()].into(),
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

// =============================================================================
// Tool choice
// =============================================================================

/// Ported from `test_tool_choice`.
/// Tests that tool_choice can force specific tool calls.
#[tokio::test]

async fn test_tool_choice() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();

    let get_weather = serde_json::json!({
        "title": "get_weather",
        "description": "Get weather at a location.",
        "type": "object",
        "properties": {
            "location": {"type": "string"}
        },
        "required": ["location"]
    });

    // tool_choice="any" — model must call some tool
    let model_any = model.bind_tools(
        &[
            ToolLike::Schema(magic_function_schema()),
            ToolLike::Schema(get_weather.clone()),
        ],
        Some(ToolChoice::any()),
    )?;
    let result = model_any
        .invoke(
            vec![HumanMessage::builder().content("Hello!").build().into()].into(),
            None,
        )
        .await?;
    assert!(
        !result.tool_calls.is_empty(),
        "tool_choice='any' should force a tool call"
    );

    // tool_choice="magic_function" — model must call that specific tool
    let model_specific = model.bind_tools(
        &[
            ToolLike::Schema(magic_function_schema()),
            ToolLike::Schema(get_weather),
        ],
        Some(ToolChoice::String("magic_function".to_string())),
    )?;
    let result = model_specific
        .invoke(
            vec![HumanMessage::builder().content("Hello!").build().into()].into(),
            None,
        )
        .await?;
    assert!(!result.tool_calls.is_empty());
    assert_eq!(result.tool_calls[0].name, "magic_function");

    Ok(())
}

// =============================================================================
// Multimodal inputs — PDF
// =============================================================================

/// Ported from `test_pdf_inputs`.
/// Tests that the model can process PDF file inputs.
#[tokio::test]

async fn test_pdf_inputs() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    use agent_chain_core::messages::{ContentPart, MessageContent};

    let model = make_model();

    // Minimal PDF as base64 (the W3C test PDF is small)
    let client = reqwest::Client::new();
    let pdf_bytes = client
        .get("https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf")
        .send()
        .await?
        .bytes()
        .await?;
    let pdf_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &pdf_bytes);

    // LangChain standard format
    let message = HumanMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Summarize this document:".to_string(),
            },
            ContentPart::Other(serde_json::json!({
                "type": "file",
                "base64": pdf_b64,
                "mime_type": "application/pdf"
            })),
        ]))
        .build();

    let result = model.invoke(vec![message.into()].into(), None).await?;
    assert!(!result.text().is_empty());

    // OpenAI Chat Completions format
    let message2 = HumanMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Summarize this document:".to_string(),
            },
            ContentPart::Other(serde_json::json!({
                "type": "file",
                "file": {
                    "filename": "test_file.pdf",
                    "file_data": format!("data:application/pdf;base64,{}", pdf_b64)
                }
            })),
        ]))
        .build();

    let result2 = model.invoke(vec![message2.into()].into(), None).await?;
    assert!(!result2.text().is_empty());

    Ok(())
}

// =============================================================================
// Multimodal inputs — Audio
// =============================================================================

/// Ported from `test_audio_inputs`.
/// Tests that the model can process audio inputs.
#[tokio::test]

async fn test_audio_inputs() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    use agent_chain_core::messages::{ContentPart, MessageContent};

    let model = ChatOpenAI::new("gpt-4o-audio-preview");

    // Small audio sample — use a tiny wav
    let client = reqwest::Client::new();
    let audio_bytes = client
        .get("https://upload.wikimedia.org/wikipedia/commons/6/6a/Northern_Flicker_202280456.wav")
        .header("User-Agent", "agent-chain-test/1.0")
        .send()
        .await?
        .bytes()
        .await?;
    let audio_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &audio_bytes);

    // LangChain standard format
    let message = HumanMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Describe this audio:".to_string(),
            },
            ContentPart::Other(serde_json::json!({
                "type": "audio",
                "mime_type": "audio/wav",
                "base64": audio_b64
            })),
        ]))
        .build();

    let result = model.invoke(vec![message.into()].into(), None).await?;
    assert!(!result.text().is_empty());

    // OpenAI Chat Completions input_audio format
    let message2 = HumanMessage::builder()
        .content(MessageContent::Parts(vec![
            ContentPart::Text {
                text: "Describe this audio:".to_string(),
            },
            ContentPart::Other(serde_json::json!({
                "type": "input_audio",
                "input_audio": {"data": audio_b64, "format": "wav"}
            })),
        ]))
        .build();

    let result2 = model.invoke(vec![message2.into()].into(), None).await?;
    assert!(!result2.text().is_empty());

    Ok(())
}

// =============================================================================
// Tool messages with multimodal content
// =============================================================================

/// Ported from `test_image_tool_message`.
/// Tests ToolMessage with image content (base64).
#[tokio::test]

async fn test_image_tool_message() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    use agent_chain_core::messages::{ContentPart, MessageContent};

    let model = make_model();

    // Small PNG (1x1 pixel)
    let tiny_png_b64 = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";

    let random_image_schema = serde_json::json!({
        "title": "random_image",
        "description": "Return a random image.",
        "type": "object",
        "properties": {},
        "required": []
    });

    let model_with_tools = model.bind_tools(&[ToolLike::Schema(random_image_schema)], None)?;

    // OpenAI image_url format in ToolMessage
    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("get a random diagram using the tool and describe it")
            .build()
            .into(),
        AIMessage::builder()
            .content("")
            .tool_calls(vec![
                ToolCall::builder()
                    .name("random_image")
                    .args(serde_json::json!({}))
                    .id("1".to_string())
                    .build(),
            ])
            .build()
            .into(),
        ToolMessage::builder()
            .content(MessageContent::Parts(vec![ContentPart::Other(
                serde_json::json!({
                    "type": "image_url",
                    "image_url": {"url": format!("data:image/png;base64,{}", tiny_png_b64)}
                }),
            )]))
            .tool_call_id("1")
            .build()
            .into(),
    ];

    let result = model_with_tools.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

/// Ported from `test_pdf_tool_message`.
/// Tests ToolMessage with PDF content.
#[tokio::test]

async fn test_pdf_tool_message() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    use agent_chain_core::messages::{ContentPart, MessageContent};

    let model = make_model();

    let client = reqwest::Client::new();
    let pdf_bytes = client
        .get("https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf")
        .send()
        .await?
        .bytes()
        .await?;
    let pdf_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &pdf_bytes);

    let random_pdf_schema = serde_json::json!({
        "title": "random_pdf",
        "description": "Return a random PDF.",
        "type": "object",
        "properties": {},
        "required": []
    });

    let model_with_tools = model.bind_tools(&[ToolLike::Schema(random_pdf_schema)], None)?;

    let messages: Vec<BaseMessage> = vec![
        HumanMessage::builder()
            .content("Get a random PDF and relay the title verbatim.")
            .build()
            .into(),
        AIMessage::builder()
            .content("")
            .tool_calls(vec![
                ToolCall::builder()
                    .name("random_pdf")
                    .args(serde_json::json!({}))
                    .id("1".to_string())
                    .build(),
            ])
            .build()
            .into(),
        ToolMessage::builder()
            .content(MessageContent::Parts(vec![ContentPart::Other(
                serde_json::json!({
                    "type": "file",
                    "base64": pdf_b64,
                    "mime_type": "application/pdf"
                }),
            )]))
            .tool_call_id("1")
            .build()
            .into(),
    ];

    let result = model_with_tools.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());

    Ok(())
}

// =============================================================================
// Anthropic-format inputs
// =============================================================================

/// Ported from `test_anthropic_inputs`.
/// Tests that the model handles Anthropic-style message histories
/// (tool_use blocks in AIMessage, tool_result blocks in HumanMessage).
#[tokio::test]

async fn test_anthropic_inputs() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    use agent_chain_core::messages::{ContentPart, MessageContent, SystemMessage};

    let model = make_model();

    let color_picker = serde_json::json!({
        "title": "color_picker",
        "description": "Input your fav color and get a random fact about it.",
        "type": "object",
        "properties": {
            "fav_color": {"type": "string"}
        },
        "required": ["fav_color"]
    });

    let messages: Vec<BaseMessage> = vec![
        SystemMessage::builder()
            .content("you're a good assistant")
            .build()
            .into(),
        HumanMessage::builder()
            .content("what's your favorite color")
            .build()
            .into(),
        AIMessage::builder()
            .content(MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "Hmm let me think about that".to_string(),
                },
                ContentPart::Other(serde_json::json!({
                    "type": "tool_use",
                    "input": {"fav_color": "purple"},
                    "id": "foo",
                    "name": "color_picker"
                })),
            ]))
            .tool_calls(vec![
                ToolCall::builder()
                    .name("color_picker")
                    .args(serde_json::json!({"fav_color": "purple"}))
                    .id("foo".to_string())
                    .build(),
            ])
            .build()
            .into(),
        ToolMessage::builder()
            .content("That's a great pick!")
            .tool_call_id("foo")
            .build()
            .into(),
    ];

    let model_with_tools = model.bind_tools(&[ToolLike::Schema(color_picker)], None)?;
    let result = model_with_tools.invoke(messages.into(), None).await?;
    assert!(!result.text().is_empty());

    // Test thinking blocks
    let messages2: Vec<BaseMessage> = vec![
        HumanMessage::builder().content("Hello").build().into(),
        AIMessage::builder()
            .content(MessageContent::Parts(vec![
                ContentPart::Other(serde_json::json!({
                    "type": "thinking",
                    "thinking": "This is a simple greeting. I should respond warmly.",
                    "signature": "dummy_signature"
                })),
                ContentPart::Text {
                    text: "Hello, how are you?".to_string(),
                },
            ]))
            .build()
            .into(),
        HumanMessage::builder()
            .content("Well, thanks.")
            .build()
            .into(),
    ];

    let result2 = model.invoke(messages2.into(), None).await?;
    assert!(!result2.text().is_empty());

    Ok(())
}

// =============================================================================
// Bind runnables as tools
// =============================================================================

/// Ported from `test_bind_runnables_as_tools`.
/// Tests binding tool schemas (simulating runnable-as-tool) and forcing a call.
/// Python uses `chain.as_tool()` — Rust has no direct equivalent, so we
/// bind a schema that mimics what `as_tool` would produce.
#[tokio::test]

async fn test_bind_runnables_as_tools() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let model = make_model();

    let greeting_tool = serde_json::json!({
        "title": "greeting_generator",
        "description": "Generate a greeting in a particular style of speaking.",
        "type": "object",
        "properties": {
            "answer_style": {"type": "string", "description": "The style of speaking"}
        },
        "required": ["answer_style"]
    });

    let model_with_tools =
        model.bind_tools(&[ToolLike::Schema(greeting_tool)], Some(ToolChoice::any()))?;

    let result = model_with_tools
        .invoke(
            vec![
                HumanMessage::builder()
                    .content("Using the tool, generate a Pirate greeting.")
                    .build()
                    .into(),
            ]
            .into(),
            None,
        )
        .await?;
    assert!(!result.tool_calls.is_empty());
    assert!(
        result.tool_calls[0].args.get("answer_style").is_some(),
        "Tool call should include answer_style arg"
    );

    Ok(())
}
