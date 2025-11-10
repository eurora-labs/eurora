use std::{
    io::{self, Write},
    pin::Pin,
    sync::Arc,
};

use anyhow::{Result, anyhow};
use eur_proto::{
    ipc::{MessageRequest, MessageResponse, tauri_ipc_server::TauriIpc},
    nm_ipc::{SwitchActivityRequest, native_messaging_ipc_client::NativeMessagingIpcClient},
};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::transport::Channel;
use tonic::{Request, Response, Status, Streaming};
use tracing::{debug, error, info};

use crate::types::{ChromeMessage, NativeMessage, NativeMetadata};

type IpcResult<T> = Result<Response<T>, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<MessageResponse, Status>> + Send>>;

// Message type for native messaging
#[derive(Clone, Debug)]
pub struct NativeCommand {
    pub command: String,
}

// Request type for internal communication
#[derive(Debug)]
struct NativeMessageRequest {
    message: NativeCommand,
    response_sender: oneshot::Sender<anyhow::Result<NativeMessage>>,
}

#[derive(Clone)]
pub struct TauriIpcServer {
    message_sender: mpsc::Sender<NativeMessageRequest>,
    /// gRPC client for native messaging IPC (stored for future direct use if needed)
    #[allow(dead_code)]
    client: Option<NativeMessagingIpcClient<Channel>>,
}

impl TauriIpcServer {
    pub async fn new() -> (
        Self,
        mpsc::Sender<ChromeMessage>,
        mpsc::Sender<NativeMessage>,
    ) {
        let (tx, rx) = mpsc::channel::<NativeMessageRequest>(32);
        let (native_tx, native_rx) = mpsc::channel::<ChromeMessage>(32);
        let (stdin_tx, stdin_rx) = mpsc::channel::<NativeMessage>(32);

        let client = NativeMessagingIpcClient::connect(format!("http://[::1]:{}", "1422"))
            .await
            .ok();

        // Clone the client for the background task
        let client_for_task = client.clone();

        // Spawn a task to handle the stdio communication
        tokio::spawn(Self::handle_stdio_task(
            rx,
            native_rx,
            stdin_rx,
            client_for_task,
        ));

        (
            Self {
                message_sender: tx,
                client,
            },
            native_tx,
            stdin_tx,
        )
    }

    async fn handle_stdio_task(
        mut request_rx: mpsc::Receiver<NativeMessageRequest>,
        mut native_rx: mpsc::Receiver<ChromeMessage>,
        mut stdin_rx: mpsc::Receiver<NativeMessage>,
        mut client: Option<NativeMessagingIpcClient<Channel>>,
    ) {
        let stdout = io::stdout();
        let stdout_mutex = Arc::new(tokio::sync::Mutex::new(stdout));

        // Map to track pending requests by command
        let pending_requests: Arc<
            tokio::sync::Mutex<
                std::collections::HashMap<String, oneshot::Sender<anyhow::Result<NativeMessage>>>,
            >,
        > = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

        let pending_requests_clone = pending_requests.clone();

        loop {
            tokio::select! {
                Some(request) = request_rx.recv() => {
                    let NativeMessageRequest { message, response_sender } = request;
                    let command = message.command.clone();

                    // Create the message to send to Chrome extension
                    let message_value = json!({
                        "command": command
                    });

                    // Store the response sender
                    pending_requests_clone.lock().await.insert(command.clone(), response_sender);

                    // Acquire mutex for stdout
                    let stdout_guard = stdout_mutex.lock().await;

                    // Write command to stdout
                    if let Err(e) = write_message(&*stdout_guard, &message_value) {
                        error!("Failed to write command to stdout: {}", e);
                        // Remove from pending and send error
                        if let Some(sender) = pending_requests_clone.lock().await.remove(&command) {
                            let _ = sender.send(Err(anyhow!("Write error: {}", e)));
                        }
                    }

                    drop(stdout_guard);
                },
                Some(native_message) = stdin_rx.recv() => {
                    debug!("Received native response message");

                    // Match response to pending request based on message type
                    // For now, we'll use a simple FIFO approach - take the first pending request
                    let mut pending = pending_requests_clone.lock().await;
                    if let Some((_, sender)) = pending.drain().next() {
                        let _ = sender.send(Ok(native_message));
                    } else {
                        debug!("Received response but no pending request found");
                    }
                },
                Some(chrome_message) = native_rx.recv() => {
                    info!("Received chrome message");

                    // Handle different types of Chrome messages
                    match chrome_message {
                        ChromeMessage::NativeMetadata(metadata) => {
                            Self::handle_metadata_message(metadata, &mut client).await;
                        }
                    }
                }
                else => break,
            }
        }
    }

    /// Handle incoming metadata messages from Chrome and trigger activity switching
    async fn handle_metadata_message(
        metadata: NativeMetadata,
        client: &mut Option<NativeMessagingIpcClient<Channel>>,
    ) {
        debug!("Received metadata: {:#?}", metadata);
        // Validate that we have a URL
        let Some(url) = metadata.url else {
            debug!("Received metadata without URL, skipping activity switch");
            return;
        };

        // Check if the client is available
        let Some(client) = client.as_mut() else {
            debug!("NativeMessagingIpcClient not available, cannot switch activity");
            return;
        };

        // // Decode icon from base64 if present
        // let icon_bytes = metadata.icon_base64.and_then(|base64_str| {
        //     base64::engine::general_purpose::STANDARD
        //         .decode(base64_str)
        //         .map_err(|e| {
        //             warn!("Failed to decode base64 icon: {}", e);
        //             e
        //         })
        //         .ok()
        // });

        // Create the switch activity request
        let request = SwitchActivityRequest {
            url: url.clone(),
            icon: None,
        };

        // Call the switch_activity RPC
        match client.switch_activity(Request::new(request)).await {
            Ok(response) => {
                info!("Successfully switched activity for URL: {}", url);
                debug!("Switch activity response: {:?}", response);
            }
            Err(e) => {
                error!("Failed to switch activity: {}", e);
            }
        }
    }

    pub async fn handle_stdio(&self) -> Result<()> {
        // Keep the process alive to handle gRPC requests
        tokio::signal::ctrl_c().await?;
        Ok(())
    }

    async fn send_native_message(&self, command: &str) -> Result<NativeMessage> {
        let message = NativeCommand {
            command: command.to_string(),
        };

        let (tx, rx) = oneshot::channel();
        let request = NativeMessageRequest {
            message,
            response_sender: tx,
        };

        self.message_sender
            .send(request)
            .await
            .map_err(|_| anyhow!("Failed to send message request"))?;

        rx.await
            .map_err(|_| anyhow!("Failed to receive response"))?
    }

    fn native_message_to_response(&self, asset: NativeMessage) -> Result<MessageResponse> {
        // Serialize the NativeMessage to bytes
        let content = serde_json::to_vec(&asset)
            .map_err(|e| anyhow!("Failed to serialize NativeMessage: {}", e))?;

        // Get the kind from the enum variant
        let kind = asset.as_ref().to_owned();

        Ok(MessageResponse { kind, content })
    }
}

#[tonic::async_trait]
impl TauriIpc for TauriIpcServer {
    type GetAssetsStreamingStream = ResponseStream;

    async fn get_assets(&self, _req: Request<MessageRequest>) -> IpcResult<MessageResponse> {
        debug!("Received get_assets request");

        match self.send_native_message("GENERATE_ASSETS").await {
            Ok(native_asset) => match self.native_message_to_response(native_asset) {
                Ok(response) => Ok(Response::new(response)),
                Err(e) => {
                    debug!("Error converting asset to response: {}", e);
                    Err(Status::internal(format!("Conversion error: {}", e)))
                }
            },
            Err(e) => {
                debug!("Error in native messaging: {}", e);
                Err(Status::internal(format!("Native messaging error: {}", e)))
            }
        }
    }

    async fn get_snapshots(&self, _req: Request<MessageRequest>) -> IpcResult<MessageResponse> {
        debug!("Received get_snapshots request");

        match self.send_native_message("GENERATE_SNAPSHOTS").await {
            Ok(native_asset) => match self.native_message_to_response(native_asset) {
                Ok(response) => Ok(Response::new(response)),
                Err(e) => {
                    debug!("Error converting asset to response: {}", e);
                    Err(Status::internal(format!("Conversion error: {}", e)))
                }
            },
            Err(e) => {
                debug!("Error in native messaging: {}", e);
                Err(Status::internal(format!("Native messaging error: {}", e)))
            }
        }
    }

    async fn get_assets_streaming(
        &self,
        req: Request<Streaming<MessageRequest>>,
    ) -> IpcResult<Self::GetAssetsStreamingStream> {
        let mut in_stream = req.into_inner();
        let (tx, rx) = mpsc::channel::<Result<MessageResponse, Status>>(128);
        let server_clone = self.clone();

        tokio::spawn(async move {
            while let Some(request) = in_stream.next().await {
                match request {
                    Ok(_) => {
                        debug!("Received streaming assets request");

                        match server_clone.send_native_message("GENERATE_ASSETS").await {
                            Ok(native_asset) => {
                                match server_clone.native_message_to_response(native_asset) {
                                    Ok(response) => {
                                        if tx.send(Ok(response)).await.is_err() {
                                            debug!("Client disconnected");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        debug!("Error converting asset: {}", e);
                                        if tx
                                            .send(Err(Status::internal(format!(
                                                "Conversion error: {}",
                                                e
                                            ))))
                                            .await
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                debug!("Error in native messaging: {}", e);
                                if tx
                                    .send(Err(Status::internal(format!(
                                        "Native messaging error: {}",
                                        e
                                    ))))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        debug!("Error in streaming request: {}", err);
                        if tx
                            .send(Err(Status::internal(format!("Stream error: {}", err))))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            debug!("Streaming connection closed");
        });

        let out_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(out_stream) as Self::GetAssetsStreamingStream
        ))
    }

    async fn get_metadata(&self, _req: Request<MessageRequest>) -> IpcResult<MessageResponse> {
        debug!("Received get_metadata request");

        match self.send_native_message("GET_METADATA").await {
            Ok(native_asset) => match self.native_message_to_response(native_asset) {
                Ok(response) => Ok(Response::new(response)),
                Err(e) => {
                    debug!("Error converting asset to response: {}", e);
                    Err(Status::internal(format!("Conversion error: {}", e)))
                }
            },
            Err(e) => {
                debug!("Error in native messaging: {}", e);
                Err(Status::internal(format!("Native messaging error: {}", e)))
            }
        }
    }

    async fn get_icon(&self, _req: Request<MessageRequest>) -> IpcResult<MessageResponse> {
        debug!("Received get_icon request");

        match self.send_native_message("GET_ICON").await {
            Ok(native_asset) => match self.native_message_to_response(native_asset) {
                Ok(response) => Ok(Response::new(response)),
                Err(e) => {
                    debug!("Error converting asset to response: {}", e);
                    Err(Status::internal(format!("Conversion error: {}", e)))
                }
            },
            Err(e) => {
                debug!("Error in native messaging: {}", e);
                Err(Status::internal(format!("Native messaging error: {}", e)))
            }
        }
    }
}

/// Write a message to the given writer
fn write_message<W: Write>(mut output: W, message: &Value) -> Result<()> {
    let message_bytes = serde_json::to_vec(message)?;
    let message_size = message_bytes.len() as u32;

    output.write_all(&message_size.to_ne_bytes())?;
    output.write_all(&message_bytes)?;
    output.flush()?;

    Ok(())
}
