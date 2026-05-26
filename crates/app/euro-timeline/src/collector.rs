use crate::{
    ActivityStorage, ActivityStrategy,
    error::{TimelineError, TimelineResult},
    storage::TimelineStorage,
    types::{ActivityEvent, SavedActivityEndedEvent, SavedActivityEvent},
};
use chrono::{DateTime, Utc};
use euro_activity::strategies::{ActivityReport, StrategySupport};
use euro_activity::{
    ActivitySession, ContextChip, NoStrategy, strategies::ActivityStrategyFunctionality,
};
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
};
use uuid::Uuid;

/// Upper bound on concurrent in-flight HTTP syncs spawned by the
/// collector. Prevents a flaky network from piling up unbounded tokio
/// tasks (each one holds an `Arc<ActivityStorage>` clone plus the
/// session payload).
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

    /// PATCH the current session's real `ended_at` (best-effort, bounded
    /// by [`SHUTDOWN_FLUSH_TIMEOUT`]). Called by
    /// [`crate::TimelineManager::stop`] so a clean shutdown closes the
    /// live row in the cloud before the process exits.
    pub async fn flush_current_end(&self) {
        let (session_id, ended_at) = {
            let mut storage = self.storage.lock().await;
            let Some(session) = storage.get_all_sessions_mut().back_mut() else {
                return;
            };
            if session.ended_at.is_none() {
                session.end_session();
            }
            let Some(ended_at) = session.ended_at else {
                return;
            };
            (session.id, ended_at)
        };

        let storage = Arc::clone(&self.activity_storage);
        let patch = async move {
            if let Err(err) = storage.update_session_end(session_id, ended_at).await {
                tracing::warn!(
                    session_id = %session_id,
                    error = %err,
                    "Final session end PATCH failed during shutdown",
                );
            }
        };
        if tokio::time::timeout(SHUTDOWN_FLUSH_TIMEOUT, patch)
            .await
            .is_err()
        {
            tracing::warn!(session_id = %session_id, "Final session end PATCH timed out during shutdown");
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
        let assets_event_tx_for_reports = assets_event_tx.clone();
        let saved_activity_event_tx_for_reports = self.saved_activity_event_tx.clone();
        let saved_activity_ended_event_tx_for_reports = self.saved_activity_ended_event_tx.clone();
        self.current_task = Some(tokio::spawn(async move {
            let activity_event_tx_inner = activity_event_tx.clone();
            // Dedupe key for back-to-back identical reports — same
            // intent as the old `last_activity_name` guard, but keyed
            // on the canonical `identity_key` so different URLs on the
            // same domain (or different windows of the same process)
            // don't open spurious new sessions.
            let mut last_identity_key: Option<String> = None;
            while let Some(report) = activity_rx.recv().await {
                match report {
                    ActivityReport::NewActivity(session) => {
                        let identity_key = session.activity.key.clone();
                        let is_duplicate = last_identity_key
                            .as_ref()
                            .is_some_and(|prev| prev == &identity_key);

                        if is_duplicate {
                            tracing::debug!(
                                "Suppressing duplicate activity report: {}",
                                identity_key
                            );
                            continue;
                        }

                        tracing::debug!("Received new activity report: {}", identity_key);
                        last_identity_key = Some(identity_key);

                        // Close the previous session locally and snapshot
                        // its (id, ended_at) for the closing PATCH. Then
                        // push the new one into storage.
                        let previous_end = {
                            let mut storage = storage_for_reports.lock().await;
                            let prev = storage.get_all_sessions_mut().back_mut().and_then(|prev| {
                                if prev.ended_at.is_none() {
                                    prev.end_session();
                                }
                                prev.ended_at.map(|ended_at| (prev.id, ended_at))
                            });
                            storage.add_session(session.clone());
                            prev
                        };

                        let context_chip = session.get_context_chip();
                        let _ = assets_event_tx_for_reports.send(vec![context_chip]);

                        let focus_event = ActivityEvent {
                            name: session.activity.display_name.clone(),
                            process_name: session.process_name.clone(),
                            process_id: session.process_id,
                            icon: session.icon.clone(),
                        };
                        let _ = activity_event_tx_inner.send(focus_event);

                        if let Some((prev_id, prev_ended_at)) = previous_end {
                            spawn_session_patch_end(
                                &activity_storage,
                                &sync_permits,
                                &saved_activity_ended_event_tx_for_reports,
                                prev_id,
                                prev_ended_at,
                            );
                        }

                        spawn_session_insert(
                            &activity_storage,
                            &sync_permits,
                            &saved_activity_event_tx_for_reports,
                            session,
                        );
                    }
                    ActivityReport::TitleUpdated { title, url } => {
                        tracing::debug!("Received title update: {} ({})", title, url);
                        // Track-by-domain dedupe key — same parent, so
                        // the next NewActivity from elsewhere can still
                        // dedupe correctly.
                        let updated = {
                            let mut storage = storage_for_reports.lock().await;
                            storage.get_all_sessions_mut().back_mut().map(|session| {
                                session.window_title = Some(title.clone());
                                session.set_url(url.clone());
                                let chip = session.get_context_chip();
                                (session.id, chip)
                            })
                        };
                        if let Some((session_id, chip)) = updated {
                            let _ = assets_event_tx_for_reports.send(vec![chip]);
                            spawn_session_patch_title(
                                &activity_storage,
                                &sync_permits,
                                session_id,
                                title,
                                Some(url.to_string()),
                            );
                        }
                    }
                    ActivityReport::Stopping => {
                        tracing::debug!("Strategy reported stopping");
                        last_identity_key = None;
                        let ending = {
                            let mut storage = storage_for_reports.lock().await;
                            storage.get_all_sessions_mut().back_mut().and_then(|prev| {
                                if prev.ended_at.is_none() {
                                    prev.end_session();
                                }
                                prev.ended_at.map(|ended_at| (prev.id, ended_at))
                            })
                        };
                        if let Some((id, ended_at)) = ending {
                            spawn_session_patch_end(
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
    }
}

/// Spawn a bounded-concurrency tokio task that POSTs the session.
///
/// On a successful insert the task fires [`SavedActivityEvent`] on
/// `saved_tx` carrying both the (possibly upserted) parent and the new
/// session, so subscribers can update the timeline rail atomically.
/// Failures log-and-drop; the collector loop must never block on the
/// network.
fn spawn_session_insert(
    storage: &Arc<ActivityStorage>,
    permits: &Arc<Semaphore>,
    saved_tx: &broadcast::Sender<SavedActivityEvent>,
    session: ActivitySession,
) {
    let storage = Arc::clone(storage);
    let permits = Arc::clone(permits);
    let saved_tx = saved_tx.clone();
    let session_id = session.id;
    let identity_key = session.activity.key.clone();
    tokio::spawn(async move {
        let _permit = match permits.try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(
                    session_id = %session_id,
                    identity_key = %identity_key,
                    "Dropping session insert: in-flight sync cap reached",
                );
                return;
            }
        };
        match storage.save_session_to_service(&session).await {
            Ok(response) => {
                let event = SavedActivityEvent {
                    activity: response.activity,
                    session: response.session,
                    icon: session.icon.clone(),
                };
                let _ = saved_tx.send(event);
            }
            Err(err) => {
                tracing::warn!(
                    session_id = %session_id,
                    identity_key = %identity_key,
                    error = %err,
                    "Session insert failed",
                );
            }
        }
    });
}

/// PATCH the closing `ended_at` for a session and, on success, fan out
/// a [`SavedActivityEndedEvent`] so subscribers can flip the rail's
/// live indicator off in place.
///
/// The broadcast is fire-and-forget: a closed channel (no listeners) is
/// normal during boot and a `Lagged` consumer is handled on the receive
/// side, so we don't propagate either back up to the patch task.
fn spawn_session_patch_end(
    storage: &Arc<ActivityStorage>,
    permits: &Arc<Semaphore>,
    ended_tx: &broadcast::Sender<SavedActivityEndedEvent>,
    session_id: Uuid,
    ended_at: DateTime<Utc>,
) {
    let storage = Arc::clone(storage);
    let permits = Arc::clone(permits);
    let ended_tx = ended_tx.clone();
    tokio::spawn(async move {
        let _permit = match permits.try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(
                    session_id = %session_id,
                    "Dropping end PATCH: in-flight sync cap reached",
                );
                return;
            }
        };
        match storage.update_session_end(session_id, ended_at).await {
            Ok(response) => {
                let _ = ended_tx.send(SavedActivityEndedEvent {
                    activity_id: response.session.activity_id,
                    session_id,
                    ended_at,
                });
            }
            Err(err) => {
                tracing::warn!(
                    session_id = %session_id,
                    error = %err,
                    "Session end PATCH failed",
                );
            }
        }
    });
}

fn spawn_session_patch_title(
    storage: &Arc<ActivityStorage>,
    permits: &Arc<Semaphore>,
    session_id: Uuid,
    title: String,
    url: Option<String>,
) {
    let storage = Arc::clone(storage);
    let permits = Arc::clone(permits);
    tokio::spawn(async move {
        let _permit = match permits.try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                tracing::warn!(
                    session_id = %session_id,
                    "Dropping title PATCH: in-flight sync cap reached",
                );
                return;
            }
        };
        if let Err(err) = storage.update_session_title(session_id, title, url).await {
            tracing::warn!(
                session_id = %session_id,
                error = %err,
                "Session title PATCH failed",
            );
        }
    });
}
