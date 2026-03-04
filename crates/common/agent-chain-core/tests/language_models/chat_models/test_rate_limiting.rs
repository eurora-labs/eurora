use std::sync::Arc;
use std::time::{Duration, Instant};

use agent_chain_core::GenericFakeChatModel;
use agent_chain_core::language_models::{BaseChatModel, ChatModelConfig, LanguageModelInput};
use agent_chain_core::messages::AIMessage;
use agent_chain_core::rate_limiters::InMemoryRateLimiter;

// Use a slow rate (4 req/s → 250ms period) so that contention from concurrent
// tests can never accidentally mask the wait. Governor's GCRA bucket refills
// after 250ms; even under heavy test load a single call + scheduling overhead
// won't reach that.
const RATE: f64 = 4.0;
const CHECK_INTERVAL: f64 = 0.01;
// Lower bound for "must have waited" assertions.  Well below the 250ms period
// but well above any reasonable scheduling jitter.
const MIN_WAIT: Duration = Duration::from_millis(100);
// Upper bound for "should be instant" assertions.
const MAX_INSTANT: Duration = Duration::from_millis(100);

fn make_rate_limited_model(messages: Vec<AIMessage>) -> GenericFakeChatModel {
    let rate_limiter = Arc::new(
        InMemoryRateLimiter::builder()
            .requests_per_second(RATE)
            .check_every_n_seconds(CHECK_INTERVAL)
            .max_bucket_size(1.0)
            .build(),
    );
    let config = ChatModelConfig::builder()
        .rate_limiter(rate_limiter)
        .build();
    GenericFakeChatModel::from_vec(messages).with_config(config)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_invoke() {
    let model = make_rate_limited_model(vec![
        AIMessage::builder().content("hello").build(),
        AIMessage::builder().content("world").build(),
    ]);

    let tic = Instant::now();
    let _ = model
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed < MAX_INSTANT,
        "First call took {:?}, expected < {:?} (burst token available)",
        elapsed,
        MAX_INSTANT,
    );

    let tic = Instant::now();
    let _ = model
        .invoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= MIN_WAIT,
        "Second call took {:?}, expected >= {:?} (must wait for replenishment)",
        elapsed,
        MIN_WAIT,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_ainvoke() {
    let model = make_rate_limited_model(vec![
        AIMessage::builder().content("hello").build(),
        AIMessage::builder().content("world").build(),
        AIMessage::builder().content("!").build(),
    ]);

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed < MAX_INSTANT,
        "First call took {:?}, expected < {:?}",
        elapsed,
        MAX_INSTANT,
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= MIN_WAIT,
        "Second call took {:?}, expected >= {:?} (must wait for replenishment)",
        elapsed,
        MIN_WAIT,
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("baz"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= MIN_WAIT,
        "Third call took {:?}, expected >= {:?} (must wait for replenishment)",
        elapsed,
        MIN_WAIT,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_skips_cache() {
    use agent_chain_core::caches::InMemoryCache;

    let cache = Arc::new(InMemoryCache::unbounded());
    let rate_limiter = Arc::new(
        InMemoryRateLimiter::builder()
            .requests_per_second(RATE)
            .check_every_n_seconds(CHECK_INTERVAL)
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
        elapsed < MAX_INSTANT,
        "First call took {:?}, expected < {:?} (burst token)",
        elapsed,
        MAX_INSTANT,
    );

    for i in 0..2 {
        let tic = Instant::now();
        let _ = model
            .invoke(LanguageModelInput::from("foo"), None)
            .await
            .unwrap();
        let elapsed = tic.elapsed();
        assert!(
            elapsed < MAX_INSTANT,
            "Cache hit {} took {:?}, expected < {:?}",
            i + 1,
            elapsed,
            MAX_INSTANT,
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_stream() {
    let model = make_rate_limited_model(vec![
        AIMessage::builder().content("hello world").build(),
        AIMessage::builder().content("hello world").build(),
        AIMessage::builder().content("hello world").build(),
    ]);

    let tic = Instant::now();
    let result = model
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(result.content.contains("hello"));
    assert!(
        elapsed < MAX_INSTANT,
        "First invoke took {:?}, expected < {:?}",
        elapsed,
        MAX_INSTANT,
    );

    let tic = Instant::now();
    let _ = model
        .invoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= MIN_WAIT,
        "Second invoke took {:?}, expected >= {:?}",
        elapsed,
        MIN_WAIT,
    );

    let tic = Instant::now();
    let _ = model
        .invoke(LanguageModelInput::from("baz"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= MIN_WAIT,
        "Third invoke took {:?}, expected >= {:?}",
        elapsed,
        MIN_WAIT,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_astream() {
    let model = make_rate_limited_model(vec![
        AIMessage::builder().content("hello world").build(),
        AIMessage::builder().content("hello world").build(),
        AIMessage::builder().content("hello world").build(),
    ]);

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed < MAX_INSTANT,
        "First call took {:?}, expected < {:?}",
        elapsed,
        MAX_INSTANT,
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= MIN_WAIT,
        "Second call took {:?}, expected >= {:?}",
        elapsed,
        MIN_WAIT,
    );

    let tic = Instant::now();
    let _ = model
        .ainvoke(LanguageModelInput::from("baz"), None)
        .await
        .unwrap();
    let elapsed = tic.elapsed();
    assert!(
        elapsed >= MIN_WAIT,
        "Third call took {:?}, expected >= {:?}",
        elapsed,
        MIN_WAIT,
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_skips_cache_async() {
    use agent_chain_core::caches::InMemoryCache;

    let cache = Arc::new(InMemoryCache::unbounded());
    let rate_limiter = Arc::new(
        InMemoryRateLimiter::builder()
            .requests_per_second(RATE)
            .check_every_n_seconds(CHECK_INTERVAL)
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
        elapsed < MAX_INSTANT,
        "First call took {:?}, expected < {:?} (burst token)",
        elapsed,
        MAX_INSTANT,
    );

    for i in 0..2 {
        let tic = Instant::now();
        let _ = model
            .ainvoke(LanguageModelInput::from("foo"), None)
            .await
            .unwrap();
        let elapsed = tic.elapsed();
        assert!(
            elapsed < MAX_INSTANT,
            "Cache hit {} took {:?}, expected < {:?}",
            i + 1,
            elapsed,
            MAX_INSTANT,
        );
    }
}
