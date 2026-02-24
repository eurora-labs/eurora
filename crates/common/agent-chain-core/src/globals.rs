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
