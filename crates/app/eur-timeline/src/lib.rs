//! Timeline module for storing system state over time
//!
//! This crate provides functionality to capture system state at regular intervals
//! and store it in memory for later retrieval. It works by sampling data every
//! 3 seconds and maintaining a rolling history.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use eur_activity::BrowserStrategy;
use eur_prompt_kit::Message;
use eur_proto::ipc::{ProtoArticleState, ProtoPdfState, ProtoTranscriptLine, ProtoYoutubeState};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Handle; // Added import
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time;
use tracing::{debug, error, info};

pub mod focus_tracker;

// pub mod asset_strategy;
// pub use asset_strategy::AssetStrategy;

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
    activities_temp: Arc<RwLock<Vec<eur_activity::Activity>>>,

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
            activities_temp: Arc::new(RwLock::new(Vec::new())),
            fragments: Arc::new(RwLock::new(Vec::with_capacity(capacity))),
            capacity,
            interval_seconds,
        }
    }

    /// Create a shareable reference to this Timeline
    pub fn clone_ref(&self) -> TimelineRef {
        Arc::new(Timeline {
            activities_temp: Arc::clone(&self.activities_temp),
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

    pub fn add_activity_temp(&self, activity: eur_activity::Activity) {
        let mut activities = self.activities_temp.write();
        // eprintln!("Adding activity: {:?}", activity);
        activities.push(activity.into());
    }

    pub fn get_activities_temp(&self) -> Vec<DisplayAsset> {
        let activities = self.activities_temp.read();

        if activities.is_empty() {
            return Vec::new();
        }

        activities.last().unwrap().get_display_assets()
    }

    pub fn construct_asset_messages(&self) -> Vec<Message> {
        let activities = self.activities_temp.read();
        let last_activity = activities.last().unwrap();

        last_activity
            .assets
            .iter()
            .map(|asset| asset.construct_message())
            .collect()
    }

    pub async fn start_collection_activity(
        &self,
        mut activity_strategy: Box<dyn ActivityStrategy>,
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
    }

    /// Start the timeline collection process
    pub async fn start_collection(&self) -> Result<()> {
        // info!(
        //     "Starting timeline collection every {} seconds",
        //     self.interval_seconds
        // );

        // // Start the focus tracker
        focus_tracker::spawn(self);

        return Ok(());
    }
}

/// Create a new timeline with default settings
pub fn create_default_timeline() -> Timeline {
    // Default to 1 hour of history (1200 fragments at 3-second intervals)
    Timeline::new(1200, 3)
}
