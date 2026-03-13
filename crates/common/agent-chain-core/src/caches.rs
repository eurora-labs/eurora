use async_trait::async_trait;

use crate::outputs::ChatGeneration;
pub use crate::runnables::run_in_executor;

pub type CacheReturnValue = Vec<ChatGeneration>;

#[async_trait]
pub trait BaseCache: Send + Sync {
    async fn lookup(&self, prompt: &str, llm_string: &str) -> Option<CacheReturnValue>;

    async fn update(&self, prompt: &str, llm_string: &str, return_val: CacheReturnValue);

    async fn clear(&self);
}

#[derive(Debug)]
pub struct InMemoryCache {
    cache: moka::sync::Cache<(String, String), CacheReturnValue>,
}

impl InMemoryCache {
    pub fn new(maxsize: Option<usize>) -> crate::Result<Self> {
        if let Some(size) = maxsize
            && size == 0
        {
            return Err(crate::Error::InvalidConfig(
                "maxsize must be greater than 0".to_string(),
            ));
        }
        let cache = match maxsize {
            Some(size) => moka::sync::Cache::new(size as u64),
            None => moka::sync::Cache::new(u64::MAX),
        };
        Ok(Self { cache })
    }

    pub fn unbounded() -> Self {
        Self {
            cache: moka::sync::Cache::new(u64::MAX),
        }
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::unbounded()
    }
}

#[async_trait]
impl BaseCache for InMemoryCache {
    async fn lookup(&self, prompt: &str, llm_string: &str) -> Option<CacheReturnValue> {
        self.cache
            .get(&(prompt.to_string(), llm_string.to_string()))
    }

    async fn update(&self, prompt: &str, llm_string: &str, return_val: CacheReturnValue) {
        let key = (prompt.to_string(), llm_string.to_string());
        self.cache.insert(key, return_val);
    }

    async fn clear(&self) {
        self.cache.invalidate_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::AIMessage;
    use crate::outputs::ChatGeneration;

    fn chat_gen(text: &str) -> ChatGeneration {
        let msg = AIMessage::builder().content(text).build();
        ChatGeneration::builder().message(msg.into()).build()
    }

    #[tokio::test]
    async fn test_in_memory_cache_new() {
        let cache = InMemoryCache::new(None).unwrap();
        assert!(cache.lookup("prompt", "llm").await.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_cache_unbounded() {
        let cache = InMemoryCache::unbounded();
        assert!(cache.lookup("prompt", "llm").await.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_cache_default() {
        let cache = InMemoryCache::default();
        assert!(cache.lookup("prompt", "llm").await.is_none());
    }

    #[test]
    fn test_in_memory_cache_zero_maxsize() {
        let result = InMemoryCache::new(Some(0));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("maxsize must be greater than 0"));
    }

    #[tokio::test]
    async fn test_in_memory_cache_lookup_miss() {
        let cache = InMemoryCache::new(None).unwrap();
        let result = cache.lookup("prompt", "llm_string").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_cache_update_and_lookup() {
        let cache = InMemoryCache::new(None).unwrap();
        let generations = vec![chat_gen("Hello, world!")];

        cache
            .update("prompt", "llm_string", generations.clone())
            .await;

        let result = cache.lookup("prompt", "llm_string").await;
        assert!(result.is_some());
        let cached = result.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].message.text(), "Hello, world!");
    }

    #[tokio::test]
    async fn test_in_memory_cache_clear() {
        let cache = InMemoryCache::new(None).unwrap();
        let generations = vec![chat_gen("Hello")];

        cache.update("prompt1", "llm", generations.clone()).await;
        cache.update("prompt2", "llm", generations.clone()).await;

        assert!(cache.lookup("prompt1", "llm").await.is_some());
        assert!(cache.lookup("prompt2", "llm").await.is_some());

        cache.clear().await;

        assert!(cache.lookup("prompt1", "llm").await.is_none());
        assert!(cache.lookup("prompt2", "llm").await.is_none());
    }

    #[tokio::test]
    async fn test_in_memory_cache_maxsize() {
        let cache = InMemoryCache::new(Some(2)).unwrap();

        for i in 0..5 {
            cache
                .update(
                    &format!("prompt{}", i),
                    "llm",
                    vec![chat_gen(&format!("{}", i))],
                )
                .await;
        }

        cache.cache.run_pending_tasks();

        let mut present = 0;
        for i in 0..5 {
            if cache.lookup(&format!("prompt{}", i), "llm").await.is_some() {
                present += 1;
            }
        }
        assert!(present <= 2, "expected at most 2 entries, got {}", present);
    }

    #[tokio::test]
    async fn test_in_memory_cache_update_existing_key() {
        let cache = InMemoryCache::new(None).unwrap();

        cache.update("prompt", "llm", vec![chat_gen("first")]).await;
        let result = cache.lookup("prompt", "llm").await.unwrap();
        assert_eq!(result[0].message.text(), "first");

        cache
            .update("prompt", "llm", vec![chat_gen("second")])
            .await;
        let result = cache.lookup("prompt", "llm").await.unwrap();
        assert_eq!(result[0].message.text(), "second");
    }

    #[tokio::test]
    async fn test_in_memory_cache_different_llm_strings() {
        let cache = InMemoryCache::new(None).unwrap();

        cache
            .update("prompt", "llm1", vec![chat_gen("from llm1")])
            .await;
        cache
            .update("prompt", "llm2", vec![chat_gen("from llm2")])
            .await;

        let result1 = cache.lookup("prompt", "llm1").await.unwrap();
        assert_eq!(result1[0].message.text(), "from llm1");

        let result2 = cache.lookup("prompt", "llm2").await.unwrap();
        assert_eq!(result2[0].message.text(), "from llm2");
    }

    #[tokio::test]
    async fn test_in_memory_cache_multiple_generations() {
        let cache = InMemoryCache::new(None).unwrap();
        let generations = vec![
            chat_gen("First generation"),
            chat_gen("Second generation"),
            chat_gen("Third generation"),
        ];

        cache.update("prompt", "llm", generations).await;

        let result = cache.lookup("prompt", "llm").await.unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].message.text(), "First generation");
        assert_eq!(result[1].message.text(), "Second generation");
        assert_eq!(result[2].message.text(), "Third generation");
    }
}
