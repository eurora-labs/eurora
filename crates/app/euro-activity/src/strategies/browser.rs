pub use crate::strategies::ActivityStrategyFunctionality;
pub use crate::strategies::processes::*;
use crate::strategies::{ActivityReport, StrategyMetadata};
pub use crate::strategies::{ActivityStrategy, StrategySupport};
use crate::{
    Activity, ActivityError,
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};
use async_trait::async_trait;
use euro_native_messaging::NativeMessage;
use focus_tracker::FocusedWindow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use tokio::sync::{RwLock, mpsc};
use url::Url;

/// Resolves a browser subprocess PID to the root browser process PID.
///
/// Chrome on Windows spawns isolated renderer processes per tab, each with its
/// own PID. The foreground window may belong to any of these subprocesses, but
/// the native messaging host registers under the root `chrome.exe` PID. This
/// function walks the process tree upward to find the topmost ancestor that
/// shares the same executable name (e.g., `chrome.exe`), which is the root
/// browser process.
///
/// On non-Windows platforms, or if the walk fails, returns the PID unchanged.
fn resolve_browser_pid(pid: u32, process_name: &str) -> u32 {
    #[cfg(target_os = "windows")]
    {
        resolve_root_browser_pid_windows(pid, process_name)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = process_name;
        pid
    }
}

#[cfg(target_os = "windows")]
fn resolve_root_browser_pid_windows(pid: u32, process_name: &str) -> u32 {
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::mem::MaybeUninit;
    use std::os::windows::ffi::OsStringExt;

    #[repr(C)]
    #[allow(non_snake_case)]
    struct PROCESSENTRY32W {
        dwSize: u32,
        cntUsage: u32,
        th32ProcessID: u32,
        th32DefaultHeapID: usize,
        th32ModuleID: u32,
        cntThreads: u32,
        th32ParentProcessID: u32,
        pcPriClassBase: i32,
        dwFlags: u32,
        szExeFile: [u16; 260],
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateToolhelp32Snapshot(dwFlags: u32, th32ProcessID: u32) -> isize;
        fn Process32FirstW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        fn Process32NextW(hSnapshot: isize, lppe: *mut PROCESSENTRY32W) -> i32;
        fn CloseHandle(hObject: isize) -> i32;
    }

    const TH32CS_SNAPPROCESS: u32 = 0x00000002;
    const INVALID_HANDLE_VALUE: isize = -1;

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        tracing::warn!("Failed to create process snapshot for browser PID resolution");
        return pid;
    }

    // Build a map of pid -> (parent_pid, exe_name) from the snapshot.
    let mut process_map: HashMap<u32, (u32, String)> = HashMap::new();

    unsafe {
        let mut entry: MaybeUninit<PROCESSENTRY32W> = MaybeUninit::uninit();
        (*entry.as_mut_ptr()).dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, entry.as_mut_ptr()) != 0 {
            loop {
                let e = entry.assume_init_ref();
                let name_len = e
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(e.szExeFile.len());
                let exe_name = OsString::from_wide(&e.szExeFile[..name_len])
                    .to_string_lossy()
                    .into_owned();

                process_map.insert(e.th32ProcessID, (e.th32ParentProcessID, exe_name));

                if Process32NextW(snapshot, entry.as_mut_ptr()) == 0 {
                    break;
                }
            }
        }

        CloseHandle(snapshot);
    }

    // Walk up the process tree while the parent shares the same executable name.
    let mut current = pid;
    let target_name_lower = process_name.to_lowercase();
    loop {
        let Some(&(parent_pid, ref parent_name)) = process_map.get(&current) else {
            break;
        };

        if parent_pid == 0 || parent_pid == current {
            break;
        }

        if parent_name.to_lowercase() != target_name_lower {
            break;
        }

        current = parent_pid;
    }

    if current != pid {
        tracing::debug!(
            "Resolved browser subprocess PID {} to root PID {} ({})",
            pid,
            current,
            process_name
        );
    }

    current
}

pub use euro_browser::{
    BrowserBridgeServer, BrowserBridgeService, EventFrame, Frame, FrameKind, RequestFrame,
    ResponseFrame,
};

#[derive(Clone, Default)]
struct BrowserCache {
    metadata: Option<StrategyMetadata>,
    asset: Option<ActivityAsset>,
    snapshot: Option<ActivitySnapshot>,
}

type BrowserCacheMap = Arc<RwLock<HashMap<u32, BrowserCache>>>;

static CACHE: std::sync::OnceLock<BrowserCacheMap> = std::sync::OnceLock::new();
static CACHE_TASK_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn global_cache() -> &'static BrowserCacheMap {
    CACHE.get_or_init(|| Arc::new(RwLock::new(HashMap::default())))
}

fn start_cache_task(service: &'static BrowserBridgeService) {
    if CACHE_TASK_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let mut events_rx = service.subscribe_to_events();
    let mut disconnects_rx = service.subscribe_to_disconnects();
    let cache = global_cache().clone();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                result = events_rx.recv() => {
                    let (browser_pid, event_frame) = match result {
                        Ok(val) => val,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Cache task lagged by {} events, resuming", n);
                            continue;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    };

                    let Some(payload_str) = event_frame.payload else {
                        continue;
                    };

                    let native_message = match serde_json::from_str::<NativeMessage>(&payload_str) {
                        Ok(msg) => msg,
                        Err(_) => continue,
                    };

                    let mut map = cache.write().await;
                    let entry = map.entry(browser_pid).or_default();

                    match event_frame.action.as_str() {
                        "TAB_ACTIVATED" => {
                            let NativeMessage::NativeMetadata(data) = native_message else {
                                continue;
                            };
                            entry.metadata = Some(StrategyMetadata::from(data));
                        }
                        "ASSETS" => {
                            if let Ok(asset) = ActivityAsset::try_from(native_message) {
                                entry.asset = Some(asset);
                            }
                        }
                        "SNAPSHOT" => {
                            if let Ok(snapshot) = ActivitySnapshot::try_from(native_message) {
                                entry.snapshot = Some(snapshot);
                            }
                        }
                        _ => {}
                    }
                }
                result = disconnects_rx.recv() => {
                    match result {
                        Ok(browser_pid) => {
                            let mut map = cache.write().await;
                            map.remove(&browser_pid);
                            tracing::debug!("Removed cache entry for disconnected browser PID {}", browser_pid);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("Cache disconnect listener lagged by {} events", n);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    });
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct BrowserStrategy {
    #[serde(skip)]
    sender: Option<mpsc::UnboundedSender<ActivityReport>>,

    #[serde(skip)]
    bridge_service: Option<&'static BrowserBridgeService>,

    #[serde(skip)]
    event_subscription_handle: Option<Arc<tokio::task::JoinHandle<()>>>,

    #[serde(skip)]
    active_browser: Option<String>,

    #[serde(skip)]
    active_browser_pid: Arc<AtomicU32>,

    #[serde(skip)]
    last_url: Arc<tokio::sync::Mutex<Option<Url>>>,
}

impl BrowserStrategy {
    async fn initialize_service(&mut self) -> ActivityResult<()> {
        let service = BrowserBridgeService::get_or_init().await;
        self.bridge_service = Some(service);
        start_cache_task(service);
        Ok(())
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

                let Some(payload_str) = event_frame.payload else {
                    continue;
                };

                let native_message = match serde_json::from_str::<NativeMessage>(&payload_str) {
                    Ok(msg) => msg,
                    Err(e) => {
                        tracing::warn!("Failed to parse native message: {}", e);
                        continue;
                    }
                };

                match event_frame.action.as_str() {
                    "TAB_ACTIVATED" => {
                        let NativeMessage::NativeMetadata(data) = native_message else {
                            continue;
                        };
                        let metadata = StrategyMetadata::from(data);

                        let mut prev = last_url.lock().await;
                        let url = match Url::parse(&metadata.url.clone().unwrap_or_default()) {
                            Ok(u) => u,
                            Err(_) => continue,
                        };

                        if let Some(prev_url) = prev.take()
                            && prev_url.domain() == url.domain()
                        {
                            *prev = Some(url);
                            continue;
                        }
                        *prev = Some(url);

                        let icon = metadata.icon.clone();
                        let url_str = metadata.url.clone().unwrap_or_default();

                        let activity = Activity::new(url_str, icon, "".to_string(), vec![]);

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
                    "ASSETS" => {
                        let asset = match ActivityAsset::try_from(native_message) {
                            Ok(a) => a,
                            Err(e) => {
                                tracing::warn!("Failed to convert asset: {}", e);
                                continue;
                            }
                        };

                        tracing::debug!("Received asset from browser PID {}", browser_pid);
                        if sender.send(ActivityReport::Assets(vec![asset])).is_err() {
                            tracing::warn!("Failed to send assets - receiver dropped");
                            break;
                        }
                    }
                    "SNAPSHOT" => {
                        let snapshot = match ActivitySnapshot::try_from(native_message) {
                            Ok(s) => s,
                            Err(e) => {
                                tracing::warn!("Failed to convert snapshot: {}", e);
                                continue;
                            }
                        };

                        tracing::debug!("Received snapshot from browser PID {}", browser_pid);
                        if sender
                            .send(ActivityReport::Snapshots(vec![snapshot]))
                            .is_err()
                        {
                            tracing::warn!("Failed to send snapshots - receiver dropped");
                            break;
                        }
                    }
                    other => {
                        tracing::debug!("Ignoring unknown event action: {}", other);
                    }
                }
            }

            tracing::debug!("Event subscription task ended");
        });

        self.event_subscription_handle = Some(Arc::new(handle));
        Ok(())
    }

    async fn flush_cache(
        pid: u32,
        process_name: &str,
        sender: &mpsc::UnboundedSender<ActivityReport>,
        last_url: &tokio::sync::Mutex<Option<Url>>,
    ) -> bool {
        let cached = {
            let map = global_cache().read().await;
            map.get(&pid).cloned()
        };
        let Some(cached) = cached else { return false };
        if cached.metadata.is_none() {
            return false;
        }

        if let Some(metadata) = cached.metadata {
            if let Some(ref url_str) = metadata.url
                && let Ok(url) = Url::parse(url_str)
            {
                *last_url.lock().await = Some(url);
            }
            let activity = Activity::new(
                metadata.url.unwrap_or_default(),
                metadata.icon,
                process_name.to_string(),
                vec![],
            );
            let _ = sender.send(ActivityReport::NewActivity(activity));
        }
        if let Some(asset) = cached.asset {
            let _ = sender.send(ActivityReport::Assets(vec![asset]));
        }
        if let Some(snapshot) = cached.snapshot {
            let _ = sender.send(ActivityReport::Snapshots(vec![snapshot]));
        }
        true
    }

    pub async fn new() -> ActivityResult<Self> {
        let mut strategy = BrowserStrategy::default();
        strategy.initialize_service().await?;
        Ok(strategy)
    }
}

#[async_trait]
impl StrategySupport for BrowserStrategy {
    fn get_supported_processes() -> Vec<&'static str> {
        vec![Librewolf.get_name(), Firefox.get_name(), Chrome.get_name()]
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
        BrowserStrategy::get_supported_processes().contains(&focus_window.process_name.as_str())
    }

    async fn start_tracking(
        &mut self,
        focus_window: &FocusedWindow,
        sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()> {
        self.sender = Some(sender.clone());
        let process_name = focus_window.process_name.clone();
        let browser_pid =
            resolve_browser_pid(focus_window.process_id, &focus_window.process_name);
        self.active_browser = Some(process_name.clone());
        self.active_browser_pid
            .store(browser_pid, Ordering::Relaxed);

        self.init_collection().await?;

        if !Self::flush_cache(
            browser_pid,
            &process_name,
            &sender,
            &self.last_url,
        )
        .await
        {
            match self.get_metadata().await {
                Ok(metadata) => {
                    if let Some(ref url_str) = metadata.url
                        && let Ok(url) = Url::parse(url_str)
                    {
                        *self.last_url.lock().await = Some(url);
                    }
                    let activity = Activity::new(
                        metadata.url.unwrap_or_default(),
                        metadata.icon,
                        process_name.clone(),
                        vec![],
                    );
                    if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                        tracing::warn!("Failed to send new activity report - receiver dropped");
                    }
                }
                Err(err) => {
                    let activity = Activity::new(
                        focus_window.process_name.clone(),
                        focus_window.icon.clone(),
                        focus_window.process_name.clone(),
                        vec![],
                    );
                    if sender.send(ActivityReport::NewActivity(activity)).is_err() {
                        tracing::warn!("Failed to send new activity report - receiver dropped");
                    }
                    tracing::warn!("Failed to get metadata: {}", err);
                }
            }
        }

        tracing::debug!("Browser strategy starting tracking for: {:?}", process_name);
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

        if self.can_handle_process(focus_window) {
            let browser_pid =
                resolve_browser_pid(focus_window.process_id, &focus_window.process_name);
            self.active_browser_pid
                .store(browser_pid, Ordering::Relaxed);
            self.active_browser = Some(focus_window.process_name.to_string());

            if let Some(sender) = &self.sender {
                Self::flush_cache(
                    browser_pid,
                    &focus_window.process_name,
                    sender,
                    &self.last_url,
                )
                .await;
            }

            Ok(true)
        } else {
            tracing::debug!(
                "Browser strategy cannot handle: {}, stopping tracking",
                focus_window.process_name
            );
            self.stop_tracking().await?;
            Ok(false)
        }
    }

    async fn stop_tracking(&mut self) -> ActivityResult<()> {
        tracing::debug!("Browser strategy stopping tracking");

        self.active_browser = None;
        self.active_browser_pid.store(0, Ordering::Relaxed);

        Ok(())
    }

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>> {
        Ok(vec![])
    }

    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>> {
        Ok(vec![])
    }

    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata> {
        tracing::debug!("Retrieving metadata for browser strategy");

        let service = self
            .bridge_service
            .as_ref()
            .ok_or_else(|| ActivityError::invalid_data("Bridge service not available"))?;

        let browser_pid = self.active_browser_pid.load(Ordering::Relaxed);
        if browser_pid == 0 {
            return Err(ActivityError::invalid_data("No active browser PID set"));
        }

        let response_frame = service
            .get_metadata(browser_pid)
            .await
            .map_err(|e| ActivityError::invalid_data(format!("Failed to get metadata: {}", e)))?;

        let Some(payload) = response_frame.payload else {
            tracing::warn!("No payload in metadata response");
            return Ok(StrategyMetadata::default());
        };

        let native_metadata = serde_json::from_str::<NativeMessage>(&payload)
            .map_err(|e| -> ActivityError { ActivityError::from(e) })?;

        let metadata = match native_metadata {
            NativeMessage::NativeMetadata(metadata) => {
                if let Some(ref url) = metadata.url
                    && !url.starts_with("http")
                    && !url.starts_with("chrome-extension:")
                {
                    return Err(ActivityError::invalid_data(format!(
                        "Invalid metadata URL: must start with 'http', got: {}",
                        url
                    )));
                }
                StrategyMetadata::from(metadata)
            }
            _ => StrategyMetadata::default(),
        };
        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use crate::strategies::*;

    #[test]
    fn test_supported_processes() {
        let processes = BrowserStrategy::get_supported_processes();
        assert!(!processes.is_empty());

        #[cfg(target_os = "windows")]
        assert!(processes.contains(&"firefox.exe"));

        #[cfg(target_os = "linux")]
        assert!(processes.contains(&"firefox"));

        #[cfg(target_os = "macos")]
        assert!(processes.contains(&"Firefox"));
    }
}
