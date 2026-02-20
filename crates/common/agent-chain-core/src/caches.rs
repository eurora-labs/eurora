use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::outputs::Generation;

pub type CacheReturnValue = Vec<Generation>;

#[async_trait]
pub trait BaseCache: Send + Sync {
    fn lookup(&self, prompt: &str, llm_string: &str) -> Option<CacheReturnValue>;

    fn update(&self, prompt: &str, llm_string: &str, return_val: CacheReturnValue);

    fn clear(&self);

    async fn alookup(&self, prompt: &str, llm_string: &str) -> Option<CacheReturnValue> {
        self.lookup(prompt, llm_string)
    }

    async fn aupdate(&self, prompt: &str, llm_string: &str, return_val: CacheReturnValue) {
        self.update(prompt, llm_string, return_val);
    }

    async fn aclear(&self) {
        self.clear();
    }
}

#[derive(Debug)]
pub struct InMemoryCache {
    cache: RwLock<HashMap<(String, String), CacheReturnValue>>,
    maxsize: Option<usize>,
    key_order: RwLock<Vec<(String, String)>>,
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
        Ok(Self {
            cache: RwLock::new(HashMap::new()),
            maxsize,
            key_order: RwLock::new(Vec::new()),
        })
    }

    pub fn unbounded() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            maxsize: None,
            key_order: RwLock::new(Vec::new()),
        }
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::unbounded()
    }
}

impl InMemoryCache {
    fn lock_read_cache(
        &self,
    ) -> Option<std::sync::RwLockReadGuard<'_, HashMap<(String, String), CacheReturnValue>>> {
        match self.cache.read() {
            Ok(guard) => Some(guard),
            Err(error) => {
                tracing::error!("Cache read lock poisoned: {}", error);
                None
            }
        }
    }

    fn lock_write_cache(
        &self,
    ) -> Option<std::sync::RwLockWriteGuard<'_, HashMap<(String, String), CacheReturnValue>>> {
        match self.cache.write() {
            Ok(guard) => Some(guard),
            Err(error) => {
                tracing::error!("Cache write lock poisoned: {}", error);
                None
            }
        }
    }

    fn lock_write_key_order(
        &self,
    ) -> Option<std::sync::RwLockWriteGuard<'_, Vec<(String, String)>>> {
        match self.key_order.write() {
            Ok(guard) => Some(guard),
            Err(error) => {
                tracing::error!("Cache key_order lock poisoned: {}", error);
                None
            }
        }
    }
}

#[async_trait]
impl BaseCache for InMemoryCache {
    fn lookup(&self, prompt: &str, llm_string: &str) -> Option<CacheReturnValue> {
        let cache = self.lock_read_cache()?;
        cache
            .get(&(prompt.to_string(), llm_string.to_string()))
            .cloned()
    }

    fn update(&self, prompt: &str, llm_string: &str, return_val: CacheReturnValue) {
        let key = (prompt.to_string(), llm_string.to_string());
        let Some(mut cache) = self.lock_write_cache() else {
            return;
        };
        let Some(mut key_order) = self.lock_write_key_order() else {
            return;
        };

        if cache.contains_key(&key) {
            key_order.retain(|k| k != &key);
        } else if let Some(maxsize) = self.maxsize
            && cache.len() >= maxsize
            && let Some(oldest_key) = key_order.first().cloned()
        {
            cache.remove(&oldest_key);
            key_order.remove(0);
        }

        cache.insert(key.clone(), return_val);
        key_order.push(key);
    }

    fn clear(&self) {
        let Some(mut cache) = self.lock_write_cache() else {
            return;
        };
        let Some(mut key_order) = self.lock_write_key_order() else {
            return;
        };
        cache.clear();
        key_order.clear();
    }

    async fn alookup(&self, prompt: &str, llm_string: &str) -> Option<CacheReturnValue> {
        self.lookup(prompt, llm_string)
    }

    async fn aupdate(&self, prompt: &str, llm_string: &str, return_val: CacheReturnValue) {
        self.update(prompt, llm_string, return_val);
    }

    async fn aclear(&self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outputs::Generation;

    #[test]
    fn test_in_memory_cache_new() {
        let cache = InMemoryCache::new(None).unwrap();
        assert!(cache.lookup("prompt", "llm").is_none());
    }

    #[test]
    fn test_in_memory_cache_unbounded() {
        let cache = InMemoryCache::unbounded();
        assert!(cache.lookup("prompt", "llm").is_none());
    }

    #[test]
    fn test_in_memory_cache_default() {
        let cache = InMemoryCache::default();
        assert!(cache.lookup("prompt", "llm").is_none());
    }

    #[test]
    fn test_in_memory_cache_zero_maxsize() {
        let result = InMemoryCache::new(Some(0));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("maxsize must be greater than 0"));
    }

    #[test]
    fn test_in_memory_cache_lookup_miss() {
        let cache = InMemoryCache::new(None).unwrap();
        let result = cache.lookup("prompt", "llm_string");
        assert!(result.is_none());
    }

    #[test]
    fn test_in_memory_cache_update_and_lookup() {
        let cache = InMemoryCache::new(None).unwrap();
        let generations = vec![Generation::new("Hello, world!")];

        cache.update("prompt", "llm_string", generations.clone());

        let result = cache.lookup("prompt", "llm_string");
        assert!(result.is_some());
        let cached = result.unwrap();
        assert_eq!(cached.len(), 1);
        assert_eq!(cached[0].text, "Hello, world!");
    }

    #[test]
    fn test_in_memory_cache_clear() {
        let cache = InMemoryCache::new(None).unwrap();
        let generations = vec![Generation::new("Hello")];

        cache.update("prompt1", "llm", generations.clone());
        cache.update("prompt2", "llm", generations.clone());

        assert!(cache.lookup("prompt1", "llm").is_some());
        assert!(cache.lookup("prompt2", "llm").is_some());

        cache.clear();

        assert!(cache.lookup("prompt1", "llm").is_none());
        assert!(cache.lookup("prompt2", "llm").is_none());
    }

    #[test]
    fn test_in_memory_cache_maxsize() {
        let cache = InMemoryCache::new(Some(2)).unwrap();

        cache.update("prompt1", "llm", vec![Generation::new("1")]);
        cache.update("prompt2", "llm", vec![Generation::new("2")]);

        assert!(cache.lookup("prompt1", "llm").is_some());
        assert!(cache.lookup("prompt2", "llm").is_some());

        cache.update("prompt3", "llm", vec![Generation::new("3")]);

        assert!(cache.lookup("prompt1", "llm").is_none()); // Evicted
        assert!(cache.lookup("prompt2", "llm").is_some());
        assert!(cache.lookup("prompt3", "llm").is_some());
    }

    #[test]
    fn test_in_memory_cache_update_existing_key() {
        let cache = InMemoryCache::new(None).unwrap();

        cache.update("prompt", "llm", vec![Generation::new("first")]);
        let result = cache.lookup("prompt", "llm").unwrap();
        assert_eq!(result[0].text, "first");

        cache.update("prompt", "llm", vec![Generation::new("second")]);
        let result = cache.lookup("prompt", "llm").unwrap();
        assert_eq!(result[0].text, "second");
    }

    #[test]
    fn test_in_memory_cache_different_llm_strings() {
        let cache = InMemoryCache::new(None).unwrap();

        cache.update("prompt", "llm1", vec![Generation::new("from llm1")]);
        cache.update("prompt", "llm2", vec![Generation::new("from llm2")]);

        let result1 = cache.lookup("prompt", "llm1").unwrap();
        assert_eq!(result1[0].text, "from llm1");

        let result2 = cache.lookup("prompt", "llm2").unwrap();
        assert_eq!(result2[0].text, "from llm2");
    }

    #[tokio::test]
    async fn test_in_memory_cache_alookup() {
        let cache = InMemoryCache::new(None).unwrap();
        let generations = vec![Generation::new("async test")];

        cache.update("prompt", "llm", generations);

        let result = cache.alookup("prompt", "llm").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap()[0].text, "async test");
    }

    #[tokio::test]
    async fn test_in_memory_cache_aupdate() {
        let cache = InMemoryCache::new(None).unwrap();
        let generations = vec![Generation::new("async update")];

        cache.aupdate("prompt", "llm", generations).await;

        let result = cache.lookup("prompt", "llm");
        assert!(result.is_some());
        assert_eq!(result.unwrap()[0].text, "async update");
    }

    #[tokio::test]
    async fn test_in_memory_cache_aclear() {
        let cache = InMemoryCache::new(None).unwrap();

        cache.update("prompt", "llm", vec![Generation::new("test")]);
        assert!(cache.lookup("prompt", "llm").is_some());

        cache.aclear().await;
        assert!(cache.lookup("prompt", "llm").is_none());
    }

    #[test]
    fn test_in_memory_cache_maxsize_update_refreshes_position() {
        let cache = InMemoryCache::new(Some(2)).unwrap();

        cache.update("prompt1", "llm", vec![Generation::new("1")]);
        cache.update("prompt2", "llm", vec![Generation::new("2")]);

        cache.update("prompt1", "llm", vec![Generation::new("1 updated")]);

        cache.update("prompt3", "llm", vec![Generation::new("3")]);

        assert!(cache.lookup("prompt1", "llm").is_some()); // Still present
        assert!(cache.lookup("prompt2", "llm").is_none()); // Evicted
        assert!(cache.lookup("prompt3", "llm").is_some()); // New
    }

    #[test]
    fn test_in_memory_cache_multiple_generations() {
        let cache = InMemoryCache::new(None).unwrap();
        let generations = vec![
            Generation::new("First generation"),
            Generation::new("Second generation"),
            Generation::new("Third generation"),
        ];

        cache.update("prompt", "llm", generations);

        let result = cache.lookup("prompt", "llm").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].text, "First generation");
        assert_eq!(result[1].text, "Second generation");
        assert_eq!(result[2].text, "Third generation");
    }
}
