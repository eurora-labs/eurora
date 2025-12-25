//! Memory stream for communication between async tasks.
//!
//! This module provides a way to communicate between two async tasks using channels.
//! The writer and reader can be in the same task or different tasks.
//! Mirrors `langchain_core.tracers.memory_stream`.

use std::sync::Arc;
use tokio::sync::mpsc;

/// A sender for the memory stream.
#[derive(Debug)]
pub struct SendStream<T> {
    sender: mpsc::UnboundedSender<Option<T>>,
}

impl<T> SendStream<T> {
    /// Send an item to the stream.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to send.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the item was sent successfully, `Err` if the receiver was dropped.
    pub async fn send(&self, item: T) -> Result<(), mpsc::error::SendError<Option<T>>> {
        self.send_nowait(item)
    }

    /// Send an item to the stream without waiting.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to send.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the item was sent successfully, `Err` if the receiver was dropped.
    pub fn send_nowait(&self, item: T) -> Result<(), mpsc::error::SendError<Option<T>>> {
        self.sender.send(Some(item))
    }

    /// Close the stream.
    pub async fn aclose(&self) -> Result<(), mpsc::error::SendError<Option<T>>> {
        self.close()
    }

    /// Close the stream.
    pub fn close(&self) -> Result<(), mpsc::error::SendError<Option<T>>> {
        self.sender.send(None)
    }
}

impl<T> Clone for SendStream<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

/// A receiver for the memory stream.
#[derive(Debug)]
pub struct ReceiveStream<T> {
    receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Option<T>>>>,
    is_closed: Arc<std::sync::atomic::AtomicBool>,
}

impl<T> ReceiveStream<T> {
    /// Check if the stream is closed.
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl<T: Send + 'static> ReceiveStream<T> {
    /// Create an async iterator over the stream.
    pub fn into_stream(self) -> impl futures::Stream<Item = T> {
        futures::stream::unfold(self, |state| async move {
            if state.is_closed() {
                return None;
            }

            let mut receiver = state.receiver.lock().await;
            match receiver.recv().await {
                Some(Some(item)) => {
                    drop(receiver);
                    Some((item, state))
                }
                Some(None) | None => {
                    state
                        .is_closed
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    None
                }
            }
        })
    }
}

/// A memory stream for communication between async tasks.
///
/// This stream uses unbounded channels to communicate between tasks.
/// It is designed for single producer, single consumer scenarios.
#[derive(Debug)]
pub struct MemoryStream<T> {
    sender: mpsc::UnboundedSender<Option<T>>,
    receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Option<T>>>>,
}

impl<T> MemoryStream<T> {
    /// Create a new memory stream.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }

    /// Get a sender for the stream.
    pub fn get_send_stream(&self) -> SendStream<T> {
        SendStream {
            sender: self.sender.clone(),
        }
    }

    /// Get a receiver for the stream.
    pub fn get_receive_stream(&self) -> ReceiveStream<T> {
        ReceiveStream {
            receiver: self.receiver.clone(),
            is_closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl<T> Default for MemoryStream<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A bounded memory stream with a maximum capacity.
#[derive(Debug)]
pub struct BoundedMemoryStream<T> {
    sender: mpsc::Sender<Option<T>>,
    receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<Option<T>>>>,
}

impl<T> BoundedMemoryStream<T> {
    /// Create a new bounded memory stream.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The maximum number of items the stream can hold.
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = mpsc::channel(capacity);
        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }

    /// Get a sender for the stream.
    pub fn get_send_stream(&self) -> BoundedSendStream<T> {
        BoundedSendStream {
            sender: self.sender.clone(),
        }
    }

    /// Get a receiver for the stream.
    pub fn get_receive_stream(&self) -> BoundedReceiveStream<T> {
        BoundedReceiveStream {
            receiver: self.receiver.clone(),
            is_closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

/// A bounded sender for the memory stream.
#[derive(Debug, Clone)]
pub struct BoundedSendStream<T> {
    sender: mpsc::Sender<Option<T>>,
}

impl<T> BoundedSendStream<T> {
    /// Send an item to the stream.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to send.
    pub async fn send(&self, item: T) -> Result<(), mpsc::error::SendError<Option<T>>> {
        self.sender.send(Some(item)).await
    }

    /// Try to send an item without waiting.
    pub fn try_send(&self, item: T) -> Result<(), mpsc::error::TrySendError<Option<T>>> {
        self.sender.try_send(Some(item))
    }

    /// Close the stream.
    pub async fn close(&self) -> Result<(), mpsc::error::SendError<Option<T>>> {
        self.sender.send(None).await
    }
}

/// A bounded receiver for the memory stream.
#[derive(Debug)]
pub struct BoundedReceiveStream<T> {
    receiver: Arc<tokio::sync::Mutex<mpsc::Receiver<Option<T>>>>,
    is_closed: Arc<std::sync::atomic::AtomicBool>,
}

impl<T> BoundedReceiveStream<T> {
    /// Check if the stream is closed.
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl<T: Send + 'static> BoundedReceiveStream<T> {
    /// Create an async iterator over the stream.
    pub fn into_stream(self) -> impl futures::Stream<Item = T> {
        futures::stream::unfold(self, |state| async move {
            if state.is_closed() {
                return None;
            }

            let mut receiver = state.receiver.lock().await;
            match receiver.recv().await {
                Some(Some(item)) => {
                    drop(receiver);
                    Some((item, state))
                }
                Some(None) | None => {
                    state
                        .is_closed
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                    None
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::pin::pin;

    #[tokio::test]
    async fn test_memory_stream_basic() {
        let stream = MemoryStream::<i32>::new();
        let sender = stream.get_send_stream();
        let receiver = stream.get_receive_stream();

        sender.send_nowait(1).unwrap();
        sender.send_nowait(2).unwrap();
        sender.send_nowait(3).unwrap();
        sender.close().unwrap();

        let mut results = Vec::new();
        let mut stream = pin!(receiver.into_stream());
        while let Some(item) = stream.next().await {
            results.push(item);
        }

        assert_eq!(results, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_memory_stream_async_send() {
        let stream = MemoryStream::<String>::new();
        let sender = stream.get_send_stream();
        let receiver = stream.get_receive_stream();

        sender.send("hello".to_string()).await.unwrap();
        sender.send("world".to_string()).await.unwrap();
        sender.aclose().await.unwrap();

        let mut results = Vec::new();
        let mut stream = pin!(receiver.into_stream());
        while let Some(item) = stream.next().await {
            results.push(item);
        }

        assert_eq!(results, vec!["hello".to_string(), "world".to_string()]);
    }

    #[tokio::test]
    async fn test_bounded_memory_stream() {
        let stream = BoundedMemoryStream::<i32>::new(10);
        let sender = stream.get_send_stream();
        let receiver = stream.get_receive_stream();

        sender.send(1).await.unwrap();
        sender.send(2).await.unwrap();
        sender.close().await.unwrap();

        let mut results = Vec::new();
        let mut stream = pin!(receiver.into_stream());
        while let Some(item) = stream.next().await {
            results.push(item);
        }

        assert_eq!(results, vec![1, 2]);
    }

    #[tokio::test]
    async fn test_send_stream_clone() {
        let stream = MemoryStream::<i32>::new();
        let sender1 = stream.get_send_stream();
        let sender2 = sender1.clone();
        let receiver = stream.get_receive_stream();

        sender1.send_nowait(1).unwrap();
        sender2.send_nowait(2).unwrap();
        sender1.close().unwrap();

        let mut results = Vec::new();
        let mut stream = pin!(receiver.into_stream());
        while let Some(item) = stream.next().await {
            results.push(item);
        }

        assert_eq!(results, vec![1, 2]);
    }
}
