use futures::Stream;
use std::pin::Pin;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::info;

pub use crate::types::proto::{
    Frame,
    browser_bridge_client::BrowserBridgeClient,
    browser_bridge_server::{BrowserBridge, BrowserBridgeServer},
};

#[derive(Clone)]
pub struct BrowserBridgeService {
    /// Frames going to Chrome (native writer)
    pub chrome_tx: mpsc::UnboundedSender<Frame>,
    /// Frames coming from Chrome (broadcast to all gRPC clients)
    pub chrome_from_tx: broadcast::Sender<Frame>,
}

#[tonic::async_trait]
impl BrowserBridge for BrowserBridgeService {
    type OpenStream = Pin<Box<dyn Stream<Item = Result<Frame, Status>> + Send + 'static>>;

    async fn open(
        &self,
        request: Request<tonic::Streaming<Frame>>,
    ) -> Result<Response<Self::OpenStream>, Status> {
        let mut inbound = request.into_inner();

        // Client-specific outbound stream
        let (tx_to_client, rx_to_client) = mpsc::channel::<Result<Frame, Status>>(32);

        // Subscribe to Chrome → host broadcast
        let mut chrome_from_rx = self.chrome_from_tx.subscribe();
        let chrome_tx = self.chrome_tx.clone();

        // Task: client → host → Chrome
        tokio::spawn(async move {
            info!("gRPC client connected, starting forward task(client → Chrome)");
            loop {
                match inbound.message().await {
                    Ok(Some(frame)) => {
                        info!("Forwarding frame from gRPC client to Chrome: {:?}", frame);
                        if let Err(e) = chrome_tx.send(frame) {
                            info!("Error forwarding frame to Chrome: {e:?}");
                            break;
                        }
                    }
                    Ok(None) => {
                        info!("The gRPC client disconnected");
                        break;
                    }
                    Err(e) => {
                        info!("Error receiving frame from gRPC client: {e:?}");
                        break;
                    }
                }
            }
            info!("Forward task(client → Chrome) completed");
        });

        // Task: Chrome → host → client
        tokio::spawn(async move {
            info!("Starting forward task(Chrome → host → client)");
            loop {
                match chrome_from_rx.recv().await {
                    Ok(frame) => {
                        info!("Forwarding frame from Chrome to gRPC client: {:?}", frame);
                        if let Err(e) = tx_to_client.send(Ok(frame)).await {
                            info!("Error forwarding frame to gRPC client: {e:?}");
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        info!("The gRPC client is lagging behind by {n} frames");
                    }
                    Err(e) => {
                        info!("Error receiving frame from Chrome: {e:?}");
                        break;
                    }
                }
            }
            info!("Forward task(Chrome → host → client) completed");
        });

        let out_stream = ReceiverStream::new(rx_to_client);
        Ok(Response::new(Box::pin(out_stream) as Self::OpenStream))
    }
}
