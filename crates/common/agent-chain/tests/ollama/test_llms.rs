use agent_chain::providers::ollama::ChatOllama;
use agent_chain_core::language_models::chat_models::BaseChatModel;
use agent_chain_core::messages::HumanMessage;
use futures::StreamExt;

const DEFAULT_MODEL: &str = "llama3.1";
const REASONING_MODEL: &str = "deepseek-r1:1.5b";
const SAMPLE_PROMPT: &str = "What is 3^3?";
fn load_env() {
    let _ = dotenv::dotenv();
}

// =============================================================================

/// Ported from `test_invoke` (OllamaLLM).
/// Uses ChatOllama since OllamaLLM is not implemented in Rust.
#[tokio::test]

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
