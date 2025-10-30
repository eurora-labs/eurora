//! Timeline collector service implementation

use chrono::{DateTime, Utc};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use eur_activity::processes::{Eurora, ProcessFunctionality};
use eur_activity::{DefaultStrategy, strategies::ActivityStrategyFunctionality};
use ferrous_focus::{
    FerrousFocusResult, FocusTracker, FocusTrackerConfig, FocusedWindow, IconConfig,
};
use tokio::{
    sync::{Mutex, RwLock, broadcast, mpsc},
    task::JoinHandle,
    time,
};
use tracing::{debug, error, warn};

use crate::{
    ActivityStrategy,
    config::CollectorConfig,
    error::{TimelineError, TimelineResult},
    storage::TimelineStorage,
};

/// Event emitted when focus changes to a new application
#[derive(Debug, Clone)]
pub struct FocusedWindowEvent {
    /// The name of the process that received focus
    pub process_name: String,
    /// The title of the window that received focus
    pub window_title: String,
    /// The icon of the application (if available)
    pub icon: Option<image::RgbaImage>,
    /// Timestamp when the focus change occurred
    pub timestamp: DateTime<Utc>,
}

impl FocusedWindowEvent {
    /// Create a new focus change event
    pub fn new(process_name: String, window_title: String, icon: Option<image::RgbaImage>) -> Self {
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
    /// Focus tracking task handle
    focus_thread_handle: Option<JoinHandle<()>>,
    /// Shutdown signal for focus thread
    focus_shutdown_signal: Option<Arc<AtomicBool>>,
    /// Restart attempt counter
    restart_attempts: u32,
    /// Broadcast channel for focus change events
    focus_event_tx: broadcast::Sender<FocusedWindowEvent>,
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

        self.start_with_focus_tracking().await?;

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
                // Give the task a moment to see the shutdown signal
                tokio::time::sleep(Duration::from_millis(100)).await;

                // Abort the blocking task and wait for it to finish
                thread_handle.abort();
                match thread_handle.await {
                    Ok(()) => {
                        debug!("Focus tracking task stopped gracefully");
                    }
                    Err(e) if e.is_cancelled() => {
                        debug!("Focus tracking task was cancelled");
                    }
                    Err(e) => {
                        warn!("Focus tracking task ended with error: {}", e);
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
            collection_interval: self.config.collection_interval,
            restart_attempts: self.restart_attempts,
        }
    }

    /// Subscribe to focus change events
    pub fn subscribe_to_focus_events(&self) -> broadcast::Receiver<FocusedWindowEvent> {
        self.focus_event_tx.subscribe()
    }

    /// Alternative start implementation
    async fn start_alternative(&mut self) -> TimelineResult<()> {
        let strategy = Arc::new(RwLock::new(ActivityStrategy::DefaultStrategy(
            DefaultStrategy::default(),
        )));
        let strategy_clone = Arc::clone(&strategy);
        tokio::spawn(async move {
            let config =
                FocusTrackerConfig::new().with_icon_config(IconConfig::new().with_size(64));
            let tracker = FocusTracker::with_config(config);
            let mut prev_focus: Option<(String, Option<String>)> = None;
            let _ = tracker
                .track_focus_async(|window: FocusedWindow| async move {
                    if let Some(process_name) = window.process_name {
                        let new_focus = Some((process_name.clone(), window.window_title));
                        let strategy = ActivityStrategy::new(&process_name).await;
                        // if new_focus != prev_focus.clone() {
                        // }
                        // prev_focus = new_focus;
                    }
                    Ok(())
                })
                .await;
        });

        let strategy_clone = Arc::clone(&strategy);
        self.current_task = Some(tokio::spawn(async move {}));

        Ok(())
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
        let thread_handle = tokio::task::spawn_blocking(move || {
            let config =
                FocusTrackerConfig::new().with_icon_config(IconConfig::new().with_size(64));
            let tracker = FocusTracker::with_config(config);

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

                                // Emit focus change event
                                let focus_event = FocusedWindowEvent::new(
                                    process_name.clone(),
                                    window_title.clone(),
                                    window.icon.clone(),
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

                        match ActivityStrategy::new(&process_name).await {
                            Ok(mut strategy) => {
                                // Collect initial activity
                                if let Ok(assets) = strategy.retrieve_assets().await {
                                    // let activity = crate::Activity::new(
                                    //     strategy.get_name().to_string(),
                                    //     "".to_string(),
                                    //     strategy.get_process_name().to_string(),
                                    //     assets,
                                    // );
                                    let activity = crate::Activity::new(
                                        "".to_string(),
                                        "".to_string(),
                                        "".to_string(),
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

        // Cancel focus task if it exists (non-blocking in Drop)
        if let Some(thread_handle) = self.focus_thread_handle.take() {
            thread_handle.abort();
        }
    }
}

/// Statistics about the collector service
#[derive(Debug, Clone)]
pub struct CollectorStats {
    /// Whether the collector is currently running
    pub is_running: bool,
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
