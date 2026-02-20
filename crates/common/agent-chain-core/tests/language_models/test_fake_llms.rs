use std::time::Duration;

use agent_chain_core::language_models::{
    BaseLLM, BaseLanguageModel, FakeListLLM, FakeListLLMError, FakeStreamingListLLM, LLM,
    LanguageModelInput,
};
use agent_chain_core::messages::{BaseMessage, HumanMessage};
use agent_chain_core::outputs::GenerationType;
use futures::StreamExt;

#[test]
fn test_fake_list_llm_initialization() {
    let llm = FakeListLLM::new(vec!["response1".to_string(), "response2".to_string()]);
    assert_eq!(llm.current_index(), 0);
}

#[test]
fn test_fake_list_llm_initialization_with_sleep() {
    let llm = FakeListLLM::new(vec!["response".to_string()]).with_sleep(Duration::from_millis(100));
    assert_eq!(llm.current_index(), 0);
}

#[test]
fn test_fake_list_llm_type() {
    let llm = FakeListLLM::new(vec!["response".to_string()]);
    assert_eq!(llm.llm_type(), "fake-list");
}

#[tokio::test]
async fn test_fake_list_llm_invoke_single_response() {
    let llm = FakeListLLM::new(vec!["hello".to_string()]);
    let result = llm.invoke("any prompt".into(), None).await.unwrap();
    assert_eq!(result, "hello");
}

#[tokio::test]
async fn test_fake_list_llm_invoke_cycles_through_responses() {
    let llm = FakeListLLM::new(vec![
        "first".to_string(),
        "second".to_string(),
        "third".to_string(),
    ]);

    let result = llm.call("prompt1".to_string(), None, None).await.unwrap();
    assert_eq!(result, "first");
    assert_eq!(llm.current_index(), 1);

    let result = llm.call("prompt2".to_string(), None, None).await.unwrap();
    assert_eq!(result, "second");
    assert_eq!(llm.current_index(), 2);

    let result = llm.call("prompt3".to_string(), None, None).await.unwrap();
    assert_eq!(result, "third");
    assert_eq!(llm.current_index(), 0);

    let result = llm.call("prompt4".to_string(), None, None).await.unwrap();
    assert_eq!(result, "first");
}

#[tokio::test]
async fn test_fake_list_llm_single_response_stays_at_same() {
    let llm = FakeListLLM::new(vec!["only".to_string()]);

    let result = llm.call("prompt1".to_string(), None, None).await.unwrap();
    assert_eq!(result, "only");
    let result = llm.call("prompt2".to_string(), None, None).await.unwrap();
    assert_eq!(result, "only");
    let result = llm.call("prompt3".to_string(), None, None).await.unwrap();
    assert_eq!(result, "only");
    assert_eq!(llm.current_index(), 0);
}

#[tokio::test]
async fn test_fake_list_llm_ainvoke_single_response() {
    let llm = FakeListLLM::new(vec!["async hello".to_string()]);
    let result = llm.invoke("any prompt".into(), None).await.unwrap();
    assert_eq!(result, "async hello");
}

#[tokio::test]
async fn test_fake_list_llm_ainvoke_cycles_through_responses() {
    let llm = FakeListLLM::new(vec!["first".to_string(), "second".to_string()]);

    let result = llm.invoke("prompt1".into(), None).await.unwrap();
    assert_eq!(result, "first");
    let result = llm.invoke("prompt2".into(), None).await.unwrap();
    assert_eq!(result, "second");
    let result = llm.invoke("prompt3".into(), None).await.unwrap();
    assert_eq!(result, "first");
}

#[test]
fn test_fake_list_llm_identifying_params() {
    let llm = FakeListLLM::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let params = llm.identifying_params();
    assert_eq!(
        params.get("responses").unwrap(),
        &serde_json::json!(["a", "b", "c"])
    );
}

#[tokio::test]
async fn test_fake_list_llm_generate_returns_llm_result() {
    let llm = FakeListLLM::new(vec!["response".to_string()]);
    let result = llm
        .generate_prompts(vec!["prompt".to_string()], None, None)
        .await
        .unwrap();

    assert_eq!(result.generations.len(), 1);
    assert_eq!(result.generations[0].len(), 1);
    match &result.generations[0][0] {
        GenerationType::Generation(generation) => {
            assert_eq!(generation.text, "response");
        }
        _ => panic!("Expected Generation variant"),
    }
}

#[tokio::test]
async fn test_fake_list_llm_call_method() {
    let llm = FakeListLLM::new(vec!["direct call".to_string()]);
    let result = llm.call("prompt".to_string(), None, None).await.unwrap();
    assert_eq!(result, "direct call");
}

#[tokio::test]
async fn test_fake_list_llm_acall_method() {
    let llm = FakeListLLM::new(vec!["async direct call".to_string()]);
    let result = llm.call("prompt".to_string(), None, None).await.unwrap();
    assert_eq!(result, "async direct call");
}

#[test]
fn test_fake_list_llm_error_can_be_raised() {
    let error = FakeListLLMError;
    assert_eq!(format!("{}", error), "FakeListLLM error");
}

#[test]
fn test_fake_list_llm_error_is_exception() {
    let error = FakeListLLMError;
    let _: &dyn std::error::Error = &error;
}

#[test]
fn test_fake_streaming_initialization() {
    let llm = FakeStreamingListLLM::new(vec!["response".to_string()]);
    assert_eq!(llm.current_index(), 0);
}

#[test]
fn test_fake_streaming_initialization_with_error_on_chunk() {
    let llm = FakeStreamingListLLM::new(vec!["response".to_string()]).with_error_on_chunk(2);
    assert_eq!(llm.current_index(), 0);
}

#[tokio::test]
async fn test_fake_streaming_stream_yields_characters() {
    let llm = FakeStreamingListLLM::new(vec!["hello".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["h", "e", "l", "l", "o"]);
}

#[tokio::test]
async fn test_fake_streaming_stream_cycles_through_responses() {
    let llm = FakeStreamingListLLM::new(vec!["ab".to_string(), "cd".to_string()]);

    let mut stream = llm
        .stream_prompt("p1".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks1 = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks1.push(chunk.unwrap().text);
    }
    assert_eq!(chunks1, vec!["a", "b"]);

    let mut stream = llm
        .stream_prompt("p2".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks2 = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks2.push(chunk.unwrap().text);
    }
    assert_eq!(chunks2, vec!["c", "d"]);

    let mut stream = llm
        .stream_prompt("p3".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks3 = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks3.push(chunk.unwrap().text);
    }
    assert_eq!(chunks3, vec!["a", "b"]);
}

#[tokio::test]
async fn test_fake_streaming_stream_error_on_chunk() {
    let llm = FakeStreamingListLLM::new(vec!["hello".to_string()]).with_error_on_chunk(2);

    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let chunk0 = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk0.text, "h");
    let chunk1 = stream.next().await.unwrap().unwrap();
    assert_eq!(chunk1.text, "e");
    let chunk2 = stream.next().await.unwrap();
    assert!(chunk2.is_err());
}

#[tokio::test]
async fn test_fake_streaming_stream_error_on_first_chunk() {
    let llm = FakeStreamingListLLM::new(vec!["hello".to_string()]).with_error_on_chunk(0);

    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let chunk0 = stream.next().await.unwrap();
    assert!(chunk0.is_err());
}

#[tokio::test]
async fn test_fake_streaming_invoke_returns_full_response() {
    let llm = FakeStreamingListLLM::new(vec!["hello world".to_string()]);
    let result = llm.call("prompt".to_string(), None, None).await.unwrap();
    assert_eq!(result, "hello world");
}

#[test]
fn test_fake_streaming_llm_type() {
    let llm = FakeStreamingListLLM::new(vec!["test".to_string()]);
    assert_eq!(llm.llm_type(), "fake-list");
}

#[tokio::test]
async fn test_fake_streaming_stream_empty_response() {
    let llm = FakeStreamingListLLM::new(vec!["".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert!(chunks.is_empty());
}

#[tokio::test]
async fn test_fake_streaming_stream_unicode_characters() {
    let llm = FakeStreamingListLLM::new(vec!["你好".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["你", "好"]);
}

#[tokio::test]
async fn test_fake_streaming_stream_with_spaces() {
    let llm = FakeStreamingListLLM::new(vec!["a b".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["a", " ", "b"]);
}

#[tokio::test]
async fn test_fake_streaming_stream_with_newlines() {
    let llm = FakeStreamingListLLM::new(vec!["a\nb".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["a", "\n", "b"]);
}

#[tokio::test]
async fn test_generate_with_multiple_prompts() {
    let llm = FakeListLLM::new(vec![
        "alpha".to_string(),
        "beta".to_string(),
        "gamma".to_string(),
    ]);
    let result = llm
        .generate_prompts(
            vec!["p1".to_string(), "p2".to_string(), "p3".to_string()],
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.generations.len(), 3);
    match &result.generations[0][0] {
        GenerationType::Generation(g) => assert_eq!(g.text, "alpha"),
        _ => panic!("Expected Generation"),
    }
    match &result.generations[1][0] {
        GenerationType::Generation(g) => assert_eq!(g.text, "beta"),
        _ => panic!("Expected Generation"),
    }
    match &result.generations[2][0] {
        GenerationType::Generation(g) => assert_eq!(g.text, "gamma"),
        _ => panic!("Expected Generation"),
    }
}

#[tokio::test]
async fn test_generate_with_more_prompts_than_responses() {
    let llm = FakeListLLM::new(vec!["first".to_string(), "second".to_string()]);
    let result = llm
        .generate_prompts(
            vec!["p1".to_string(), "p2".to_string(), "p3".to_string()],
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.generations.len(), 3);
    match &result.generations[0][0] {
        GenerationType::Generation(g) => assert_eq!(g.text, "first"),
        _ => panic!("Expected Generation"),
    }
    match &result.generations[1][0] {
        GenerationType::Generation(g) => assert_eq!(g.text, "second"),
        _ => panic!("Expected Generation"),
    }
    match &result.generations[2][0] {
        GenerationType::Generation(g) => assert_eq!(g.text, "first"),
        _ => panic!("Expected Generation"),
    }
}

#[tokio::test]
async fn test_two_responses_exact_counter_state() {
    let llm = FakeListLLM::new(vec!["a".to_string(), "b".to_string()]);
    assert_eq!(llm.current_index(), 0);

    let result = llm.call("p1".to_string(), None, None).await.unwrap();
    assert_eq!(result, "a");
    assert_eq!(llm.current_index(), 1);

    let result = llm.call("p2".to_string(), None, None).await.unwrap();
    assert_eq!(result, "b");
    assert_eq!(llm.current_index(), 0);

    let result = llm.call("p3".to_string(), None, None).await.unwrap();
    assert_eq!(result, "a");
    assert_eq!(llm.current_index(), 1);

    let result = llm.call("p4".to_string(), None, None).await.unwrap();
    assert_eq!(result, "b");
    assert_eq!(llm.current_index(), 0);
}

#[tokio::test]
async fn test_call_resets_counter_at_end() {
    let llm = FakeListLLM::new(vec!["only_one".to_string()]);
    assert_eq!(llm.current_index(), 0);

    let result = llm.call("prompt".to_string(), None, None).await.unwrap();
    assert_eq!(result, "only_one");
    assert_eq!(llm.current_index(), 0);
}

#[tokio::test]
async fn test_generate_returns_proper_llm_result_structure() {
    let llm = FakeListLLM::new(vec!["hello".to_string(), "world".to_string()]);
    let result = llm
        .generate_prompts(
            vec!["prompt1".to_string(), "prompt2".to_string()],
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result.generations.len(), 2);
    assert_eq!(result.generations[0].len(), 1);
    assert_eq!(result.generations[1].len(), 1);

    match &result.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "hello"),
        _ => panic!("Expected Generation variant"),
    }
    match &result.generations[1][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "world"),
        _ => panic!("Expected Generation variant"),
    }
}

#[tokio::test]
async fn test_generate_single_prompt_structure() {
    let llm = FakeListLLM::new(vec!["single response".to_string()]);
    let result = llm
        .generate_prompts(vec!["one prompt".to_string()], None, None)
        .await
        .unwrap();

    assert_eq!(result.generations.len(), 1);
    assert_eq!(result.generations[0].len(), 1);
    match &result.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "single response"),
        _ => panic!("Expected Generation variant"),
    }
}

#[tokio::test]
async fn test_stream_single_character() {
    let llm = FakeStreamingListLLM::new(vec!["x".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["x"]);
    assert_eq!(chunks.len(), 1);
}

#[tokio::test]
async fn test_stream_error_on_exact_last_chunk() {
    let llm = FakeStreamingListLLM::new(vec!["abc".to_string()]).with_error_on_chunk(2);

    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => chunks.push(c.text),
            Err(_) => break,
        }
    }
    assert_eq!(chunks, vec!["a", "b"]);
}

#[tokio::test]
async fn test_stream_error_on_last_chunk_single_char() {
    let llm = FakeStreamingListLLM::new(vec!["z".to_string()]).with_error_on_chunk(0);

    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => chunks.push(c.text),
            Err(_) => break,
        }
    }
    assert!(chunks.is_empty());
}

#[tokio::test]
async fn test_stream_advances_counter() {
    let llm = FakeStreamingListLLM::new(vec!["ab".to_string(), "cd".to_string(), "ef".to_string()]);

    let mut stream = llm
        .stream_prompt("p1".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["a", "b"]);
    assert_eq!(llm.current_index(), 1);

    let mut stream = llm
        .stream_prompt("p2".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["c", "d"]);
    assert_eq!(llm.current_index(), 2);

    let mut stream = llm
        .stream_prompt("p3".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["e", "f"]);
    assert_eq!(llm.current_index(), 0);
}

#[test]
fn test_streaming_identifying_params_inherited() {
    let llm = FakeStreamingListLLM::new(vec!["hello".to_string(), "world".to_string()]);
    let params = llm.identifying_params();
    assert_eq!(
        params.get("responses").unwrap(),
        &serde_json::json!(["hello", "world"])
    );
}

#[tokio::test]
async fn test_invoke_with_human_message_list() {
    let llm = FakeListLLM::new(vec!["message response".to_string()]);
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("Hello").build(),
    )];
    let input = LanguageModelInput::from(messages);
    let result = llm.invoke(input, None).await.unwrap();
    assert_eq!(result, "message response");
}

#[tokio::test]
async fn test_stream_with_tabs() {
    let llm = FakeStreamingListLLM::new(vec!["a\tb".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["a", "\t", "b"]);
}

#[tokio::test]
async fn test_stream_with_carriage_return() {
    let llm = FakeStreamingListLLM::new(vec!["a\r\nb".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["a", "\r", "\n", "b"]);
}

#[tokio::test]
async fn test_stream_with_null_byte() {
    let llm = FakeStreamingListLLM::new(vec!["a\x00b".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["a", "\x00", "b"]);
}

#[tokio::test]
async fn test_stream_with_emoji() {
    let llm = FakeStreamingListLLM::new(vec!["hi\u{1f600}".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["h", "i", "\u{1f600}"]);
}

#[tokio::test]
async fn test_sleep_stored_but_does_not_affect_call() {
    let llm = FakeListLLM::new(vec!["response".to_string()]).with_sleep(Duration::from_secs(10));

    let start = std::time::Instant::now();
    let result = llm.call("prompt".to_string(), None, None).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(result, "response");
    assert!(elapsed < Duration::from_secs(1));
}

#[tokio::test]
async fn test_stream_with_sleep() {
    let llm =
        FakeStreamingListLLM::new(vec!["ab".to_string()]).with_sleep(Duration::from_millis(10));

    let start = std::time::Instant::now();
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    let elapsed = start.elapsed();

    assert_eq!(chunks, vec!["a", "b"]);
    assert!(elapsed >= Duration::from_millis(20));
}

#[tokio::test]
async fn test_batch_processing() {
    let llm = FakeListLLM::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);
    let results = llm
        .batch(
            vec![
                LanguageModelInput::from("p1"),
                LanguageModelInput::from("p2"),
                LanguageModelInput::from("p3"),
            ],
            None,
        )
        .await
        .unwrap();
    assert_eq!(results, vec!["r1", "r2", "r3"]);
}

#[tokio::test]
async fn test_abatch_processing() {
    let llm = FakeListLLM::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);
    let results = llm
        .batch(
            vec![
                LanguageModelInput::from("p1"),
                LanguageModelInput::from("p2"),
                LanguageModelInput::from("p3"),
            ],
            None,
        )
        .await
        .unwrap();
    assert_eq!(results, vec!["r1", "r2", "r3"]);
}

#[tokio::test]
async fn test_batch_cycles_correctly_and_updates_counter() {
    let llm = FakeListLLM::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);
    let results = llm
        .batch(
            vec![
                LanguageModelInput::from("p1"),
                LanguageModelInput::from("p2"),
                LanguageModelInput::from("p3"),
            ],
            None,
        )
        .await
        .unwrap();
    assert_eq!(results, vec!["r1", "r2", "r3"]);
    assert_eq!(llm.current_index(), 0);
}

#[tokio::test]
async fn test_batch_partial_cycle_updates_counter() {
    let llm = FakeListLLM::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);
    let results = llm
        .batch(vec![LanguageModelInput::from("p1")], None)
        .await
        .unwrap();
    assert_eq!(results, vec!["r1"]);
    assert_eq!(llm.current_index(), 1);
}

#[tokio::test]
async fn test_call_resets_counter_at_end_multiple_responses() {
    let llm = FakeListLLM::new(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    let _ = llm.call("p".to_string(), None, None).await;
    let _ = llm.call("p".to_string(), None, None).await;
    assert_eq!(llm.current_index(), 2);

    let result = llm.call("p".to_string(), None, None).await.unwrap();
    assert_eq!(result, "c");
    assert_eq!(llm.current_index(), 0);
}

#[test]
fn test_identifying_params_with_extra_attributes() {
    let llm = FakeStreamingListLLM::new(vec!["test".to_string()])
        .with_error_on_chunk(5)
        .with_sleep(Duration::from_millis(500));
    let params = llm.identifying_params();
    assert_eq!(
        params.get("responses").unwrap(),
        &serde_json::json!(["test"])
    );
    assert!(!params.contains_key("error_on_chunk_number"));
    assert!(!params.contains_key("sleep"));
}

#[tokio::test]
async fn test_multiple_sequential_streams_cycle() {
    let llm = FakeStreamingListLLM::new(vec!["AB".to_string(), "CD".to_string()]);

    async fn collect(llm: &FakeStreamingListLLM, prompt: &str) -> Vec<String> {
        use futures::StreamExt;
        let mut stream = llm
            .stream_prompt(prompt.to_string(), None, None)
            .await
            .unwrap();
        let mut chunks = Vec::new();
        while let Some(chunk) = stream.next().await {
            chunks.push(chunk.unwrap().text);
        }
        chunks
    }

    assert_eq!(collect(&llm, "p1").await, vec!["A", "B"]);
    assert_eq!(collect(&llm, "p2").await, vec!["C", "D"]);
    assert_eq!(collect(&llm, "p3").await, vec!["A", "B"]);
    assert_eq!(collect(&llm, "p4").await, vec!["C", "D"]);
}

#[tokio::test]
async fn test_invoke_with_multiple_messages() {
    use agent_chain_core::messages::SystemMessage;

    let llm = FakeListLLM::new(vec!["multi message response".to_string()]);
    let messages = vec![
        BaseMessage::System(
            SystemMessage::builder()
                .content("You are a helper.")
                .build(),
        ),
        BaseMessage::Human(HumanMessage::builder().content("What is 2+2?").build()),
    ];
    let result = llm
        .invoke(LanguageModelInput::from(messages), None)
        .await
        .unwrap();
    assert_eq!(result, "multi message response");
}

#[tokio::test]
async fn test_ainvoke_with_human_message_list() {
    let llm = FakeListLLM::new(vec!["async message response".to_string()]);
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("Hello").build(),
    )];
    let result = llm
        .invoke(LanguageModelInput::from(messages), None)
        .await
        .unwrap();
    assert_eq!(result, "async message response");
}

#[tokio::test]
async fn test_streaming_ainvoke_returns_full_response() {
    let llm = FakeStreamingListLLM::new(vec!["hello world".to_string()]);
    let result = llm.call("prompt".to_string(), None, None).await.unwrap();
    assert_eq!(result, "hello world");
}

#[tokio::test]
async fn test_stream_with_mixed_special_characters() {
    let llm = FakeStreamingListLLM::new(vec!["\u{1f44d}\n\t".to_string()]);
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["\u{1f44d}", "\n", "\t"]);
}

#[tokio::test]
async fn test_astream_sleep_delays_proportional_to_chunks() {
    let llm =
        FakeStreamingListLLM::new(vec!["abcde".to_string()]).with_sleep(Duration::from_millis(20));

    let start = std::time::Instant::now();
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    let elapsed = start.elapsed();

    assert_eq!(chunks, vec!["a", "b", "c", "d", "e"]);
    assert!(elapsed >= Duration::from_millis(100));
}

#[tokio::test]
async fn test_astream_no_sleep_is_fast() {
    let llm = FakeStreamingListLLM::new(vec!["abcde".to_string()]);

    let start = std::time::Instant::now();
    let mut stream = llm
        .stream_prompt("prompt".to_string(), None, None)
        .await
        .unwrap();
    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    let elapsed = start.elapsed();

    assert_eq!(chunks, vec!["a", "b", "c", "d", "e"]);
    assert!(elapsed < Duration::from_secs(1));
}
