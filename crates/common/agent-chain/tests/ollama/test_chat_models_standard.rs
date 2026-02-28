use agent_chain::providers::ollama::ChatOllama;
use agent_chain_core::language_models::ToolLike;
use agent_chain_core::language_models::chat_models::BaseChatModel;
use agent_chain_core::messages::HumanMessage;

const DEFAULT_MODEL: &str = "llama3.1";
fn load_env() {
    let _ = dotenv::dotenv();
}

// =============================================================================

/// Ported from `TestChatOllama.test_tool_calling` (from ChatModelIntegrationTests).
/// Tests basic tool calling with ChatOllama.
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
