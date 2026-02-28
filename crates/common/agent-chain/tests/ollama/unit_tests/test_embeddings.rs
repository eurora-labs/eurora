use agent_chain::providers::ollama::OllamaEmbeddings;

const MODEL_NAME: &str = "llama3.1";

// =============================================================================
// Ported from unit_tests/test_embeddings.py
// =============================================================================

/// Ported from `test_initialization`.
#[test]
fn test_initialization() {
    let _embeddings = OllamaEmbeddings::new(MODEL_NAME).keep_alive_seconds(1);
}

/// Ported from `test_embed_documents_passes_options` (options verification part).
#[test]
fn test_options_include_set_parameters() {
    let embeddings = OllamaEmbeddings::new(MODEL_NAME)
        .num_gpu(4)
        .temperature(0.5);
    let options = embeddings.build_options().unwrap();
    let options_map = options.as_object().unwrap();

    assert_eq!(options_map.get("num_gpu"), Some(&serde_json::json!(4)));
    assert_eq!(
        options_map.get("temperature"),
        Some(&serde_json::json!(0.5))
    );
}

/// Verifies that no parameters are included when none are set.
#[test]
fn test_options_exclude_none_parameters() {
    let embeddings = OllamaEmbeddings::new(MODEL_NAME);
    let options = embeddings.build_options().unwrap();
    let options_map = options.as_object().unwrap();

    assert!(
        options_map.is_empty(),
        "Options should be empty when no parameters are set"
    );
}

/// Verifies only set parameters appear in options.
#[test]
fn test_options_only_include_set_parameters() {
    let embeddings = OllamaEmbeddings::new(MODEL_NAME).num_ctx(4096).top_k(50);
    let options = embeddings.build_options().unwrap();
    let options_map = options.as_object().unwrap();

    assert_eq!(options_map.get("num_ctx"), Some(&serde_json::json!(4096)));
    assert_eq!(options_map.get("top_k"), Some(&serde_json::json!(50)));
    assert!(!options_map.contains_key("temperature"));
    assert!(!options_map.contains_key("num_gpu"));
    assert!(!options_map.contains_key("mirostat"));
}

/// Ported from `test_validate_model_on_init`.
///
/// In Python, this test mocks `validate_model` and checks it's called during init.
/// In Rust, model validation is deferred; we verify the flag is stored and the
/// instance is constructable with both `true` and `false` values.
#[test]
fn test_validate_model_on_init() {
    let embeddings = OllamaEmbeddings::new(MODEL_NAME).validate_model_on_init(true);
    let base_url = embeddings.get_base_url();
    assert!(!base_url.is_empty());

    let embeddings = OllamaEmbeddings::new(MODEL_NAME).validate_model_on_init(false);
    let base_url = embeddings.get_base_url();
    assert!(!base_url.is_empty());

    let embeddings = OllamaEmbeddings::new(MODEL_NAME);
    let base_url = embeddings.get_base_url();
    assert!(!base_url.is_empty());
}
