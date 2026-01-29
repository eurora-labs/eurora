//! Browser Bridge gRPC Server
//!
//! This module implements a gRPC server that accepts connections from multiple
//! native messaging hosts. Each host registers with its browser PID, and the
//! server routes requests to the appropriate channel based on the active browser PID.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use tokio::sync::{RwLock, broadcast, mpsc};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

use super::proto::{Frame, browser_bridge_server::BrowserBridge, frame::Kind as FrameKind};

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
    /// The currently active browser PID that should receive requests
    pub active_browser_pid: Arc<AtomicU32>,
    /// Broadcast channel for frames coming from native messengers
    pub frames_from_messengers_tx: broadcast::Sender<(u32, Frame)>,
}

impl BrowserBridgeService {
    /// Create a new BrowserBridgeService
    pub fn new() -> Self {
        let (frames_from_messengers_tx, _) = broadcast::channel(1024);
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            active_browser_pid: Arc::new(AtomicU32::new(0)),
            frames_from_messengers_tx,
        }
    }

    /// Set the active browser PID
    pub fn set_active_browser_pid(&self, pid: u32) {
        let old_pid = self.active_browser_pid.swap(pid, Ordering::SeqCst);
        if old_pid != pid {
            info!("Active browser PID changed from {} to {}", old_pid, pid);
        }
    }

    /// Get the active browser PID
    pub fn get_active_browser_pid(&self) -> u32 {
        self.active_browser_pid.load(Ordering::SeqCst)
    }

    /// Send a frame to the active native messenger
    pub async fn send_to_active(&self, frame: Frame) -> Result<(), Status> {
        let active_pid = self.get_active_browser_pid();
        if active_pid == 0 {
            return Err(Status::unavailable("No active browser PID set"));
        }

        let registry = self.registry.read().await;
        if let Some(messenger) = registry.get(&active_pid) {
            messenger.tx.send(Ok(frame)).await.map_err(|e| {
                error!(
                    "Failed to send frame to messenger for browser PID {}: {}",
                    active_pid, e
                );
                Status::internal(format!("Failed to send frame: {}", e))
            })
        } else {
            Err(Status::not_found(format!(
                "No native messenger registered for browser PID {}",
                active_pid
            )))
        }
    }

    /// Send a frame to a specific browser PID
    pub async fn send_to_pid(&self, browser_pid: u32, frame: Frame) -> Result<(), Status> {
        let registry = self.registry.read().await;
        if let Some(messenger) = registry.get(&browser_pid) {
            messenger.tx.send(Ok(frame)).await.map_err(|e| {
                error!(
                    "Failed to send frame to messenger for browser PID {}: {}",
                    browser_pid, e
                );
                Status::internal(format!("Failed to send frame: {}", e))
            })
        } else {
            Err(Status::not_found(format!(
                "No native messenger registered for browser PID {}",
                browser_pid
            )))
        }
    }

    /// Check if a browser PID is registered
    pub async fn is_registered(&self, browser_pid: u32) -> bool {
        let registry = self.registry.read().await;
        registry.contains_key(&browser_pid)
    }

    /// Get a list of all registered browser PIDs
    pub async fn get_registered_pids(&self) -> Vec<u32> {
        let registry = self.registry.read().await;
        registry.keys().copied().collect()
    }

    /// Subscribe to frames from native messengers
    pub fn subscribe_to_frames(&self) -> broadcast::Receiver<(u32, Frame)> {
        self.frames_from_messengers_tx.subscribe()
    }
}

impl Default for BrowserBridgeService {
    fn default() -> Self {
        Self::new()
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

        // Wait for the first frame which should be a RegisterFrame
        let first_frame = inbound.message().await.map_err(|e| {
            error!("Failed to receive first frame: {}", e);
            Status::internal("Failed to receive registration frame")
        })?;

        let Some(frame) = first_frame else {
            return Err(Status::invalid_argument(
                "Expected RegisterFrame as first message",
            ));
        };

        let Some(FrameKind::Register(register_frame)) = frame.kind else {
            return Err(Status::invalid_argument(
                "First message must be a RegisterFrame",
            ));
        };

        let browser_pid = register_frame.browser_pid;
        let host_pid = register_frame.host_pid;

        info!(
            "Native messenger registered: host_pid={}, browser_pid={}",
            host_pid, browser_pid
        );

        // Create the channel for this connection
        let (tx_to_client, rx_to_client) = mpsc::channel::<Result<Frame, Status>>(32);

        // Register this messenger
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
                "Registered native messenger for browser PID {}. Total registered: {}",
                browser_pid,
                registry.len()
            );
        }

        // Clone what we need for the spawned task
        let registry = self.registry.clone();
        let frames_tx = self.frames_from_messengers_tx.clone();

        // Spawn task to handle incoming frames from this client
        tokio::spawn(async move {
            debug!("Starting frame handler for browser PID {}", browser_pid);

            loop {
                match inbound.message().await {
                    Ok(Some(frame)) => {
                        debug!(
                            "Received frame from native messenger (browser_pid={}): {:?}",
                            browser_pid, frame
                        );
                        // Broadcast the frame with the browser PID
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

            // Unregister this messenger when it disconnects
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
