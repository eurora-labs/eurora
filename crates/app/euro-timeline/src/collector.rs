//! Timeline collector service implementation
use crate::{
    ActivityStrategy,
    error::{TimelineError, TimelineResult},
    storage::TimelineStorage,
    types::ActivityEvent,
};
use euro_activity::DefaultStrategy;
use euro_activity::strategies::ActivityReport;
use euro_activity::{ContextChip, strategies::ActivityStrategyFunctionality};
use focus_tracker::{FocusTracker, FocusTrackerConfig, FocusedWindow, IconConfig};
use log::{debug, error};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    sync::{Mutex, RwLock, broadcast, mpsc},
    task::JoinHandle,
};

/// Service responsible for collecting activities and managing the collection lifecycle
pub struct CollectorService {
    /// Shared storage for timeline data
    storage: Arc<Mutex<TimelineStorage>>,
    /// Current collection task handle
    current_task: Option<JoinHandle<()>>,
    /// Focus tracking task handle
    focus_thread_handle: Option<JoinHandle<()>>,
    /// Shutdown signal for focus thread
    focus_shutdown_signal: Option<Arc<AtomicBool>>,
    /// Broadcast channel for focus change events
    activity_event_tx: broadcast::Sender<ActivityEvent>,
    /// Broadcast channel for new assets event
    assets_event_tx: broadcast::Sender<Vec<ContextChip>>,
}

impl CollectorService {
    /// Create a new collector service with full timeline config
    pub fn new_with_timeline_config(
        storage: Arc<Mutex<TimelineStorage>>,
        timeline_config: crate::config::TimelineConfig,
    ) -> Self {
        debug!(
            "Creating collector service with interval: {:?}",
            timeline_config.collector.collection_interval
        );

        let (activity_event_tx, _) = broadcast::channel(100);
        let (assets_event_tx, _) = broadcast::channel(100);

        Self {
            storage,
            current_task: None,
            focus_thread_handle: None,
            focus_shutdown_signal: None,
            activity_event_tx,
            assets_event_tx,
        }
    }

    /// Start the collection service
    pub async fn start(&mut self) -> TimelineResult<()> {
        if self.is_running() {
            return Err(TimelineError::AlreadyRunning);
        }

        debug!("Starting timeline collection service");

        self.start_focus_tracking().await?;

        Ok(())
    }

    /// Check if the collector is currently running
    pub fn is_running(&self) -> bool {
        self.current_task
            .as_ref()
            .is_some_and(|task| !task.is_finished())
    }

    /// Subscribe to activity events
    pub fn subscribe_to_activity_events(&self) -> broadcast::Receiver<ActivityEvent> {
        self.activity_event_tx.subscribe()
    }

    /// Subscribe to assets change events
    pub fn subscribe_to_assets_events(&self) -> broadcast::Receiver<Vec<ContextChip>> {
        self.assets_event_tx.subscribe()
    }

    /// Start focus tracking with new strategy-driven architecture
    async fn start_focus_tracking(&mut self) -> TimelineResult<()> {
        let strategy = Arc::new(RwLock::new(ActivityStrategy::DefaultStrategy(
            DefaultStrategy,
        )));
        let strategy_clone = Arc::clone(&strategy);
        let activity_event_tx = self.activity_event_tx.clone();
        let assets_event_tx = self.assets_event_tx.clone();

        // Create channel for activity reports from strategies
        let (activity_tx, mut activity_rx) = mpsc::unbounded_channel::<ActivityReport>();

        // Spawn task to handle activity reports from strategies
        let storage_for_reports = Arc::clone(&self.storage);
        let assets_event_tx_for_reports = assets_event_tx.clone();
        tokio::spawn(async move {
            let activity_event_tx_inner = activity_event_tx.clone();
            while let Some(report) = activity_rx.recv().await {
                match report {
                    ActivityReport::NewActivity(activity) => {
                        debug!("Received new activity report: {}", activity.name);
                        let context_chips = activity.get_context_chips();
                        let _ = assets_event_tx_for_reports.send(context_chips);

                        let focus_event = ActivityEvent {
                            name: activity.name.clone(),
                            icon: activity.icon.clone(),
                        };
                        let _ = activity_event_tx_inner.send(focus_event);

                        let mut storage = storage_for_reports.lock().await;
                        storage.add_activity(activity);
                    }
                    ActivityReport::Snapshots(snapshots) => {
                        debug!("Received {} snapshots", snapshots.len());
                        let mut storage = storage_for_reports.lock().await;
                        if let Some(current_activity) = storage.get_all_activities_mut().back_mut()
                        {
                            current_activity.snapshots.clear();
                            current_activity.snapshots.extend(snapshots);
                        }
                    }
                    ActivityReport::Assets(assets) => {
                        debug!("Received {} additional assets", assets.len());
                        let mut storage = storage_for_reports.lock().await;
                        if let Some(current_activity) = storage.get_all_activities_mut().back_mut()
                        {
                            current_activity.assets.clear();
                            current_activity.assets.extend(assets);
                        }
                    }
                    ActivityReport::Stopping => {
                        debug!("Strategy reported stopping");
                    }
                }
            }
        });

        // Spawn focus tracking task
        self.focus_thread_handle = Some(tokio::spawn(async move {
            let config =
                FocusTrackerConfig::new().with_icon_config(IconConfig::new().with_size(64));
            let tracker = FocusTracker::with_config(config);
            let prev_focus = Arc::new(Mutex::new(String::new()));

            let strategy_inner = Arc::clone(&strategy_clone);
            let _ = tracker
                .track_focus_async(move |window: FocusedWindow| {
                    let prev_focus = Arc::clone(&prev_focus);
                    let strategy_for_update = Arc::clone(&strategy_inner);
                    let activity_tx_inner = activity_tx.clone();

                    async move {
                        let process_name = window.process_name.clone();
                        let new_focus = process_name.clone();
                        debug!("New focus: {:?}", new_focus);

                        let mut prev = prev_focus.lock().await;
                        if new_focus != *prev {
                            let mut strategy_write = strategy_for_update.write().await;

                            match strategy_write.handle_process_change(&window).await {
                                Ok(true) => {
                                    debug!("Strategy can continue handling: {}", process_name);
                                }
                                Ok(false) => {
                                    debug!("Strategy can no longer handle: {}", process_name);
                                    match ActivityStrategy::new(&process_name).await {
                                        Ok(mut new_strategy) => {
                                            // Start tracking with new strategy
                                            let _ = new_strategy
                                                .start_tracking(&window, activity_tx_inner.clone())
                                                .await
                                                .map_err(|err| {
                                                    error!("Failed to start tracking: {}", err);
                                                });

                                            *strategy_write = new_strategy;
                                        }
                                        Err(err) => {
                                            error!("Failed to create new strategy: {}", err);
                                        }
                                    };
                                }
                                Err(err) => {
                                    debug!("Error handling process change: {}", err);
                                }
                            }
                            *prev = new_focus;
                        }
                        Ok(())
                    }
                })
                .await;
        }));

        Ok(())
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
