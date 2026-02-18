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


/// Ported from `test_batch`.
#[tokio::test]
async fn test_batch() {
    let llm = FakeListLLM::new(vec!["foo".to_string(); 3]);
    let output = llm
        .batch(
            vec![
                LanguageModelInput::from("foo"),
                LanguageModelInput::from("bar"),
                LanguageModelInput::from("foo"),
            ],
            None,
        )
        .await
        .unwrap();
    assert_eq!(output, vec!["foo", "foo", "foo"]);
}

/// Ported from `test_abatch`.
#[tokio::test]
async fn test_abatch() {
    let llm = FakeListLLM::new(vec!["foo".to_string(); 3]);
    let output = llm
        .batch(
            vec![
                LanguageModelInput::from("foo"),
                LanguageModelInput::from("bar"),
                LanguageModelInput::from("foo"),
            ],
            None,
        )
        .await
        .unwrap();
    assert_eq!(output, vec!["foo", "foo", "foo"]);
}

/// Ported from `test_batch_empty_inputs_returns_empty_list`.
#[tokio::test]
async fn test_batch_empty_inputs_returns_empty_list() {
    let llm = FakeListLLM::new(vec!["a".to_string()]);
    let result = llm.batch(vec![], None).await.unwrap();
    assert!(result.is_empty());
}

/// Ported from `test_abatch_empty_inputs_returns_empty_list`.
#[tokio::test]
async fn test_abatch_empty_inputs_returns_empty_list() {
    let llm = FakeListLLM::new(vec!["a".to_string()]);
    let result = llm.batch(vec![], None).await.unwrap();
    assert!(result.is_empty());
}


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
    assert!(result.contains("hi"));
}


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


/// Ported from `test_get_ls_params`.
#[test]
fn test_get_ls_params() {
    let llm = FakeListLLM::new(vec!["foo".to_string()]);

    let params = llm.get_llm_ls_params(None);
    assert_eq!(params.ls_model_type, Some("llm".to_string()));
    assert!(params.ls_provider.is_some());
    assert!(params.ls_model_name.is_some());

    let params = llm.get_llm_ls_params(Some(&["stop".to_string()]));
    assert_eq!(params.ls_stop, Some(vec!["stop".to_string()]));
}


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

    assert!(existing.is_empty());
}


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


/// Ported from `test_invoke` (via FakeListLLM).
#[tokio::test]
async fn test_invoke() {
    let llm = FakeListLLM::new(vec!["hello".to_string()]);
    let result = llm
        .invoke(LanguageModelInput::from("prompt"), None)
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



/// Ported from `test_none_run_id`.
#[test]
fn test_get_run_ids_list_none() {
    use agent_chain_core::language_models::RunIdInput;
    use agent_chain_core::language_models::get_run_ids_list;

    let result = get_run_ids_list(RunIdInput::None, 3).unwrap();
    assert_eq!(result, vec![None, None, None]);
}

/// Ported from `test_single_uuid`.
#[test]
fn test_get_run_ids_list_single_uuid() {
    use agent_chain_core::language_models::{RunIdInput, get_run_ids_list};

    let uid = uuid::Uuid::new_v4();
    let result = get_run_ids_list(RunIdInput::Single(uid), 3).unwrap();
    assert_eq!(result[0], Some(uid));
    assert_eq!(result[1], None);
    assert_eq!(result[2], None);
}

/// Ported from `test_list_of_uuids`.
#[test]
fn test_get_run_ids_list_list_of_uuids() {
    use agent_chain_core::language_models::{RunIdInput, get_run_ids_list};

    let uid1 = uuid::Uuid::new_v4();
    let uid2 = uuid::Uuid::new_v4();
    let result = get_run_ids_list(RunIdInput::List(vec![uid1, uid2]), 2).unwrap();
    assert_eq!(result, vec![Some(uid1), Some(uid2)]);
}

/// Ported from `test_mismatched_list_length_raises`.
#[test]
fn test_get_run_ids_list_mismatched_length() {
    use agent_chain_core::language_models::{RunIdInput, get_run_ids_list};

    let uid = uuid::Uuid::new_v4();
    let result = get_run_ids_list(RunIdInput::List(vec![uid]), 3);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("does not match batch length"));
}

/// Ported from `test_single_prompt_with_uuid`.
#[test]
fn test_get_run_ids_list_single_prompt_with_uuid() {
    use agent_chain_core::language_models::{RunIdInput, get_run_ids_list};

    let uid = uuid::Uuid::new_v4();
    let result = get_run_ids_list(RunIdInput::Single(uid), 1).unwrap();
    assert_eq!(result, vec![Some(uid)]);
}


/// Ported from `test_cache_is_base_cache_instance`.
#[test]
fn test_resolve_cache_instance() {
    use agent_chain_core::language_models::{CacheValue, resolve_cache};
    use std::sync::Arc;

    let cache = Arc::new(InMemoryCache::unbounded());
    let result = resolve_cache(Some(CacheValue::Instance(cache.clone()))).unwrap();
    assert!(result.is_some());
}

/// Ported from `test_cache_is_none_returns_global_cache`.
#[test]
fn test_resolve_cache_none_returns_global() {
    use agent_chain_core::language_models::resolve_cache;
    use agent_chain_core::set_llm_cache;
    use std::sync::Arc;

    let cache = Arc::new(InMemoryCache::unbounded());
    set_llm_cache(Some(cache));

    let result = resolve_cache(None).unwrap();
    assert!(result.is_some());

    set_llm_cache(None);
}

/// Ported from `test_cache_is_none_no_global_returns_none`.
#[test]
fn test_resolve_cache_none_no_global_returns_none() {
    use agent_chain_core::language_models::resolve_cache;
    use agent_chain_core::set_llm_cache;

    set_llm_cache(None);
    let result = resolve_cache(None).unwrap();
    assert!(result.is_none());
}

/// Ported from `test_cache_is_true_with_global_cache`.
#[test]
fn test_resolve_cache_true_with_global() {
    use agent_chain_core::language_models::{CacheValue, resolve_cache};
    use agent_chain_core::set_llm_cache;
    use std::sync::Arc;

    let cache = Arc::new(InMemoryCache::unbounded());
    set_llm_cache(Some(cache));

    let result = resolve_cache(Some(CacheValue::Flag(true))).unwrap();
    assert!(result.is_some());

    set_llm_cache(None);
}

/// Ported from `test_cache_is_true_without_global_cache_raises`.
#[test]
fn test_resolve_cache_true_without_global_raises() {
    use agent_chain_core::language_models::{CacheValue, resolve_cache};
    use agent_chain_core::set_llm_cache;

    set_llm_cache(None);

    let result = resolve_cache(Some(CacheValue::Flag(true)));
    assert!(result.is_err());
    if let Err(err) = result {
        let err_msg = format!("{}", err);
        assert!(err_msg.contains("No global cache was configured"));
    }
}

/// Ported from `test_cache_is_false`.
#[test]
fn test_resolve_cache_false() {
    use agent_chain_core::language_models::{CacheValue, resolve_cache};

    let result = resolve_cache(Some(CacheValue::Flag(false))).unwrap();
    assert!(result.is_none());
}


/// Ported from `test_batch_return_exceptions_true`.
#[tokio::test]
async fn test_batch_with_exceptions() {
    let llm = FakeListLLM::new(vec!["r1".to_string(), "r2".to_string(), "r3".to_string()]);
    let results = llm
        .batch_with_exceptions(
            vec![
                LanguageModelInput::from("p1"),
                LanguageModelInput::from("p2"),
                LanguageModelInput::from("p3"),
            ],
            None,
        )
        .await;

    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.is_ok()));
    assert_eq!(results[0].as_ref().unwrap(), "r1");
    assert_eq!(results[1].as_ref().unwrap(), "r2");
    assert_eq!(results[2].as_ref().unwrap(), "r3");
}


/// Ported from `test_save_json`.
#[test]
fn test_save_json() {
    use agent_chain_core::language_models::save_llm;

    let llm = FakeListLLM::new(vec!["a".to_string(), "b".to_string()]);
    let params = llm.identifying_params();

    let dir = std::env::temp_dir().join(format!("test_save_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let file_path = dir.join("llm.json");

    save_llm(&params, &file_path).unwrap();

    assert!(file_path.exists());
    let content = std::fs::read_to_string(&file_path).unwrap();
    let data: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(data["_type"], "fake-list");
    assert_eq!(data["responses"], json!(["a", "b"]));

    std::fs::remove_dir_all(&dir).unwrap();
}

/// Ported from `test_save_invalid_extension_raises`.
#[test]
fn test_save_invalid_extension_raises() {
    use agent_chain_core::language_models::save_llm;

    let llm = FakeListLLM::new(vec!["a".to_string()]);
    let params = llm.identifying_params();

    let dir = std::env::temp_dir().join(format!("test_save_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let file_path = dir.join("llm.txt");

    let result = save_llm(&params, &file_path);
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("must be json"));

    std::fs::remove_dir_all(&dir).unwrap();
}


/// Ported from `test_retries_on_specified_error`.
#[test]
fn test_retry_on_specified_error() {
    use agent_chain_core::language_models::create_base_retry;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let call_count = AtomicUsize::new(0);

    let result = create_base_retry(
        |_err| true, // retry on all errors
        3,
        || {
            let count = call_count.fetch_add(1, Ordering::SeqCst) + 1;
            if count < 3 {
                Err(agent_chain_core::error::Error::Other(
                    "transient".to_string(),
                ))
            } else {
                Ok("success".to_string())
            }
        },
    );

    assert_eq!(result.unwrap(), "success");
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

/// Ported from `test_does_not_retry_on_unspecified_error`.
#[test]
fn test_retry_does_not_retry_on_unspecified_error() {
    use agent_chain_core::language_models::create_base_retry;

    let result: Result<String, _> = create_base_retry(
        |_err| false, // never retry
        3,
        || {
            Err(agent_chain_core::error::Error::Other(
                "unrecoverable".to_string(),
            ))
        },
    );

    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("unrecoverable"));
}

/// Ported from `test_max_retries_one_means_no_retry`.
#[test]
fn test_retry_max_retries_one_means_no_retry() {
    use agent_chain_core::language_models::create_base_retry;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let call_count = AtomicUsize::new(0);

    let result: Result<String, _> = create_base_retry(
        |_err| true,
        1,
        || {
            call_count.fetch_add(1, Ordering::SeqCst);
            Err(agent_chain_core::error::Error::Other(
                "always fails".to_string(),
            ))
        },
    );

    assert!(result.is_err());
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}
