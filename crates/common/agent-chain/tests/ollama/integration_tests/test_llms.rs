use agent_chain::providers::ollama::OllamaLLM;
use agent_chain_core::language_models::BaseLLM;
use agent_chain_core::outputs::GenerationChunk;
use futures::StreamExt;

const DEFAULT_MODEL: &str = "llama3.1";
const REASONING_MODEL: &str = "deepseek-r1:1.5b";
const SAMPLE_PROMPT: &str = "What is 3^3?";

fn load_env() {
    let _ = dotenv::dotenv();
}

// =============================================================================
// Ported from integration_tests/test_llms.py
// =============================================================================

/// Ported from `test_invoke`.
#[tokio::test]
async fn test_llm_invoke() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(DEFAULT_MODEL);

    let result = llm.invoke("I'm Pickle Rick".into(), None).await?;
    assert!(!result.is_empty());

    Ok(())
}

/// Ported from `test_ainvoke`.
#[tokio::test]
async fn test_llm_ainvoke() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(DEFAULT_MODEL);

    let result = llm.ainvoke("I'm Pickle Rick".into(), None).await?;
    assert!(!result.is_empty());

    Ok(())
}

/// Ported from `test_stream_text_tokens`.
#[tokio::test]
async fn test_llm_stream() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(DEFAULT_MODEL);

    let mut stream = llm.stream("Hi.".into(), None, None).await?;

    let mut got_chunk = false;
    while let Some(chunk) = stream.next().await {
        let _chunk = chunk?;
        got_chunk = true;
    }
    assert!(got_chunk, "Stream should produce at least one chunk");

    Ok(())
}

/// Ported from `test_astream_text_tokens`.
#[tokio::test]
async fn test_llm_astream() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(DEFAULT_MODEL);

    let mut stream = llm.astream("Hi.".into(), None, None).await?;

    let mut got_chunk = false;
    while let Some(chunk) = stream.next().await {
        let _chunk = chunk?;
        got_chunk = true;
    }
    assert!(got_chunk, "Async stream should produce at least one chunk");

    Ok(())
}

/// Ported from `test_batch`.
#[tokio::test]
async fn test_llm_batch() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(DEFAULT_MODEL);

    let result = llm
        .batch(
            vec!["I'm Pickle Rick".into(), "I'm not Pickle Rick".into()],
            None,
        )
        .await?;

    assert_eq!(result.len(), 2);
    for token in &result {
        assert!(!token.is_empty());
    }

    Ok(())
}

/// Ported from `test_abatch`.
#[tokio::test]
async fn test_llm_abatch() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(DEFAULT_MODEL);

    let result = llm
        .abatch(
            vec!["I'm Pickle Rick".into(), "I'm not Pickle Rick".into()],
            None,
        )
        .await?;

    assert_eq!(result.len(), 2);
    for token in &result {
        assert!(!token.is_empty());
    }

    Ok(())
}

/// Ported from `test_batch_tags`.
#[tokio::test]
async fn test_llm_batch_tags() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(DEFAULT_MODEL);

    let result = llm
        .batch(
            vec!["I'm Pickle Rick".into(), "I'm not Pickle Rick".into()],
            None,
        )
        .await?;

    assert_eq!(result.len(), 2);
    for token in &result {
        assert!(!token.is_empty());
    }

    Ok(())
}

/// Ported from `test_abatch_tags`.
#[tokio::test]
async fn test_llm_abatch_tags() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(DEFAULT_MODEL);

    let result = llm
        .abatch(
            vec!["I'm Pickle Rick".into(), "I'm not Pickle Rick".into()],
            None,
        )
        .await?;

    assert_eq!(result.len(), 2);
    for token in &result {
        assert!(!token.is_empty());
    }

    Ok(())
}

/// Ported from `test__stream_no_reasoning`.
#[tokio::test]
async fn test_llm_stream_no_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(REASONING_MODEL).num_ctx(4096);

    let mut stream = llm.stream(SAMPLE_PROMPT.into(), None, None).await?;

    let mut result_chunk: Option<GenerationChunk> = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        result_chunk = Some(match result_chunk {
            Some(existing) => existing + chunk,
            None => chunk,
        });
    }

    let result_chunk = result_chunk.expect("Should have received at least one chunk");
    assert!(!result_chunk.text.is_empty());
    assert!(result_chunk.generation_info.is_some());
    let info = result_chunk.generation_info.as_ref().unwrap();
    assert!(
        info.get("reasoning_content").is_none(),
        "Should not have reasoning_content when reasoning is not enabled"
    );

    Ok(())
}

/// Ported from `test__astream_no_reasoning`.
#[tokio::test]
async fn test_llm_astream_no_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(REASONING_MODEL).num_ctx(4096);

    let mut stream = llm.astream(SAMPLE_PROMPT.into(), None, None).await?;

    let mut result_chunk: Option<GenerationChunk> = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        result_chunk = Some(match result_chunk {
            Some(existing) => existing + chunk,
            None => chunk,
        });
    }

    let result_chunk = result_chunk.expect("Should have received at least one chunk");
    assert!(!result_chunk.text.is_empty());
    assert!(result_chunk.generation_info.is_some());
    let info = result_chunk.generation_info.as_ref().unwrap();
    assert!(
        info.get("reasoning_content").is_none(),
        "Should not have reasoning_content when reasoning is not enabled"
    );

    Ok(())
}

/// Ported from `test__stream_with_reasoning`.
#[tokio::test]
async fn test_llm_stream_with_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(true);

    let mut stream = llm.stream(SAMPLE_PROMPT.into(), None, None).await?;

    let mut result_chunk: Option<GenerationChunk> = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        result_chunk = Some(match result_chunk {
            Some(existing) => existing + chunk,
            None => chunk,
        });
    }

    let result_chunk = result_chunk.expect("Should have received at least one chunk");
    assert!(!result_chunk.text.is_empty());
    assert!(result_chunk.generation_info.is_some());
    let info = result_chunk.generation_info.as_ref().unwrap();
    let reasoning_content = info
        .get("reasoning_content")
        .and_then(|v| v.as_str())
        .expect("Should have reasoning_content");
    assert!(!reasoning_content.is_empty());
    assert!(!result_chunk.text.contains("<think>"));
    assert!(!result_chunk.text.contains("</think>"));
    assert!(!reasoning_content.contains("<think>"));
    assert!(!reasoning_content.contains("</think>"));

    Ok(())
}

/// Ported from `test__astream_with_reasoning`.
#[tokio::test]
async fn test_llm_astream_with_reasoning() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let llm = OllamaLLM::new(REASONING_MODEL)
        .num_ctx(4096)
        .reasoning(true);

    let mut stream = llm.astream(SAMPLE_PROMPT.into(), None, None).await?;

    let mut result_chunk: Option<GenerationChunk> = None;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        result_chunk = Some(match result_chunk {
            Some(existing) => existing + chunk,
            None => chunk,
        });
    }

    let result_chunk = result_chunk.expect("Should have received at least one chunk");
    assert!(!result_chunk.text.is_empty());
    assert!(result_chunk.generation_info.is_some());
    let info = result_chunk.generation_info.as_ref().unwrap();
    let reasoning_content = info
        .get("reasoning_content")
        .and_then(|v| v.as_str())
        .expect("Should have reasoning_content");
    assert!(!reasoning_content.is_empty());
    assert!(!result_chunk.text.contains("<think>"));
    assert!(!result_chunk.text.contains("</think>"));
    assert!(!reasoning_content.contains("<think>"));
    assert!(!reasoning_content.contains("</think>"));

    Ok(())
}
