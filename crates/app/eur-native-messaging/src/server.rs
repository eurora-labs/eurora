use std::{
    io::{self, Read, Write},
    pin::Pin,
    sync::Arc,
};

use anyhow::{Result, anyhow};
use eur_proto::ipc::{MessageRequest, MessageResponse};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, Streaming};
use tracing::info;

use crate::types::NativeMessage;

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
}

impl TauriIpcServer {
    pub fn new() -> (Self, mpsc::Sender<NativeCommand>) {
        let (tx, rx) = mpsc::channel::<NativeMessageRequest>(32);
        let (native_tx, native_rx) = mpsc::channel::<NativeCommand>(32);

        // Spawn a task to handle the stdio communication
        tokio::spawn(Self::handle_stdio_task(rx, native_rx));

        (Self { message_sender: tx }, native_tx)
    }

    async fn handle_stdio_task(
        mut request_rx: mpsc::Receiver<NativeMessageRequest>,
        mut native_rx: mpsc::Receiver<NativeCommand>,
    ) {
        let stdin = io::stdin();
        let stdout = io::stdout();

        // Use mutexes to prevent concurrent access to stdin/stdout
        let stdin_mutex = Arc::new(tokio::sync::Mutex::new(stdin));
        let stdout_mutex = Arc::new(tokio::sync::Mutex::new(stdout));

        loop {
            tokio::select! {
                Some(request) = request_rx.recv() => {
                    let NativeMessageRequest { message, response_sender } = request;

                    // Create the message to send to Chrome extension
                    let message_value = json!({
                        "command": message.command
                    });

                    // Acquire mutexes for atomic stdio operation
                    let stdout_guard = stdout_mutex.lock().await;
                    let stdin_guard = stdin_mutex.lock().await;

                    // Perform write and read as atomic operation
                    let result = async {
                        write_message(&*stdout_guard, &message_value)
                            .map_err(|e| anyhow!("Write error: {}", e))?;
                        let response = read_message(&*stdin_guard)
                            .map_err(|e| anyhow!("Read error: {}", e))?;

                        // Parse the response as NativeMessage
                        let native_asset: NativeMessage = serde_json::from_value(response)
                            .map_err(|e| anyhow!("Failed to parse response as NativeMessage: {}", e))?;

                        Ok(native_asset)
                    }.await;

                    let _ = response_sender.send(result);
                },
                Some(native_message) = native_rx.recv() => {
                    // Process incoming native messages (if any)
                    info!("Received native message: {:?}", native_message);
                }
                else => break,
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
impl eur_proto::ipc::tauri_ipc_server::TauriIpc for TauriIpcServer {
    type GetAssetsStreamingStream = ResponseStream;

    async fn get_assets(&self, _req: Request<MessageRequest>) -> IpcResult<MessageResponse> {
        info!("Received get_assets request");

        match self.send_native_message("GENERATE_ASSETS").await {
            Ok(native_asset) => match self.native_message_to_response(native_asset) {
                Ok(response) => Ok(Response::new(response)),
                Err(e) => {
                    info!("Error converting asset to response: {}", e);
                    Err(Status::internal(format!("Conversion error: {}", e)))
                }
            },
            Err(e) => {
                info!("Error in native messaging: {}", e);
                Err(Status::internal(format!("Native messaging error: {}", e)))
            }
        }
    }

    async fn get_snapshots(&self, _req: Request<MessageRequest>) -> IpcResult<MessageResponse> {
        info!("Received get_snapshots request");

        match self.send_native_message("GENERATE_SNAPSHOTS").await {
            Ok(native_asset) => match self.native_message_to_response(native_asset) {
                Ok(response) => Ok(Response::new(response)),
                Err(e) => {
                    info!("Error converting asset to response: {}", e);
                    Err(Status::internal(format!("Conversion error: {}", e)))
                }
            },
            Err(e) => {
                info!("Error in native messaging: {}", e);
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
                        info!("Received streaming assets request");

                        match server_clone.send_native_message("GENERATE_ASSETS").await {
                            Ok(native_asset) => {
                                match server_clone.native_message_to_response(native_asset) {
                                    Ok(response) => {
                                        if tx.send(Ok(response)).await.is_err() {
                                            info!("Client disconnected");
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        info!("Error converting asset: {}", e);
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
                                info!("Error in native messaging: {}", e);
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
                        info!("Error in streaming request: {}", err);
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
            info!("Streaming connection closed");
        });

        let out_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(out_stream) as Self::GetAssetsStreamingStream
        ))
    }
}

/// Read a message from the given reader
fn read_message<R: Read>(mut input: R) -> Result<Value> {
    let mut size_bytes = [0u8; 4];
    input.read_exact(&mut size_bytes)?;

    let message_size = u32::from_ne_bytes(size_bytes) as usize;
    let mut buffer = vec![0u8; message_size];
    input.read_exact(&mut buffer)?;

    Ok(serde_json::from_slice(&buffer)?)
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
