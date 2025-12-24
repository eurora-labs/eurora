//! Utilities for working with iterators.
//!
//! Adapted from langchain_core/utils/iter.py

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// A dummy lock that provides the proper interface but no protection.
///
/// This is used as a default lock when no synchronization is needed.
pub struct NoLock;

impl NoLock {
    /// Create a new NoLock.
    pub fn new() -> Self {
        Self
    }

    /// Acquire the lock (no-op for NoLock).
    pub fn lock(&self) -> NoLockGuard {
        NoLockGuard
    }
}

impl Default for NoLock {
    fn default() -> Self {
        Self::new()
    }
}

/// A guard for NoLock that does nothing.
pub struct NoLockGuard;

/// Utility batching function for iterables.
///
/// # Arguments
///
/// * `size` - The size of each batch. If `None`, returns a single batch with all items.
/// * `iterable` - The iterable to batch.
///
/// # Returns
///
/// An iterator over batches.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::iter::batch_iterate;
///
/// let items = vec![1, 2, 3, 4, 5];
/// let batches: Vec<Vec<i32>> = batch_iterate(Some(2), items).collect();
/// assert_eq!(batches, vec![vec![1, 2], vec![3, 4], vec![5]]);
/// ```
pub fn batch_iterate<T, I>(size: Option<usize>, iterable: I) -> BatchIterator<T, I::IntoIter>
where
    I: IntoIterator<Item = T>,
{
    BatchIterator {
        size,
        iter: iterable.into_iter(),
    }
}

/// An iterator that yields batches from an underlying iterator.
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
                if batch.is_empty() {
                    None
                } else {
                    Some(batch)
                }
            }
            None => {
                let batch: Vec<T> = self.iter.by_ref().collect();
                if batch.is_empty() {
                    None
                } else {
                    Some(batch)
                }
            }
        }
    }
}

/// Create `n` separate iterators over an iterable.
///
/// This splits a single iterable into multiple iterators, each providing
/// the same items in the same order. All child iterators may advance separately
/// but share the same items from the source -- when the most advanced iterator
/// retrieves an item, it is buffered until the least advanced iterator has
/// yielded it as well.
///
/// # Arguments
///
/// * `iterable` - The iterable to split.
/// * `n` - The number of iterators to create.
///
/// # Returns
///
/// A `Tee` containing `n` child iterators.
///
/// # Example
///
/// ```
/// use agent_chain_core::utils::iter::tee;
///
/// let items = vec![1, 2, 3];
/// let t = tee(items, 2);
/// // Now t contains 2 iterators that will each yield 1, 2, 3
/// ```
pub fn tee<T, I>(iterable: I, n: usize) -> Tee<T>
where
    T: Clone,
    I: IntoIterator<Item = T>,
    <I as IntoIterator>::IntoIter: Send + 'static,
{
    Tee::new(iterable, n)
}

/// A tee implementation that creates multiple iterators from a single source.
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
    /// Create a new Tee with `n` child iterators.
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

    /// Get the number of child iterators.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Check if the tee is empty.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Get a child iterator by index.
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

    /// Consume the tee and return all child iterators.
    pub fn into_children(self) -> Vec<TeeChild<T>> {
        self.children
    }
}

/// A child iterator of a Tee.
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

/// A safe version of tee that ensures thread safety.
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