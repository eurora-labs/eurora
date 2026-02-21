use super::proto::{
    EventFrame, Frame, RequestFrame, ResponseFrame, browser_bridge_server::BrowserBridge,
    browser_bridge_server::BrowserBridgeServer, frame::Kind as FrameKind,
};
use dashmap::DashMap;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;
use tokio::sync::{OnceCell, RwLock, broadcast, mpsc, oneshot, watch};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tonic::{Request, Response, Status, transport::Server};

pub const BROWSER_BRIDGE_PORT: &str = "1431";

static GLOBAL_SERVICE: OnceCell<BrowserBridgeService> = OnceCell::const_new();
static SERVER_STARTED: AtomicBool = AtomicBool::new(false);
static SHUTDOWN_TX: OnceCell<watch::Sender<bool>> = OnceCell::const_new();

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

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
            tracing::error!("Failed to send frame to waiting request");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct RegisteredMessenger {
    pub tx: mpsc::Sender<Result<Frame, Status>>,
    pub host_pid: u32,
    pub browser_pid: u32,
}

#[derive(Clone)]
pub struct BrowserBridgeService {
    pub registry: Arc<RwLock<HashMap<u32, RegisteredMessenger>>>,
    pub app_from_tx: broadcast::Sender<Frame>,
    pub frames_from_messengers_tx: broadcast::Sender<(u32, Frame)>,
    events_tx: broadcast::Sender<(u32, EventFrame)>,
    pending_requests: Arc<DashMap<u32, PendingRequest>>,
    request_id_counter: Arc<AtomicU32>,
    frame_handler_handle: Arc<OnceCell<tokio::task::JoinHandle<()>>>,
}

impl BrowserBridgeService {
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

    pub fn start_frame_handler(&self) {
        let pending_requests = Arc::clone(&self.pending_requests);
        let frames_from_messengers_tx = self.frames_from_messengers_tx.clone();
        let events_tx = self.events_tx.clone();
        let mut frames_rx = frames_from_messengers_tx.subscribe();

        let handle = tokio::spawn(async move {
            tracing::debug!("Frame handler task started");
            while let Ok((browser_pid, frame)) = frames_rx.recv().await {
                let kind = match &frame.kind {
                    Some(k) => k.clone(),
                    None => {
                        tracing::warn!(
                            "Received frame with no kind from browser PID {}",
                            browser_pid
                        );
                        continue;
                    }
                };

                match kind {
                    FrameKind::Request(req_frame) => {
                        tracing::debug!(
                            "Received request frame from browser PID {}: id={}, action={}",
                            browser_pid,
                            req_frame.id,
                            req_frame.action
                        );
                        tracing::warn!(
                            "Received unsupported request from browser extension: action={}",
                            req_frame.action
                        );
                    }
                    FrameKind::Response(resp_frame) => {
                        if let Some((_, pending_request)) = pending_requests.remove(&resp_frame.id)
                        {
                            let frame = Frame {
                                kind: Some(FrameKind::Response(resp_frame.clone())),
                            };
                            if let Err(err) = pending_request.send(frame) {
                                tracing::warn!(
                                    "Failed to send frame to waiting request: {:?}",
                                    err
                                );
                            }
                        } else {
                            tracing::debug!(
                                "Received frame with no pending request: id={} action={}",
                                resp_frame.id,
                                resp_frame.action,
                            );
                        }
                    }
                    FrameKind::Event(evt_frame) => {
                        tracing::debug!(
                            "Received event frame from browser PID {}: action={}",
                            browser_pid,
                            evt_frame.action
                        );
                        if let Err(e) = events_tx.send((browser_pid, evt_frame)) {
                            tracing::debug!(
                                "No event subscribers for event frame from browser PID {}: {}",
                                browser_pid,
                                e
                            );
                        }
                    }
                    FrameKind::Error(err_frame) => {
                        tracing::error!(
                            "Received error frame: id={}, message={}",
                            err_frame.id,
                            err_frame.message
                        );
                        if let Some((_, pending_request)) = pending_requests.remove(&err_frame.id) {
                            let frame = Frame {
                                kind: Some(FrameKind::Error(err_frame)),
                            };
                            if let Err(err) = pending_request.send(frame) {
                                tracing::warn!(
                                    "Failed to send error frame to waiting request: {:?}",
                                    err
                                );
                            }
                        }
                    }
                    FrameKind::Cancel(cancel_frame) => {
                        tracing::debug!("Received cancel frame: id={}", cancel_frame.id);
                        if pending_requests.remove(&cancel_frame.id).is_some() {
                            tracing::debug!("Cancelled pending request: id={}", cancel_frame.id);
                        }
                    }
                    FrameKind::Register(_) => {
                        tracing::debug!("Received register frame (should be handled by server)");
                    }
                }
            }
            tracing::debug!("Frame handler task ended");
        });

        let _ = self.frame_handler_handle.set(handle);
    }

    pub async fn get_or_init() -> &'static BrowserBridgeService {
        GLOBAL_SERVICE
            .get_or_init(|| async { BrowserBridgeService::new() })
            .await
    }

    pub async fn start_server(&self) {
        if SERVER_STARTED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            tracing::debug!("Browser Bridge gRPC server already running");
            return;
        }

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        let _ = SHUTDOWN_TX.set(shutdown_tx);

        let service_clone = self.clone();

        tokio::spawn(async move {
            let addr = format!("[::1]:{}", BROWSER_BRIDGE_PORT)
                .to_socket_addrs()
                .expect("Invalid server address")
                .next()
                .expect("No valid socket address");

            tracing::info!("Starting Browser Bridge gRPC server at {}", addr);

            let server = Server::builder()
                .add_service(BrowserBridgeServer::new(service_clone))
                .serve_with_shutdown(addr, async move {
                    loop {
                        if shutdown_rx.changed().await.is_err() {
                            break;
                        }
                        if *shutdown_rx.borrow() {
                            tracing::info!(
                                "Received shutdown signal for Browser Bridge gRPC server"
                            );
                            break;
                        }
                    }
                });

            if let Err(e) = server.await {
                tracing::error!("Browser Bridge gRPC server error: {}", e);
            }

            SERVER_STARTED.store(false, Ordering::SeqCst);
            tracing::info!("Browser Bridge gRPC server ended");
        });
    }

    pub async fn stop_server() {
        if !SERVER_STARTED.load(Ordering::SeqCst) {
            tracing::debug!("Browser Bridge gRPC server is not running");
            return;
        }

        if let Some(tx) = SHUTDOWN_TX.get() {
            tracing::info!("Sending shutdown signal to Browser Bridge gRPC server");
            let _ = tx.send(true);
        }
    }

    pub fn subscribe_to_frames(&self) -> broadcast::Receiver<(u32, Frame)> {
        self.frames_from_messengers_tx.subscribe()
    }

    pub fn subscribe_to_events(&self) -> broadcast::Receiver<(u32, EventFrame)> {
        self.events_tx.subscribe()
    }

    pub async fn is_registered(&self, browser_pid: u32) -> bool {
        let registry = self.registry.read().await;
        registry.contains_key(&browser_pid)
    }

    pub async fn get_registered_pids(&self) -> Vec<u32> {
        let registry = self.registry.read().await;
        registry.keys().copied().collect()
    }

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

    pub async fn send_request(
        &self,
        browser_pid: u32,
        action: &str,
        payload: Option<String>,
    ) -> Result<ResponseFrame, Status> {
        let request_id = self.request_id_counter.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();

        self.pending_requests
            .insert(request_id, PendingRequest::new(tx));

        let request_frame = RequestFrame {
            id: request_id,
            action: action.to_string(),
            payload,
        };

        tracing::debug!(
            "Sending request frame: id={}, action={}, browser_pid={}",
            request_id,
            action,
            browser_pid
        );

        let frame = Frame {
            kind: Some(FrameKind::Request(request_frame)),
        };

        if let Err(e) = self.send_to_browser(browser_pid, frame).await {
            self.pending_requests.remove(&request_id);
            return Err(e);
        }

        match tokio::time::timeout(DEFAULT_REQUEST_TIMEOUT, rx).await {
            Ok(Ok(frame)) => match frame.kind {
                Some(FrameKind::Response(response_frame)) => {
                    tracing::debug!("Received response for request {}", request_id);
                    Ok(response_frame)
                }
                Some(FrameKind::Error(error_frame)) => Err(Status::internal(format!(
                    "Browser error: {}",
                    error_frame.message
                ))),
                _ => Err(Status::internal("Unexpected frame kind in response")),
            },
            Ok(Err(_)) => {
                tracing::error!("Response channel closed for request {}", request_id);
                Err(Status::internal("Response channel closed"))
            }
            Err(_) => {
                tracing::error!("Timeout waiting for response to request {}", request_id);
                self.pending_requests.remove(&request_id);
                Err(Status::deadline_exceeded("Request timeout"))
            }
        }
    }

    pub async fn get_metadata(&self, browser_pid: u32) -> Result<ResponseFrame, Status> {
        self.send_request(browser_pid, "GET_METADATA", None).await
    }

    pub async fn generate_assets(&self, browser_pid: u32) -> Result<ResponseFrame, Status> {
        self.send_request(browser_pid, "GENERATE_ASSETS", None)
            .await
    }

    pub async fn generate_snapshot(&self, browser_pid: u32) -> Result<ResponseFrame, Status> {
        self.send_request(browser_pid, "GENERATE_SNAPSHOT", None)
            .await
    }

    pub async fn connection_count(&self) -> usize {
        let registry = self.registry.read().await;
        registry.len()
    }

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
        tracing::info!("Received first browser open request");
        let mut inbound = request.into_inner();

        let first_frame = inbound.message().await.map_err(|e| {
            tracing::error!("Failed to receive the Register frame as first frame: {}", e);
            Status::internal("Failed to receive the Register frame as first frame")
        })?;

        let Some(frame) = first_frame else {
            tracing::error!("Received an unexpected frame type as the first frame");
            return Err(Status::internal(
                "Received an unexpected frame type as the first frame",
            ));
        };

        let Some(FrameKind::Register(register_frame)) = frame.kind else {
            tracing::error!("Received an unexpected frame type as the first frame");
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
            tracing::debug!(
                "Registered browser with browser_pid: {} and host_pid: {}. Total registered browsers: {}",
                browser_pid,
                host_pid,
                registry.len()
            );
        }
        let registry = self.registry.clone();
        let frames_tx = self.frames_from_messengers_tx.clone();

        tokio::spawn(async move {
            tracing::info!(
                "gRPC client connected, starting forward task: Eurora -> Native Messenger -> Chrome"
            );
            loop {
                match inbound.message().await {
                    Ok(Some(frame)) => {
                        tracing::info!(
                            "Received frame from native messenger browser_pid={}",
                            browser_pid
                        );
                        if let Err(e) = frames_tx.send((browser_pid, frame)) {
                            tracing::warn!(
                                "Failed to broadcast frame from browser PID {}: {}",
                                browser_pid,
                                e
                            );
                        }
                    }
                    Ok(None) => {
                        tracing::info!(
                            "Native messenger disconnected (browser_pid={})",
                            browser_pid
                        );
                        break;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Error receiving frame from native messenger (browser_pid={}): {}",
                            browser_pid,
                            e
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
                tracing::info!(
                    "Unregistered native messenger for browser PID {} and host PID {}. Remaining: {}",
                    browser_pid,
                    host_pid,
                    registry.len()
                );
            } else {
                tracing::warn!(
                    "Failed to unregister native messenger: browser_pid={} host_pid={} not found or mismatch",
                    browser_pid,
                    host_pid
                );
            }
        });
        let out_stream = ReceiverStream::new(rx_to_client);
        Ok(Response::new(Box::pin(out_stream) as Self::OpenStream))
    }
}
