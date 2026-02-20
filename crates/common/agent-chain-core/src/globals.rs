use std::sync::{Arc, RwLock};

use crate::caches::BaseCache;

static VERBOSE: RwLock<bool> = RwLock::new(false);
static DEBUG: RwLock<bool> = RwLock::new(false);
static LLM_CACHE: RwLock<Option<Arc<dyn BaseCache>>> = RwLock::new(None);

pub fn set_verbose(value: bool) {
    if let Ok(mut verbose) = VERBOSE.write() {
        *verbose = value;
    } else {
        tracing::error!("VERBOSE lock poisoned");
    }
}

pub fn get_verbose() -> bool {
    VERBOSE.read().map(|v| *v).unwrap_or(false)
}

pub fn set_debug(value: bool) {
    if let Ok(mut debug) = DEBUG.write() {
        *debug = value;
    } else {
        tracing::error!("DEBUG lock poisoned");
    }
}

pub fn get_debug() -> bool {
    DEBUG.read().map(|d| *d).unwrap_or(false)
}

pub fn set_llm_cache(value: Option<Arc<dyn BaseCache>>) {
    if let Ok(mut cache) = LLM_CACHE.write() {
        *cache = value;
    } else {
        tracing::error!("LLM_CACHE lock poisoned");
    }
}

pub fn get_llm_cache() -> Option<Arc<dyn BaseCache>> {
    LLM_CACHE.read().ok().and_then(|cache| cache.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caches::InMemoryCache;

    #[test]
    fn test_verbose_default() {
        set_verbose(false);
        assert!(!get_verbose());
    }

    #[test]
    fn test_set_and_get_verbose() {
        set_verbose(true);
        assert!(get_verbose());
        set_verbose(false);
        assert!(!get_verbose());
    }

    #[test]
    fn test_debug_default() {
        set_debug(false);
        assert!(!get_debug());
    }

    #[test]
    fn test_set_and_get_debug() {
        set_debug(true);
        assert!(get_debug());
        set_debug(false);
        assert!(!get_debug());
    }

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
