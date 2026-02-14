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
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    sync::{Mutex, RwLock, broadcast, mpsc},
    task::JoinHandle,
};
use tracing::{debug, error};

pub struct CollectorService {
    storage: Arc<Mutex<TimelineStorage>>,
    current_task: Option<JoinHandle<()>>,
    focus_thread_handle: Option<JoinHandle<()>>,
    focus_shutdown_signal: Option<Arc<AtomicBool>>,
    activity_event_tx: broadcast::Sender<ActivityEvent>,
    assets_event_tx: broadcast::Sender<Vec<ContextChip>>,
}

impl CollectorService {
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

    pub async fn start(&mut self) -> TimelineResult<()> {
        if self.is_running() {
            return Err(TimelineError::AlreadyRunning);
        }

        debug!("Starting timeline collection service");

        self.start_focus_tracking().await?;

        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.current_task
            .as_ref()
            .is_some_and(|task| !task.is_finished())
    }

    pub fn subscribe_to_activity_events(&self) -> broadcast::Receiver<ActivityEvent> {
        self.activity_event_tx.subscribe()
    }

    pub fn subscribe_to_assets_events(&self) -> broadcast::Receiver<Vec<ContextChip>> {
        self.assets_event_tx.subscribe()
    }

    async fn start_focus_tracking(&mut self) -> TimelineResult<()> {
        let strategy = Arc::new(RwLock::new(ActivityStrategy::DefaultStrategy(
            DefaultStrategy,
        )));
        let strategy_clone = Arc::clone(&strategy);
        let activity_event_tx = self.activity_event_tx.clone();
        let assets_event_tx = self.assets_event_tx.clone();

        let (activity_tx, mut activity_rx) = mpsc::unbounded_channel::<ActivityReport>();

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
                            let context_chips = current_activity.get_context_chips();
                            let _ = assets_event_tx_for_reports.send(context_chips);
                        }
                    }
                    ActivityReport::Stopping => {
                        debug!("Strategy reported stopping");
                    }
                }
            }
        });

        self.focus_thread_handle = Some(tokio::spawn(async move {
            let config = FocusTrackerConfig::builder()
                .icon(
                    IconConfig::builder()
                        .size(64)
                        .expect("valid icon size")
                        .build(),
                )
                .build();
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

        if let Some(shutdown_signal) = &self.focus_shutdown_signal {
            shutdown_signal.store(true, Ordering::Relaxed);
        }

        if let Some(thread_handle) = self.focus_thread_handle.take() {
            thread_handle.abort();
        }
    }
}
