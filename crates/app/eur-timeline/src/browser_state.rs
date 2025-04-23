use anyhow::Result;
use eur_native_messaging::{Channel, TauriIpcClient, create_grpc_ipc_client};
use eur_proto::ipc::{self, StateRequest, StateResponse};
use eur_proto::ipc::{ProtoArticleState, ProtoPdfState, ProtoYoutubeState};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, mpsc};
use tokio_stream::{StreamExt, wrappers::ReceiverStream};
use tonic::Streaming;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserState {
    Youtube(ProtoYoutubeState),
    Article(ProtoArticleState),
    Pdf(ProtoPdfState),
}

impl BrowserState {
    pub fn content_type(&self) -> String {
        match self {
            BrowserState::Youtube(_) => "youtube".to_string(),
            BrowserState::Article(_) => "article".to_string(),
            BrowserState::Pdf(_) => "pdf".to_string(),
        }
    }
    pub fn youtube(self) -> Option<ProtoYoutubeState> {
        match self {
            BrowserState::Youtube(youtube) => Some(youtube),
            _ => None,
        }
    }

    pub fn article(self) -> Option<ProtoArticleState> {
        match self {
            BrowserState::Article(article) => Some(article),
            _ => None,
        }
    }

    pub fn pdf(self) -> Option<ProtoPdfState> {
        match self {
            BrowserState::Pdf(pdf) => Some(pdf),
            _ => None,
        }
    }
}

pub struct BrowserCollector {
    client: Mutex<TauriIpcClient<Channel>>,
    stream: Mutex<Streaming<StateResponse>>,
    request_tx: mpsc::Sender<StateRequest>,
}

impl BrowserCollector {
    /// Create a new BrowserCollector with an established gRPC connection
    /// and a persistent state stream
    pub async fn new() -> Result<Self> {
        let mut client = create_grpc_ipc_client().await?;

        // Create a channel for requests
        let (tx, rx) = mpsc::channel::<StateRequest>(32);
        // Convert receiver to a stream that can be used with gRPC
        let request_stream = ReceiverStream::new(rx);

        // Create a persistent bidirectional stream
        let result = client.get_state_streaming(request_stream).await?;
        let stream = result.into_inner();

        // Send initial request to get first state
        tx.send(StateRequest {}).await?;

        Ok(Self {
            client: Mutex::new(client),
            stream: Mutex::new(stream),
            request_tx: tx,
        })
    }

    /// Collect the current browser state by tapping into the existing stream
    pub async fn collect_state(&mut self) -> Result<Option<BrowserState>> {
        // Send a request to get the latest state
        self.request_tx
            .send(StateRequest {})
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send state request: {}", e))?;

        // Get the response
        let mut stream_lock = self.stream.lock().await;

        // Try to get a message from the stream
        match stream_lock.message().await {
            Ok(Some(state_response)) => match &state_response.state {
                Some(ipc::state_response::State::Youtube(youtube)) => {
                    eprintln!("Collected Youtube state");
                    return Ok(Some(BrowserState::Youtube(youtube.clone())));
                }
                Some(ipc::state_response::State::Article(article)) => {
                    eprintln!("Collected Article state");
                    return Ok(Some(BrowserState::Article(article.clone())));
                }
                Some(ipc::state_response::State::Pdf(pdf)) => {
                    eprintln!("Collected Pdf state");
                    return Ok(Some(BrowserState::Pdf(pdf.clone())));
                }
                _ => {}
            },
            Ok(None) => {
                // Stream ended unexpectedly
                eprintln!("Stream ended unexpectedly, recreating...");
                drop(stream_lock); // Release the lock before creating a new stream
                self.recreate_stream().await?;
            }
            Err(e) => {
                // Error reading from stream
                eprintln!("Error reading from stream: {}, recreating...", e);
                drop(stream_lock);
                self.recreate_stream().await?;
                return Err(anyhow::anyhow!("Stream error: {}", e));
            }
        }

        Ok(None)
    }

    /// Recreate the stream if it has ended
    async fn recreate_stream(&mut self) -> Result<()> {
        eprintln!("Recreating stream");

        // Create a new client
        let mut new_client = create_grpc_ipc_client().await?;

        // Create a new channel for requests
        let (tx, rx) = mpsc::channel::<StateRequest>(32);
        let request_stream = ReceiverStream::new(rx);

        // Create a new persistent bidirectional stream
        let result = new_client.get_state_streaming(request_stream).await?;
        let new_stream = result.into_inner();

        // Update the client
        {
            let mut client_lock = self.client.lock().await;
            *client_lock = new_client;
        }

        // Update the stream
        {
            let mut stream_lock = self.stream.lock().await;
            *stream_lock = new_stream;
        }

        // Send an initial request through the new channel
        tx.send(StateRequest {}).await.map_err(|e| {
            anyhow::anyhow!("Failed to send initial request after recreation: {}", e)
        })?;

        // Update the request_tx
        // NOTE: In a proper implementation, request_tx should be behind a Mutex
        // For now, we're replacing it directly which isn't thread-safe
        // Consider updating the design to make this field a Mutex<mpsc::Sender<StateRequest>>
        self.request_tx = tx;

        Ok(())
    }

    /// Get the raw client if needed for other operations
    pub async fn get_client(&self) -> TauriIpcClient<Channel> {
        self.client.lock().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_browser_collector() -> Result<()> {
        let mut collector = BrowserCollector::new().await?;
        let state = collector.collect_state().await?;

        // Just verify we can collect state without errors
        // The actual state will depend on what's in the browser
        println!("Collected state: {:?}", state);

        // Test multiple calls to collect_state to ensure the stream remains open
        let state2 = collector.collect_state().await?;
        println!("Second collected state: {:?}", state2);

        Ok(())
    }
}
