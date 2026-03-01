use agent_chain::providers::ollama::OllamaEmbeddings;
use agent_chain_core::embeddings::Embeddings;

const DEFAULT_MODEL: &str = "llama3.1";

fn load_env() {
    let _ = dotenv::dotenv();
}

// =============================================================================
// Ported from integration_tests/test_embeddings.py
// =============================================================================

/// Ported from `TestOllamaEmbeddings.test_embed_documents`.
#[tokio::test]
async fn test_embed_documents() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let embeddings = OllamaEmbeddings::new(DEFAULT_MODEL);

    let texts = vec!["Hello world".to_string(), "The meaning of life".to_string()];
    let result = embeddings.aembed_documents(texts).await?;

    assert_eq!(result.len(), 2);
    assert!(!result[0].is_empty(), "First embedding should not be empty");
    assert!(
        !result[1].is_empty(),
        "Second embedding should not be empty"
    );

    Ok(())
}

/// Ported from `TestOllamaEmbeddings.test_embed_query`.
#[tokio::test]
async fn test_embed_query() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let embeddings = OllamaEmbeddings::new(DEFAULT_MODEL);

    let result = embeddings.aembed_query("Hello world").await?;

    assert!(!result.is_empty(), "Embedding should not be empty");

    Ok(())
}

/// Test that sync embed_documents works.
#[tokio::test(flavor = "multi_thread")]
async fn test_sync_embed_documents() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let embeddings = OllamaEmbeddings::new(DEFAULT_MODEL);

    let texts = vec!["Hello world".to_string()];
    let result = embeddings.embed_documents(texts)?;

    assert_eq!(result.len(), 1);
    assert!(!result[0].is_empty());

    Ok(())
}

/// Test that sync embed_query works.
#[tokio::test(flavor = "multi_thread")]
async fn test_sync_embed_query() -> Result<(), Box<dyn std::error::Error>> {
    load_env();
    let embeddings = OllamaEmbeddings::new(DEFAULT_MODEL);

    let result = embeddings.embed_query("Hello world")?;

    assert!(!result.is_empty());

    Ok(())
}
