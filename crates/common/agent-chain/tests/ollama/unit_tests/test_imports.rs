/// Ported from `test_all_imports` in `unit_tests/test_imports.py`.
///
/// Verifies that the primary Ollama types are accessible through the public API.
#[test]
fn test_all_imports() {
    // Verify core types are importable (compile-time check).
    fn _assert_importable() {
        let _: fn(&str) -> agent_chain::providers::ollama::OllamaLLM =
            |model| agent_chain::providers::ollama::OllamaLLM::new(model);
        let _: fn(&str) -> agent_chain::providers::ollama::ChatOllama =
            |model| agent_chain::providers::ollama::ChatOllama::new(model);
        let _: fn(&str) -> agent_chain::providers::ollama::OllamaEmbeddings =
            |model| agent_chain::providers::ollama::OllamaEmbeddings::new(model);
    }
}
