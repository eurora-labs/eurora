use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;

use agent_chain_core::GenericFakeChatModel;
use agent_chain_core::language_models::{BaseChatModel, ChatModelConfig, LanguageModelInput};
use agent_chain_core::messages::AIMessage;
use agent_chain_core::rate_limiters::BaseRateLimiter;

struct CountingRateLimiter {
    acquire_count: AtomicUsize,
    aacquire_count: AtomicUsize,
}

impl CountingRateLimiter {
    fn new() -> Self {
        Self {
            acquire_count: AtomicUsize::new(0),
            aacquire_count: AtomicUsize::new(0),
        }
    }

    fn acquire_count(&self) -> usize {
        self.acquire_count.load(Ordering::SeqCst)
    }

    fn aacquire_count(&self) -> usize {
        self.aacquire_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl BaseRateLimiter for CountingRateLimiter {
    fn acquire(&self, _blocking: bool) -> bool {
        self.acquire_count.fetch_add(1, Ordering::SeqCst);
        true
    }

    async fn aacquire(&self, _blocking: bool) -> bool {
        self.aacquire_count.fetch_add(1, Ordering::SeqCst);
        true
    }
}

fn make_model(messages: Vec<AIMessage>, limiter: Arc<CountingRateLimiter>) -> GenericFakeChatModel {
    let config = ChatModelConfig::builder()
        .rate_limiter(limiter as Arc<dyn BaseRateLimiter>)
        .build();
    GenericFakeChatModel::from_vec(messages).with_config(config)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_invoke() {
    let limiter = Arc::new(CountingRateLimiter::new());
    let model = make_model(
        vec![
            AIMessage::builder().content("hello").build(),
            AIMessage::builder().content("world").build(),
        ],
        limiter.clone(),
    );

    let _ = model
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    assert_eq!(limiter.acquire_count(), 1);

    let _ = model
        .invoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    assert_eq!(limiter.acquire_count(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_ainvoke() {
    let limiter = Arc::new(CountingRateLimiter::new());
    let model = make_model(
        vec![
            AIMessage::builder().content("hello").build(),
            AIMessage::builder().content("world").build(),
            AIMessage::builder().content("!").build(),
        ],
        limiter.clone(),
    );

    let _ = model
        .ainvoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    assert_eq!(limiter.aacquire_count(), 1);

    let _ = model
        .ainvoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    assert_eq!(limiter.aacquire_count(), 2);

    let _ = model
        .ainvoke(LanguageModelInput::from("baz"), None)
        .await
        .unwrap();
    assert_eq!(limiter.aacquire_count(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_skips_cache() {
    use agent_chain_core::caches::InMemoryCache;

    let cache = Arc::new(InMemoryCache::unbounded());
    let limiter = Arc::new(CountingRateLimiter::new());
    let config = ChatModelConfig::builder()
        .rate_limiter(limiter.clone() as Arc<dyn BaseRateLimiter>)
        .cache_instance(cache.clone())
        .build();

    let model = GenericFakeChatModel::from_vec(vec![
        AIMessage::builder().content("hello").build(),
        AIMessage::builder().content("world").build(),
        AIMessage::builder().content("!").build(),
    ])
    .with_config(config);

    let _ = model
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    assert_eq!(limiter.acquire_count(), 1);

    for _ in 0..3 {
        let _ = model
            .invoke(LanguageModelInput::from("foo"), None)
            .await
            .unwrap();
    }
    assert_eq!(
        limiter.acquire_count(),
        1,
        "cache hits must skip rate limiter"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_stream() {
    let limiter = Arc::new(CountingRateLimiter::new());
    let model = make_model(
        vec![
            AIMessage::builder().content("hello world").build(),
            AIMessage::builder().content("hello world").build(),
            AIMessage::builder().content("hello world").build(),
        ],
        limiter.clone(),
    );

    let result = model
        .invoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    assert!(result.content.contains("hello"));
    assert_eq!(limiter.acquire_count(), 1);

    let _ = model
        .invoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    assert_eq!(limiter.acquire_count(), 2);

    let _ = model
        .invoke(LanguageModelInput::from("baz"), None)
        .await
        .unwrap();
    assert_eq!(limiter.acquire_count(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_astream() {
    let limiter = Arc::new(CountingRateLimiter::new());
    let model = make_model(
        vec![
            AIMessage::builder().content("hello world").build(),
            AIMessage::builder().content("hello world").build(),
            AIMessage::builder().content("hello world").build(),
        ],
        limiter.clone(),
    );

    let _ = model
        .ainvoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    assert_eq!(limiter.aacquire_count(), 1);

    let _ = model
        .ainvoke(LanguageModelInput::from("bar"), None)
        .await
        .unwrap();
    assert_eq!(limiter.aacquire_count(), 2);

    let _ = model
        .ainvoke(LanguageModelInput::from("baz"), None)
        .await
        .unwrap();
    assert_eq!(limiter.aacquire_count(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_rate_limit_skips_cache_async() {
    use agent_chain_core::caches::InMemoryCache;

    let cache = Arc::new(InMemoryCache::unbounded());
    let limiter = Arc::new(CountingRateLimiter::new());
    let config = ChatModelConfig::builder()
        .rate_limiter(limiter.clone() as Arc<dyn BaseRateLimiter>)
        .cache_instance(cache.clone())
        .build();

    let model = GenericFakeChatModel::from_vec(vec![
        AIMessage::builder().content("hello").build(),
        AIMessage::builder().content("world").build(),
        AIMessage::builder().content("!").build(),
    ])
    .with_config(config);

    let _ = model
        .ainvoke(LanguageModelInput::from("foo"), None)
        .await
        .unwrap();
    assert_eq!(limiter.aacquire_count(), 1);

    for _ in 0..3 {
        let _ = model
            .ainvoke(LanguageModelInput::from("foo"), None)
            .await
            .unwrap();
    }
    assert_eq!(
        limiter.aacquire_count(),
        1,
        "cache hits must skip rate limiter"
    );
}
