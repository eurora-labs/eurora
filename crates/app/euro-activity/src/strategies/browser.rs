use async_trait::async_trait;
pub use euro_bridge::{BridgeService, EventFrame, Frame, FrameKind, RequestFrame, ResponseFrame};
use euro_bridge::{BridgeError, Payload};
use euro_browser::{NativeMessage, NativeMetadata};
use euro_process::Browser;
use focus_tracker::FocusedWindow;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicU32, Ordering},
};
use thread_core::{ToolBackendCall, ToolErrorWire, WireToolDescriptor};
use tokio::sync::mpsc;
use url::Url;

pub use crate::strategies::ActivityStrategyFunctionality;
use crate::strategies::{ActivityReport, StrategyMetadata};
pub use crate::strategies::{ActivityStrategy, StrategySupport};
use crate::{Activity, ActivityError, error::ActivityResult};

/// Bridge action the extension answers with the active tab's tool list.
const ACTION_LIST_TOOLS: &str = "LIST_TOOLS";

/// Bridge action used to run one tool on the active tab.
const ACTION_INVOKE_TOOL: &str = "INVOKE_TOOL";

/// Bridge action used to abort an in-flight `INVOKE_TOOL`. The extension
/// dispatches it to the same tab and the content-script tool framework
/// aborts the matching `AbortController` by `call_id`.
const ACTION_CANCEL_TOOL: &str = "CANCEL_TOOL";

/// Bridge action used to fetch fresh tab metadata (URL, title, icon,
/// `tab_id`) from the extension. Reused by `get_context` /
/// `dispatch_tool` to pull the active `tab_id` at call time rather
/// than maintaining a desktop-side cache that could drift from the
/// browser's true focus.
const ACTION_GET_METADATA: &str = "GET_METADATA";

/// Wire payload returned by the extension for [`ACTION_LIST_TOOLS`]. The
/// shape stays in sync with `apps/browser/src/shared/background/native-messenger.ts`.
#[derive(Debug, Deserialize)]
struct ListToolsPayload {
    #[serde(default)]
    tools: Vec<WireToolDescriptor>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BrowserStrategy {
    #[serde(skip)]
    sender: Option<mpsc::UnboundedSender<ActivityReport>>,

    #[serde(skip)]
    bridge_service: Option<&'static BridgeService>,

    #[serde(skip)]
    event_subscription_handle: Option<Arc<tokio::task::JoinHandle<()>>>,

    /// Name of the focused browser process. Shared with the event-listening
    /// task so that `Activity` records emitted from tab events carry the
    /// correct `process_name` instead of an empty placeholder.
    #[serde(skip)]
    active_browser: Arc<RwLock<Option<String>>>,

    #[serde(skip)]
    active_browser_pid: Arc<AtomicU32>,

    #[serde(skip)]
    last_url: Arc<tokio::sync::Mutex<Option<Url>>>,
}

impl BrowserStrategy {
    async fn initialize_service(&mut self) -> ActivityResult<()> {
        let service = BridgeService::get_or_init();
        self.bridge_service = Some(service);
        Ok(())
    }

    fn require_service(&self) -> ActivityResult<&'static BridgeService> {
        self.bridge_service
            .ok_or_else(|| ActivityError::invalid_data("Bridge service not available"))
    }

    async fn init_collection(&mut self) -> ActivityResult<()> {
        if self.event_subscription_handle.is_some() {
            return Ok(());
        }

        let Some(sender) = self.sender.clone() else {
            return Err(ActivityError::Strategy(
                "Sender not initialized".to_string(),
            ));
        };

        let service = self
            .bridge_service
            .as_ref()
            .ok_or_else(|| ActivityError::Strategy("Bridge service not initialized".to_string()))?;

        let mut events_rx = service.subscribe_to_events();
        let last_url = Arc::clone(&self.last_url);
        let active_pid = Arc::clone(&self.active_browser_pid);
        let active_browser = Arc::clone(&self.active_browser);

        let handle = tokio::spawn(async move {
            let last_url = Arc::clone(&last_url);

            loop {
                let (browser_pid, event_frame) = match events_rx.recv().await {
                    Ok(val) => val,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Event subscription lagged by {} events, resuming", n);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };

                let expected_pid = active_pid.load(Ordering::Relaxed);
                if expected_pid == 0 || browser_pid != expected_pid {
                    continue;
                }

                let Some(payload_value) = event_frame.payload else {
                    continue;
                };

                let native_message = match payload_value.deserialize::<NativeMessage>() {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::warn!("Failed to parse native message: {}", e);
                        continue;
                    }
                };

                if event_frame.action.as_str() == "TAB_ACTIVATED"
                    || event_frame.action.as_str() == "TAB_UPDATED"
                {
                    let NativeMessage::NativeMetadata(data) = native_message;
                    let metadata = StrategyMetadata::from(data);

                    let Some(url) = metadata.url else {
                        tracing::debug!("Ignoring TAB_ACTIVATED event without a parseable URL");
                        continue;
                    };

                    let mut prev = last_url.lock().await;
                    if let Some(prev_url) = prev.take()
                        && prev_url.domain() == url.domain()
                    {
                        let title = metadata.title.unwrap_or_else(|| url.to_string());
                        *prev = Some(url.clone());
                        let _ = sender.send(ActivityReport::TitleUpdated { title, url });
                        continue;
                    }
                    *prev = Some(url.clone());

                    let process_name = active_browser
                        .read()
                        .ok()
                        .and_then(|guard| guard.clone())
                        .unwrap_or_default();

                    let activity = Activity::new_browser(
                        url,
                        metadata.title,
                        metadata.icon,
                        process_name,
                        browser_pid,
                    );

                    tracing::info!(
                        "Creating new activity from event: browser_pid={}, name={}",
                        browser_pid,
                        activity.name
                    );
                    if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                        tracing::warn!("Failed to send new activity report - receiver dropped");
                        break;
                    }
                }
            }

            tracing::debug!("Event subscription task ended");
        });

        self.event_subscription_handle = Some(Arc::new(handle));
        Ok(())
    }

    /// Round-trip `GET_METADATA` to the active browser messenger and
    /// return the raw `NativeMetadata`. Centralised so `get_metadata`,
    /// `get_context`, and `dispatch_tool` all share the same probe —
    /// `tab_id` resolution is one place, not three.
    async fn fetch_native_metadata(&self) -> ActivityResult<NativeMetadata> {
        let service = self.require_service()?;

        let browser_pid = self.active_browser_pid.load(Ordering::Relaxed);
        if browser_pid == 0 {
            return Err(ActivityError::invalid_data("No active browser PID set"));
        }

        let response_frame = service
            .send_request(browser_pid, ACTION_GET_METADATA, None)
            .await
            .map_err(|e| ActivityError::invalid_data(format!("Failed to get metadata: {}", e)))?;

        let payload = response_frame
            .payload
            .ok_or_else(|| ActivityError::invalid_data("Metadata response contained no payload"))?;

        let native_message: NativeMessage = payload.deserialize().map_err(|e| {
            ActivityError::invalid_data(format!("Failed to decode metadata: {}", e))
        })?;

        let NativeMessage::NativeMetadata(metadata) = native_message;
        Ok(metadata)
    }

    /// `(service, pid, metadata)` triple for the active tab, suitable
    /// for follow-up bridge requests. Returns `None` when the bridge
    /// service isn't initialised, no browser is being tracked, or the
    /// metadata probe fails — callers downgrade gracefully (the chat
    /// surface just sees no tools).
    async fn fetch_active_tab(&self) -> Option<(&'static BridgeService, u32, NativeMetadata)> {
        let service = self.bridge_service?;
        let pid = self.active_browser_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return None;
        }
        match self.fetch_native_metadata().await {
            Ok(metadata) => Some((service, pid, metadata)),
            Err(err) => {
                tracing::debug!("active-tab probe failed: {err}");
                None
            }
        }
    }

    /// Best-effort `CANCEL_TOOL` to the extension. Failures are logged
    /// and ignored — the desktop has already resolved the call as
    /// cancelled; the bridge frame is purely so the content-script
    /// handler can abort its in-flight work.
    async fn fire_cancel_tool(&self, pid: u32, tab_id: i32, call_id: u32) {
        let Some(service) = self.bridge_service else {
            return;
        };
        let payload =
            match Payload::from_value(&json!({ "tab_id": tab_id, "call_id": call_id })) {
                Ok(p) => p,
                Err(err) => {
                    tracing::debug!("CANCEL_TOOL payload encode failed: {err}");
                    return;
                }
            };
        if let Err(err) = service
            .send_request(pid, ACTION_CANCEL_TOOL, Some(payload))
            .await
        {
            tracing::debug!("CANCEL_TOOL bridge call failed (best-effort): {err}");
        }
    }

    async fn resolve_messenger_pid(&self, process_name: &str, fallback_pid: u32) -> u32 {
        if let Some(service) = &self.bridge_service
            && let Some(pid) = service.find_pid_by_app_name(process_name)
        {
            return pid;
        }
        fallback_pid
    }

    /// Refresh `active_browser` / `active_browser_pid` for `focus_window`, fetch
    /// fresh metadata, and emit a `NewActivity` report. Falls back to a
    /// process-level activity when the bridge has no URL for us yet.
    ///
    /// Assumes `self.sender` is already populated (i.e. `start_tracking` ran).
    async fn emit_activity_for_focus(
        &mut self,
        focus_window: &FocusedWindow,
    ) -> ActivityResult<()> {
        let process_name = focus_window.process_name.clone();
        let focus_pid = focus_window.process_id;

        let messenger_pid = self.resolve_messenger_pid(&process_name, focus_pid).await;
        self.active_browser_pid
            .store(messenger_pid, Ordering::Relaxed);
        if let Ok(mut guard) = self.active_browser.write() {
            *guard = Some(process_name.clone());
        }

        let sender = self
            .sender
            .clone()
            .ok_or_else(|| ActivityError::Strategy("Sender not initialized".to_string()))?;

        let activity = match self.get_metadata().await {
            Ok(metadata) => match metadata.url {
                Some(url) => {
                    *self.last_url.lock().await = Some(url.clone());
                    Activity::new_browser(
                        url,
                        metadata.title,
                        metadata.icon,
                        process_name.clone(),
                        focus_pid,
                    )
                }
                None => {
                    tracing::warn!(
                        "Browser metadata arrived without a URL; emitting process-level fallback for {}",
                        process_name
                    );
                    *self.last_url.lock().await = None;
                    Activity::new(
                        process_name.clone(),
                        None,
                        focus_window.icon.clone(),
                        process_name.clone(),
                        focus_pid,
                    )
                }
            },
            Err(err) => {
                tracing::warn!("Failed to get browser metadata: {}", err);
                *self.last_url.lock().await = None;
                Activity::new(
                    process_name.clone(),
                    None,
                    focus_window.icon.clone(),
                    process_name.clone(),
                    focus_pid,
                )
            }
        };

        if sender.send(ActivityReport::NewActivity(activity)).is_err() {
            tracing::warn!("Failed to send new activity report - receiver dropped");
        }

        Ok(())
    }

    pub async fn new() -> ActivityResult<Self> {
        let mut strategy = BrowserStrategy::default();
        strategy.initialize_service().await?;
        Ok(strategy)
    }
}

#[async_trait]
impl StrategySupport for BrowserStrategy {
    fn matches_process(process_name: &str) -> bool {
        Browser::from_process_name(process_name).is_some()
    }

    async fn create() -> ActivityResult<ActivityStrategy> {
        Ok(ActivityStrategy::BrowserStrategy(
            BrowserStrategy::new().await?,
        ))
    }
}

#[async_trait]
impl ActivityStrategyFunctionality for BrowserStrategy {
    fn can_handle_process(&self, focus_window: &FocusedWindow) -> bool {
        BrowserStrategy::matches_process(&focus_window.process_name)
    }

    async fn start_tracking(
        &mut self,
        focus_window: &FocusedWindow,
        sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        self.sender = Some(sender);
        self.init_collection().await?;
        self.emit_activity_for_focus(focus_window).await?;

        tracing::debug!(
            "Browser strategy starting tracking for: {}",
            focus_window.process_name
        );
        Ok(())
    }

    async fn handle_process_change(
        &mut self,
        focus_window: &FocusedWindow,
    ) -> ActivityResult<bool> {
        tracing::debug!(
            "Browser strategy handling process change to: {}",
            focus_window.process_name
        );

        if !self.can_handle_process(focus_window) {
            tracing::debug!(
                "Browser strategy cannot handle: {}, stopping tracking",
                focus_window.process_name
            );
            self.stop_tracking().await?;
            return Ok(false);
        }

        let already_active = self
            .active_browser
            .read()
            .ok()
            .and_then(|guard| guard.clone())
            .as_deref()
            == Some(focus_window.process_name.as_str());
        if already_active {
            return Ok(true);
        }

        self.emit_activity_for_focus(focus_window).await?;
        Ok(true)
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        tracing::debug!("Browser strategy stopping tracking");

        if let Ok(mut guard) = self.active_browser.write() {
            *guard = None;
        }
        self.active_browser_pid.store(0, Ordering::Relaxed);

        Ok(())
    }

    async fn get_metadata(&self) -> ActivityResult<StrategyMetadata> {
        tracing::debug!("Retrieving metadata for browser strategy");

        let metadata = self.fetch_native_metadata().await?;
        let strategy_metadata = StrategyMetadata::from(metadata);

        if let Some(ref url) = strategy_metadata.url
            && !matches!(url.scheme(), "http" | "https" | "chrome-extension")
        {
            return Err(ActivityError::invalid_data(format!(
                "Unsupported metadata URL scheme: {}",
                url.scheme()
            )));
        }

        Ok(strategy_metadata)
    }

    /// Ask the active browser's content script which tools the LLM
    /// should see right now. Two round trips per call by design:
    /// `GET_METADATA` resolves the active tab id from the browser at
    /// call time (so the strategy never caches a tab id that could
    /// drift from the user's true focus), then `LIST_TOOLS { tab_id }`
    /// asks the matching content-script watcher for its descriptors.
    async fn get_context(&self) -> ActivityResult<Vec<WireToolDescriptor>> {
        let Some((service, pid, metadata)) = self.fetch_active_tab().await else {
            return Ok(Vec::new());
        };
        let payload = match Payload::from_value(&json!({ "tab_id": metadata.tab_id })) {
            Ok(p) => p,
            Err(err) => {
                tracing::warn!("LIST_TOOLS payload encode failed: {err}");
                return Ok(Vec::new());
            }
        };
        let response = match service.send_request(pid, ACTION_LIST_TOOLS, Some(payload)).await {
            Ok(resp) => resp,
            Err(BridgeError::NotFound { .. }) | Err(BridgeError::ChannelClosed) => {
                // The browser messenger disconnected between focus and
                // turn start. The LLM simply sees no tools this turn.
                return Ok(Vec::new());
            }
            Err(err) => {
                tracing::warn!("LIST_TOOLS bridge call failed: {err}");
                return Ok(Vec::new());
            }
        };

        let Some(payload) = response.payload else {
            return Ok(Vec::new());
        };

        match payload.deserialize::<ListToolsPayload>() {
            Ok(payload) => Ok(payload.tools),
            Err(err) => {
                tracing::warn!("LIST_TOOLS payload decode failed: {err}");
                Ok(Vec::new())
            }
        }
    }

    async fn dispatch_tool(&self, call: ToolBackendCall) -> Result<Value, ToolErrorWire> {
        let service = self
            .bridge_service
            .ok_or_else(|| ToolErrorWire::Transport {
                message: "bridge service not initialized".to_string(),
            })?;

        let pid = self.active_browser_pid.load(Ordering::Relaxed);
        if pid == 0 {
            return Err(ToolErrorWire::ContextUnavailable {
                tool: call.name,
                reason: "no active browser messenger".to_string(),
            });
        }

        // Resolve the active tab freshly per call — the user may have
        // switched tabs between LLM rounds; whichever tab is focused
        // *now* is where the tool runs.
        let tab_id = match self.fetch_native_metadata().await {
            Ok(metadata) => metadata.tab_id,
            Err(err) => {
                return Err(ToolErrorWire::ContextUnavailable {
                    tool: call.name,
                    reason: format!("failed to resolve active tab: {err}"),
                });
            }
        };

        let payload_value = json!({
            "tab_id": tab_id,
            "call_id": call.call_id,
            "name": call.name,
            "arguments": call.arguments,
        });
        let payload = Payload::from_value(&payload_value).map_err(|err| ToolErrorWire::Encode {
            message: err.to_string(),
        })?;

        let invoke_fut = service.send_request(pid, ACTION_INVOKE_TOOL, Some(payload));

        let response = tokio::select! {
            _ = call.cancel.cancelled() => {
                // Best-effort: tell the extension to abort the matching
                // content-script handler. The bridge's dropped-future
                // cleanup also removes the pending request entry, so a
                // late response from the browser is discarded if our
                // CANCEL_TOOL frame doesn't beat it.
                self.fire_cancel_tool(pid, tab_id, call.call_id).await;
                return Err(ToolErrorWire::Cancelled);
            }
            res = invoke_fut => res.map_err(|err| map_bridge_error(&call.name, err))?,
        };

        let Some(payload) = response.payload else {
            return Err(ToolErrorWire::Decode {
                message: format!("INVOKE_TOOL response for `{}` was empty", call.name),
            });
        };

        // The extension answers with `Result<Value, ToolErrorWire>` shape:
        // a successful invocation returns the tool's value directly; a
        // failure returns a `ToolErrorWire`-shaped object so the LLM sees
        // the original error verbatim instead of a transport-wrapped one.
        let body: Value = payload.deserialize().map_err(|err| ToolErrorWire::Decode {
            message: format!("INVOKE_TOOL payload decode failed: {err}"),
        })?;
        decode_invoke_response(&call.name, body)
    }
}

/// The extension wraps its reply in `{"ok": <value>}` on success and
/// `{"err": <ToolErrorWire>}` on failure so the discriminator is
/// explicit. Any other shape is treated as a transport-side decode error.
fn decode_invoke_response(tool: &str, body: Value) -> Result<Value, ToolErrorWire> {
    let Value::Object(mut obj) = body else {
        return Err(ToolErrorWire::Decode {
            message: format!("INVOKE_TOOL response for `{tool}` was not an object"),
        });
    };

    if let Some(value) = obj.remove("ok") {
        return Ok(value);
    }
    if let Some(err_value) = obj.remove("err") {
        return Err(serde_json::from_value::<ToolErrorWire>(err_value).unwrap_or_else(|err| {
            ToolErrorWire::Decode {
                message: format!("INVOKE_TOOL `{tool}` error decode failed: {err}"),
            }
        }));
    }

    Err(ToolErrorWire::Decode {
        message: format!("INVOKE_TOOL response for `{tool}` missing `ok`/`err`"),
    })
}

/// Translate a bridge transport error into the wire-side tool error.
/// `NotFound` / `ChannelClosed` mean the messenger has gone away — surface
/// as `ContextUnavailable` so the LLM treats the capability as stale.
fn map_bridge_error(tool: &str, err: BridgeError) -> ToolErrorWire {
    match err {
        BridgeError::NotFound { .. } | BridgeError::ChannelClosed => {
            ToolErrorWire::ContextUnavailable {
                tool: tool.to_string(),
                reason: "browser bridge client disconnected".to_string(),
            }
        }
        BridgeError::Timeout => ToolErrorWire::Timeout,
        BridgeError::Client {
            code,
            message,
            details,
        } => ToolErrorWire::Remote {
            code,
            message,
            details: details.and_then(|p| p.deserialize().ok()),
        },
        other => ToolErrorWire::Transport {
            message: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::strategies::*;

    #[test]
    fn matches_known_browser_per_target_os() {
        #[cfg(target_os = "windows")]
        assert!(BrowserStrategy::matches_process("firefox.exe"));

        #[cfg(target_os = "linux")]
        assert!(BrowserStrategy::matches_process("firefox"));

        #[cfg(target_os = "macos")]
        assert!(BrowserStrategy::matches_process("Firefox"));
    }

    #[test]
    fn does_not_match_unknown_process() {
        assert!(!BrowserStrategy::matches_process(""));
        assert!(!BrowserStrategy::matches_process("not-a-browser"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_dispatch_is_case_insensitive() {
        assert!(BrowserStrategy::matches_process("FIREFOX.EXE"));
    }
}
