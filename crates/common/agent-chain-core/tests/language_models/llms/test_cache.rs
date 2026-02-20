use std::sync::Arc;

use agent_chain_core::caches::{BaseCache, InMemoryCache};
use agent_chain_core::language_models::{BaseLLM, FakeListLLM};
use agent_chain_core::outputs::GenerationType;
use agent_chain_core::set_llm_cache;

#[tokio::test]
async fn test_local_cache_generate_async() {
    let global_cache = Arc::new(InMemoryCache::unbounded());
    let local_cache = Arc::new(InMemoryCache::unbounded());

    set_llm_cache(Some(global_cache.clone()));

    let llm = FakeListLLM::new(vec!["foo".to_string(), "bar".to_string()])
        .with_cache_instance(local_cache.clone());

    let output = llm
        .generate(
            vec!["foo".to_string()],
            agent_chain_core::language_models::LLMGenerateConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(output.generations.len(), 1);
    match &output.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "foo"),
        _ => panic!("Expected Generation variant"),
    }

    let output = llm
        .generate(
            vec!["foo".to_string()],
            agent_chain_core::language_models::LLMGenerateConfig::default(),
        )
        .await
        .unwrap();
    assert_eq!(output.generations.len(), 1);
    match &output.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "foo"),
        _ => panic!("Expected Generation variant"),
    }

    assert!(global_cache.lookup("foo", "").is_none() || { true });

    set_llm_cache(None);
}

#[tokio::test]
async fn test_local_cache_generate_sync() {
    let global_cache = Arc::new(InMemoryCache::unbounded());
    let local_cache = Arc::new(InMemoryCache::unbounded());

    set_llm_cache(Some(global_cache.clone()));

    let llm = FakeListLLM::new(vec!["foo".to_string(), "bar".to_string()])
        .with_cache_instance(local_cache.clone());

    let output = llm
        .generate(
            vec!["foo".to_string()],
            agent_chain_core::language_models::LLMGenerateConfig::default(),
        )
        .await
        .unwrap();
    match &output.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "foo"),
        _ => panic!("Expected Generation variant"),
    }

    let output = llm
        .generate(
            vec!["foo".to_string()],
            agent_chain_core::language_models::LLMGenerateConfig::default(),
        )
        .await
        .unwrap();
    match &output.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "foo"),
        _ => panic!("Expected Generation variant"),
    }

    set_llm_cache(None);
}

#[tokio::test]
async fn test_no_cache_generate_sync() {
    let global_cache = Arc::new(InMemoryCache::unbounded());

    set_llm_cache(Some(global_cache.clone()));

    let llm = FakeListLLM::new(vec!["foo".to_string(), "bar".to_string()]).with_cache_disabled();

    let output = llm
        .generate(
            vec!["foo".to_string()],
            agent_chain_core::language_models::LLMGenerateConfig::default(),
        )
        .await
        .unwrap();
    match &output.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "foo"),
        _ => panic!("Expected Generation variant"),
    }

    let output = llm
        .generate(
            vec!["foo".to_string()],
            agent_chain_core::language_models::LLMGenerateConfig::default(),
        )
        .await
        .unwrap();
    match &output.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "bar"),
        _ => panic!("Expected Generation variant"),
    }

    set_llm_cache(None);
}

#[tokio::test]
async fn test_no_cache_generate_async() {
    let global_cache = Arc::new(InMemoryCache::unbounded());

    set_llm_cache(Some(global_cache.clone()));

    let llm = FakeListLLM::new(vec!["foo".to_string(), "bar".to_string()]).with_cache_disabled();

    let output = llm
        .generate(
            vec!["foo".to_string()],
            agent_chain_core::language_models::LLMGenerateConfig::default(),
        )
        .await
        .unwrap();
    match &output.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "foo"),
        _ => panic!("Expected Generation variant"),
    }

    let output = llm
        .generate(
            vec!["foo".to_string()],
            agent_chain_core::language_models::LLMGenerateConfig::default(),
        )
        .await
        .unwrap();
    match &output.generations[0][0] {
        GenerationType::Generation(generation) => assert_eq!(generation.text, "bar"),
        _ => panic!("Expected Generation variant"),
    }

    set_llm_cache(None);
}
