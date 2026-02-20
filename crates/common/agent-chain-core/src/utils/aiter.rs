use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::stream::Stream;
use tokio::sync::Mutex;

pub struct NoLock;

impl NoLock {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoLock {
    fn default() -> Self {
        Self::new()
    }
}

struct TeeShared<T> {
    source: Pin<Box<dyn Stream<Item = T> + Send>>,
    buffers: Vec<VecDeque<T>>,
    exhausted: bool,
}

pub fn atee<T>(source: Pin<Box<dyn Stream<Item = T> + Send>>, n: usize) -> Vec<TeePeer<T>>
where
    T: Clone + Send + 'static,
{
    let shared = Arc::new(Mutex::new(TeeShared {
        source,
        buffers: (0..n).map(|_| VecDeque::new()).collect(),
        exhausted: false,
    }));

    (0..n)
        .map(|index| TeePeer {
            shared: Arc::clone(&shared),
            index,
        })
        .collect()
}

pub struct TeePeer<T> {
    shared: Arc<Mutex<TeeShared<T>>>,
    index: usize,
}

impl<T> Stream for TeePeer<T>
where
    T: Clone + Send + 'static,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let mut guard = match this.shared.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                context.waker().wake_by_ref();
                return Poll::Pending;
            }
        };

        if let Some(item) = guard.buffers[this.index].pop_front() {
            return Poll::Ready(Some(item));
        }

        if guard.exhausted {
            return Poll::Ready(None);
        }

        match guard.source.as_mut().poll_next(context) {
            Poll::Ready(Some(item)) => {
                for (i, buffer) in guard.buffers.iter_mut().enumerate() {
                    if i != this.index {
                        buffer.push_back(item.clone());
                    }
                }
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => {
                guard.exhausted = true;
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct AClosing<S> {
    stream: Option<S>,
}

impl<S> AClosing<S> {
    pub fn new(stream: S) -> Self {
        Self {
            stream: Some(stream),
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut S> {
        self.stream.as_mut()
    }

    pub fn into_inner(mut self) -> Option<S> {
        self.stream.take()
    }
}

impl<S> Drop for AClosing<S> {
    fn drop(&mut self) {
        self.stream.take();
    }
}

pub fn abatch_iterate<S, T>(size: usize, source: S) -> ABatchIterator<S>
where
    S: Stream<Item = T>,
{
    ABatchIterator {
        source,
        size,
        batch: Vec::with_capacity(size),
    }
}

pub struct ABatchIterator<S: Stream> {
    source: S,
    size: usize,
    batch: Vec<S::Item>,
}

impl<S> Stream for ABatchIterator<S>
where
    S::Item: Unpin,
    S: Stream + Unpin,
{
    type Item = Vec<S::Item>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match Pin::new(&mut this.source).poll_next(context) {
                Poll::Ready(Some(item)) => {
                    this.batch.push(item);
                    if this.batch.len() >= this.size {
                        let batch =
                            std::mem::replace(&mut this.batch, Vec::with_capacity(this.size));
                        return Poll::Ready(Some(batch));
                    }
                }
                Poll::Ready(None) => {
                    if this.batch.is_empty() {
                        return Poll::Ready(None);
                    }
                    let batch = std::mem::replace(&mut this.batch, Vec::with_capacity(this.size));
                    return Poll::Ready(Some(batch));
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use futures::stream;

    #[tokio::test]
    async fn test_abatch_iterate() {
        let source = stream::iter(vec![1, 2, 3, 4, 5]);
        let batches: Vec<Vec<i32>> = abatch_iterate(2, source).collect().await;
        assert_eq!(batches, vec![vec![1, 2], vec![3, 4], vec![5]]);
    }

    #[tokio::test]
    async fn test_abatch_iterate_exact() {
        let source = stream::iter(vec![1, 2, 3, 4]);
        let batches: Vec<Vec<i32>> = abatch_iterate(2, source).collect().await;
        assert_eq!(batches, vec![vec![1, 2], vec![3, 4]]);
    }

    #[tokio::test]
    async fn test_abatch_iterate_empty() {
        let source = stream::iter(Vec::<i32>::new());
        let batches: Vec<Vec<i32>> = abatch_iterate(2, source).collect().await;
        assert!(batches.is_empty());
    }

    #[tokio::test]
    async fn test_atee_basic() {
        let source = stream::iter(vec![1, 2, 3]);
        let peers = atee(Box::pin(source), 2);
        let mut peer0 = peers.into_iter();
        let first = peer0.next().unwrap();
        let second = peer0.next().unwrap();

        let result0: Vec<i32> = first.collect().await;
        let result1: Vec<i32> = second.collect().await;

        assert_eq!(result0, vec![1, 2, 3]);
        assert_eq!(result1, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_aclosing() {
        let source = stream::iter(vec![1, 2, 3]);
        let mut guard = AClosing::new(source);
        let stream = guard.get_mut().unwrap();
        let items: Vec<i32> = stream.collect().await;
        assert_eq!(items, vec![1, 2, 3]);
    }
}
