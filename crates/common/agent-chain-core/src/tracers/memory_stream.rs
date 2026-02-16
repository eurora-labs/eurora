use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct SendStream<T> {
    sender: mpsc::UnboundedSender<Option<T>>,
}

impl<T> SendStream<T> {
    pub fn send(&self, item: T) -> Result<(), mpsc::error::SendError<Option<T>>> {
        self.sender.send(Some(item))
    }

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

#[derive(Debug)]
pub struct ReceiveStream<T> {
    receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Option<T>>>>,
    is_closed: Arc<std::sync::atomic::AtomicBool>,
}

impl<T> ReceiveStream<T> {
    pub fn is_closed(&self) -> bool {
        self.is_closed.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl<T: Send + 'static> ReceiveStream<T> {
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

#[derive(Debug)]
pub struct MemoryStream<T> {
    sender: mpsc::UnboundedSender<Option<T>>,
    receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Option<T>>>>,
}

impl<T> MemoryStream<T> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        Self {
            sender,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        }
    }

    pub fn get_send_stream(&self) -> SendStream<T> {
        SendStream {
            sender: self.sender.clone(),
        }
    }

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

        sender.send(1).unwrap();
        sender.send(2).unwrap();
        sender.send(3).unwrap();
        sender.close().unwrap();

        let mut results = Vec::new();
        let mut stream = pin!(receiver.into_stream());
        while let Some(item) = stream.next().await {
            results.push(item);
        }

        assert_eq!(results, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_send_stream_clone() {
        let stream = MemoryStream::<i32>::new();
        let sender1 = stream.get_send_stream();
        let sender2 = sender1.clone();
        let receiver = stream.get_receive_stream();

        sender1.send(1).unwrap();
        sender2.send(2).unwrap();
        sender1.close().unwrap();

        let mut results = Vec::new();
        let mut stream = pin!(receiver.into_stream());
        while let Some(item) = stream.next().await {
            results.push(item);
        }

        assert_eq!(results, vec![1, 2]);
    }
}
