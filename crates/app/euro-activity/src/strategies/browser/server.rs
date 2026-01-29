use std::pin::Pin;

use tokio::sync::mpsc;
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

use super::{
    BrowserStrategy,
    proto::{Frame, browser_bridge_server::BrowserBridge},
};

#[tonic::async_trait]
impl BrowserBridge for BrowserStrategy {
    type OpenStream = Pin<Box<dyn Stream<Item = Result<Frame, Status>> + Send + 'static>>;

    async fn open(
        &self,
        request: Request<tonic::Streaming<Frame>>,
    ) -> Result<Response<Self::OpenStream>, Status> {
        let mut inbound = request.into_inner();
        let registry = self.registry.clone();

        let (tx_to_client, rx_to_client) = mpsc::channel::<Result<Frame, Status>>(32);

        todo!()
    }
}
