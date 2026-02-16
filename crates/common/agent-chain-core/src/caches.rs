//! Optional caching layer for language models.
//!
//! Distinct from provider-based prompt caching.
//!
//! A cache is useful for two reasons:
//!
//! 1. It can save you money by reducing the number of API calls you make to the LLM
//!    provider if you're often requesting the same completion multiple times.
//! 2. It can speed up your application by reducing the number of API calls you make to the
//!    LLM provider.
//!
//! Mirrors `langchain_core.caches`.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::outputs::Generation;

/// The return type for cache operations - a sequence of Generations.
pub type CacheReturnValue = Vec<Generation>;

/// Interface for a caching layer for LLMs and Chat models.
///
/// The cache interface consists of the following methods:
///
/// - `lookup`: Look up a value based on a prompt and `llm_string`.
/// - `update`: Update the cache based on a prompt and `llm_string`.
/// - `clear`: Clear the cache.
///
/// In addition, the cache interface provides an async version of each method.
///
/// The default implementation of the async methods is to run the synchronous
/// method directly. It's recommended to override the async methods
/// and provide async implementations to avoid unnecessary overhead.
#[async_trait]
pub trait BaseCache: Send + Sync {
    /// Look up based on `prompt` and `llm_string`.
    ///
    /// A cache implementation is expected to generate a key from the 2-tuple
    /// of `prompt` and `llm_string` (e.g., by concatenating them with a delimiter).
    ///
    /// # Arguments
    ///
    /// * `prompt` - A string representation of the prompt.
    ///   In the case of a chat model, the prompt is a non-trivial
    ///   serialization of the prompt into the language model.
    /// * `llm_string` - A string representation of the LLM configuration.
    ///   This is used to capture the invocation parameters of the LLM
    ///   (e.g., model name, temperature, stop tokens, max tokens, etc.).
    ///   These invocation parameters are serialized into a string representation.
    ///
    /// # Returns
    ///
    /// On a cache miss, return `None`. On a cache hit, return the cached value.
    /// The cached value is a list of `Generation` (or subclasses).
    fn lookup(&self, prompt: &str, llm_string: &str) -> Option<CacheReturnValue>;

    /// Update cache based on `prompt` and `llm_string`.
    ///
    /// The prompt and llm_string are used to generate a key for the cache.
    /// The key should match that of the lookup method.
    ///
    /// # Arguments
    ///
    /// * `prompt` - A string representation of the prompt.
    ///   In the case of a chat model, the prompt is a non-trivial
    ///   serialization of the prompt into the language model.
    /// * `llm_string` - A string representation of the LLM configuration.
    ///   This is used to capture the invocation parameters of the LLM
    ///   (e.g., model name, temperature, stop tokens, max tokens, etc.).
    ///   These invocation parameters are serialized into a string representation.
    /// * `return_val` - The value to be cached. The value is a list of `Generation`
    ///   (or subclasses).
    fn update(&self, prompt: &str, llm_string: &str, return_val: CacheReturnValue);

    /// Clear cache that can take additional keyword arguments.
    fn clear(&self);

    /// Async look up based on `prompt` and `llm_string`.
    ///
    /// A cache implementation is expected to generate a key from the 2-tuple
    /// of `prompt` and `llm_string` (e.g., by concatenating them with a delimiter).
    ///
    /// # Arguments
    ///
    /// * `prompt` - A string representation of the prompt.
    ///   In the case of a chat model, the prompt is a non-trivial
    ///   serialization of the prompt into the language model.
    /// * `llm_string` - A string representation of the LLM configuration.
    ///   This is used to capture the invocation parameters of the LLM
    ///   (e.g., model name, temperature, stop tokens, max tokens, etc.).
    ///   These invocation parameters are serialized into a string representation.
    ///
    /// # Returns
    ///
    /// On a cache miss, return `None`. On a cache hit, return the cached value.
    /// The cached value is a list of `Generation` (or subclasses).
    async fn alookup(&self, prompt: &str, llm_string: &str) -> Option<CacheReturnValue> {
        self.lookup(prompt, llm_string)
    }

    /// Async update cache based on `prompt` and `llm_string`.
    ///
    /// The prompt and llm_string are used to generate a key for the cache.
    /// The key should match that of the look up method.
    ///
    /// # Arguments
    ///
    /// * `prompt` - A string representation of the prompt.
    ///   In the case of a chat model, the prompt is a non-trivial
    ///   serialization of the prompt into the language model.
    /// * `llm_string` - A string representation of the LLM configuration.
    ///   This is used to capture the invocation parameters of the LLM
    ///   (e.g., model name, temperature, stop tokens, max tokens, etc.).
    ///   These invocation parameters are serialized into a string representation.
    /// * `return_val` - The value to be cached. The value is a list of `Generation`
    ///   (or subclasses).
    async fn aupdate(&self, prompt: &str, llm_string: &str, return_val: CacheReturnValue) {
        self.update(prompt, llm_string, return_val);
    }

    /// Async clear cache.
    async fn aclear(&self) {
        self.clear();
    }
}

/// Cache that stores things in memory.
#[derive(Debug)]
pub struct InMemoryCache {
    /// The internal cache storage using (prompt, llm_string) as key.
    cache: RwLock<HashMap<(String, String), CacheReturnValue>>,
    /// The maximum number of items to store in the cache.
    /// If `None`, the cache has no maximum size.
    maxsize: Option<usize>,
    /// Order of keys for LRU-style eviction (stores keys in insertion order).
    key_order: RwLock<Vec<(String, String)>>,
}

impl InMemoryCache {
    /// Initialize with empty cache.
    ///
    /// # Arguments
    ///
    /// * `maxsize` - The maximum number of items to store in the cache.
    ///   If `None`, the cache has no maximum size.
    ///   If the cache exceeds the maximum size, the oldest items are removed.
    ///
    /// # Panics
    ///
    /// Panics if `maxsize` is less than or equal to `0`.
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

    /// Create a new InMemoryCache with no maximum size.
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

        // If key already exists, remove it from the order list (it will be added at the end)
        if cache.contains_key(&key) {
            key_order.retain(|k| k != &key);
        } else if let Some(maxsize) = self.maxsize {
            // If at capacity, remove the oldest item
            if cache.len() >= maxsize
                && let Some(oldest_key) = key_order.first().cloned()
            {
                cache.remove(&oldest_key);
                key_order.remove(0);
            }
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

        // Adding third item should evict the first (oldest)
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

        // Update prompt1 - should move it to the end of the queue
        cache.update("prompt1", "llm", vec![Generation::new("1 updated")]);

        // Adding prompt3 should evict prompt2 (now oldest) instead of prompt1
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
