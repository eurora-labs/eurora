use agent_chain_core::outputs;

const EXPECTED_ALL: &[&str] = &[
    "ChatGeneration",
    "ChatGenerationChunk",
    "ChatResult",
    "Generation",
    "GenerationChunk",
    "LLMResult",
    "RunInfo",
];

#[test]
fn test_all_imports() {
    use agent_chain_core::messages::AIMessage;
    use uuid::Uuid;

    let _: outputs::ChatGeneration = outputs::ChatGeneration::builder()
        .message(AIMessage::builder().content("test").build().into())
        .build();

    let _: outputs::ChatGenerationChunk = outputs::ChatGenerationChunk::builder()
        .message(AIMessage::builder().content("test").build().into())
        .build();

    let cg = outputs::ChatGeneration::builder()
        .message(AIMessage::builder().content("test").build().into())
        .build();
    let _: outputs::ChatResult = outputs::ChatResult::builder().generations(vec![cg]).build();

    let _: outputs::Generation = outputs::Generation::builder().text("test").build();

    let _: outputs::GenerationChunk = outputs::GenerationChunk::builder().text("test").build();

    let generation = outputs::Generation::builder().text("test").build();
    let _: outputs::LLMResult = outputs::LLMResult::builder()
        .generations(vec![vec![generation.into()]])
        .build();

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

#[test]
fn test_imports_from_crate_root() {
    use agent_chain_core::messages::AIMessage;
    use agent_chain_core::{
        ChatGeneration, ChatGenerationChunk, ChatResult, Generation, GenerationChunk, LLMResult,
        RunInfo,
    };
    use uuid::Uuid;

    let _ = Generation::builder().text("test").build();
    let _ = GenerationChunk::builder().text("test").build();
    let _ = ChatGeneration::builder()
        .message(AIMessage::builder().content("test").build().into())
        .build();
    let _ = ChatGenerationChunk::builder()
        .message(AIMessage::builder().content("test").build().into())
        .build();
    let generation = Generation::builder().text("test").build();
    let _ = LLMResult::builder()
        .generations(vec![vec![generation.into()]])
        .build();
    let cg = ChatGeneration::builder()
        .message(AIMessage::builder().content("test").build().into())
        .build();
    let _ = ChatResult::builder().generations(vec![cg]).build();
    let _ = RunInfo::new(Uuid::new_v4());
}

#[test]
fn test_merge_function_export() {
    use agent_chain_core::messages::AIMessage;
    use agent_chain_core::outputs::merge_chat_generation_chunks;

    let chunks = vec![
        agent_chain_core::outputs::ChatGenerationChunk::builder()
            .message(AIMessage::builder().content("Hello ").build().into())
            .build(),
        agent_chain_core::outputs::ChatGenerationChunk::builder()
            .message(AIMessage::builder().content("world").build().into())
            .build(),
    ];

    let merged = merge_chat_generation_chunks(chunks);
    assert!(merged.is_some());
}

#[test]
fn test_generation_type_export() {
    use agent_chain_core::outputs::GenerationType;

    let generation = agent_chain_core::outputs::Generation::builder()
        .text("test")
        .build();
    let generation_type: GenerationType = generation.into();

    match generation_type {
        GenerationType::Generation(g) => assert_eq!(g.text, "test"),
        _ => panic!("Expected Generation variant"),
    }
}
