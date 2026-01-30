//! Browser Bridge gRPC Server
//!
//! This module implements a gRPC server that accepts connections from multiple
//! native messaging hosts. Each host registers with its browser PID, and the
//! server routes requests to the appropriate channel based on the active browser PID.

use super::proto::{
    Frame, browser_bridge_server::BrowserBridge, frame::Kind as FrameKind,
};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast, mpsc};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

/// Holds the sender channel for a registered native messenger
#[derive(Debug)]
pub struct RegisteredMessenger {
    /// Channel to send frames to this native messenger
    pub tx: mpsc::Sender<Result<Frame, Status>>,
    /// The PID of the native messaging host process
    pub host_pid: u32,
    /// The PID of the parent browser process
    pub browser_pid: u32,
}

/// Service that manages multiple native messenger connections
#[derive(Clone)]
pub struct BrowserBridgeService {
    /// Registry of connected native messengers, keyed by browser PID
    pub registry: Arc<RwLock<HashMap<u32, RegisteredMessenger>>>,
    // Frames coming from the app
    pub app_from_tx: broadcast::Sender<Frame>,
    /// Broadcast channel for frames coming from native messengers
    pub frames_from_messengers_tx: broadcast::Sender<(u32, Frame)>,
}

impl BrowserBridgeService {
    pub async fn get_metadata(&self, _browser_pid: u32) -> Result<String, Status> {
        todo!()
    }
}

#[tonic::async_trait]
impl BrowserBridge for BrowserBridgeService {
    type OpenStream = Pin<Box<dyn Stream<Item = Result<Frame, Status>> + Send + 'static>>;

    async fn open(
        &self,
        request: Request<tonic::Streaming<Frame>>,
    ) -> Result<Response<Self::OpenStream>, Status> {
        let mut inbound = request.into_inner();

        let first_frame = inbound.message().await.map_err(|e| {
            error!("Failed to receive the Register frame as first frame: {}", e);
            Status::internal("Failed to receive the Register frame as first frame")
        })?;

        let Some(frame) = first_frame else {
            error!("Received an unexpected frame type as the first frame");
            return Err(Status::internal(
                "Received an unexpected frame type as the first frame",
            ));
        };

        let Some(FrameKind::Register(register_frame)) = frame.kind else {
            error!("Received an unexpected frame type as the first frame");
            return Err(Status::internal(
                "Received an unexpected frame type as the first frame",
            ));
        };

        let browser_pid = register_frame.browser_pid;
        let host_pid = register_frame.host_pid;

        let (tx_to_client, rx_to_client) = mpsc::channel::<Result<Frame, Status>>(32);

        {
            let mut registry = self.registry.write().await;
            registry.insert(
                browser_pid,
                RegisteredMessenger {
                    tx: tx_to_client.clone(),
                    host_pid,
                    browser_pid,
                },
            );
            info!(
                "Registered browser with browser_pid: {} and host_pid: {}. Total registered browsers: {}",
                browser_pid,
                host_pid,
                registry.len()
            );
        }
        let registry = self.registry.clone();
        let frames_tx = self.frames_from_messengers_tx.clone();

        tokio::spawn(async move {
            info!(
                "gRPC client connected, starting forward task: Eurora -> Native Messenger -> Chrome"
            );
            loop {
                match inbound.message().await {
                    Ok(Some(frame)) => {
                        info!(
                            "Received frame from native messenger (browser_pid={}): {:?}",
                            browser_pid, frame
                        );
                        if let Err(e) = frames_tx.send((browser_pid, frame)) {
                            warn!(
                                "Failed to broadcast frame from browser PID {}: {}",
                                browser_pid, e
                            );
                        }
                    }
                    Ok(None) => {
                        info!(
                            "Native messenger disconnected (browser_pid={})",
                            browser_pid
                        );
                        break;
                    }
                    Err(e) => {
                        error!(
                            "Error receiving frame from native messenger (browser_pid={}): {}",
                            browser_pid, e
                        );
                        break;
                    }
                }
            }

            let mut registry = registry.write().await;
            registry.remove(&browser_pid);
            info!(
                "Unregistered native messenger for browser PID {}. Remaining: {}",
                browser_pid,
                registry.len()
            );
        });
        let out_stream = ReceiverStream::new(rx_to_client);
        Ok(Response::new(Box::pin(out_stream) as Self::OpenStream))
    }
}
