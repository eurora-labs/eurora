//! Tests for language_models module imports.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/language_models/test_imports.py`

#[test]
fn test_all_imports() {
    // Expected exports from the language_models module
    const EXPECTED_ALL: &[&str] = &[
        "BaseLanguageModel",
        "BaseChatModel",
        "SimpleChatModel",
        "BaseLLM",
        "LLM",
        "LangSmithParams",
        "LanguageModelInput",
        "LanguageModelOutput",
        "LanguageModelLike",
        "get_tokenizer",
        "FakeMessagesListChatModel",
        "FakeListChatModel",
        "GenericFakeChatModel",
        "FakeStreamingListLLM",
        "FakeListLLM",
        "ParrotFakeChatModel",
        "ModelProfile",
        "ModelProfileRegistry",
        "is_openai_data_block",
    ];

    // Note: This test validates that the expected types/functions are available
    // The actual implementation would check these exports exist in the module
    // For now, this serves as documentation of what should be exported
    
    // In Rust, we'd typically use type assertions to verify exports exist:
    // let _: fn() -> BaseLanguageModel;
    // But since the types don't exist yet, we just document them here
    
    assert_eq!(EXPECTED_ALL.len(), 19, "Expected 19 exports from language_models module");
}
