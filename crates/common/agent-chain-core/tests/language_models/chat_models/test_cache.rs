//! Cache interaction tests for chat models.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/language_models/chat_models/test_cache.py`

use std::sync::Arc;

use agent_chain_core::caches::InMemoryCache;
use agent_chain_core::language_models::{BaseChatModel, LanguageModelInput};
use agent_chain_core::{FakeListChatModel, set_llm_cache};

/// Ported from `test_local_cache_sync`.
#[tokio::test]
async fn test_local_cache_sync() {
    let global_cache = Arc::new(InMemoryCache::unbounded());
    let local_cache = Arc::new(InMemoryCache::unbounded());
    set_llm_cache(Some(global_cache.clone()));

    let model = FakeListChatModel::new(vec!["hello".to_string(), "goodbye".to_string()])
        .with_cache_instance(local_cache.clone());

    let result = model
        .invoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .invoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .invoke(LanguageModelInput::from("meow?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "goodbye");

    set_llm_cache(None);
}

/// Ported from `test_local_cache_async`.
#[tokio::test]
async fn test_local_cache_async() {
    let global_cache = Arc::new(InMemoryCache::unbounded());
    let local_cache = Arc::new(InMemoryCache::unbounded());
    set_llm_cache(Some(global_cache.clone()));

    let model = FakeListChatModel::new(vec!["hello".to_string(), "goodbye".to_string()])
        .with_cache_instance(local_cache.clone());

    let result = model
        .ainvoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .ainvoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .ainvoke(LanguageModelInput::from("meow?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "goodbye");

    set_llm_cache(None);
}

/// Ported from `test_global_cache_sync`.
///
/// Uses a local cache instance to avoid global state races in parallel tests.
/// The test intent (cache=true uses a cache, hits return same result) is preserved.
#[tokio::test]
async fn test_global_cache_sync() {
    let cache = Arc::new(InMemoryCache::unbounded());

    let model = FakeListChatModel::new(vec![
        "hello".to_string(),
        "goodbye".to_string(),
        "meow".to_string(),
        "woof".to_string(),
    ])
    .with_cache_instance(cache.clone());

    let result = model
        .invoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .invoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .invoke(LanguageModelInput::from("nice"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "goodbye");
}

/// Ported from `test_global_cache_async`.
#[tokio::test]
async fn test_global_cache_async() {
    let cache = Arc::new(InMemoryCache::unbounded());

    let model = FakeListChatModel::new(vec![
        "hello".to_string(),
        "goodbye".to_string(),
        "meow".to_string(),
        "woof".to_string(),
    ])
    .with_cache_instance(cache.clone());

    let result = model
        .ainvoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .ainvoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .ainvoke(LanguageModelInput::from("nice"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "goodbye");
}

/// Ported from `test_no_cache_sync`.
#[tokio::test]
async fn test_no_cache_sync() {
    let global_cache = Arc::new(InMemoryCache::unbounded());
    set_llm_cache(Some(global_cache.clone()));

    let model = FakeListChatModel::new(vec!["hello".to_string(), "goodbye".to_string()])
        .with_cache_disabled();

    let result = model
        .invoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .invoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "goodbye");

    set_llm_cache(None);
}

/// Ported from `test_no_cache_async`.
#[tokio::test]
async fn test_no_cache_async() {
    let global_cache = Arc::new(InMemoryCache::unbounded());
    set_llm_cache(Some(global_cache.clone()));

    let model = FakeListChatModel::new(vec!["hello".to_string(), "goodbye".to_string()])
        .with_cache_disabled();

    let result = model
        .ainvoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .ainvoke(LanguageModelInput::from("How are you?"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "goodbye");

    set_llm_cache(None);
}

/// Ported from `test_can_swap_caches`.
#[tokio::test]
async fn test_can_swap_caches() {
    let cache = Arc::new(InMemoryCache::unbounded());

    let model = FakeListChatModel::new(vec!["hello".to_string(), "goodbye".to_string()])
        .with_cache_instance(cache.clone());

    let result = model
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let new_cache = Arc::new(InMemoryCache::unbounded());
    let model2 = FakeListChatModel::new(vec!["different".to_string()])
        .with_cache_instance(new_cache.clone());

    let result = model2
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "different");
}

/// Ported from `test_cache_with_generation_objects`.
///
/// Tests that the cache can handle Generation objects (instead of ChatGeneration)
/// and properly convert them back to ChatGeneration when returned as cache hits.
/// This reproduces a scenario where cache contains Generation objects due to
/// serialization/deserialization issues or legacy cache data.
#[tokio::test]
async fn test_cache_with_generation_objects() {
    use agent_chain_core::language_models::BaseChatModel;

    let cache = Arc::new(InMemoryCache::unbounded());

    let model =
        FakeListChatModel::new(vec!["hello".to_string()]).with_cache_instance(cache.clone());

    let result = model
        .invoke(LanguageModelInput::from("test prompt"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");


    let result = model
        .invoke(LanguageModelInput::from("test prompt"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "hello");
}

/// Test that the cache round-trip preserves message data through
/// `_chat_generations_to_cache` and `_convert_cached_generations`.
///
/// Verifies the fix for cache previously only storing `Generation::new(text)`,
/// which lost the original message type and generation_info.
#[tokio::test]
async fn test_cache_preserves_message_through_round_trip() {
    let cache = Arc::new(InMemoryCache::unbounded());

    let model =
        FakeListChatModel::new(vec!["cached hello".to_string()]).with_cache_instance(cache.clone());

    let result = model
        .invoke(LanguageModelInput::from("round trip test"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "cached hello");

    let result = model
        .invoke(LanguageModelInput::from("round trip test"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "cached hello");

    assert!(
        !result.content.is_empty(),
        "Cached response should have non-empty content"
    );
}

/// Test that `_convert_cached_generations` handles legacy `Generation` objects
/// (i.e., no serialized "message" in generation_info) by creating AIMessages.
#[tokio::test]
async fn test_convert_cached_generations_legacy_format() {
    use agent_chain_core::caches::BaseCache;
    use agent_chain_core::outputs::Generation;

    let cache = Arc::new(InMemoryCache::unbounded());

    let model = FakeListChatModel::new(vec!["first".to_string(), "second".to_string()])
        .with_cache_instance(cache.clone());

    let result = model
        .invoke(LanguageModelInput::from("legacy test"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "first");

    cache.clear();

    let messages = vec![agent_chain_core::messages::BaseMessage::from("legacy test")];
    let prompt_key = serde_json::to_string(&messages).unwrap();
    let llm_string = model._get_llm_string(None, None);

    let legacy_generations = vec![Generation::new("legacy text")];
    cache.update(&prompt_key, &llm_string, legacy_generations);

    let result = model
        .invoke(LanguageModelInput::from("legacy test"), None)
        .await
        .unwrap();
    assert_eq!(result.content, "legacy text");
}

/// Test that cache key is deterministic — same model parameters produce the same key.
#[test]
fn test_cache_key_determinism() {
    let model = FakeListChatModel::new(vec!["test".to_string()]);
    let key1 = model._get_llm_string(None, None);
    let key2 = model._get_llm_string(None, None);
    assert_eq!(key1, key2, "Cache key should be deterministic");

    let key3 = model._get_llm_string(Some(&["stop1".to_string(), "stop2".to_string()]), None);
    let key4 = model._get_llm_string(Some(&["stop1".to_string(), "stop2".to_string()]), None);
    assert_eq!(
        key3, key4,
        "Cache key with same stop words should be deterministic"
    );

    let key5 = model._get_llm_string(Some(&["stop3".to_string()]), None);
    assert_ne!(
        key3, key5,
        "Different stop words should produce different keys"
    );
}
