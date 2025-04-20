//! Timeline module for storing system state over time
//!
//! This crate provides functionality to capture system state at regular intervals
//! and store it in memory for later retrieval. It works by sampling data every
//! 3 seconds and maintaining a rolling history.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use eur_proto::ipc::{ProtoArticleState, ProtoPdfState, ProtoTranscriptLine, ProtoYoutubeState};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time;
use tracing::{debug, error, info};

pub mod browser_state;
pub use browser_state::*;

pub mod activity;
pub use activity::*;

pub mod focus_tracker;

pub mod asset_strategy;
pub use asset_strategy::*;

use eur_activity;
use eur_activity::{ActivityStrategy, DisplayAsset};

// Custom serialization for ImageBuffer
mod image_serde {
    use super::*;
    use image::{ImageBuffer, Rgba};
    use serde::de::{self};
    use serde::ser::SerializeStruct;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        img: &ImageBuffer<Rgba<u8>, Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (width, height) = img.dimensions();
        let raw_data = img.as_raw();

        let mut state = serializer.serialize_struct("ImageBuffer", 3)?;
        state.serialize_field("width", &width)?;
        state.serialize_field("height", &height)?;
        state.serialize_field("data", raw_data)?;
        state.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ImageData {
            width: u32,
            height: u32,
            data: Vec<u8>,
        }

        let data = ImageData::deserialize(deserializer)?;
        ImageBuffer::from_raw(data.width, data.height, data.data)
            .ok_or_else(|| de::Error::custom("Failed to create ImageBuffer from data"))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SystemState {}

/// A Fragment represents a snapshot of system state at a single point in time
#[derive(Clone, Debug)]
pub struct Fragment {
    /// When this fragment was captured
    pub timestamp: DateTime<Utc>,

    pub browser_state: Option<BrowserState>,
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
    activities: Arc<RwLock<Vec<Activity>>>,

    /// The activities stored in the timeline (temporary)
    activities_temp: Arc<RwLock<Vec<eur_activity::Activity>>>,

    /// The activities new stored in the timeline
    // activities_new: Arc<RwLock<Vec<ActivityStrategy>>>,

    /// The fragments stored in the timeline
    fragments: Arc<RwLock<Vec<Fragment>>>,

    /// How many fragments to keep in history
    capacity: usize,

    /// How often to capture a new fragment (in seconds)
    interval_seconds: u64,

    /// Persistent browser collector to avoid recreating it on each collection
    browser_collector: Arc<Mutex<Option<BrowserCollector>>>,
}

impl Timeline {
    /// Create a new timeline with the specified capacity
    pub fn new(capacity: usize, interval_seconds: u64) -> Self {
        Self {
            activities: Arc::new(RwLock::new(Vec::new())),
            activities_temp: Arc::new(RwLock::new(Vec::new())),
            fragments: Arc::new(RwLock::new(Vec::with_capacity(capacity))),
            capacity,
            interval_seconds,
            browser_collector: Arc::new(Mutex::new(None)),
        }
    }

    /// Create a shareable reference to this Timeline
    pub fn clone_ref(&self) -> TimelineRef {
        Arc::new(Timeline {
            activities: Arc::clone(&self.activities),
            activities_temp: Arc::clone(&self.activities_temp),
            fragments: Arc::clone(&self.fragments),
            capacity: self.capacity,
            interval_seconds: self.interval_seconds,
            browser_collector: Arc::clone(&self.browser_collector),
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

    pub fn add_activity(&self, activity: Activity) {
        let mut activities = self.activities.write();
        // eprintln!("Adding activity: {:?}", activity);
        activities.push(activity);
    }

    pub fn add_activity_temp(&self, activity: eur_activity::Activity) {
        let mut activities = self.activities_temp.write();
        // eprintln!("Adding activity: {:?}", activity);
        activities.push(activity.into());
    }

    pub fn get_activities(&self) -> Vec<Activity> {
        let activities = self.activities.read();
        activities.clone()
    }

    pub fn get_activities_temp(&self) -> Vec<DisplayAsset> {
        let activities = self.activities_temp.read();

        if activities.is_empty() {
            return Vec::new();
        }

        activities.last().unwrap().get_display_assets()
    }

    pub async fn start_collection_activity<T: ActivityStrategy>(
        &self,
        mut activity_strategy: T,
        s: &mut String,
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
        self.add_activity_temp(activity);

        eprintln!("Activity added: ");

        // // Create an Arc to share the activity between threads
        // let activity = Arc::new(activity_strategy);

        // // Clone references for the async task
        // // let activity_clone = Arc::clone(&activity);

        // // Spawn a tokio task to periodically gather state
        // tokio::spawn(async move {
        //     // Create an interval timer that ticks every 3 seconds
        //     let mut interval = time::interval(Duration::from_secs(3));

        //     // Create a single activity in the timeline that we'll update with snapshots
        //     let process_name = "activity-reporter";
        //     let activity_name = "Monitored Activity";
        //     let icon = "📊"; // Using an emoji as a simple icon

        //     let mut timeline_activity = eur_timeline::activity::Activity::new(
        //         activity_name.to_string(),
        //         icon.to_string(),
        //         process_name.to_string(),
        //     );

        //     // Add the initial activity to the timeline
        //     timeline_clone.add_activity(timeline_activity.clone());

        //     // Run indefinitely, collecting state at each interval
        //     loop {
        //         // Wait for the next interval tick
        //         interval.tick().await;

        //         // Gather the current state from the activity
        //         let state = match std::panic::catch_unwind(|| activity_clone.gather_state()) {
        //             Ok(state) => state,
        //             Err(e) => {
        //                 error!("Error gathering state: {:?}", e);
        //                 "Error gathering state".to_string()
        //             }
        //         };

        //         // Log the gathered state (for debugging)
        //         debug!("Gathered state: {}", state);

        //         // Create a timestamp for this snapshot
        //         let now = chrono::Utc::now().timestamp() as u64;

        //         // Create a new activity snapshot with the gathered state
        //         let snapshot = eur_timeline::activity::ActivitySnapshot {
        //             session: eur_timeline::activity::AppSession {},
        //             screenshot: None,
        //             updated_at: now,
        //             created_at: now,
        //         };

        //         // Create a JSON representation of the state
        //         let state_json = serde_json::json!({
        //             "state": state,
        //             "timestamp": now,
        //         });

        //         // Create an asset from the state
        //         let asset = eur_timeline::activity::ActivityAsset::new(
        //             state_json,
        //             eur_timeline::activity::AssetType::Custom,
        //         );

        //         // Get the activities from the timeline
        //         let activities = timeline_clone.get_activities();

        //         // Find our activity and update it with the new snapshot and asset
        //         // In a real implementation, you would use a more robust way to identify the activity
        //         for mut activity in activities {
        //             if activity.process_name == process_name {
        //                 activity.add_snapshot(snapshot.clone());
        //                 activity.add_asset(asset.clone());

        //                 // Update the activity in the timeline
        //                 // Note: In a real implementation, you would need a proper way to update
        //                 // an existing activity in the timeline, possibly through a method like
        //                 // timeline.update_activity(activity)
        //                 break;
        //             }
        //         }
        //     }
        // });
    }

    /// Start the timeline collection process
    pub async fn start_collection(&self) -> Result<()> {
        info!(
            "Starting timeline collection every {} seconds",
            self.interval_seconds
        );

        // Start the focus tracker
        focus_tracker::spawn(self);

        return Ok(());

        let fragments = Arc::clone(&self.fragments);
        let browser_collector = Arc::clone(&self.browser_collector);
        let capacity = self.capacity;
        let interval = Duration::from_secs(self.interval_seconds);

        tokio::spawn(async move {
            let mut interval_timer = time::interval(interval);

            // Initialize the browser collector if it hasn't been initialized yet
            {
                let mut collector_guard = browser_collector.lock().await;
                if collector_guard.is_none() {
                    match BrowserCollector::new().await {
                        Ok(collector) => {
                            *collector_guard = Some(collector);
                        }
                        Err(e) => {
                            error!("Failed to initialize browser collector: {}", e);
                            return;
                        }
                    }
                }
            }

            loop {
                interval_timer.tick().await;

                // Step 1: Collect the new fragment
                // The collect_fragment function ensures all collectors finish their work
                // before returning the completed fragment
                let fragment_result = Self::collect_fragment(Arc::clone(&browser_collector)).await;

                // Step 2: Only after all collectors have finished, we add the fragment to the timeline
                match fragment_result {
                    Ok(fragment) => {
                        // All collectors have definitively completed their work at this point
                        let mut fragments_write = fragments.write();
                        fragments_write.push(fragment);

                        // Trim the vector if it exceeds capacity
                        if fragments_write.len() > capacity {
                            fragments_write.remove(0);
                        }

                        debug!(
                            "Collected new fragment, now have {}/{} fragments",
                            fragments_write.len(),
                            capacity
                        );
                    }
                    Err(e) => {
                        error!("Failed to collect fragment: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Collect a new fragment on demand
    pub async fn collect_new_fragment(&self) -> Result<Fragment> {
        let browser_collector = Arc::clone(&self.browser_collector);
        Self::collect_fragment(browser_collector).await
    }

    /// Collect a new fragment from system state
    /// Uses a JoinSet to ensure all collectors finish before returning the fragment
    async fn collect_fragment(
        browser_collector: Arc<Mutex<Option<BrowserCollector>>>,
    ) -> Result<Fragment> {
        let timestamp = Utc::now();

        // Create a JoinSet to manage all collector tasks
        // This approach allows for easily adding more collectors in the future
        let mut collection_tasks = JoinSet::new();

        // Spawn a task for the browser collector
        let browser_collector_clone = Arc::clone(&browser_collector);
        collection_tasks.spawn(async move {
            let mut collector_guard = browser_collector_clone.lock().await;
            let collector = collector_guard
                .as_mut()
                .context("Browser collector not initialized")?;

            collector
                .collect_state()
                .await
                .context("Failed to collect browser state")
        });

        // Here you could add additional collectors in the future:
        // collection_tasks.spawn(async move { collect_system_state().await });
        // collection_tasks.spawn(async move { collect_network_state().await });

        // Wait for browser state collection to complete
        let mut browser_state: Option<BrowserState> = None;

        // Process the results from all collectors
        while let Some(result) = collection_tasks.join_next().await {
            match result {
                Ok(Ok(state_option)) => {
                    // Store each type of state in its appropriate place
                    // Currently we only have browser state
                    // collect_state() returns an Option<BrowserState>
                    if let Some(state) = state_option {
                        browser_state = Some(state);
                    }
                }
                Ok(Err(e)) => {
                    // A collector task returned an error
                    error!("Collector task error: {}", e);
                    // Continue with other collectors instead of failing completely
                }
                Err(e) => {
                    // A collector task panicked
                    error!("Collector task panicked: {}", e);
                    // Continue with other collectors instead of failing completely
                }
            }
        }

        // All collectors have completed at this point
        // Now we can create the fragment with all collected data
        Ok(Fragment {
            timestamp,
            browser_state,
        })
    }
}

/// Create a new timeline with default settings
pub fn create_default_timeline() -> Timeline {
    // Default to 1 hour of history (1200 fragments at 3-second intervals)
    Timeline::new(1200, 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_state_types() {
        // Create a YoutubeState
        let youtube_proto = ProtoYoutubeState {
            url: "https://youtube.com/watch?v=12345".to_string(),
            title: "Test Video".to_string(),
            transcript: vec![ProtoTranscriptLine {
                text: "Test transcript".to_string(),
                start: 0.0,
                duration: 30.0,
            }],
            current_time: 30.0,
            video_frame: None,
        };
        let youtube_state = ProtoYoutubeState::from(youtube_proto.clone());
        let browser_state = BrowserState::Youtube(youtube_state);

        // Verify conversion back to proto
        let youtube_state_ref = browser_state_to_youtube(&browser_state).unwrap();
        let proto_from_state: ProtoYoutubeState = youtube_state_ref.clone();
        assert_eq!(proto_from_state.url, youtube_proto.url);
        assert_eq!(proto_from_state.title, youtube_proto.title);
        assert_eq!(proto_from_state.transcript, youtube_proto.transcript);
        assert_eq!(proto_from_state.current_time, youtube_proto.current_time);

        // Create an ArticleState
        let article_proto = ProtoArticleState {
            url: "https://example.com/article".to_string(),
            title: "Test Article".to_string(),
            content: "Article content".to_string(),
            selected_text: "Selected text".to_string(),
        };
        let article_state = ProtoArticleState::from(article_proto.clone());
        let browser_state2 = BrowserState::Article(article_state);

        // Verify conversion back to proto
        let article_state_ref = browser_state_to_article(&browser_state2).unwrap();
        let proto_from_state: ProtoArticleState = article_state_ref.clone();
        assert_eq!(proto_from_state.url, article_proto.url);
        assert_eq!(proto_from_state.title, article_proto.title);
        assert_eq!(proto_from_state.content, article_proto.content);
        assert_eq!(proto_from_state.selected_text, article_proto.selected_text);

        // Create a PdfState
        let pdf_proto = ProtoPdfState {
            url: "https://example.com/document.pdf".to_string(),
            title: "Test PDF".to_string(),
            content: "PDF content".to_string(),
            selected_text: "Selected PDF text".to_string(),
        };
        let pdf_state = ProtoPdfState::from(pdf_proto.clone());
        let browser_state3 = BrowserState::Pdf(pdf_state);

        // Verify conversion back to proto
        let pdf_state_ref = browser_state_to_pdf(&browser_state3).unwrap();
        let proto_from_state: ProtoPdfState = pdf_state_ref.clone();
        assert_eq!(proto_from_state.url, pdf_proto.url);
        assert_eq!(proto_from_state.title, pdf_proto.title);
        assert_eq!(proto_from_state.content, pdf_proto.content);
        assert_eq!(proto_from_state.selected_text, pdf_proto.selected_text);
    }

    // Helper functions for the tests
    fn browser_state_to_youtube(state: &BrowserState) -> Option<&ProtoYoutubeState> {
        match state {
            BrowserState::Youtube(youtube) => Some(youtube),
            _ => None,
        }
    }

    fn browser_state_to_article(state: &BrowserState) -> Option<&ProtoArticleState> {
        match state {
            BrowserState::Article(article) => Some(article),
            _ => None,
        }
    }

    fn browser_state_to_pdf(state: &BrowserState) -> Option<&ProtoPdfState> {
        match state {
            BrowserState::Pdf(pdf) => Some(pdf),
            _ => None,
        }
    }
}
