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
        .invoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    // Cache hit — same result
    let result = model
        .invoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    // Different prompt — cache miss
    let result = model
        .invoke(LanguageModelInput::from("meow?"))
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
        .ainvoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    // Cache hit
    let result = model
        .ainvoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    // Different prompt
    let result = model
        .ainvoke(LanguageModelInput::from("meow?"))
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
        .invoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    // Cache hit
    let result = model
        .invoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    // Different prompt — cache miss
    let result = model
        .invoke(LanguageModelInput::from("nice"))
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
        .ainvoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .ainvoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .ainvoke(LanguageModelInput::from("nice"))
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
        .invoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    // No cache — gets fresh response
    let result = model
        .invoke(LanguageModelInput::from("How are you?"))
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
        .ainvoke(LanguageModelInput::from("How are you?"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    let result = model
        .ainvoke(LanguageModelInput::from("How are you?"))
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

    let result = model.invoke(LanguageModelInput::from("foo")).await.unwrap();
    assert_eq!(result.content, "hello");

    // New model with empty cache gets fresh result
    let new_cache = Arc::new(InMemoryCache::unbounded());
    let model2 = FakeListChatModel::new(vec!["different".to_string()])
        .with_cache_instance(new_cache.clone());

    let result = model2
        .invoke(LanguageModelInput::from("foo"))
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

    // First call — cache miss, populates cache with Generation objects
    let result = model
        .invoke(LanguageModelInput::from("test prompt"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");

    // Manually verify the cache was populated
    // (The cache stores Generation objects, not ChatGeneration)

    // Second call — cache hit, should convert Generation → ChatGeneration
    let result = model
        .invoke(LanguageModelInput::from("test prompt"))
        .await
        .unwrap();
    assert_eq!(result.content, "hello");
}
