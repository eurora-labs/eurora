//! Timeline collector service implementation

use eur_activity::{ActivityStrategy, select_strategy_for_process};
use ferrous_focus::{FerrousFocusResult, FocusTracker, FocusedWindow};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::config::CollectorConfig;
use crate::error::{Result, TimelineError};
use crate::storage::TimelineStorage;

/// Service responsible for collecting activities and managing the collection lifecycle
pub struct CollectorService {
    /// Shared storage for timeline data
    storage: Arc<Mutex<TimelineStorage>>,
    /// Current collection task handle
    current_task: Option<JoinHandle<()>>,
    /// Configuration for the collector
    config: CollectorConfig,
    /// Channel for focus events
    focus_sender: Option<mpsc::UnboundedSender<FocusedWindow>>,
    /// Restart attempt counter
    restart_attempts: u32,
}

impl CollectorService {
    /// Create a new collector service
    pub fn new(storage: Arc<Mutex<TimelineStorage>>, config: CollectorConfig) -> Self {
        info!(
            "Creating collector service with interval: {:?}",
            config.collection_interval
        );

        Self {
            storage,
            current_task: None,
            config,
            focus_sender: None,
            restart_attempts: 0,
        }
    }

    /// Start the collection service
    pub async fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Err(TimelineError::AlreadyRunning);
        }

        info!("Starting timeline collection service");

        if self.config.focus_tracking_enabled {
            self.start_with_focus_tracking().await?;
        } else {
            self.start_without_focus_tracking().await?;
        }

        self.restart_attempts = 0;
        Ok(())
    }

    /// Stop the collection service
    pub async fn stop(&mut self) -> Result<()> {
        if !self.is_running() {
            return Err(TimelineError::NotRunning);
        }

        info!("Stopping timeline collection service");

        // Stop the current task
        if let Some(task) = self.current_task.take() {
            task.abort();

            // Wait for the task to finish with a timeout
            match tokio::time::timeout(Duration::from_secs(5), task).await {
                Ok(result) => {
                    if let Err(e) = result {
                        if !e.is_cancelled() {
                            warn!("Collection task ended with error: {}", e);
                        }
                    }
                }
                Err(_) => {
                    warn!("Collection task did not stop within timeout");
                }
            }
        }

        // Clear focus sender
        self.focus_sender = None;

        info!("Timeline collection service stopped");
        Ok(())
    }

    /// Restart the collection service
    pub async fn restart(&mut self) -> Result<()> {
        info!("Restarting timeline collection service");

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
            .map_or(false, |task| !task.is_finished())
    }

    /// Collect activity once using the provided strategy
    pub async fn collect_once(&self, mut strategy: Box<dyn ActivityStrategy>) -> Result<()> {
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
        let activity = eur_activity::Activity::new(
            strategy.get_name().clone(),
            strategy.get_icon().clone(),
            strategy.get_process_name().clone(),
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
        info!("Updating collector configuration");
        self.config = config;
    }

    /// Get collector statistics
    pub fn get_stats(&self) -> CollectorStats {
        CollectorStats {
            is_running: self.is_running(),
            focus_tracking_enabled: self.config.focus_tracking_enabled,
            collection_interval: self.config.collection_interval,
            restart_attempts: self.restart_attempts,
        }
    }

    /// Start collection with focus tracking
    async fn start_with_focus_tracking(&mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<FocusedWindow>();
        self.focus_sender = Some(tx.clone());

        // Start focus tracking thread
        let focus_tx = tx.clone();
        std::thread::spawn(move || {
            let tracker = FocusTracker::new();
            loop {
                info!("Starting focus tracker...");

                let tx_clone = focus_tx.clone();
                let result =
                    tracker.track_focus(|window: FocusedWindow| -> FerrousFocusResult<()> {
                        if let Some(process_name) = &window.process_name {
                            if let Some(window_title) = &window.window_title {
                                // Filter out ignored processes
                                #[cfg(target_os = "windows")]
                                let eurora_process = "eur-tauri.exe";
                                #[cfg(not(target_os = "windows"))]
                                let eurora_process = "eur-tauri";

                                if process_name != eurora_process {
                                    info!("▶ {}: {}", process_name, window_title);
                                    let _ = tx_clone.send(window);
                                }
                            }
                        }
                        Ok(())
                    });

                match result {
                    Ok(_) => {
                        warn!("Focus tracker ended unexpectedly, restarting...");
                    }
                    Err(e) => {
                        error!("Focus tracker crashed with error: {:?}", e);
                        warn!("Restarting focus tracker in 1 second...");
                    }
                }

                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        });

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

                current_collection_task = Some(tokio::spawn(async move {
                    if let Some(process_name) = event.process_name {
                        if let Some(window_title) = event.window_title {
                            let display_name = format!("{}: {}", process_name, window_title);
                            let icon = event.icon.unwrap_or_default();

                            match select_strategy_for_process(&process_name, display_name, icon)
                                .await
                            {
                                Ok(mut strategy) => {
                                    // Collect initial activity
                                    if let Ok(assets) = strategy.retrieve_assets().await {
                                        let activity = eur_activity::Activity::new(
                                            strategy.get_name().clone(),
                                            strategy.get_icon().clone(),
                                            strategy.get_process_name().clone(),
                                            assets,
                                        );

                                        {
                                            let mut storage = storage_clone.lock().await;
                                            storage.add_activity(activity);
                                        }
                                    }

                                    // Start periodic snapshot collection
                                    let mut interval = time::interval(collection_interval);
                                    loop {
                                        interval.tick().await;

                                        match strategy.retrieve_snapshots().await {
                                            Ok(snapshots) => {
                                                if !snapshots.is_empty() {
                                                    let mut storage = storage_clone.lock().await;
                                                    if let Some(current_activity) =
                                                        storage.get_all_activities_mut().back_mut()
                                                    {
                                                        for snapshot in snapshots {
                                                            current_activity
                                                                .snapshots
                                                                .push(snapshot);
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
                    }
                }));
            }
        }));

        Ok(())
    }

    /// Start collection without focus tracking (manual mode)
    async fn start_without_focus_tracking(&mut self) -> Result<()> {
        info!("Starting collection without focus tracking");

        // For now, just create a placeholder task that does periodic cleanup
        let storage = Arc::clone(&self.storage);
        let cleanup_interval = Duration::from_secs(300); // 5 minutes

        self.current_task = Some(tokio::spawn(async move {
            let mut interval = time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                // Perform periodic cleanup
                {
                    let mut storage = storage.lock().await;
                    if storage.needs_cleanup() {
                        storage.force_cleanup();
                    }
                }
            }
        }));

        Ok(())
    }

    /// Handle restart with exponential backoff
    async fn handle_restart_with_backoff(&mut self) -> Result<()> {
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
    use super::*;
    use std::time::Duration;

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
        let config = CollectorConfig {
            focus_tracking_enabled: false,
            collection_interval: Duration::from_millis(100),
            ..Default::default()
        };

        let mut collector = CollectorService::new(storage, config);

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
        let config = CollectorConfig {
            focus_tracking_enabled: false,
            collection_interval: Duration::from_millis(100),
            restart_delay: Duration::from_millis(10),
            ..Default::default()
        };

        let mut collector = CollectorService::new(storage, config);

        // Start and restart
        assert!(collector.start().await.is_ok());
        assert!(collector.restart().await.is_ok());
        assert!(collector.is_running());

        // Clean up
        assert!(collector.stop().await.is_ok());
    }
}
