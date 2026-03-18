use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

pub(crate) struct IconCache {
    map: HashMap<String, Arc<image::RgbaImage>>,
    order: VecDeque<String>,
    capacity: usize,
}

impl IconCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn get(&self, key: &str) -> Option<&Arc<image::RgbaImage>> {
        self.map.get(key)
    }

    pub fn insert(&mut self, key: String, value: Arc<image::RgbaImage>) {
        if self.map.contains_key(&key) {
            return;
        }
        if self.map.len() >= self.capacity
            && let Some(oldest) = self.order.pop_front()
        {
            self.map.remove(&oldest);
        }
        self.order.push_back(key.clone());
        self.map.insert(key, value);
    }
}
