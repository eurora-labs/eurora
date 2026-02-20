use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct NoLock;

impl NoLock {
    pub fn new() -> Self {
        Self
    }

    pub fn lock(&self) -> NoLockGuard {
        NoLockGuard
    }
}

impl Default for NoLock {
    fn default() -> Self {
        Self::new()
    }
}

pub struct NoLockGuard;

pub fn batch_iterate<T, I>(size: Option<usize>, iterable: I) -> BatchIterator<T, I::IntoIter>
where
    I: IntoIterator<Item = T>,
{
    BatchIterator {
        size,
        iter: iterable.into_iter(),
    }
}

pub struct BatchIterator<T, I>
where
    I: Iterator<Item = T>,
{
    size: Option<usize>,
    iter: I,
}

impl<T, I> Iterator for BatchIterator<T, I>
where
    I: Iterator<Item = T>,
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.size {
            Some(size) => {
                let batch: Vec<T> = self.iter.by_ref().take(size).collect();
                if batch.is_empty() { None } else { Some(batch) }
            }
            None => {
                let batch: Vec<T> = self.iter.by_ref().collect();
                if batch.is_empty() { None } else { Some(batch) }
            }
        }
    }
}

pub fn tee<T, I>(iterable: I, n: usize) -> Tee<T>
where
    T: Clone,
    I: IntoIterator<Item = T>,
    <I as IntoIterator>::IntoIter: Send + 'static,
{
    Tee::new(iterable, n)
}

pub struct Tee<T> {
    source: Arc<Mutex<TeeSource<T>>>,
    children: Vec<TeeChild<T>>,
}

struct TeeSource<T> {
    iter: Box<dyn Iterator<Item = T> + Send>,
    buffers: Vec<VecDeque<T>>,
}

impl<T> Tee<T>
where
    T: Clone,
{
    pub fn new<I>(iterable: I, n: usize) -> Self
    where
        I: IntoIterator<Item = T>,
        <I as IntoIterator>::IntoIter: Send + 'static,
    {
        let iter: Box<dyn Iterator<Item = T> + Send> = Box::new(iterable.into_iter());
        let buffers: Vec<VecDeque<T>> = (0..n).map(|_| VecDeque::new()).collect();

        let source = Arc::new(Mutex::new(TeeSource { iter, buffers }));

        let children: Vec<TeeChild<T>> = (0..n)
            .map(|index| TeeChild {
                source: Arc::clone(&source),
                index,
            })
            .collect();

        Self { source, children }
    }

    pub fn len(&self) -> usize {
        self.children.len()
    }

    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<TeeChild<T>> {
        if index < self.children.len() {
            Some(TeeChild {
                source: Arc::clone(&self.source),
                index,
            })
        } else {
            None
        }
    }

    pub fn into_children(self) -> Vec<TeeChild<T>> {
        self.children
    }
}

pub struct TeeChild<T> {
    source: Arc<Mutex<TeeSource<T>>>,
    index: usize,
}

impl<T> Clone for TeeChild<T> {
    fn clone(&self) -> Self {
        Self {
            source: Arc::clone(&self.source),
            index: self.index,
        }
    }
}

impl<T: Clone> Iterator for TeeChild<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let mut source = self.source.lock().ok()?;

        if let Some(item) = source.buffers.get_mut(self.index)?.pop_front() {
            return Some(item);
        }

        if let Some(item) = source.iter.next() {
            for (i, buffer) in source.buffers.iter_mut().enumerate() {
                if i != self.index {
                    buffer.push_back(item.clone());
                }
            }
            Some(item)
        } else {
            None
        }
    }
}

pub fn safetee<T, I>(iterable: I, n: usize) -> Tee<T>
where
    T: Clone,
    I: IntoIterator<Item = T>,
    <I as IntoIterator>::IntoIter: Send + 'static,
{
    tee(iterable, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_iterate() {
        let items = vec![1, 2, 3, 4, 5];
        let batches: Vec<Vec<i32>> = batch_iterate(Some(2), items).collect();
        assert_eq!(batches, vec![vec![1, 2], vec![3, 4], vec![5]]);
    }

    #[test]
    fn test_batch_iterate_exact() {
        let items = vec![1, 2, 3, 4];
        let batches: Vec<Vec<i32>> = batch_iterate(Some(2), items).collect();
        assert_eq!(batches, vec![vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn test_batch_iterate_empty() {
        let items: Vec<i32> = vec![];
        let batches: Vec<Vec<i32>> = batch_iterate(Some(2), items).collect();
        assert!(batches.is_empty());
    }

    #[test]
    fn test_batch_iterate_none_size() {
        let items = vec![1, 2, 3, 4, 5];
        let batches: Vec<Vec<i32>> = batch_iterate(None, items).collect();
        assert_eq!(batches, vec![vec![1, 2, 3, 4, 5]]);
    }

    #[test]
    fn test_tee_basic() {
        let items = vec![1, 2, 3];
        let t = tee(items, 2);

        assert_eq!(t.len(), 2);

        let children = t.into_children();
        let first: Vec<i32> = children[0].clone().collect();
        let second: Vec<i32> = children[1].clone().collect();

        assert_eq!(first, vec![1, 2, 3]);
        assert_eq!(second, vec![1, 2, 3]);
    }

    #[test]
    fn test_tee_interleaved() {
        let items = vec![1, 2, 3];
        let t = tee(items, 2);
        let mut children = t.into_children();

        assert_eq!(children[0].next(), Some(1));
        assert_eq!(children[1].next(), Some(1));
        assert_eq!(children[0].next(), Some(2));
        assert_eq!(children[0].next(), Some(3));
        assert_eq!(children[1].next(), Some(2));
        assert_eq!(children[1].next(), Some(3));
        assert_eq!(children[0].next(), None);
        assert_eq!(children[1].next(), None);
    }

    #[test]
    fn test_no_lock() {
        let lock = NoLock::new();
        let _guard = lock.lock();
    }
}
