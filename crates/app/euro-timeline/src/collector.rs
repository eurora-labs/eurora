use crate::{
    ActivityStorage, ActivityStrategy,
    error::{TimelineError, TimelineResult},
    storage::TimelineStorage,
    types::{ActivityEvent, SavedActivityEndedEvent, SavedActivityEvent},
};
use chrono::{DateTime, Utc};
use euro_activity::strategies::{ActivityReport, StrategySupport};
use euro_activity::{Activity, ContextChip, NoStrategy, strategies::ActivityStrategyFunctionality};
use focus_tracker::{
    FocusTracker, FocusTrackerConfig, FocusedWindow, IconConfig, IgnoreRule, WindowTitleMatch,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::{
    sync::{Mutex, RwLock, Semaphore, broadcast, mpsc},
    task::JoinHandle,
    time::Instant,
};
use uuid::Uuid;

/// How often the collector PATCHes `ended_at` for the live activity so
/// that an unexpected shutdown leaves a bounded end time on the server
/// instead of a row that stays open forever.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

/// Upper bound on concurrent in-flight HTTP syncs spawned by the
/// collector. Prevents a flaky network from piling up unbounded tokio
/// tasks (each one holds an `Arc<ActivityStorage>` clone plus the
/// activity payload).
const MAX_IN_FLIGHT_SYNCS: usize = 8;

/// How long [`CollectorService::flush_current_end`] waits for the final
/// PATCH on graceful shutdown before giving up.
const SHUTDOWN_FLUSH_TIMEOUT: Duration = Duration::from_secs(2);

pub struct CollectorService {
    storage: Arc<Mutex<TimelineStorage>>,
    activity_storage: Arc<ActivityStorage>,
    sync_permits: Arc<Semaphore>,
    strategy: Arc<RwLock<ActivityStrategy>>,
    current_task: Option<JoinHandle<()>>,
    focus_thread_handle: Option<JoinHandle<()>>,
    focus_shutdown_signal: Option<Arc<AtomicBool>>,
    /// Heartbeat handle for the currently-live activity. Replaced on
    /// every `NewActivity` and aborted on `Stopping` / shutdown. Held
    /// behind a mutex so the report-drain task and `flush_current_end`
    /// can rotate it without racing.
    heartbeat: Arc<Mutex<Option<JoinHandle<()>>>>,
    activity_event_tx: broadcast::Sender<ActivityEvent>,
    assets_event_tx: broadcast::Sender<Vec<ContextChip>>,
    saved_activity_event_tx: broadcast::Sender<SavedActivityEvent>,
    saved_activity_ended_event_tx: broadcast::Sender<SavedActivityEndedEvent>,
}

impl CollectorService {
    pub fn new_with_timeline_config(
        storage: Arc<Mutex<TimelineStorage>>,
        activity_storage: Arc<ActivityStorage>,
        timeline_config: crate::config::TimelineConfig,
    ) -> Self {
        tracing::debug!(
            "Creating collector service with interval: {:?}",
            timeline_config.collector.collection_interval
        );

        let (activity_event_tx, _) = broadcast::channel(100);
        let (assets_event_tx, _) = broadcast::channel(100);
        let (saved_activity_event_tx, _) = broadcast::channel(100);
        let (saved_activity_ended_event_tx, _) = broadcast::channel(100);
        // `DefaultStrategy` is window-bound and can't exist without a
        // focused window, so we boot in `NoStrategy` (a no-op that
        // refuses to handle any external process). The very first focus
        // event will cause `handle_process_change` to return `false`,
        // triggering the redispatch path below and replacing this with
        // the right strategy for whatever the user is looking at.
        let strategy = Arc::new(RwLock::new(ActivityStrategy::NoStrategy(NoStrategy)));

        Self {
            storage,
            activity_storage,
            sync_permits: Arc::new(Semaphore::new(MAX_IN_FLIGHT_SYNCS)),
            strategy,
            current_task: None,
            focus_thread_handle: None,
            focus_shutdown_signal: None,
            heartbeat: Arc::new(Mutex::new(None)),
            activity_event_tx,
            assets_event_tx,
            saved_activity_event_tx,
            saved_activity_ended_event_tx,
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

    pub fn subscribe_to_saved_activity_events(&self) -> broadcast::Receiver<SavedActivityEvent> {
        self.saved_activity_event_tx.subscribe()
    }

    pub fn subscribe_to_saved_activity_ended_events(
        &self,
    ) -> broadcast::Receiver<SavedActivityEndedEvent> {
        self.saved_activity_ended_event_tx.subscribe()
    }

    /// Handle to the currently active strategy. Cloned `Arc` shares the
    /// same lock the collector swaps on focus changes, so consumers (the
    /// chat tool backend) always see the freshest strategy.
    pub fn active_strategy(&self) -> Arc<RwLock<ActivityStrategy>> {
        Arc::clone(&self.strategy)
    }

    /// Stop the heartbeat and PATCH the current activity's real
    /// `ended_at` (best-effort, bounded by [`SHUTDOWN_FLUSH_TIMEOUT`]).
    /// Called by [`crate::TimelineManager::stop`] so a clean shutdown
    /// overrides the last heartbeat value with the precise end timestamp.
    pub async fn flush_current_end(&self) {
        abort_heartbeat(&self.heartbeat).await;

        let (id, ended_at) = {
            let mut storage = self.storage.lock().await;
            let Some(activity) = storage.get_all_activities_mut().back_mut() else {
                return;
            };
            if activity.end.is_none() {
                activity.end_activity();
            }
            let Some(ended_at) = activity.end else {
                return;
            };
            (activity.id, ended_at)
        };

        let storage = Arc::clone(&self.activity_storage);
        let patch = async move {
            if let Err(err) = storage.update_activity_end(id, ended_at).await {
                tracing::warn!(
                    activity_id = %id,
                    error = %err,
                    "Final activity end PATCH failed during shutdown",
                );
            }
        };
        if tokio::time::timeout(SHUTDOWN_FLUSH_TIMEOUT, patch)
            .await
            .is_err()
        {
            tracing::warn!(activity_id = %id, "Final activity end PATCH timed out during shutdown");
        }
    }

    async fn start_focus_tracking(&mut self) -> TimelineResult<()> {
        let strategy_clone = Arc::clone(&self.strategy);
        let activity_event_tx = self.activity_event_tx.clone();
        let assets_event_tx = self.assets_event_tx.clone();

        let (activity_tx, mut activity_rx) = mpsc::unbounded_channel::<ActivityReport>();

        let storage_for_reports = Arc::clone(&self.storage);
        let activity_storage = Arc::clone(&self.activity_storage);
        let sync_permits = Arc::clone(&self.sync_permits);
        let heartbeat = Arc::clone(&self.heartbeat);
        let assets_event_tx_for_reports = assets_event_tx.clone();
        let saved_activity_event_tx_for_reports = self.saved_activity_event_tx.clone();
        let saved_activity_ended_event_tx_for_reports = self.saved_activity_ended_event_tx.clone();
        self.current_task = Some(tokio::spawn(async move {
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

                        // End the previous activity locally and capture
                        // its (id, ended_at) for the closing PATCH.
                        // Then push the new one into storage.
                        let previous_end = {
                            let mut storage = storage_for_reports.lock().await;
                            let prev =
                                storage
                                    .get_all_activities_mut()
                                    .back_mut()
                                    .and_then(|prev| {
                                        if prev.end.is_none() {
                                            prev.end_activity();
                                        }
                                        prev.end.map(|ended_at| (prev.id, ended_at))
                                    });
                            storage.add_activity(activity.clone());
                            prev
                        };

                        let context_chip = activity.get_context_chip();
                        let _ = assets_event_tx_for_reports.send(vec![context_chip]);

                        let focus_event = ActivityEvent {
                            name: activity.name.clone(),
                            process_name: activity.process_name.clone(),
                            process_id: activity.process_id,
                            icon: activity.icon.clone(),
                        };
                        let _ = activity_event_tx_inner.send(focus_event);

                        if let Some((prev_id, prev_ended_at)) = previous_end {
                            spawn_patch_end(
                                &activity_storage,
                                &sync_permits,
                                &saved_activity_ended_event_tx_for_reports,
                                prev_id,
                                prev_ended_at,
                            );
                        }

                        // The initial POST sends `id` and
                        // `ended_at = now()` so a crash before the
                        // first heartbeat still leaves a bounded row.
                        let new_id = activity.id;
                        spawn_insert(
                            &activity_storage,
                            &sync_permits,
                            &saved_activity_event_tx_for_reports,
                            activity,
                        );

                        restart_heartbeat(&heartbeat, &activity_storage, &sync_permits, new_id)
                            .await;
                    }
                    ActivityReport::TitleUpdated { title, url } => {
                        tracing::debug!("Received title update: {} ({})", title, url);
                        last_activity_name = Some(url.to_string());
                        let updated = {
                            let mut storage = storage_for_reports.lock().await;
                            storage.get_all_activities_mut().back_mut().map(|activity| {
                                activity.title = Some(title.clone());
                                activity.set_url(url);
                                let chip = activity.get_context_chip();
                                (activity.id, chip)
                            })
                        };
                        if let Some((id, chip)) = updated {
                            let _ = assets_event_tx_for_reports.send(vec![chip]);
                            spawn_patch_title(&activity_storage, &sync_permits, id, title);
                        }
                    }
                    ActivityReport::Stopping => {
                        tracing::debug!("Strategy reported stopping");
                        abort_heartbeat(&heartbeat).await;
                        let ending = {
                            let mut storage = storage_for_reports.lock().await;
                            storage
                                .get_all_activities_mut()
                                .back_mut()
                                .and_then(|prev| {
                                    if prev.end.is_none() {
                                        prev.end_activity();
                                    }
                                    prev.end.map(|ended_at| (prev.id, ended_at))
                                })
                        };
                        if let Some((id, ended_at)) = ending {
                            spawn_patch_end(
                                &activity_storage,
                                &sync_permits,
                                &saved_activity_ended_event_tx_for_reports,
                                id,
                                ended_at,
                            );
                        }
                    }
                }
            }
        }));

        self.focus_thread_handle = Some(tokio::spawn(async move {
            let config = FocusTrackerConfig::builder()
                .icon(
                    IconConfig::builder()
                        .size(64)
                        .expect("valid icon size")
                        .build(),
                )
                // Suppress Explorer.EXE noise on Windows: the Alt-Tab
                // "Task Switching" overlay and titleless Explorer
                // pseudo-windows both surface as focus events but
                // neither represents a real user-facing application
                // context.
                .windows_ignore_rules([
                    IgnoreRule::builder()
                        .process_name("Explorer.EXE")
                        .window_title(WindowTitleMatch::Exact("Task Switching".into()))
                        .build(),
                    IgnoreRule::builder()
                        .process_name("Explorer.EXE")
                        .window_title(WindowTitleMatch::Missing)
                        .build(),
                ])
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
                            if NoStrategy::matches_process(&process_name) {
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
                                    match ActivityStrategy::new(&window).await {
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

        // Best-effort heartbeat abort. We cannot await the mutex from
        // a sync Drop, so fall back to `try_lock`; if it's contended,
        // the runtime will collect the orphan when the task itself
        // attempts its next HTTP call.
        if let Ok(mut guard) = self.heartbeat.try_lock()
            && let Some(handle) = guard.take()
        {
            handle.abort();
        }
    }
}

/// Spawn a bounded-concurrency tokio task that POSTs the activity.
/// Failures log-and-drop; the collector loop must never block on the
/// network. Offline resilience is intentionally out of scope here.
///
/// On a successful insert the task fires [`SavedActivityEvent`] on
/// `saved_tx` so subscribers (the desktop tauri layer in particular)
/// can surface the freshly-persisted row in the timeline rail without
/// re-polling `GET /activities`. The send is best-effort: a closed
/// channel (no listeners) is normal during boot and not an error.
fn spawn_insert(
    storage: &Arc<ActivityStorage>,
    permits: &Arc<Semaphore>,
    saved_tx: &broadcast::Sender<SavedActivityEvent>,
    activity: Activity,
) {
    let storage = Arc::clone(storage);
    let permits = Arc::clone(permits);
    let saved_tx = saved_tx.clone();
    let id = activity.id;
    let name = activity.name.clone();
    tokio::spawn(async move {
        let _permit = match permits.try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(
                    activity_id = %id,
                    name = %name,
                    "Dropping activity insert: in-flight sync cap reached",
                );
                return;
            }
        };
        match storage.save_activity_to_service(&activity).await {
            Ok(_) => {
                let event = SavedActivityEvent {
                    id: activity.id,
                    name: activity.name.clone(),
                    process_name: activity.process_name.clone(),
                    window_title: activity.window_title(),
                    started_at: activity.start,
                    ended_at: activity.end,
                    icon: activity.icon.clone(),
                };
                let _ = saved_tx.send(event);
            }
            Err(err) => {
                tracing::warn!(
                    activity_id = %id,
                    name = %name,
                    error = %err,
                    "Activity insert failed",
                );
            }
        }
    });
}

/// PATCH the closing `ended_at` for `id` and, on success, fan out a
/// [`SavedActivityEndedEvent`] so subscribers (the desktop tauri layer in
/// particular) can patch the row's `endedAt` in place. Without that
/// event the frontend keeps `endedAt: null` for every row it received
/// via [`SavedActivityEvent`] and the timeline rail collapses them all
/// to the minimum connector height until the next page reload.
///
/// The broadcast is fire-and-forget: a closed channel (no listeners) is
/// normal during boot and a `Lagged` consumer is handled on the receive
/// side, so we don't propagate either back up to the patch task.
fn spawn_patch_end(
    storage: &Arc<ActivityStorage>,
    permits: &Arc<Semaphore>,
    ended_tx: &broadcast::Sender<SavedActivityEndedEvent>,
    id: Uuid,
    ended_at: DateTime<Utc>,
) {
    let storage = Arc::clone(storage);
    let permits = Arc::clone(permits);
    let ended_tx = ended_tx.clone();
    tokio::spawn(async move {
        let _permit = match permits.try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(activity_id = %id, "Dropping end PATCH: in-flight sync cap reached");
                return;
            }
        };
        match storage.update_activity_end(id, ended_at).await {
            Ok(_) => {
                let _ = ended_tx.send(SavedActivityEndedEvent { id, ended_at });
            }
            Err(err) => {
                tracing::warn!(activity_id = %id, error = %err, "Activity end PATCH failed");
            }
        }
    });
}

fn spawn_patch_title(
    storage: &Arc<ActivityStorage>,
    permits: &Arc<Semaphore>,
    id: Uuid,
    title: String,
) {
    let storage = Arc::clone(storage);
    let permits = Arc::clone(permits);
    tokio::spawn(async move {
        let _permit = match permits.try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(activity_id = %id, "Dropping title PATCH: in-flight sync cap reached");
                return;
            }
        };
        if let Err(err) = storage.update_activity_title(id, title).await {
            tracing::warn!(activity_id = %id, error = %err, "Activity title PATCH failed");
        }
    });
}

/// Replace the active heartbeat task with one bound to `activity_id`.
///
/// The first tick fires [`HEARTBEAT_INTERVAL`] after the call (not
/// immediately) so the initial POST has time to land — otherwise the
/// heartbeat would 404 on a freshly-created row.
async fn restart_heartbeat(
    heartbeat: &Arc<Mutex<Option<JoinHandle<()>>>>,
    storage: &Arc<ActivityStorage>,
    permits: &Arc<Semaphore>,
    activity_id: Uuid,
) {
    let mut guard = heartbeat.lock().await;
    if let Some(prev) = guard.take() {
        prev.abort();
    }

    let storage = Arc::clone(storage);
    let permits = Arc::clone(permits);
    let handle = tokio::spawn(async move {
        let mut ticker =
            tokio::time::interval_at(Instant::now() + HEARTBEAT_INTERVAL, HEARTBEAT_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            ticker.tick().await;

            let permit = match permits.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    tracing::debug!(
                        activity_id = %activity_id,
                        "Skipping heartbeat: in-flight sync cap reached",
                    );
                    continue;
                }
            };

            let now = Utc::now();
            if let Err(err) = storage.update_activity_end(activity_id, now).await {
                tracing::debug!(
                    activity_id = %activity_id,
                    error = %err,
                    "Heartbeat PATCH failed",
                );
            }
            drop(permit);
        }
    });

    *guard = Some(handle);
}

async fn abort_heartbeat(heartbeat: &Arc<Mutex<Option<JoinHandle<()>>>>) {
    let mut guard = heartbeat.lock().await;
    if let Some(handle) = guard.take() {
        handle.abort();
    }
}
