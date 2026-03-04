use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_chain_core::GenericFakeChatModel;
use agent_chain_core::language_models::{BaseChatModel, ChatModelConfig, LanguageModelInput};
use agent_chain_core::messages::AIMessage;
use agent_chain_core::rate_limiters::InMemoryRateLimiter;

fn make_rate_limited_model(
    messages: Vec<AIMessage>,
    requests_per_second: f64,
    check_every_n_seconds: f64,
    max_bucket_size: f64,
) -> GenericFakeChatModel {
    let rate_limiter = Arc::new(
        InMemoryRateLimiter::builder()
            .requests_per_second(requests_per_second)
            .check_every_n_seconds(check_every_n_seconds)
            .max_bucket_size(max_bucket_size)
            .build(),
    );
    let config = ChatModelConfig::builder()
        .rate_limiter(rate_limiter)
        .build();
    GenericFakeChatModel::from_vec(messages).with_config(config)
}

#[tokio::test]
async fn test_rate_limit_invoke() {
    let model = make_rate_limited_model(
        vec![
            AIMessage::builder().content("hello").build(),
            AIMessage::builder().content("world").build(),
        ],
        20.0,
        0.1,
        1.0,
    );

    let tic = Instant::now();
    let _ = model
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "First call took {:?}, expected < 50ms (burst token available)",
        elapsed
    );

    let tic = Instant::now();
    let _ = model
        .invoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= Duration::from_millis(30),
        "Second call took {:?}, expected >= 30ms (must wait for replenishment)",
        elapsed
    );
}

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
        1.0,
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "First call took {:?}, expected < 50ms",
        elapsed
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= Duration::from_millis(30),
        "Second call took {:?}, expected >= 30ms",
        elapsed
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("baz"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= Duration::from_millis(30),
        "Third call took {:?}, expected >= 30ms",
        elapsed
    );
}

#[tokio::test]
async fn test_rate_limit_skips_cache() {
    use agent_chain_core::caches::InMemoryCache;

    let cache = Arc::new(InMemoryCache::unbounded());
    let rate_limiter = Arc::new(
        InMemoryRateLimiter::builder()
            .requests_per_second(20.0)
            .check_every_n_seconds(0.1)
            .build(),
    );
    let config = ChatModelConfig::builder()
        .rate_limiter(rate_limiter)
        .cache_instance(cache.clone())
        .build();

    let model = GenericFakeChatModel::from_vec(vec![
        AIMessage::builder().content("hello").build(),
        AIMessage::builder().content("world").build(),
        AIMessage::builder().content("!").build(),
    ])
    .with_config(config);

    let tic = Instant::now();
    let _ = model
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "First call took {:?}, expected < 50ms (burst token)",
        elapsed
    );

    for i in 0..2 {
        let tic = Instant::now();
        let _ = model
            .invoke(LanguageModelInput::from("foo"), None)
            .await
            .unwrap();
        let elapsed = tic.elapsed();
        assert!(
            elapsed < Duration::from_millis(50),
            "Cache hit {} took {:?}, expected < 50ms",
            i + 1,
            elapsed
        );
    }
}

#[tokio::test]
async fn test_rate_limit_stream() {
    let model = make_rate_limited_model(
        vec![
            AIMessage::builder().content("hello world").build(),
            AIMessage::builder().content("hello world").build(),
            AIMessage::builder().content("hello world").build(),
        ],
        20.0,
        0.1,
        1.0,
    );

    let tic = Instant::now();
    let result = model
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(result.content.contains("hello"));
    assert!(
        elapsed < Duration::from_millis(50),
        "First invoke took {:?}, expected < 50ms",
        elapsed
    );

    let tic = Instant::now();
    let _ = model
        .invoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= Duration::from_millis(30),
        "Second invoke took {:?}, expected >= 30ms",
        elapsed
    );

    let tic = Instant::now();
    let _ = model
        .invoke(LanguageModelInput::from("baz"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= Duration::from_millis(30),
        "Third invoke took {:?}, expected >= 30ms",
        elapsed
    );
}

#[tokio::test]
async fn test_rate_limit_astream() {
    let model = make_rate_limited_model(
        vec![
            AIMessage::builder().content("hello world").build(),
            AIMessage::builder().content("hello world").build(),
            AIMessage::builder().content("hello world").build(),
        ],
        20.0,
        0.1,
        1.0,
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "First call took {:?}, expected < 50ms",
        elapsed
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= Duration::from_millis(30),
        "Second call took {:?}, expected >= 30ms",
        elapsed
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("baz"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= Duration::from_millis(30),
        "Third call took {:?}, expected >= 30ms",
        elapsed
    );
}

#[tokio::test]
async fn test_rate_limit_skips_cache_async() {
    use agent_chain_core::caches::InMemoryCache;

    let cache = Arc::new(InMemoryCache::unbounded());
    let rate_limiter = Arc::new(
        InMemoryRateLimiter::builder()
            .requests_per_second(20.0)
            .check_every_n_seconds(0.1)
            .build(),
    );
    let config = ChatModelConfig::builder()
        .rate_limiter(rate_limiter)
        .cache_instance(cache.clone())
        .build();

    let model = GenericFakeChatModel::from_vec(vec![
        AIMessage::builder().content("hello").build(),
        AIMessage::builder().content("world").build(),
        AIMessage::builder().content("!").build(),
    ])
    .with_config(config);

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed < Duration::from_millis(50),
        "First call took {:?}, expected < 50ms (burst token)",
        elapsed
    );

    for i in 0..2 {
        let tic = Instant::now();
        let _ = model
            .ainvoke(LanguageModelInput::from("foo"), None)
            .await
            .unwrap();
        let elapsed = tic.elapsed();
        assert!(
            elapsed < Duration::from_millis(50),
            "Cache hit {} took {:?}, expected < 50ms",
            i + 1,
            elapsed
        );
    }
}
