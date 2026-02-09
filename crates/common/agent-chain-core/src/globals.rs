//! Global values and configuration that apply to all of LangChain.

use std::sync::{Arc, RwLock};

use crate::caches::BaseCache;

static LLM_CACHE: RwLock<Option<Arc<dyn BaseCache>>> = RwLock::new(None);

/// Set a new LLM cache, overwriting the previous value, if any.
///
/// # Arguments
///
/// * `value` - The new LLM cache to use. If `None`, the LLM cache is disabled.
pub fn set_llm_cache(value: Option<Arc<dyn BaseCache>>) {
    let mut cache = LLM_CACHE.write().expect("lock poisoned");
    *cache = value;
}

/// Get the value of the `llm_cache` global setting.
///
/// # Returns
///
/// The value of the `llm_cache` global setting.
pub fn get_llm_cache() -> Option<Arc<dyn BaseCache>> {
    let cache = LLM_CACHE.read().expect("lock poisoned");
    cache.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caches::InMemoryCache;

    #[test]
    fn test_llm_cache_default() {
        set_llm_cache(None);
        assert!(get_llm_cache().is_none());
    }

    #[test]
    fn test_set_and_get_llm_cache() {
        let cache = Arc::new(InMemoryCache::unbounded());
        set_llm_cache(Some(cache.clone()));

        let retrieved = get_llm_cache();
        assert!(retrieved.is_some());

        set_llm_cache(None);
        assert!(get_llm_cache().is_none());
    }
}
