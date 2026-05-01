use crate::{
    ActivityStrategy,
    error::{TimelineError, TimelineResult},
    storage::TimelineStorage,
    types::ActivityEvent,
};
use euro_activity::DefaultStrategy;
use euro_activity::strategies::ActivityReport;
use euro_activity::strategies::StrategySupport;
use euro_activity::{ContextChip, NoStrategy, strategies::ActivityStrategyFunctionality};
use focus_tracker::{FocusTracker, FocusTrackerConfig, FocusedWindow, IconConfig};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::{
    sync::{Mutex, RwLock, broadcast, mpsc},
    task::JoinHandle,
};

pub struct CollectorService {
    storage: Arc<Mutex<TimelineStorage>>,
    strategy: Arc<RwLock<ActivityStrategy>>,
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
        tracing::debug!(
            "Creating collector service with interval: {:?}",
            timeline_config.collector.collection_interval
        );

        let (activity_event_tx, _) = broadcast::channel(100);
        let (assets_event_tx, _) = broadcast::channel(100);
        let strategy = Arc::new(RwLock::new(ActivityStrategy::DefaultStrategy(
            DefaultStrategy,
        )));

        Self {
            storage,
            strategy,
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

        tracing::debug!("Starting timeline collection service");

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

    pub async fn refresh_current_activity(&self) -> TimelineResult<()> {
        let mut strategy = self.strategy.write().await;
        let assets = strategy
            .retrieve_assets()
            .await
            .map_err(|e| TimelineError::Storage(e.to_string()))?;
        let snapshots = strategy
            .retrieve_snapshots()
            .await
            .map_err(|e| TimelineError::Storage(e.to_string()))?;

        let mut storage = self.storage.lock().await;
        if let Some(activity) = storage.get_all_activities_mut().back_mut() {
            if !assets.is_empty() {
                activity.assets.clear();
                activity.assets.extend(assets);
            }
            if !snapshots.is_empty() {
                activity.snapshots.clear();
                activity.snapshots.extend(snapshots);
            }
        }

        Ok(())
    }

    async fn start_focus_tracking(&mut self) -> TimelineResult<()> {
        let strategy_clone = Arc::clone(&self.strategy);
        let activity_event_tx = self.activity_event_tx.clone();
        let assets_event_tx = self.assets_event_tx.clone();

        let (activity_tx, mut activity_rx) = mpsc::unbounded_channel::<ActivityReport>();

        let storage_for_reports = Arc::clone(&self.storage);
        let assets_event_tx_for_reports = assets_event_tx.clone();
        tokio::spawn(async move {
            let activity_event_tx_inner = activity_event_tx.clone();
            let mut last_activity_name: Option<String> = None;
            while let Some(report) = activity_rx.recv().await {
                match report {
                    ActivityReport::NewActivity(activity) => {
                        let is_duplicate = last_activity_name
                            .as_ref()
                            .is_some_and(|prev| prev == &activity.name);

                        if is_duplicate {
                            tracing::debug!(
                                "Suppressing duplicate activity report: {}",
                                activity.name
                            );
                            continue;
                        }

                        tracing::debug!("Received new activity report: {}", activity.name);
                        last_activity_name = Some(activity.name.clone());
                        let context_chip = activity.get_context_chip();
                        let _ = assets_event_tx_for_reports.send(vec![context_chip]);

                        let focus_event = ActivityEvent {
                            name: activity.name.clone(),
                            process_name: activity.process_name.clone(),
                            process_id: activity.process_id,
                            icon: activity.icon.clone(),
                        };
                        let _ = activity_event_tx_inner.send(focus_event);

                        let mut storage = storage_for_reports.lock().await;
                        storage.add_activity(activity);
                    }
                    ActivityReport::TitleUpdated { title, url } => {
                        tracing::debug!("Received title update: {} ({})", title, url);
                        last_activity_name = Some(url.to_string());
                        let mut storage = storage_for_reports.lock().await;
                        if let Some(activity) = storage.get_all_activities_mut().back_mut() {
                            activity.title = Some(title);
                            activity.set_url(url);
                            let chip = activity.get_context_chip();
                            let _ = assets_event_tx_for_reports.send(vec![chip]);
                        }
                    }
                    ActivityReport::Stopping => {
                        tracing::debug!("Strategy reported stopping");
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
            let tracker = FocusTracker::builder().config(config).build();
            let prev_focus = Arc::new(Mutex::new(String::new()));

            let strategy_inner = Arc::clone(&strategy_clone);
            let _ = tracker
                .track_focus()
                .on_focus(move |window: FocusedWindow| {
                    let prev_focus = Arc::clone(&prev_focus);
                    let strategy_for_update = Arc::clone(&strategy_inner);
                    let activity_tx_inner = activity_tx.clone();

                    async move {
                        let process_name = window.process_name.clone();
                        let new_focus = process_name.clone();
                        tracing::debug!("New focus: {:?}", new_focus);

                        let mut prev = prev_focus.lock().await;
                        if new_focus != *prev {
                            if NoStrategy::get_supported_processes()
                                .contains(&process_name.as_str())
                            {
                                tracing::debug!(
                                    "Ignoring focus change to own process: {}",
                                    process_name
                                );
                                return Ok(());
                            }

                            let mut strategy_write = strategy_for_update.write().await;

                            match strategy_write.handle_process_change(&window).await {
                                Ok(true) => {
                                    tracing::debug!(
                                        "Strategy can continue handling: {}",
                                        process_name
                                    );
                                }
                                Ok(false) => {
                                    tracing::debug!(
                                        "Strategy can no longer handle: {}",
                                        process_name
                                    );
                                    match ActivityStrategy::new(&process_name).await {
                                        Ok(mut new_strategy) => {
                                            let _ = new_strategy
                                                .start_tracking(&window, activity_tx_inner.clone())
                                                .await
                                                .map_err(|err| {
                                                    tracing::error!(
                                                        "Failed to start tracking: {}",
                                                        err
                                                    );
                                                });

                                            *strategy_write = new_strategy;
                                        }
                                        Err(err) => {
                                            tracing::error!(
                                                "Failed to create new strategy: {}",
                                                err
                                            );
                                        }
                                    };
                                }
                                Err(err) => {
                                    tracing::debug!("Error handling process change: {}", err);
                                }
                            }
                            *prev = new_focus;
                        }
                        Ok(())
                    }
                })
                .call()
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
