//! Rate limiting tests for chat models.
//!
//! Ported from `langchain/libs/core/tests/unit_tests/language_models/chat_models/test_rate_limiting.py`

use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_chain_core::GenericFakeChatModel;
use agent_chain_core::language_models::{BaseChatModel, ChatModelConfig, LanguageModelInput};
use agent_chain_core::messages::AIMessage;
use agent_chain_core::rate_limiters::{InMemoryRateLimiter, InMemoryRateLimiterConfig};

fn make_rate_limited_model(
    messages: Vec<AIMessage>,
    requests_per_second: f64,
    check_every_n_seconds: f64,
    max_bucket_size: f64,
) -> GenericFakeChatModel {
    let rate_limiter = Arc::new(InMemoryRateLimiter::new(InMemoryRateLimiterConfig {
        requests_per_second,
        check_every_n_seconds,
        max_bucket_size,
    }));
    let config = ChatModelConfig::new().with_rate_limiter(rate_limiter);
    GenericFakeChatModel::from_vec(messages).with_config(config)
}

/// Ported from `test_rate_limit_invoke`.
#[tokio::test]
async fn test_rate_limit_invoke() {
    let model = make_rate_limited_model(
        vec![
            AIMessage::builder().content("hello").build(),
            AIMessage::builder().content("world").build(),
        ],
        20.0,
        0.1,
        10.0,
    );

    // First call — token bucket starts empty, must wait
    let tic = Instant::now();
    let _ = model.invoke(LanguageModelInput::from("foo")).await.unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= Duration::from_millis(100),
        "First call took {:?}, expected >= 100ms",
        elapsed
    );

    // Second call — should have a token available
    let tic = Instant::now();
    let _ = model.invoke(LanguageModelInput::from("foo")).await.unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed < Duration::from_millis(100),
        "Second call took {:?}, expected < 100ms",
        elapsed
    );
}

/// Ported from `test_rate_limit_ainvoke`.
#[tokio::test]
async fn test_rate_limit_ainvoke() {
    let model = make_rate_limited_model(
        vec![
            AIMessage::builder().content("hello").build(),
            AIMessage::builder().content("world").build(),
            AIMessage::builder().content("!").build(),
        ],
        20.0,
        0.1,
        10.0,
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("foo"))
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(elapsed >= Duration::from_millis(100));

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("foo"))
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(elapsed < Duration::from_millis(100));

    // Third call — needs to wait again
    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("foo"))
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(elapsed >= Duration::from_millis(100));
}

/// Ported from `test_rate_limit_skips_cache`.
#[tokio::test]
async fn test_rate_limit_skips_cache() {
    use agent_chain_core::caches::InMemoryCache;

    let cache = Arc::new(InMemoryCache::unbounded());
    let rate_limiter = Arc::new(InMemoryRateLimiter::new(InMemoryRateLimiterConfig {
        requests_per_second: 20.0,
        check_every_n_seconds: 0.1,
        max_bucket_size: 1.0,
    }));
    let config = ChatModelConfig::new()
        .with_rate_limiter(rate_limiter)
        .with_cache_instance(cache.clone());

    let model = GenericFakeChatModel::from_vec(vec![
        AIMessage::builder().content("hello").build(),
        AIMessage::builder().content("world").build(),
        AIMessage::builder().content("!").build(),
    ])
    .with_config(config);

    // First call — rate limited (cache miss)
    let tic = Instant::now();
    let _ = model.invoke(LanguageModelInput::from("foo")).await.unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= Duration::from_millis(100),
        "First call took {:?}",
        elapsed
    );

    // Second and third calls — cache hits, no rate limiting
    for i in 0..2 {
        let tic = Instant::now();
        let _ = model.invoke(LanguageModelInput::from("foo")).await.unwrap();
        let elapsed = tic.elapsed();
        assert!(
            elapsed < Duration::from_millis(50),
            "Cache hit {} took {:?}, expected < 50ms",
            i + 1,
            elapsed
        );
    }
}
