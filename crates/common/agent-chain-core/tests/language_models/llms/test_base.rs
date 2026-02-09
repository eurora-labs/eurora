//! Tests for base LLM.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/language_models/llms/test_base.py`

use std::collections::HashMap;

use agent_chain_core::caches::{BaseCache, InMemoryCache};
use agent_chain_core::language_models::{
    BaseLLM, BaseLanguageModel, FakeListLLM, FakeStreamingListLLM, LLM, LanguageModelInput,
    get_prompts_from_cache, update_cache,
};
use agent_chain_core::outputs::{Generation, GenerationType, LLMResult};
use futures::StreamExt;
use serde_json::json;

// ====================================================================
// test_batch / test_abatch
// ====================================================================

/// Ported from `test_batch`.
#[tokio::test]
async fn test_batch() {
    let llm = FakeListLLM::new(vec!["foo".to_string(); 3]);
    let output = llm
        .batch(vec![
            LanguageModelInput::from("foo"),
            LanguageModelInput::from("bar"),
            LanguageModelInput::from("foo"),
        ])
        .await
        .unwrap();
    assert_eq!(output, vec!["foo", "foo", "foo"]);
}

/// Ported from `test_abatch`.
#[tokio::test]
async fn test_abatch() {
    let llm = FakeListLLM::new(vec!["foo".to_string(); 3]);
    let output = llm
        .batch(vec![
            LanguageModelInput::from("foo"),
            LanguageModelInput::from("bar"),
            LanguageModelInput::from("foo"),
        ])
        .await
        .unwrap();
    assert_eq!(output, vec!["foo", "foo", "foo"]);
}

/// Ported from `test_batch_empty_inputs_returns_empty_list`.
#[tokio::test]
async fn test_batch_empty_inputs_returns_empty_list() {
    let llm = FakeListLLM::new(vec!["a".to_string()]);
    let result = llm.batch(vec![]).await.unwrap();
    assert!(result.is_empty());
}

/// Ported from `test_abatch_empty_inputs_returns_empty_list`.
#[tokio::test]
async fn test_abatch_empty_inputs_returns_empty_list() {
    let llm = FakeListLLM::new(vec!["a".to_string()]);
    let result = llm.batch(vec![]).await.unwrap();
    assert!(result.is_empty());
}

// ====================================================================
// test_convert_input
// ====================================================================

/// Ported from `test_convert_string_input`.
#[test]
fn test_convert_string_input() {
    let llm = FakeListLLM::new(vec!["r".to_string()]);
    let result = llm
        .convert_input(LanguageModelInput::from("hello world"))
        .unwrap();
    assert_eq!(result, "hello world");
}

/// Ported from `test_convert_prompt_value_input`.
#[test]
fn test_convert_prompt_value_input() {
    use agent_chain_core::prompt_values::StringPromptValue;

    let llm = FakeListLLM::new(vec!["r".to_string()]);
    let pv = StringPromptValue::new("already a prompt value");
    let result = llm.convert_input(LanguageModelInput::from(pv)).unwrap();
    assert_eq!(result, "already a prompt value");
}

/// Ported from `test_convert_message_sequence_input`.
#[test]
fn test_convert_message_sequence_input() {
    use agent_chain_core::messages::{BaseMessage, HumanMessage};

    let llm = FakeListLLM::new(vec!["r".to_string()]);
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("hi").build(),
    )];
    let result = llm
        .convert_input(LanguageModelInput::from(messages))
        .unwrap();
    // Messages get formatted as "human: hi"
    assert!(result.contains("hi"));
}

// ====================================================================
// test_generate (via generate_prompts)
// ====================================================================

/// Ported from `test_generate_single_prompt`.
#[tokio::test]
async fn test_generate_single_prompt() {
    let llm = FakeListLLM::new(vec!["result".to_string()]);
    let result = llm
        .generate_prompts(vec!["prompt1".to_string()], None, None)
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
    match &result.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "result"),
        _ => panic!("Expected Generation variant"),
    }
}

/// Ported from `test_generate_multiple_prompts`.
#[tokio::test]
async fn test_generate_multiple_prompts() {
    let llm = FakeListLLM::new(vec![
        "out1".to_string(),
        "out2".to_string(),
        "out3".to_string(),
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
    for generation_list in &result.generations {
        assert_eq!(generation_list.len(), 1);
    }
}

/// Ported from `test_generate_empty_prompts`.
#[tokio::test]
async fn test_generate_empty_prompts() {
    let llm = FakeListLLM::new(vec!["out".to_string()]);
    let result = llm.generate_prompts(vec![], None, None).await.unwrap();
    assert_eq!(result.generations.len(), 0);
}

// ====================================================================
// test_agenerate
// ====================================================================

/// Ported from `test_agenerate_single_prompt`.
#[tokio::test]
async fn test_agenerate_single_prompt() {
    let llm = FakeListLLM::new(vec!["async_result".to_string()]);
    let result = llm
        .generate_prompts(vec!["prompt1".to_string()], None, None)
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
    match &result.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "async_result"),
        _ => panic!("Expected Generation variant"),
    }
}

/// Ported from `test_agenerate_multiple_prompts`.
#[tokio::test]
async fn test_agenerate_multiple_prompts() {
    let llm = FakeListLLM::new(vec!["out1".to_string(), "out2".to_string()]);
    let result = llm
        .generate_prompts(vec!["p1".to_string(), "p2".to_string()], None, None)
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 2);
    for generation_list in &result.generations {
        match &generation_list[0] {
            GenerationType::Generation(_) => {}
            _ => panic!("Expected Generation variant"),
        }
    }
}

// ====================================================================
// test_astream_fallback_to_ainvoke
// ====================================================================

/// Ported from `test_astream_fallback_to_ainvoke`.
///
/// A model with only generate_prompts (no stream_prompt override) should
/// still work via the default stream_prompt implementation, which falls
/// back to generate_prompts and returns the result as a single chunk.
#[tokio::test]
async fn test_astream_fallback_to_ainvoke() {
    let llm = FakeListLLM::new(vec!["hello".to_string()]);
    let mut stream = llm
        .stream_prompt("anything".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["hello"]);
}

/// Ported from `test_astream_implementation_uses_astream`.
///
/// FakeStreamingListLLM overrides stream_prompt, so it should yield
/// individual characters.
#[tokio::test]
async fn test_astream_implementation_uses_stream() {
    let llm = FakeStreamingListLLM::new(vec!["ab".to_string()]);
    let mut stream = llm
        .stream_prompt("anything".to_string(), None, None)
        .await
        .unwrap();

    let mut chunks = Vec::new();
    while let Some(chunk) = stream.next().await {
        chunks.push(chunk.unwrap().text);
    }
    assert_eq!(chunks, vec!["a", "b"]);
}

// ====================================================================
// test_get_ls_params
// ====================================================================

/// Ported from `test_get_ls_params`.
#[test]
fn test_get_ls_params() {
    let llm = FakeListLLM::new(vec!["foo".to_string()]);

    let params = llm.get_llm_ls_params(None);
    assert_eq!(params.ls_model_type, Some("llm".to_string()));
    assert!(params.ls_provider.is_some());
    assert!(params.ls_model_name.is_some());

    // With stop words
    let params = llm.get_llm_ls_params(Some(&["stop".to_string()]));
    assert_eq!(params.ls_stop, Some(vec!["stop".to_string()]));
}

// ====================================================================
// test_get_prompts_from_cache
// ====================================================================

/// Ported from `test_no_cache_returns_all_missing`.
#[test]
fn test_get_prompts_no_cache_returns_all_missing() {
    let params = HashMap::from([("model".to_string(), json!("test"))]);
    let (existing, _llm_string, missing_idxs, missing) =
        get_prompts_from_cache(&params, &["p1".to_string(), "p2".to_string()], None);

    assert!(existing.is_empty());
    assert_eq!(missing_idxs, vec![0, 1]);
    assert_eq!(missing, vec!["p1", "p2"]);
}

/// Ported from `test_with_cache_all_miss`.
#[test]
fn test_get_prompts_with_cache_all_miss() {
    let cache = InMemoryCache::unbounded();
    let params = HashMap::from([("model".to_string(), json!("test"))]);
    let (existing, _llm_string, missing_idxs, missing) =
        get_prompts_from_cache(&params, &["p1".to_string(), "p2".to_string()], Some(&cache));

    assert!(existing.is_empty());
    assert_eq!(missing_idxs, vec![0, 1]);
    assert_eq!(missing, vec!["p1", "p2"]);
}

/// Ported from `test_with_cache_partial_hit`.
#[test]
fn test_get_prompts_with_cache_partial_hit() {
    let cache = InMemoryCache::unbounded();
    let params = HashMap::from([("model".to_string(), json!("test"))]);
    let llm_string = serde_json::to_string(&params).unwrap();
    let cached_generation = vec![Generation::new("cached".to_string())];
    cache.update("p1", &llm_string, cached_generation.clone());

    let (existing, _, missing_idxs, missing) =
        get_prompts_from_cache(&params, &["p1".to_string(), "p2".to_string()], Some(&cache));

    assert!(existing.contains_key(&0));
    assert_eq!(existing[&0][0].text, "cached");
    assert_eq!(missing_idxs, vec![1]);
    assert_eq!(missing, vec!["p2"]);
}

/// Ported from `test_with_cache_all_hit`.
#[test]
fn test_get_prompts_with_cache_all_hit() {
    let cache = InMemoryCache::unbounded();
    let params = HashMap::from([("model".to_string(), json!("test"))]);
    let llm_string = serde_json::to_string(&params).unwrap();
    cache.update("p1", &llm_string, vec![Generation::new("c1".to_string())]);
    cache.update("p2", &llm_string, vec![Generation::new("c2".to_string())]);

    let (existing, _, missing_idxs, missing) =
        get_prompts_from_cache(&params, &["p1".to_string(), "p2".to_string()], Some(&cache));

    assert_eq!(existing.len(), 2);
    assert!(missing_idxs.is_empty());
    assert!(missing.is_empty());
}

// ====================================================================
// test_update_cache
// ====================================================================

/// Ported from `test_update_cache_stores_results`.
#[test]
fn test_update_cache_stores_results() {
    let cache = InMemoryCache::unbounded();
    let llm_string = "test_llm";
    let new_results = LLMResult::new(vec![
        vec![GenerationType::Generation(Generation::new(
            "r1".to_string(),
        ))],
        vec![GenerationType::Generation(Generation::new(
            "r2".to_string(),
        ))],
    ]);
    let mut existing: HashMap<usize, Vec<Generation>> = HashMap::new();

    let _ = update_cache(
        Some(&cache),
        &mut existing,
        llm_string,
        &[0, 1],
        &new_results,
        &["p1".to_string(), "p2".to_string()],
    );

    assert!(cache.lookup("p1", llm_string).is_some());
    assert!(cache.lookup("p2", llm_string).is_some());
    assert!(existing.contains_key(&0));
    assert!(existing.contains_key(&1));
    assert_eq!(existing[&0][0].text, "r1");
}

/// Ported from `test_update_cache_with_none_does_not_store`.
#[test]
fn test_update_cache_with_none_does_not_store() {
    let new_results = LLMResult::new(vec![vec![GenerationType::Generation(Generation::new(
        "r1".to_string(),
    ))]]);
    let mut existing: HashMap<usize, Vec<Generation>> = HashMap::new();

    let _ = update_cache(
        None,
        &mut existing,
        "llm",
        &[0],
        &new_results,
        &["p1".to_string()],
    );

    // existing_prompts should not be populated when cache is None
    assert!(existing.is_empty());
}

// ====================================================================
// test_generate_prompt (via BaseLanguageModel trait)
// ====================================================================

/// Ported from `test_generate_prompt_converts_prompt_values`.
#[tokio::test]
async fn test_generate_prompt_converts_prompt_values() {
    let llm = FakeListLLM::new(vec!["resp1".to_string(), "resp2".to_string()]);
    let result = llm
        .generate_prompt(
            vec![
                LanguageModelInput::from("hello"),
                LanguageModelInput::from("world"),
            ],
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 2);
    match &result.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "resp1"),
        _ => panic!("Expected Generation variant"),
    }
    match &result.generations[1][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "resp2"),
        _ => panic!("Expected Generation variant"),
    }
}

/// Ported from `test_agenerate_prompt_converts_prompt_values`.
#[tokio::test]
async fn test_agenerate_prompt_converts_prompt_values() {
    let llm = FakeListLLM::new(vec!["async_resp".to_string()]);
    let result = llm
        .generate_prompt(vec![LanguageModelInput::from("hello")], None, None)
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
    match &result.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "async_resp"),
        _ => panic!("Expected Generation variant"),
    }
}

/// Ported from `test_generate_prompt_with_chat_prompt_value`.
#[tokio::test]
async fn test_generate_prompt_with_message_input() {
    use agent_chain_core::messages::{BaseMessage, HumanMessage};

    let llm = FakeListLLM::new(vec!["chat_resp".to_string()]);
    let messages = vec![BaseMessage::Human(
        HumanMessage::builder().content("hi there").build(),
    )];
    let result = llm
        .generate_prompt(vec![LanguageModelInput::from(messages)], None, None)
        .await
        .unwrap();
    assert_eq!(result.generations.len(), 1);
    match &result.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "chat_resp"),
        _ => panic!("Expected Generation variant"),
    }
}

// ====================================================================
// test_str_representation / test_dict_contains_type
// ====================================================================

/// Ported from `test_str_representation`.
#[test]
fn test_str_representation() {
    let llm = FakeListLLM::new(vec!["foo".to_string()]);
    let result = format!("{:?}", llm);
    assert!(result.contains("FakeListLLM"));
}

/// Ported from `test_dict_contains_type_and_identifying_params`.
#[test]
fn test_dict_contains_type_and_identifying_params() {
    let llm = FakeListLLM::new(vec!["a".to_string(), "b".to_string()]);
    let params = llm.identifying_params();
    assert!(params.contains_key("_type"));
    assert_eq!(params["_type"], json!("fake-list"));
    assert!(params.contains_key("responses"));
    assert_eq!(params["responses"], json!(["a", "b"]));
}

// ====================================================================
// test_invoke / test_call
// ====================================================================

/// Ported from `test_invoke` (via FakeListLLM).
#[tokio::test]
async fn test_invoke() {
    let llm = FakeListLLM::new(vec!["hello".to_string()]);
    let result = llm
        .invoke(LanguageModelInput::from("prompt"))
        .await
        .unwrap();
    assert_eq!(result, "hello");
}

/// Ported from `test_call_method` (via LLM trait).
#[tokio::test]
async fn test_call_method() {
    let llm = FakeListLLM::new(vec!["direct".to_string()]);
    let result = llm.call("prompt".to_string(), None, None).await.unwrap();
    assert_eq!(result, "direct");
}
