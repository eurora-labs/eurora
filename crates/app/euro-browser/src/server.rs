//! Browser Bridge gRPC Server
//!
//! This module implements a gRPC server that accepts connections from multiple
//! native messaging hosts. Each host registers with its browser PID, and the
//! server routes requests to the appropriate channel based on the active browser PID.
//!
//! The server is managed by the TimelineManager and will run as long as the manager
//! is alive. When the TimelineManager is stopped, the server will be gracefully shut down.

use super::proto::{
    EventFrame, Frame, RequestFrame, ResponseFrame, browser_bridge_server::BrowserBridge,
    browser_bridge_server::BrowserBridgeServer, frame::Kind as FrameKind,
};
use dashmap::DashMap;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::{OnceCell, RwLock, broadcast, mpsc, oneshot, watch};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, transport::Server};

/// The port for the browser bridge gRPC server
pub const BROWSER_BRIDGE_PORT: &str = "1431";

/// Global singleton for the browser bridge service
static GLOBAL_SERVICE: OnceCell<BrowserBridgeService> = OnceCell::const_new();

/// Flag to track if the server has been started
static SERVER_STARTED: AtomicBool = AtomicBool::new(false);

/// Global shutdown signal sender
static SHUTDOWN_TX: OnceCell<watch::Sender<bool>> = OnceCell::const_new();

/// Default timeout for request-response operations
const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Wrapper for pending request sender
struct PendingRequest {
    sender: oneshot::Sender<Frame>,
}

impl std::fmt::Debug for PendingRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingRequest")
            .field("sender", &"oneshot::Sender<Frame>")
            .finish()
    }
}

impl PendingRequest {
    fn new(sender: oneshot::Sender<Frame>) -> Self {
        Self { sender }
    }

    fn send(self, frame: Frame) -> Result<(), ()> {
        if self.sender.send(frame).is_err() {
            error!("Failed to send frame to waiting request");
        }
        Ok(())
    }
}

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
    /// Frames coming from the app to send to native messengers
    pub app_from_tx: broadcast::Sender<Frame>,
    /// Broadcast channel for frames coming from native messengers
    pub frames_from_messengers_tx: broadcast::Sender<(u32, Frame)>,
    /// Broadcast channel for event frames (browser_pid, EventFrame)
    events_tx: broadcast::Sender<(u32, EventFrame)>,
    /// Pending requests waiting for responses, keyed by request ID
    pending_requests: Arc<DashMap<u32, PendingRequest>>,
    /// Counter for generating unique request IDs
    request_id_counter: Arc<AtomicU32>,
    /// Handle for the frame handler task
    frame_handler_handle: Arc<OnceCell<tokio::task::JoinHandle<()>>>,
}

impl BrowserBridgeService {
    /// Creates a new BrowserBridgeService instance
    pub fn new() -> Self {
        let (app_from_tx, _) = broadcast::channel(100);
        let (frames_from_messengers_tx, _) = broadcast::channel(100);
        let (events_tx, _) = broadcast::channel(100);

        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            app_from_tx,
            frames_from_messengers_tx,
            events_tx,
            pending_requests: Arc::new(DashMap::new()),
            request_id_counter: Arc::new(AtomicU32::new(1)),
            frame_handler_handle: Arc::new(OnceCell::new()),
        }
    }

    /// Start the frame handler task that routes responses to pending requests
    ///
    /// This task listens for frames from native messengers and routes responses
    /// to their corresponding pending requests. It should be started once when
    /// the service is initialized.
    pub fn start_frame_handler(&self) {
        let pending_requests = Arc::clone(&self.pending_requests);
        let frames_from_messengers_tx = self.frames_from_messengers_tx.clone();
        let events_tx = self.events_tx.clone();
        let mut frames_rx = frames_from_messengers_tx.subscribe();

        let handle = tokio::spawn(async move {
            debug!("Frame handler task started");
            while let Ok((browser_pid, frame)) = frames_rx.recv().await {
                let kind = match &frame.kind {
                    Some(k) => k.clone(),
                    None => {
                        warn!(
                            "Received frame with no kind from browser PID {}",
                            browser_pid
                        );
                        continue;
                    }
                };

                match kind {
                    FrameKind::Request(req_frame) => {
                        debug!(
                            "Received request frame from browser PID {}: id={}, action={}",
                            browser_pid, req_frame.id, req_frame.action
                        );
                        // For now, log unsupported requests from browser extension
                        warn!(
                            "Received unsupported request from browser extension: action={}",
                            req_frame.action
                        );
                    }
                    FrameKind::Response(resp_frame) => {
                        // Match response to pending request
                        if let Some((_, pending_request)) = pending_requests.remove(&resp_frame.id)
                        {
                            let frame = Frame {
                                kind: Some(FrameKind::Response(resp_frame.clone())),
                            };
                            if let Err(err) = pending_request.send(frame) {
                                warn!("Failed to send frame to waiting request: {:?}", err);
                            }
                        } else {
                            debug!(
                                "Received frame with no pending request: id={} action={}",
                                resp_frame.id, resp_frame.action,
                            );
                        }
                    }
                    FrameKind::Event(evt_frame) => {
                        debug!(
                            "Received event frame from browser PID {}: action={}",
                            browser_pid, evt_frame.action
                        );
                        // Broadcast event frame to event subscribers
                        if let Err(e) = events_tx.send((browser_pid, evt_frame)) {
                            debug!(
                                "No event subscribers for event frame from browser PID {}: {}",
                                browser_pid, e
                            );
                        }
                    }
                    FrameKind::Error(err_frame) => {
                        error!(
                            "Received error frame: id={}, message={}",
                            err_frame.id, err_frame.message
                        );
                        // Match error to pending request if applicable
                        if let Some((_, pending_request)) = pending_requests.remove(&err_frame.id) {
                            let frame = Frame {
                                kind: Some(FrameKind::Error(err_frame)),
                            };
                            if let Err(err) = pending_request.send(frame) {
                                warn!("Failed to send error frame to waiting request: {:?}", err);
                            }
                        }
                    }
                    FrameKind::Cancel(cancel_frame) => {
                        debug!("Received cancel frame: id={}", cancel_frame.id);
                        // Remove pending request if it exists
                        if pending_requests.remove(&cancel_frame.id).is_some() {
                            debug!("Cancelled pending request: id={}", cancel_frame.id);
                        }
                    }
                    FrameKind::Register(_) => {
                        // Registration is handled by the server's Open method
                        debug!("Received register frame (should be handled by server)");
                    }
                }
            }
            debug!("Frame handler task ended");
        });

        let _ = self.frame_handler_handle.set(handle);
    }

    /// Get or initialize the global singleton instance of the service
    ///
    /// This ensures only one instance of the service exists throughout the
    /// application lifetime, allowing the server to persist independently
    /// of the browser strategy lifecycle.
    pub async fn get_or_init() -> &'static BrowserBridgeService {
        GLOBAL_SERVICE
            .get_or_init(|| async { BrowserBridgeService::new() })
            .await
    }

    /// Start the gRPC server if not already running
    ///
    /// This method is idempotent - calling it multiple times will only start
    /// the server once. The server runs in a background task and will continue
    /// running until `stop_server()` is called or the application exits.
    ///
    /// This should be called by the TimelineManager when it starts.
    pub async fn start_server(&self) {
        // Check if server is already started using atomic flag
        if SERVER_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            debug!("Browser Bridge gRPC server already running");
            return;
        }

        // Initialize shutdown channel
        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let _ = SHUTDOWN_TX.set(shutdown_tx);

        let service_clone = self.clone();

        tokio::spawn(async move {
            let addr = format!("[::1]:{}", BROWSER_BRIDGE_PORT)
                .to_socket_addrs()
                .expect("Invalid server address")
                .next()
                .expect("No valid socket address");

            info!("Starting Browser Bridge gRPC server at {}", addr);

            let server = Server::builder()
                .add_service(BrowserBridgeServer::new(service_clone))
                .serve_with_shutdown(addr, async move {
                    // Wait for shutdown signal
                    loop {
                        if shutdown_rx.changed().await.is_err() {
                            break;
                        }
                        if *shutdown_rx.borrow() {
                            info!("Received shutdown signal for Browser Bridge gRPC server");
                            break;
                        }
                    }
                });

            if let Err(e) = server.await {
                error!("Browser Bridge gRPC server error: {}", e);
            }

            // Reset the flag so server can be restarted if needed
            SERVER_STARTED.store(false, Ordering::SeqCst);
            info!("Browser Bridge gRPC server ended");
        });
    }

    /// Stop the gRPC server gracefully
    ///
    /// This sends a shutdown signal to the server, allowing it to finish
    /// processing current requests before shutting down. All connected
    /// native messengers will be disconnected.
    ///
    /// This should be called by the TimelineManager when it stops.
    pub async fn stop_server() {
        if !SERVER_STARTED.load(Ordering::SeqCst) {
            debug!("Browser Bridge gRPC server is not running");
            return;
        }

        if let Some(tx) = SHUTDOWN_TX.get() {
            info!("Sending shutdown signal to Browser Bridge gRPC server");
            let _ = tx.send(true);
        }
    }

    /// Subscribe to frames coming from native messengers
    pub fn subscribe_to_frames(&self) -> broadcast::Receiver<(u32, Frame)> {
        self.frames_from_messengers_tx.subscribe()
    }

    /// Subscribe to event frames coming from native messengers
    ///
    /// Returns a receiver that will receive tuples of (browser_pid, EventFrame)
    /// for all event frames from connected native messengers.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let service = BrowserBridgeService::get_or_init().await;
    /// let mut events_rx = service.subscribe_to_events();
    ///
    /// tokio::spawn(async move {
    ///     while let Ok((browser_pid, event)) = events_rx.recv().await {
    ///         println!("Event from browser {}: action={}", browser_pid, event.action);
    ///     }
    /// });
    /// ```
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<(u32, EventFrame)> {
        self.events_tx.subscribe()
    }

    /// Check if a browser PID is registered with a native messenger
    pub async fn is_registered(&self, browser_pid: u32) -> bool {
        let registry = self.registry.read().await;
        registry.contains_key(&browser_pid)
    }

    /// Get all registered browser PIDs
    pub async fn get_registered_pids(&self) -> Vec<u32> {
        let registry = self.registry.read().await;
        registry.keys().copied().collect()
    }

    /// Send a frame to a specific browser PID
    pub async fn send_to_browser(&self, browser_pid: u32, frame: Frame) -> Result<(), Status> {
        let registry = self.registry.read().await;
        if let Some(messenger) = registry.get(&browser_pid) {
            messenger
                .tx
                .send(Ok(frame))
                .await
                .map_err(|e| Status::internal(format!("Failed to send frame: {}", e)))
        } else {
            Err(Status::not_found(format!(
                "No messenger registered for browser PID {}",
                browser_pid
            )))
        }
    }

    /// Send a request to a specific browser and wait for a response
    ///
    /// This method handles the full request-response cycle:
    /// 1. Generates a unique request ID
    /// 2. Registers a pending request
    /// 3. Sends the request frame
    /// 4. Waits for the response with timeout
    /// 5. Returns the response frame
    pub async fn send_request(
        &self,
        browser_pid: u32,
        action: &str,
        payload: Option<String>,
    ) -> Result<ResponseFrame, Status> {
        // Generate unique request ID
        let request_id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);

        // Create oneshot channel for response
        let (tx, rx) = oneshot::channel();

        // Register pending request
        self.pending_requests
            .insert(request_id, PendingRequest::new(tx));

        // Create and send request frame
        let request_frame = RequestFrame {
            id: request_id,
            action: action.to_string(),
            payload,
        };

        debug!(
            "Sending request frame: id={}, action={}, browser_pid={}",
            request_id, action, browser_pid
        );

        let frame = Frame {
            kind: Some(FrameKind::Request(request_frame)),
        };

        if let Err(e) = self.send_to_browser(browser_pid, frame).await {
            // Clean up pending request on send failure
            self.pending_requests.remove(&request_id);
            return Err(e);
        }

        // Wait for response with timeout
        match tokio::time::timeout(DEFAULT_REQUEST_TIMEOUT, rx).await {
            Ok(Ok(frame)) => match frame.kind {
                Some(FrameKind::Response(response_frame)) => {
                    debug!("Received response for request {}", request_id);
                    Ok(response_frame)
                }
                Some(FrameKind::Error(error_frame)) => Err(Status::internal(format!(
                    "Browser error: {}",
                    error_frame.message
                ))),
                _ => Err(Status::internal("Unexpected frame kind in response")),
            },
            Ok(Err(_)) => {
                error!("Response channel closed for request {}", request_id);
                Err(Status::internal("Response channel closed"))
            }
            Err(_) => {
                error!("Timeout waiting for response to request {}", request_id);
                // Clean up pending request on timeout
                self.pending_requests.remove(&request_id);
                Err(Status::deadline_exceeded("Request timeout"))
            }
        }
    }

    /// Get metadata from a specific browser
    ///
    /// Sends a GET_METADATA request to the specified native messenger and waits for a response.
    pub async fn get_metadata(&self, browser_pid: u32) -> Result<ResponseFrame, Status> {
        self.send_request(browser_pid, "GET_METADATA", None).await
    }

    /// Generate assets from a specific browser
    ///
    /// Sends a GENERATE_ASSETS request to the specified native messenger and waits for a response.
    pub async fn generate_assets(&self, browser_pid: u32) -> Result<ResponseFrame, Status> {
        self.send_request(browser_pid, "GENERATE_ASSETS", None)
            .await
    }

    /// Generate a snapshot from a specific browser
    ///
    /// Sends a GENERATE_SNAPSHOT request to the specified native messenger and waits for a response.
    pub async fn generate_snapshot(&self, browser_pid: u32) -> Result<ResponseFrame, Status> {
        self.send_request(browser_pid, "GENERATE_SNAPSHOT", None)
            .await
    }

    /// Get the number of currently connected native messengers
    pub async fn connection_count(&self) -> usize {
        let registry = self.registry.read().await;
        registry.len()
    }

    /// Check if the server is running
    pub fn is_server_running() -> bool {
        SERVER_STARTED.load(Ordering::SeqCst)
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
        info!("Received first browser open request");
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
            debug!(
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
            if registry
                .get(&browser_pid)
                .is_some_and(|m| m.host_pid == host_pid)
            {
                registry.remove(&browser_pid);
                info!(
                    "Unregistered native messenger for browser PID {} and host PID {}. Remaining: {}",
                    browser_pid,
                    host_pid,
                    registry.len()
                );
            } else {
                warn!(
                    "Failed to unregister native messenger: browser_pid={} host_pid={} not found or mismatch",
                    browser_pid, host_pid
                );
            }
        });
        let out_stream = ReceiverStream::new(rx_to_client);
        Ok(Response::new(Box::pin(out_stream) as Self::OpenStream))
    }
}
