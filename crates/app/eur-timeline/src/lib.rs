//! Timeline module for storing system state over time
//!
//! This crate provides functionality to capture system state at regular intervals
//! and store it in memory for later retrieval. It works by sampling data every
//! 3 seconds and maintaining a rolling history.

use anyhow::Result;
use chrono::{DateTime, Utc};
use eur_activity::select_strategy_for_process;
use eur_prompt_kit::LLMMessage;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{error, info, warn};
mod focus_tracker;
pub use focus_tracker::FocusEvent;

#[cfg(target_os = "linux")]
#[path = "linux/mod.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

#[cfg(target_os = "windows")]
#[path = "windows/mod.rs"]
mod platform;

use eur_activity;
use eur_activity::{ActivityStrategy, DisplayAsset};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemState {}

/// A Fragment represents a snapshot of system state at a single point in time
#[derive(Clone, Debug)]
pub struct Fragment {
    /// When this fragment was captured
    pub timestamp: DateTime<Utc>,
    // pub system_state: Option<SystemState>,

    // Screenshot data, if available
    // pub screenshot: Option<Vec<u8>>,

    // Additional metadata about this fragment
    // #[serde(default)]
    // pub metadata: serde_json::Value,
}

/// A reference to a Timeline that can be safely shared between threads
pub type TimelineRef = Arc<Timeline>;

/// Timeline store that holds fragments of system state over time
pub struct Timeline {
    /// The activities stored in the timeline
    activities: Arc<RwLock<Vec<eur_activity::Activity>>>,

    /// The activities new stored in the timeline
    // activities_new: Arc<RwLock<Vec<ActivityStrategy>>>,

    /// The fragments stored in the timeline
    fragments: Arc<RwLock<Vec<Fragment>>>,

    /// How many fragments to keep in history
    capacity: usize,

    /// How often to capture a new fragment (in seconds)
    interval_seconds: u64,
}

impl Timeline {
    /// Create a new timeline with the specified capacity
    pub fn new(capacity: usize, interval_seconds: u64) -> Self {
        // Browser strategy registration is now handled within eur_activity::REGISTRY initialization.
        info!("Timeline created.");
        Timeline {
            activities: Arc::new(RwLock::new(Vec::new())),
            fragments: Arc::new(RwLock::new(Vec::with_capacity(capacity))),
            capacity,
            interval_seconds,
        }
    }

    /// Create a shareable reference to this Timeline
    pub fn clone_ref(&self) -> TimelineRef {
        Arc::new(Timeline {
            activities: Arc::clone(&self.activities),
            fragments: Arc::clone(&self.fragments),
            capacity: self.capacity,
            interval_seconds: self.interval_seconds,
        })
    }

    /// Get a fragment from the specified number of seconds ago
    pub fn get_fragment_from_seconds_ago(&self, seconds_ago: u64) -> Option<Fragment> {
        let index = (seconds_ago / self.interval_seconds) as usize;
        self.get_fragment_at_index(index)
    }

    /// Get a fragment at the specified index
    pub fn get_fragment_at_index(&self, index: usize) -> Option<Fragment> {
        let fragments = self.fragments.read();
        if index >= fragments.len() {
            return None;
        }

        // Calculate the actual index, accounting for the circular buffer
        let actual_index = (fragments.len() - 1) - index;
        fragments.get(actual_index).cloned()
    }

    /// Get all fragments in chronological order (oldest first)
    pub fn get_all_fragments(&self) -> Vec<Fragment> {
        let fragments = self.fragments.read();
        fragments.clone()
    }

    pub fn get_most_recent_fragment(&self) -> Option<Fragment> {
        let fragments = self.fragments.read();
        fragments.last().cloned()
    }

    pub fn add_activity(&self, activity: eur_activity::Activity) {
        let mut activities = self.activities.write();
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
        let (tx, mut rx) = mpsc::unbounded_channel::<FocusEvent>();

        // ----------------------------------  X11 thread
        {
            let tx = tx.clone(); // move into the thread
            std::thread::spawn(move || {
                loop {
                    let tracker = focus_tracker::FocusTracker::new(
                        platform::impl_focus_tracker::ImplFocusTracker::new(),
                    );

                    info!("Starting focus tracker...");

                    // Clone tx for this iteration
                    let tx_clone = tx.clone();

                    // this never blocks: it just ships events into the channel
                    let result = tracker.track_focus(move |event| {
                        info!("▶ {}: {}", event.process, event.title);

                        // ignore the tracker's own window, if desired
                        if event.process != "eur-tauri" {
                            // it's OK if the receiver has gone away
                            let _ = tx_clone.send(event);
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
                if let Ok(strategy) = select_strategy_for_process(
                    &event.process,
                    format!("{}: {}", event.process, event.title),
                    event.icon_base64,
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
    // Default to 1 hour of history (1200 fragments at 3-second intervals)
    Timeline::new(1200, 3)
}
