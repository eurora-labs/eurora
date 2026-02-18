//! Unit tests for outputs module exports.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/outputs/test_imports.py`
//!
//! Note: Rust doesn't have a direct equivalent to Python's `__all__`. Instead, we verify
//! that the expected types are re-exported from the outputs module and can be used.

use agent_chain_core::outputs;

/// Expected exports from the outputs module (matching Python's EXPECTED_ALL).
const EXPECTED_ALL: &[&str] = &[
    "ChatGeneration",
    "ChatGenerationChunk",
    "ChatResult",
    "Generation",
    "GenerationChunk",
    "LLMResult",
    "RunInfo",
];

/// Test that all expected types are exported from the outputs module.
///
/// In Rust, we verify this by ensuring the types can be imported and used.
/// If any of these types were not exported, this code would fail to compile.
#[test]
fn test_all_imports() {

    use agent_chain_core::messages::AIMessage;
    use uuid::Uuid;

    let _: outputs::ChatGeneration =
        outputs::ChatGeneration::new(AIMessage::builder().content("test").build().into());

    let _: outputs::ChatGenerationChunk =
        outputs::ChatGenerationChunk::new(AIMessage::builder().content("test").build().into());

    let cg = outputs::ChatGeneration::new(AIMessage::builder().content("test").build().into());
    let _: outputs::ChatResult = outputs::ChatResult::new(vec![cg]);

    let _: outputs::Generation = outputs::Generation::new("test");

    let _: outputs::GenerationChunk = outputs::GenerationChunk::new("test");

    let generation = outputs::Generation::new("test");
    let _: outputs::LLMResult = outputs::LLMResult::new(vec![vec![generation.into()]]);

    let _: outputs::RunInfo = outputs::RunInfo::new(Uuid::new_v4());

    assert_eq!(EXPECTED_ALL.len(), 7);
    assert!(EXPECTED_ALL.contains(&"ChatGeneration"));
    assert!(EXPECTED_ALL.contains(&"ChatGenerationChunk"));
    assert!(EXPECTED_ALL.contains(&"ChatResult"));
    assert!(EXPECTED_ALL.contains(&"Generation"));
    assert!(EXPECTED_ALL.contains(&"GenerationChunk"));
    assert!(EXPECTED_ALL.contains(&"LLMResult"));
    assert!(EXPECTED_ALL.contains(&"RunInfo"));
}

/// Additionally test that types can be imported from the crate root.
/// This mirrors how Python users typically import from langchain_core.outputs.
#[test]
fn test_imports_from_crate_root() {
    use agent_chain_core::messages::AIMessage;
    use agent_chain_core::{
        ChatGeneration, ChatGenerationChunk, ChatResult, Generation, GenerationChunk, LLMResult,
        RunInfo,
    };
    use uuid::Uuid;

    let _ = Generation::new("test");
    let _ = GenerationChunk::new("test");
    let _ = ChatGeneration::new(AIMessage::builder().content("test").build().into());
    let _ = ChatGenerationChunk::new(AIMessage::builder().content("test").build().into());
    let generation = Generation::new("test");
    let _ = LLMResult::new(vec![vec![generation.into()]]);
    let cg = ChatGeneration::new(AIMessage::builder().content("test").build().into());
    let _ = ChatResult::new(vec![cg]);
    let _ = RunInfo::new(Uuid::new_v4());
}

/// Test that merge_chat_generation_chunks is also exported.
/// This is an additional function in the Python module.
#[test]
fn test_merge_function_export() {
    use agent_chain_core::messages::AIMessage;
    use agent_chain_core::outputs::merge_chat_generation_chunks;

    let chunks = vec![
        agent_chain_core::outputs::ChatGenerationChunk::new(
            AIMessage::builder().content("Hello ").build().into(),
        ),
        agent_chain_core::outputs::ChatGenerationChunk::new(
            AIMessage::builder().content("world").build().into(),
        ),
    ];

    let merged = merge_chat_generation_chunks(chunks);
    assert!(merged.is_some());
}

/// Test that GenerationType enum is also exported.
/// This is needed for working with LLMResult generations.
#[test]
fn test_generation_type_export() {
    use agent_chain_core::outputs::GenerationType;

    let generation = agent_chain_core::outputs::Generation::new("test");
    let generation_type: GenerationType = generation.into();

    match generation_type {
        GenerationType::Generation(g) => assert_eq!(g.text, "test"),
        _ => panic!("Expected Generation variant"),
    }
}
