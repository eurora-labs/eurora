//! Timeline collector service implementation

use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Utc};
use image::{ImageBuffer, Rgb, Rgba};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use eur_activity::processes::{Eurora, ProcessFunctionality};
use ferrous_focus::{FerrousFocusResult, FocusTracker, FocusedWindow};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{Mutex, broadcast, mpsc},
    task::JoinHandle,
    time,
};
use tracing::{debug, error, warn};

use crate::{
    ActivityStrategy,
    config::CollectorConfig,
    error::{TimelineError, TimelineResult},
    select_strategy_for_process,
    storage::TimelineStorage,
};

/// Event emitted when focus changes to a new application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusChangeEvent {
    /// The name of the process that received focus
    pub process_name: String,
    /// The title of the window that received focus
    pub window_title: String,
    /// The icon of the application (if available)
    pub icon: Option<String>,
    /// Timestamp when the focus change occurred
    pub timestamp: DateTime<Utc>,
}

pub fn image_to_base64(image: ImageBuffer<Rgba<u8>, Vec<u8>>) -> Result<String> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);

    image
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| anyhow!("Failed to encode image: {}", e))?;

    let base64 = general_purpose::STANDARD.encode(&buffer);
    // let base64 = base64::encode(&buffer);
    Ok(format!("data:image/png;base64,{}", base64))
}

impl FocusChangeEvent {
    /// Create a new focus change event
    pub fn new(process_name: String, window_title: String, icon: Option<String>) -> Self {
        Self {
            process_name,
            window_title,
            icon,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Service responsible for collecting activities and managing the collection lifecycle
pub struct CollectorService {
    /// Shared storage for timeline data
    storage: Arc<Mutex<TimelineStorage>>,
    /// Current collection task handle
    current_task: Option<JoinHandle<()>>,
    /// Configuration for the collector
    config: CollectorConfig,
    /// Focus tracking configuration
    focus_config: crate::config::FocusTrackingConfig,
    /// Channel for focus events
    focus_sender: Option<mpsc::UnboundedSender<FocusedWindow>>,
    /// Focus tracking thread handle
    focus_thread_handle: Option<std::thread::JoinHandle<()>>,
    /// Shutdown signal for focus thread
    focus_shutdown_signal: Option<Arc<AtomicBool>>,
    /// Restart attempt counter
    restart_attempts: u32,
    /// Broadcast channel for focus change events
    focus_event_tx: broadcast::Sender<FocusChangeEvent>,
}

impl CollectorService {
    /// Create a new collector service
    pub fn new(storage: Arc<Mutex<TimelineStorage>>, config: CollectorConfig) -> Self {
        debug!(
            "Creating collector service with interval: {:?}",
            config.collection_interval
        );

        let (focus_event_tx, _) = broadcast::channel(100);

        Self {
            storage,
            current_task: None,
            config,
            focus_config: crate::config::FocusTrackingConfig::default(),
            focus_sender: None,
            focus_thread_handle: None,
            focus_shutdown_signal: None,
            restart_attempts: 0,
            focus_event_tx,
        }
    }

    /// Create a new collector service with full timeline config
    pub fn new_with_timeline_config(
        storage: Arc<Mutex<TimelineStorage>>,
        timeline_config: crate::config::TimelineConfig,
    ) -> Self {
        debug!(
            "Creating collector service with interval: {:?}",
            timeline_config.collector.collection_interval
        );

        let (focus_event_tx, _) = broadcast::channel(100);

        Self {
            storage,
            current_task: None,
            config: timeline_config.collector,
            focus_config: timeline_config.focus_tracking,
            focus_sender: None,
            focus_thread_handle: None,
            focus_shutdown_signal: None,
            restart_attempts: 0,
            focus_event_tx,
        }
    }

    /// Start the collection service
    pub async fn start(&mut self) -> TimelineResult<()> {
        if self.is_running() {
            return Err(TimelineError::AlreadyRunning);
        }

        debug!("Starting timeline collection service");

        if self.focus_config.enabled {
            self.start_with_focus_tracking().await?;
        } else {
            self.start_without_focus_tracking().await?;
        }

        self.restart_attempts = 0;
        Ok(())
    }

    /// Stop the collection service
    pub async fn stop(&mut self) -> TimelineResult<()> {
        if !self.is_running() {
            return Err(TimelineError::NotRunning);
        }

        debug!("Stopping timeline collection service");

        // Stop the current task
        if let Some(task) = self.current_task.take() {
            task.abort();

            // Wait for the task to finish with a timeout
            match tokio::time::timeout(Duration::from_secs(5), task).await {
                Ok(result) => {
                    if let Err(e) = result
                        && !e.is_cancelled()
                    {
                        warn!("Collection task ended with error: {}", e);
                    }
                }
                Err(_) => {
                    warn!("Collection task did not stop within timeout");
                }
            }
        }

        // Stop focus tracking thread
        if let Some(shutdown_signal) = self.focus_shutdown_signal.take() {
            shutdown_signal.store(true, Ordering::Relaxed);

            if let Some(thread_handle) = self.focus_thread_handle.take() {
                // Give the thread a moment to see the shutdown signal
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Join the thread with a timeout
                let join_result = tokio::task::spawn_blocking(move || thread_handle.join()).await;

                match join_result {
                    Ok(Ok(())) => {
                        debug!("Focus tracking thread stopped gracefully");
                    }
                    Ok(Err(_)) => {
                        warn!("Focus tracking thread panicked during shutdown");
                    }
                    Err(_) => {
                        warn!("Timeout waiting for focus tracking thread to stop");
                    }
                }
            }
        }

        // Clear focus sender
        self.focus_sender = None;

        debug!("Timeline collection service stopped");
        Ok(())
    }

    /// Restart the collection service
    pub async fn restart(&mut self) -> TimelineResult<()> {
        debug!("Restarting timeline collection service");

        if self.is_running() {
            self.stop().await?;
        }

        // Add delay before restart if configured
        if !self.config.restart_delay.is_zero() {
            tokio::time::sleep(self.config.restart_delay).await;
        }

        self.start().await
    }

    /// Check if the collector is currently running
    pub fn is_running(&self) -> bool {
        self.current_task
            .as_ref()
            .is_some_and(|task| !task.is_finished())
    }

    /// Collect activity once using the provided strategy
    pub async fn collect_once(&self, mut strategy: ActivityStrategy) -> TimelineResult<()> {
        debug!(
            "Collecting activity once for strategy: {}",
            strategy.get_name()
        );

        // Retrieve initial assets
        let assets = strategy
            .retrieve_assets()
            .await
            .map_err(|e| TimelineError::Collection(format!("Failed to retrieve assets: {}", e)))?;

        // Create activity
        let activity = crate::Activity::new(
            strategy.get_name().to_string(),
            strategy.get_icon().to_string(),
            strategy.get_process_name().to_string(),
            assets,
        );

        // Store the activity
        {
            let mut storage = self.storage.lock().await;
            storage.add_activity(activity);
        }

        debug!("Successfully collected activity: {}", strategy.get_name());
        Ok(())
    }

    /// Update collector configuration
    pub fn update_config(&mut self, config: CollectorConfig) {
        debug!("Updating collector configuration");
        self.config = config;
    }

    /// Update focus tracking configuration
    pub fn update_focus_config(&mut self, focus_config: crate::config::FocusTrackingConfig) {
        debug!("Updating focus tracking configuration");
        self.focus_config = focus_config;
    }

    /// Update configuration from timeline config
    pub fn update_from_timeline_config(&mut self, timeline_config: crate::config::TimelineConfig) {
        debug!("Updating collector from timeline configuration");
        self.config = timeline_config.collector;
        self.focus_config = timeline_config.focus_tracking;
    }

    /// Get collector statistics
    pub fn get_stats(&self) -> CollectorStats {
        CollectorStats {
            is_running: self.is_running(),
            focus_tracking_enabled: self.focus_config.enabled,
            collection_interval: self.config.collection_interval,
            restart_attempts: self.restart_attempts,
        }
    }

    /// Subscribe to focus change events
    pub fn subscribe_to_focus_events(&self) -> broadcast::Receiver<FocusChangeEvent> {
        self.focus_event_tx.subscribe()
    }

    /// Start collection with focus tracking
    async fn start_with_focus_tracking(&mut self) -> TimelineResult<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<FocusedWindow>();
        self.focus_sender = Some(tx.clone());

        // Create shutdown signal
        let shutdown_signal = Arc::new(AtomicBool::new(false));
        self.focus_shutdown_signal = Some(Arc::clone(&shutdown_signal));

        // Start focus tracking thread
        let focus_tx = tx.clone();
        let shutdown_signal_clone = Arc::clone(&shutdown_signal);

        let focus_event_tx_clone = self.focus_event_tx.clone();
        let thread_handle = std::thread::spawn(move || {
            let tracker = FocusTracker::new();

            while !shutdown_signal_clone.load(Ordering::Relaxed) {
                debug!("Starting focus tracker...");

                let tx_clone = focus_tx.clone();
                let shutdown_check = Arc::clone(&shutdown_signal_clone);
                let focus_event_tx_inner = focus_event_tx_clone.clone();

                let result =
                    tracker.track_focus(|window: FocusedWindow| -> FerrousFocusResult<()> {
                        // Check shutdown signal before processing
                        if shutdown_check.load(Ordering::Relaxed) {
                            return Ok(());
                        }

                        if let Some(process_name) = &window.process_name
                            && let Some(window_title) = &window.window_title
                        {
                            // Filter out ignored processes
                            if process_name != Eurora.get_name() {
                                debug!("▶ {}: {}", process_name, window_title);

                                let icon_base64 = match window.icon.clone() {
                                    Some(icon) => Some(image_to_base64(icon).unwrap_or_default()),
                                    None => None,
                                };

                                // Emit focus change event
                                let focus_event = FocusChangeEvent::new(
                                    process_name.clone(),
                                    window_title.clone(),
                                    icon_base64.clone(),
                                );

                                // Broadcast the focus change event (ignore errors if no listeners)
                                let _ = focus_event_tx_inner.send(focus_event);

                                let _ = tx_clone.send(window);
                            }
                        }
                        Ok(())
                    });

                match result {
                    Ok(_) => {
                        if !shutdown_signal_clone.load(Ordering::Relaxed) {
                            warn!("Focus tracker ended unexpectedly, restarting...");
                        }
                    }
                    Err(e) => {
                        if !shutdown_signal_clone.load(Ordering::Relaxed) {
                            error!("Focus tracker crashed with error: {:?}", e);
                            warn!("Restarting focus tracker in 1 second...");
                        }
                    }
                }

                // Only sleep if we're not shutting down
                if !shutdown_signal_clone.load(Ordering::Relaxed) {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }

            debug!("Focus tracking thread shutting down gracefully");
        });

        self.focus_thread_handle = Some(thread_handle);

        // Start collection task
        let storage = Arc::clone(&self.storage);
        let config = self.config.clone();

        self.current_task = Some(tokio::spawn(async move {
            let mut current_collection_task: Option<JoinHandle<()>> = None;

            while let Some(event) = rx.recv().await {
                // Stop previous collection task
                if let Some(task) = current_collection_task.take() {
                    task.abort();
                }

                // Start new collection task for this focus event
                let storage_clone = Arc::clone(&storage);
                let collection_interval = config.collection_interval;
                let shutdown_signal_clone = Arc::clone(&shutdown_signal);

                current_collection_task = Some(tokio::spawn(async move {
                    if let Some(process_name) = event.process_name
                        && let Some(window_title) = event.window_title
                    {
                        let display_name = format!("{}: {}", process_name, window_title);
                        let icon = event.icon.unwrap_or_default();

                        match select_strategy_for_process(&process_name, display_name, icon).await {
                            Ok(mut strategy) => {
                                // Collect initial activity
                                if let Ok(assets) = strategy.retrieve_assets().await {
                                    let activity = crate::Activity::new(
                                        strategy.get_name().to_string(),
                                        strategy.get_icon().to_string(),
                                        strategy.get_process_name().to_string(),
                                        assets,
                                    );

                                    {
                                        let mut storage = storage_clone.lock().await;
                                        storage.add_activity(activity);
                                    }
                                }

                                // Start periodic snapshot collection
                                let mut interval = time::interval(collection_interval);
                                while !shutdown_signal_clone.load(Ordering::Relaxed) {
                                    interval.tick().await;

                                    match strategy.retrieve_snapshots().await {
                                        Ok(snapshots) => {
                                            if !snapshots.is_empty() {
                                                let mut storage = storage_clone.lock().await;
                                                if let Some(current_activity) =
                                                    storage.get_all_activities_mut().back_mut()
                                                {
                                                    for snapshot in snapshots {
                                                        current_activity.snapshots.push(snapshot);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            debug!("Failed to retrieve snapshots: {:?}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Failed to create strategy for process {}: {}",
                                    process_name, e
                                );
                            }
                        }
                    }
                }));
            }
        }));

        Ok(())
    }

    /// Start collection without focus tracking (manual mode)
    async fn start_without_focus_tracking(&mut self) -> TimelineResult<()> {
        debug!("Starting collection without focus tracking");

        // Create shutdown signal for the cleanup task
        let shutdown_signal = Arc::new(AtomicBool::new(false));

        // For now, just create a placeholder task that does periodic cleanup
        let storage = Arc::clone(&self.storage);
        let cleanup_interval = Duration::from_secs(300); // 5 minutes

        self.current_task = Some(tokio::spawn(async move {
            let mut interval = time::interval(cleanup_interval);

            while !shutdown_signal.load(Ordering::Relaxed) {
                tokio::select! {
                    _ = interval.tick() => {
                        // Perform periodic cleanup
                        {
                            let mut storage = storage.lock().await;
                            if storage.needs_cleanup() {
                                storage.force_cleanup();
                            }
                        }
                    }
                    _ = tokio::time::sleep(Duration::from_millis(100)) => {
                        // Check shutdown signal more frequently
                        if shutdown_signal.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                }
            }

            debug!("Cleanup task shutting down gracefully");
        }));

        Ok(())
    }

    /// Handle restart with exponential backoff
    #[allow(dead_code)]
    async fn handle_restart_with_backoff(&mut self) -> TimelineResult<()> {
        if !self.config.auto_restart_on_error {
            return Err(TimelineError::Collection(
                "Auto-restart is disabled".to_string(),
            ));
        }

        if self.restart_attempts >= self.config.max_restart_attempts {
            return Err(TimelineError::Collection(format!(
                "Maximum restart attempts ({}) exceeded",
                self.config.max_restart_attempts
            )));
        }

        self.restart_attempts += 1;

        // Exponential backoff
        let delay = self.config.restart_delay * (2_u32.pow(self.restart_attempts - 1));
        warn!(
            "Restarting collector service in {:?} (attempt {})",
            delay, self.restart_attempts
        );

        tokio::time::sleep(delay).await;
        self.restart().await
    }
}

impl Drop for CollectorService {
    fn drop(&mut self) {
        if let Some(task) = self.current_task.take() {
            task.abort();
        }

        // Signal focus thread to shutdown
        if let Some(shutdown_signal) = &self.focus_shutdown_signal {
            shutdown_signal.store(true, Ordering::Relaxed);
        }

        // Join focus thread if it exists
        if let Some(thread_handle) = self.focus_thread_handle.take() {
            // Give it a brief moment to see the shutdown signal
            std::thread::sleep(Duration::from_millis(50));
            let _ = thread_handle.join();
        }
    }
}

/// Statistics about the collector service
#[derive(Debug, Clone)]
pub struct CollectorStats {
    /// Whether the collector is currently running
    pub is_running: bool,
    /// Whether focus tracking is enabled
    pub focus_tracking_enabled: bool,
    /// Collection interval
    pub collection_interval: Duration,
    /// Number of restart attempts
    pub restart_attempts: u32,
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_collector_creation() {
        let storage = Arc::new(Mutex::new(TimelineStorage::default()));
        let config = CollectorConfig::default();

        let collector = CollectorService::new(storage, config);
        assert!(!collector.is_running());
    }

    #[tokio::test]
    async fn test_collector_lifecycle() {
        let storage = Arc::new(Mutex::new(TimelineStorage::default()));
        let timeline_config = crate::config::TimelineConfig {
            collector: CollectorConfig {
                collection_interval: Duration::from_millis(100),
                ..Default::default()
            },
            focus_tracking: crate::config::FocusTrackingConfig {
                enabled: false, // Disable focus tracking for tests
                ..Default::default()
            },
            ..Default::default()
        };

        let mut collector = CollectorService::new_with_timeline_config(storage, timeline_config);

        // Start collector
        assert!(collector.start().await.is_ok());
        assert!(collector.is_running());

        // Try to start again (should fail)
        assert!(collector.start().await.is_err());

        // Stop collector
        assert!(collector.stop().await.is_ok());
        assert!(!collector.is_running());

        // Try to stop again (should fail)
        assert!(collector.stop().await.is_err());
    }

    #[tokio::test]
    async fn test_collector_restart() {
        let storage = Arc::new(Mutex::new(TimelineStorage::default()));
        let timeline_config = crate::config::TimelineConfig {
            collector: CollectorConfig {
                collection_interval: Duration::from_millis(100),
                restart_delay: Duration::from_millis(10),
                ..Default::default()
            },
            focus_tracking: crate::config::FocusTrackingConfig {
                enabled: false, // Disable focus tracking for tests
                ..Default::default()
            },
            ..Default::default()
        };

        let mut collector = CollectorService::new_with_timeline_config(storage, timeline_config);

        // Start and restart
        assert!(collector.start().await.is_ok());
        assert!(collector.restart().await.is_ok());
        assert!(collector.is_running());

        // Clean up
        assert!(collector.stop().await.is_ok());
    }
}
