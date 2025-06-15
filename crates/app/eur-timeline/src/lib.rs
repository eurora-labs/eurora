//! Timeline module for storing system state over time
//!
//! This crate provides functionality to capture system state at regular intervals
//! and store it in memory for later retrieval. It works by sampling data every
//! 3 seconds and maintaining a rolling history.

use anyhow::Result;
use eur_activity::select_strategy_for_process;
use eur_prompt_kit::LLMMessage;
use ferrous_focus::{FerrousFocusResult, FocusedWindow};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{error, info, warn};

use eur_activity::{ActivityStrategy, DisplayAsset};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemState {}

/// A reference to a Timeline that can be safely shared between threads
pub type TimelineRef = Arc<Timeline>;

/// Timeline store that holds activities of system state over time
pub struct Timeline {
    /// The activities stored in the timeline
    activities: Arc<RwLock<Vec<eur_activity::Activity>>>,

    /// How many activities to keep in history
    capacity: usize,

    /// How often to capture a new activity (in seconds)
    interval_seconds: u64,
}

impl Timeline {
    /// Create a new timeline with the specified capacity
    pub fn new(capacity: usize, interval_seconds: u64) -> Self {
        // Browser strategy registration is now handled within eur_activity::REGISTRY initialization.
        info!("Timeline created.");
        Timeline {
            activities: Arc::new(RwLock::new(Vec::new())),
            capacity,
            interval_seconds,
        }
    }

    /// Create a shareable reference to this Timeline
    pub fn clone_ref(&self) -> TimelineRef {
        Arc::new(Timeline {
            activities: Arc::clone(&self.activities),
            capacity: self.capacity,
            interval_seconds: self.interval_seconds,
        })
    }

    pub fn add_activity(&self, activity: eur_activity::Activity) {
        let mut activities = self.activities.write();
        if activities.len() >= self.capacity {
            activities.remove(0);
        }
        activities.push(activity);
    }
    pub fn get_context_chips(&self) -> Vec<eur_activity::ContextChip> {
        let activities = self.activities.read();
        info!("Number of activities: {:?}", activities.len());
        if activities.is_empty() {
            return Vec::new();
        }
        activities.last().unwrap().get_context_chips()
    }

    pub fn get_activities(&self) -> Vec<DisplayAsset> {
        let activities = self.activities.read();

        if activities.is_empty() {
            return Vec::new();
        }

        let last_activity = activities.last().unwrap();

        info!(
            "Number of snapshots: {:?}",
            last_activity.snapshots.len() as u32
        );

        activities.last().unwrap().get_display_assets()
    }

    pub fn construct_asset_messages(&self) -> Vec<LLMMessage> {
        let activities = self.activities.read();
        let last_activity = activities.last().unwrap();

        last_activity
            .assets
            .iter()
            .map(|asset| asset.construct_message())
            .collect()
    }

    pub async fn start_snapshot_collection(
        &self,
        _activity_strategy: Box<dyn ActivityStrategy>,
        _s: &mut str,
    ) {
        todo!();
    }

    pub async fn start_collection_activity(
        &self,
        mut activity_strategy: Box<dyn ActivityStrategy>,
        _s: &mut str,
    ) {
        // Retrieve initial assets from the activity
        let assets = activity_strategy
            .retrieve_assets()
            .await
            .unwrap_or(Vec::new());

        // Create a new activity
        let activity = eur_activity::Activity::new(
            activity_strategy.get_name().to_string(),
            activity_strategy.get_icon().to_string(),
            activity_strategy.get_process_name().to_string(),
            assets,
        );

        // Add the activity to the timeline
        self.add_activity(activity);

        // Clone the activities Arc for the background task
        let activities = Arc::clone(&self.activities);

        // Move the strategy into the background task
        let mut strategy = activity_strategy;

        let interval = Duration::from_secs(3);
        let mut interval_timer = time::interval(interval);

        loop {
            interval_timer.tick().await;

            match strategy.retrieve_snapshots().await {
                Ok(snapshots) => {
                    if !snapshots.is_empty() {
                        // Get write access to the activities
                        let mut activities_lock = activities.write();

                        // Find the last activity (the one we just added)
                        if let Some(last_activity) = activities_lock.last_mut() {
                            // Add the snapshots to the activity
                            for snapshot in snapshots {
                                last_activity.snapshots.push(snapshot);
                            }
                        }
                    }
                }
                Err(e) => {
                    info!("Failed to retrieve snapshots: {:?}", e);
                    // error!("Failed to retrieve snapshots: {:?}", e);
                }
            }
        }
    }

    /// Start the timeline collection process
    pub async fn start_collection(&self) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<FocusedWindow>();

        // ----------------------------------  X11 thread
        {
            let tx = tx.clone(); // move into the thread

            let tracker = ferrous_focus::FocusTracker::new();
            std::thread::spawn(move || {
                loop {
                    info!("Starting focus tracker...");

                    // Clone tx for this iteration
                    let tx_clone = tx.clone();

                    // this never blocks: it just ships events into the channel
                    let result =
                        tracker.track_focus(|window: FocusedWindow| -> FerrousFocusResult<()> {
                            let process_name = window.process_name.clone().unwrap();
                            let window_title = window.window_title.clone().unwrap();
                            info!("▶ {}: {}", process_name, window_title);
                            if process_name != "eur-tauri" {
                                let _ = tx_clone.send(window);
                            }
                            Ok(())
                        });

                    // Handle focus tracker errors gracefully
                    match result {
                        Ok(_) => {
                            warn!("Focus tracker ended unexpectedly, restarting...");
                        }
                        Err(e) => {
                            error!("Focus tracker crashed with error: {:?}", e);
                            warn!("Restarting focus tracker in 1 second...");
                        }
                    }

                    // Wait a bit before restarting to avoid rapid restart loops
                    std::thread::sleep(std::time::Duration::from_secs(1));
                }
            });
        }

        // ----------------------------------  async event loop
        let mut current_job: Option<JoinHandle<()>> = None;
        let timeline = self.clone_ref(); // Arc / Rc to use inside tasks

        while let Some(event) = rx.recv().await {
            // shut down the previous collection (if it still runs)
            if let Some(job) = current_job.take() {
                job.abort(); // instant, non-blocking
            }

            // launch a new collection for this focus event
            let timeline = timeline.clone_ref();
            current_job = Some(tokio::spawn(async move {
                // build a strategy for the newly-focused window
                let process_name = event.process_name.unwrap();
                let window_title = event.window_title.unwrap();
                let icon = event.icon.unwrap();
                if let Ok(strategy) = select_strategy_for_process(
                    &process_name,
                    format!("{}: {}", process_name, window_title),
                    icon,
                )
                .await
                {
                    // run the actual collection; ignore its own result
                    let _ = timeline
                        .start_collection_activity(strategy, &mut String::new())
                        .await;
                }
                info!("block is done"); // printed when the task ends
            }));
        }

        Ok(())
    }
}

/// Create a new timeline with default settings
pub fn create_default_timeline() -> Timeline {
    // Default to 1 hour of history (1200 activities at 3-second intervals)
    Timeline::new(1200, 3)
}
