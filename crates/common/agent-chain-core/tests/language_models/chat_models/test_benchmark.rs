//! Benchmark tests for chat models.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/language_models/chat_models/test_benchmark.py`

use std::time::Instant;

use agent_chain_core::GenericFakeChatModel;
use agent_chain_core::language_models::BaseChatModel;
use agent_chain_core::messages::AIMessage;

/// Ported from `test_benchmark_model`.
///
/// Ensures the model can handle 1000 invocations in under 1 second.
#[tokio::test]
async fn test_benchmark_model() {
    let messages: Vec<AIMessage> = (0..1000)
        .map(|i| {
            let content = match i % 3 {
                0 => "hello",
                1 => "world",
                _ => "!",
            };
            AIMessage::builder().content(content).build()
        })
        .collect();

    let model = GenericFakeChatModel::from_vec(messages);

    let start = Instant::now();
    for _ in 0..1_000 {
        let _ = model._generate(vec![], None, None).await.unwrap();
    }
    let duration = start.elapsed();

    assert!(
        duration.as_secs() < 1,
        "1000 invocations took {:?}, expected < 1s",
        duration
    );
}
