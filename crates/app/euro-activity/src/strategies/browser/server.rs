//! Browser Bridge gRPC Server
//!
//! This module implements a gRPC server that accepts connections from multiple
//! native messaging hosts. Each host registers with its browser PID, and the
//! server routes requests to the appropriate channel based on the active browser PID.

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::{RwLock, broadcast, mpsc, oneshot};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

use super::proto::{
    Frame, RequestFrame, browser_bridge_server::BrowserBridge, frame::Kind as FrameKind,
};

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

/// A pending request waiting for a response
struct PendingRequest {
    sender: oneshot::Sender<Frame>,
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
    /// Pending requests waiting for responses, keyed by request ID
    pending_requests: Arc<DashMap<u32, PendingRequest>>,
    /// Counter for generating unique request IDs
    request_id_counter: Arc<AtomicU32>,
}

impl BrowserBridgeService {
    /// Create a new BrowserBridgeService
    pub fn new() -> Self {
        let (frames_from_messengers_tx, _) = broadcast::channel(1024);
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            active_browser_pid: Arc::new(AtomicU32::new(0)),
            frames_from_messengers_tx,
            pending_requests: Arc::new(DashMap::new()),
            request_id_counter: Arc::new(AtomicU32::new(1)),
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

    /// Send a request frame and wait for a response.
    ///
    /// This is the core method that initiates the gRPC pipeline. It:
    /// 1. Generates a unique request ID
    /// 2. Creates a request frame with the specified action
    /// 3. Sends it through the bidirectional gRPC stream to the active messenger
    /// 4. Waits for and returns the matching response frame
    ///
    /// # Arguments
    /// * `action` - The action string for the request (e.g., "GET_METADATA")
    /// * `payload` - Optional JSON payload for the request
    ///
    /// # Returns
    /// - `Ok(Frame)` - The response frame
    /// - `Err(Status)` - If the request fails or times out
    pub async fn send_request(
        &self,
        action: &str,
        payload: Option<String>,
    ) -> Result<Frame, Status> {
        // Generate unique request ID
        let request_id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);

        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();

        // Register pending request
        self.pending_requests
            .insert(request_id, PendingRequest { sender: tx });

        // Create the request frame
        let request_frame = RequestFrame {
            id: request_id,
            action: action.to_string(),
            payload,
        };

        let frame = Frame {
            kind: Some(FrameKind::Request(request_frame)),
        };

        debug!(
            "Sending request frame: id={}, action={}",
            request_id, action
        );

        // Send the request to the active messenger
        if let Err(e) = self.send_to_active(frame).await {
            // Clean up pending request on send failure
            self.pending_requests.remove(&request_id);
            return Err(e);
        }

        // Wait for response with timeout
        match tokio::time::timeout(Duration::from_secs(5), rx).await {
            Ok(Ok(response_frame)) => {
                debug!("Received response for request id={}", request_id);
                Ok(response_frame)
            }
            Ok(Err(_)) => {
                error!("Response channel closed for request id={}", request_id);
                Err(Status::internal("Response channel closed"))
            }
            Err(_) => {
                error!("Timeout waiting for response to request id={}", request_id);
                // Clean up pending request on timeout
                self.pending_requests.remove(&request_id);
                Err(Status::deadline_exceeded("Request timeout"))
            }
        }
    }

    /// Initiates the gRPC pipeline to get metadata from the active native messenger.
    ///
    /// This is a convenience method that sends a "GET_METADATA" request through
    /// the bidirectional gRPC stream and returns the response.
    ///
    /// # Returns
    /// - `Ok(Frame)` - The response frame containing metadata
    /// - `Err(Status)` - If the request fails or times out
    pub async fn get_metadata(&self) -> Result<Frame, Status> {
        self.send_request("GET_METADATA", None).await
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

        // Create the channel for this connection upfront
        let (tx_to_client, rx_to_client) = mpsc::channel::<Result<Frame, Status>>(32);

        // Clone what we need for the spawned task
        let registry = self.registry.clone();
        let frames_tx = self.frames_from_messengers_tx.clone();
        let pending_requests = self.pending_requests.clone();
        let tx_to_client_clone = tx_to_client.clone();

        // Spawn task to handle incoming frames from this client
        // The client's identity (browser_pid) is not known until they send a RegisterFrame
        tokio::spawn(async move {
            debug!("Starting frame handler for new connection");

            // Track registration state for this connection
            let mut registered_browser_pid: Option<u32> = None;

            loop {
                match inbound.message().await {
                    Ok(Some(frame)) => {
                        let Some(kind) = &frame.kind else {
                            warn!("Received frame with no kind, ignoring");
                            continue;
                        };

                        match kind {
                            FrameKind::Register(register_frame) => {
                                let browser_pid = register_frame.browser_pid;
                                let host_pid = register_frame.host_pid;

                                info!(
                                    "Native messenger registering: host_pid={}, browser_pid={}",
                                    host_pid, browser_pid
                                );

                                // Register this messenger
                                {
                                    let mut reg = registry.write().await;
                                    reg.insert(
                                        browser_pid,
                                        RegisteredMessenger {
                                            tx: tx_to_client_clone.clone(),
                                            host_pid,
                                            browser_pid,
                                        },
                                    );
                                    info!(
                                        "Registered native messenger for browser PID {}. Total registered: {}",
                                        browser_pid,
                                        reg.len()
                                    );
                                }

                                registered_browser_pid = Some(browser_pid);
                            }

                            FrameKind::Response(resp) => {
                                debug!(
                                    "Received response frame: id={}, action={}",
                                    resp.id, resp.action
                                );

                                // Route to the appropriate pending request channel
                                if let Some((_, pending)) = pending_requests.remove(&resp.id) {
                                    if pending.sender.send(frame.clone()).is_err() {
                                        warn!(
                                            "Failed to send response to pending request id={}",
                                            resp.id
                                        );
                                    }
                                } else {
                                    // No pending request, broadcast it
                                    if let Some(browser_pid) = registered_browser_pid {
                                        if let Err(e) = frames_tx.send((browser_pid, frame.clone()))
                                        {
                                            warn!(
                                                "Failed to broadcast response frame from browser PID {}: {}",
                                                browser_pid, e
                                            );
                                        }
                                    } else {
                                        warn!(
                                            "Received response frame id={} from unregistered client",
                                            resp.id
                                        );
                                    }
                                }
                            }

                            FrameKind::Error(err) => {
                                debug!(
                                    "Received error frame: id={}, message={}",
                                    err.id, err.message
                                );

                                // Route to the appropriate pending request channel
                                if let Some((_, pending)) = pending_requests.remove(&err.id) {
                                    if pending.sender.send(frame.clone()).is_err() {
                                        warn!(
                                            "Failed to send error to pending request id={}",
                                            err.id
                                        );
                                    }
                                } else {
                                    warn!(
                                        "Received error frame id={} with no pending request",
                                        err.id
                                    );
                                }
                            }

                            FrameKind::Event(evt) => {
                                info!(
                                    "Received event frame: action={}, payload={:?}",
                                    evt.action, evt.payload
                                );

                                // Broadcast event to listeners
                                if let Some(browser_pid) = registered_browser_pid {
                                    if let Err(e) = frames_tx.send((browser_pid, frame.clone())) {
                                        warn!(
                                            "Failed to broadcast event frame from browser PID {}: {}",
                                            browser_pid, e
                                        );
                                    }
                                } else {
                                    warn!(
                                        "Received event frame action={} from unregistered client",
                                        evt.action
                                    );
                                }
                            }

                            FrameKind::Request(req) => {
                                // Not implemented - log for now
                                warn!(
                                    "Received request frame (not implemented): id={}, action={}",
                                    req.id, req.action
                                );
                            }

                            FrameKind::Cancel(cancel) => {
                                // Not implemented - log for now
                                warn!("Received cancel frame (not implemented): id={}", cancel.id);
                            }
                        }
                    }
                    Ok(None) => {
                        // Stream ended
                        if let Some(browser_pid) = registered_browser_pid {
                            info!(
                                "Native messenger disconnected (browser_pid={})",
                                browser_pid
                            );
                        } else {
                            info!("Unregistered client disconnected");
                        }
                        break;
                    }
                    Err(e) => {
                        if let Some(browser_pid) = registered_browser_pid {
                            error!(
                                "Error receiving frame from native messenger (browser_pid={}): {}",
                                browser_pid, e
                            );
                        } else {
                            error!("Error receiving frame from unregistered client: {}", e);
                        }
                        break;
                    }
                }
            }

            // Unregister this messenger when it disconnects (if it was registered)
            if let Some(browser_pid) = registered_browser_pid {
                let mut reg = registry.write().await;
                reg.remove(&browser_pid);
                info!(
                    "Unregistered native messenger for browser PID {}. Remaining: {}",
                    browser_pid,
                    reg.len()
                );
            }
        });

        let out_stream = ReceiverStream::new(rx_to_client);
        Ok(Response::new(Box::pin(out_stream) as Self::OpenStream))
    }
}
