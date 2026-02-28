use agent_chain::providers::ollama::ChatOllama;
use agent_chain_core::language_models::chat_models::BaseChatModel;
use agent_chain_core::messages::{BaseMessage, HumanMessage};
use futures::StreamExt;

const REASONING_MODEL: &str = "deepseek-r1:1.5b";
const SAMPLE_PROMPT: &str = "What is 3^3?";
fn load_env() {
    let _ = dotenv::dotenv();
}

// =============================================================================
// test_chat_models_reasoning.py — Reasoning mode tests
// =============================================================================

/// Ported from `test_stream_no_reasoning` (sync).
#[tokio::test]

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
