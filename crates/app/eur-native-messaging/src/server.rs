use std::{
    error::Error,
    io::{self, ErrorKind, Read, Write},
    pin::Pin,
    sync::Arc,
};

use anyhow::{Result, anyhow};
use eur_proto::ipc::{SnapshotResponse, StateRequest, StateResponse};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};
use tokio_stream::{Stream, StreamExt, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, Streaming};
use tracing::info;

use crate::{
    asset_converter::JSONToProtoAssetConverter, snapshot_converter::JSONToProtoSnapshotConverter,
};

type IpcResult<T> = Result<Response<T>, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<StateResponse, Status>> + Send>>;

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;

    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err);
        }

        // h2::Error does not expose std::io::Error with `source()`
        if let Some(h2_err) = err.downcast_ref::<h2::Error>()
            && let Some(io_err) = h2_err.get_io()
        {
            return Some(io_err);
        }

        err = err.source()?;
    }
}

// Message type for native messaging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NativeMessage {
    pub r#type: String,
    #[serde(flatten)]
    pub payload: Value,
}

// Request type for internal communication
#[derive(Debug)]
struct NativeMessageRequest {
    message: NativeMessage,
    response_sender: oneshot::Sender<Result<Value>>,
}

#[derive(Clone)]
pub struct TauriIpcServer {
    message_sender: mpsc::Sender<NativeMessageRequest>,
}

impl TauriIpcServer {
    pub fn new() -> (Self, mpsc::Sender<NativeMessage>) {
        let (tx, rx) = mpsc::channel::<NativeMessageRequest>(32);
        let (native_tx, native_rx) = mpsc::channel::<NativeMessage>(32);

        // Spawn a task to handle the stdio communication
        tokio::spawn(Self::handle_stdio_task(rx, native_rx));

        (Self { message_sender: tx }, native_tx)
    }

    async fn handle_stdio_task(
        mut request_rx: mpsc::Receiver<NativeMessageRequest>,
        mut native_rx: mpsc::Receiver<NativeMessage>,
    ) {
        let stdin = io::stdin();
        let stdout = io::stdout();

        // Use a mutex to prevent concurrent access to stdin/stdout
        let stdin_mutex = Arc::new(tokio::sync::Mutex::new(stdin));
        let stdout_mutex = Arc::new(tokio::sync::Mutex::new(stdout));

        loop {
            tokio::select! {
                Some(request) = request_rx.recv() => {
                    let NativeMessageRequest { message, response_sender } = request;
                    let message_value = match serde_json::to_value(&message) {
                        Ok(val) => val,
                        Err(e) => {
                            let _ = response_sender.send(Err(anyhow!("Serialization error: {}", e)));
                            continue;
                        }
                    };

                    // Use single mutex to prevent deadlock - acquire both stdin and stdout atomically
                    let stdout_guard = stdout_mutex.lock().await;
                    let stdin_guard = stdin_mutex.lock().await;

                    // Perform write and read as atomic operation
                    let result = async {
                        write_message(&*stdout_guard, &message_value)
                            .map_err(|e| anyhow!("Write error: {}", e))?;
                        read_message(&*stdin_guard)
                            .map_err(|e| anyhow!("Read error: {}", e))
                    }.await;

                    let _ = response_sender.send(result);
                },
                Some(native_message) = native_rx.recv() => {
                    // Process incoming native messages (if any)
                    // This is for handling incoming messages from the browser
                    info!("Received native message: {:?}", native_message);
                }
                else => break,
            }
        }
    }

    pub async fn handle_stdio(&self) -> Result<()> {
        // This function is called from main and can just wait indefinitely
        // since the actual stdio handling is done in the separate task
        std::future::pending::<()>().await;
        Ok(())
    }

    async fn send_native_message(&self, message_type: &str, payload: Value) -> Result<Value> {
        let message = NativeMessage {
            r#type: message_type.to_string(),
            payload,
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
}

#[tonic::async_trait]
impl eur_proto::ipc::tauri_ipc_server::TauriIpc for TauriIpcServer {
    type GetStateStreamingStream = ResponseStream;

    async fn get_state(&self, _req: Request<StateRequest>) -> IpcResult<StateResponse> {
        info!("Received get_state request");

        // Send GENERATE_REPORT request via native messaging
        match self.send_native_message("GENERATE_ASSETS", json!({})).await {
            Ok(response) => {
                // Convert JSON response to StateResponse proto
                let state_response =
                    JSONToProtoAssetConverter::convert(&response).map_err(|e| {
                        Status::internal(format!("Failed to convert JSON to StateResponse: {}", e))
                    })?;
                Ok(Response::new(state_response))
            }
            Err(e) => {
                info!("Error in native messaging: {}", e);
                Err(Status::internal(format!("Native messaging error: {}", e)))
            }
        }
    }

    async fn get_snapshot(&self, _req: Request<StateRequest>) -> IpcResult<SnapshotResponse> {
        info!("Received get_snapshot request");

        // Send GENERATE_REPORT request via native messaging
        match self
            .send_native_message("GENERATE_SNAPSHOT", json!({}))
            .await
        {
            Ok(response) => {
                // Convert JSON response to SnapshotResponse proto
                let snapshot_response =
                    JSONToProtoSnapshotConverter::convert(&response).map_err(|e| {
                        Status::internal(format!(
                            "Failed to convert JSON to SnapshotResponse: {}",
                            e
                        ))
                    })?;
                Ok(Response::new(snapshot_response))
            }
            Err(e) => {
                info!("Error in native messaging: {}", e);
                Err(Status::internal(format!("Native messaging error: {}", e)))
            }
        }
    }

    async fn get_state_streaming(
        &self,
        req: Request<Streaming<StateRequest>>,
    ) -> IpcResult<Self::GetStateStreamingStream> {
        let mut in_stream = req.into_inner();
        let (tx, rx) = mpsc::channel(128); // Increased buffer size to match example
        let server_clone = self.clone();

        // This spawn is required to handle the bidirectional streaming properly
        // When using a bidirectional stream, we need to process the incoming requests
        // and send responses back, all while keeping the connection open
        tokio::spawn(async move {
            while let Some(request) = in_stream.next().await {
                match request {
                    Ok(_) => {
                        info!("Received gather state request");
                        // Send GENERATE_REPORT request via native messaging
                        match server_clone
                            .send_native_message("GENERATE_REPORT", json!({}))
                            .await
                        {
                            Ok(response) => {
                                // info!("Received GENERATE_REPORT response {:?}", response);

                                let state_response = JSONToProtoAssetConverter::convert(&response);

                                match tx.send(Ok(state_response)).await {
                                    Ok(_) => {
                                        // Message successfully sent, continue processing
                                        // Unlike the previous implementation, we don't break the loop here
                                    }
                                    Err(e) => {
                                        info!("Error sending response: {}", e);
                                        break; // Channel closed, client disconnected
                                    }
                                }
                            }
                            Err(e) => {
                                info!("Error in native messaging: {}", e);
                                match tx.send(Err(e)).await {
                                    Ok(_) => {
                                        // Error message sent, but we continue processing
                                    }
                                    Err(e) => {
                                        info!("Error sending error response: {}", e);
                                        break; // Channel closed, client disconnected
                                    }
                                }
                            }
                        }
                    }
                    Err(err) => {
                        info!("Error in gather state: {}", err);
                        if let Some(io_err) = match_for_io_error(&err)
                            && io_err.kind() == ErrorKind::BrokenPipe
                        {
                            info!("Browser connection closed: broken pipe");
                            break;
                        }
                        match tx.send(Err(err.into())).await {
                            Ok(_) => {
                                // Continue processing after error
                            }
                            Err(e) => {
                                info!("Error sending state response: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            info!("Browser connection closed: stream ended");
        });

        let out_stream = ReceiverStream::new(rx);

        // Fix for the type mismatch error - map the stream items to the expected type
        let mapped_stream = out_stream.map(|result| match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(err)) => Err(Status::internal(err.to_string())),
            Err(err) => Err(Status::internal(err.to_string())),
        });

        Ok(Response::new(
            Box::pin(mapped_stream) as Self::GetStateStreamingStream
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
