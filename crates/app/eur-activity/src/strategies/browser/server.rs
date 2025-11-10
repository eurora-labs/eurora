use tonic::{Request, Response, Status};

use super::BrowserStrategy;
use eur_proto::nm_ipc::{
    SwitchActivityRequest, SwitchActivityResponse, native_messaging_ipc_server::NativeMessagingIpc,
};
use tracing::info;

pub const PORT: &str = "1422";

#[tonic::async_trait]
impl NativeMessagingIpc for BrowserStrategy {
    /// Handles the SwitchActivity RPC call
    ///
    /// This method receives a request to switch activity with a URL and optional icon data.
    async fn switch_activity(
        &self,
        request: Request<SwitchActivityRequest>,
    ) -> Result<Response<SwitchActivityResponse>, Status> {
        info!("Received switch activity request");
        let req = request.into_inner();

        // Validate the URL is not empty
        if req.url.is_empty() {
            return Err(Status::invalid_argument("URL cannot be empty"));
        }

        // TODO: Implement actual activity switching logic here
        // This is a placeholder implementation that just acknowledges the request

        // Return success response
        Ok(Response::new(SwitchActivityResponse {}))
    }
}
