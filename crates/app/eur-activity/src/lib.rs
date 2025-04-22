//! Activity reporting module
//!
//! This module provides functionality for tracking and reporting activities.
//! It defines the Activity trait and the ActivityReporter struct, which
//! can be used to collect data from activities and store it in a timeline.

use anyhow::Result;
// use eur_timeline::TimelineRef;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use eur_prompt_kit::Message;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
pub mod browser_activity;
pub mod default_activity;
pub mod strategy_factory;

pub use browser_activity::{BrowserStrategy, BrowserStrategyFactory};
pub use strategy_factory::{DefaultStrategyFactory, select_strategy_for_process};

#[derive(Serialize, Deserialize)]
pub struct DisplayAsset {
    pub name: String,
    pub icon: String,
}

impl DisplayAsset {
    pub fn new(name: String, icon: String) -> Self {
        Self { name, icon }
    }
}

pub trait ActivityAsset: Send + Sync {
    fn get_name(&self) -> &String;
    fn get_icon(&self) -> Option<&String>;

    fn construct_message(&self) -> Message;

    // fn get_display(&self) -> DisplayAsset;
}

pub trait ActivitySnapshot: Send + Sync {
    fn get_screenshot(&self) -> Option<DynamicImage>;
    fn get_updated_at(&self) -> u64;
    fn get_created_at(&self) -> u64;
}

pub struct Activity {
    /// Name of the activity
    pub name: String,

    /// Icon representing the activity
    pub icon: String,

    /// Process name of the activity
    pub process_name: String,

    /// Start time (Unix timestamp)
    pub start: DateTime<Utc>,

    /// End time (Unix timestamp)
    pub end: Option<DateTime<Utc>>,

    // /// Snapshots of the activity
    pub snapshots: Vec<Box<dyn ActivitySnapshot>>,
    /// Assets associated with the activity
    pub assets: Vec<Box<dyn ActivityAsset>>,
}

impl Activity {
    /// Create a new activity
    pub fn new(
        name: String,
        icon: String,
        process_name: String,
        assets: Vec<Box<dyn ActivityAsset>>,
    ) -> Self {
        Self {
            name,
            icon,
            process_name,
            start: Utc::now(),
            end: None,
            assets,
            snapshots: Vec::new(),
        }
    }

    pub fn get_display_assets(&self) -> Vec<DisplayAsset> {
        self.assets
            .iter()
            .filter_map(|asset| {
                if let Some(icon) = asset.get_icon() {
                    Some(DisplayAsset::new(asset.get_name().clone(), icon.clone()))
                } else {
                    Some(DisplayAsset::new(
                        asset.get_name().clone(),
                        self.icon.clone(),
                    ))
                }
            })
            .collect()
    }
}

/// Activity trait defines methods that must be implemented by activities
/// that can be tracked and reported.
#[async_trait]
pub trait ActivityStrategy: Send + Sync {
    /// Retrieve assets associated with this activity
    ///
    /// This method is called once when collection starts to gather
    /// initial assets related to the activity.
    async fn retrieve_assets(&mut self) -> Result<Vec<Box<dyn ActivityAsset>>>;

    /// Gather the current state of the activity
    ///
    /// This method is called periodically to collect the current state
    /// of the activity. The returned string should represent the state
    /// in a format that can be parsed and stored in the timeline.
    fn gather_state(&self) -> String;

    /// Get name of the activity
    fn get_name(&self) -> &String;
    /// Get icon of the activity
    fn get_icon(&self) -> &String;
    /// Get process name of the activity
    fn get_process_name(&self) -> &String;
}

/// Strategy factory trait for creating activity strategies
#[async_trait]
pub trait StrategyFactory: Send + Sync {
    /// Returns true if this factory can create a strategy for the given process
    fn supports_process(&self, process_name: &str) -> bool;

    /// Create a new strategy instance for the given process
    async fn create_strategy(
        &self,
        process_name: &str,
        display_name: String,
        icon: String,
    ) -> Result<Box<dyn ActivityStrategy>>;
}

/// ActivityReporter handles the collection and reporting of activity data
///
/// This struct provides methods for starting collection from activities
/// and reporting the collected data to a timeline.
// pub struct ActivityReporter;

// impl ActivityReporter {
//     /// Start collecting data from an activity
//     ///
//     /// This method:
//     /// 1. Retrieves initial assets from the activity
//     /// 2. Spawns a background task that gathers state every 3 seconds
//     /// 3. Writes the gathered state to the timeline
//     ///
//     /// # Arguments
//     /// * `activity` - The activity to collect data from
//     /// * `timeline` - Reference to the timeline where data will be stored
//     /// * `s` - A string to store initial collection status
//     ///
//     /// # Returns
//     /// This method doesn't return a value, but it updates the provided string
//     /// with a status message and spawns a background task that continues to
//     /// collect data from the activity.
//     pub fn start_collection<T: ActivityNew>(activity: T, timeline: TimelineRef, s: &mut String) {
//         // Retrieve initial assets from the activity
//         activity.retrieve_assets();

//         // Create an Arc to share the activity between threads
//         let activity = Arc::new(activity);

//         // Clone references for the async task
//         let activity_clone = Arc::clone(&activity);
//         let timeline_clone = timeline.clone_ref();

//         // Spawn a tokio task to periodically gather state
//         tokio::spawn(async move {
//             // Create an interval timer that ticks every 3 seconds
//             let mut interval = time::interval(Duration::from_secs(3));

//             // Create a single activity in the timeline that we'll update with snapshots
//             let process_name = "activity-reporter";
//             let activity_name = "Monitored Activity";
//             let icon = "📊"; // Using an emoji as a simple icon

//             let mut timeline_activity = eur_timeline::activity::Activity::new(
//                 activity_name.to_string(),
//                 icon.to_string(),
//                 process_name.to_string(),
//             );

//             // Add the initial activity to the timeline
//             timeline_clone.add_activity(timeline_activity.clone());

//             // Run indefinitely, collecting state at each interval
//             loop {
//                 // Wait for the next interval tick
//                 interval.tick().await;

//                 // Gather the current state from the activity
//                 let state = match std::panic::catch_unwind(|| activity_clone.gather_state()) {
//                     Ok(state) => state,
//                     Err(e) => {
//                         error!("Error gathering state: {:?}", e);
//                         "Error gathering state".to_string()
//                     }
//                 };

//                 // Log the gathered state (for debugging)
//                 debug!("Gathered state: {}", state);

//                 // Create a timestamp for this snapshot
//                 let now = chrono::Utc::now().timestamp() as u64;

//                 // Create a new activity snapshot with the gathered state
//                 let snapshot = eur_timeline::activity::ActivitySnapshot {
//                     session: eur_timeline::activity::AppSession {},
//                     screenshot: None,
//                     updated_at: now,
//                     created_at: now,
//                 };

//                 // Create a JSON representation of the state
//                 let state_json = serde_json::json!({
//                     "state": state,
//                     "timestamp": now,
//                 });

//                 // Create an asset from the state
//                 let asset = eur_timeline::activity::ActivityAsset::new(
//                     state_json,
//                     eur_timeline::activity::AssetType::Custom,
//                 );

//                 // Get the activities from the timeline
//                 let activities = timeline_clone.get_activities();

//                 // Find our activity and update it with the new snapshot and asset
//                 // In a real implementation, you would use a more robust way to identify the activity
//                 for mut activity in activities {
//                     if activity.process_name == process_name {
//                         activity.add_snapshot(snapshot.clone());
//                         activity.add_asset(asset.clone());

//                         // Update the activity in the timeline
//                         // Note: In a real implementation, you would need a proper way to update
//                         // an existing activity in the timeline, possibly through a method like
//                         // timeline.update_activity(activity)
//                         break;
//                     }
//                 }
//             }
//         });

//         // Indicate that collection has started
//         s.push_str("Data collection started");
//     }
// }

#[cfg(test)]
mod tests {

    use std::sync::{Arc, Mutex};

    // A simple implementation of the Activity trait for testing
    struct TestActivity {
        // Track how many times retrieve_assets was called
        assets_retrieved: Arc<Mutex<u32>>,

        // Track how many times gather_state was called
        states_gathered: Arc<Mutex<u32>>,
    }

    impl TestActivity {
        fn new() -> Self {
            Self {
                assets_retrieved: Arc::new(Mutex::new(0)),
                states_gathered: Arc::new(Mutex::new(0)),
            }
        }

        fn get_assets_retrieved(&self) -> u32 {
            *self.assets_retrieved.lock().unwrap()
        }

        fn get_states_gathered(&self) -> u32 {
            *self.states_gathered.lock().unwrap()
        }
    }

    // impl ActivityStrategy for TestActivity {
    //     fn retrieve_assets(&self) {
    //         // Increment the counter
    //         let mut count = self.assets_retrieved.lock().unwrap();
    //         *count += 1;

    //         // Log for debugging
    //         println!("Assets retrieved (count: {})", *count);
    //     }

    //     fn gather_state(&self) -> String {
    //         // Increment the counter
    //         let mut count = self.states_gathered.lock().unwrap();
    //         *count += 1;

    //         // Log for debugging
    //         println!("State gathered (count: {})", *count);

    //         // Return a test state that includes the count
    //         format!("Test state gathered ({})", *count)
    //     }
    // }

    // #[tokio::test]
    // async fn test_start_collection() {
    //     // Create a timeline for testing
    //     let timeline = eur_timeline::create_default_timeline();
    //     let timeline_ref = timeline.clone_ref();

    //     // Create a test activity with counters
    //     let activity = TestActivity::new();

    //     // Get references to the counters for later verification
    //     let assets_retrieved = Arc::clone(&activity.assets_retrieved);
    //     let states_gathered = Arc::clone(&activity.states_gathered);

    //     // String to store collection status
    //     let mut status = String::new();

    //     // Start collection
    //     ActivityReporter::start_collection(activity, timeline_ref, &mut status);

    //     // Verify that collection started
    //     assert_eq!(status, "Data collection started");

    //     // Verify that assets were retrieved exactly once
    //     assert_eq!(
    //         *assets_retrieved.lock().unwrap(),
    //         1,
    //         "Assets should be retrieved exactly once"
    //     );

    //     // Wait a bit to allow the background task to run
    //     // We'll wait for 7 seconds, which should allow for at least 2 state gatherings
    //     // (initial + at least 1 from the 3-second interval)
    //     tokio::time::sleep(Duration::from_secs(7)).await;

    //     // Verify that state was gathered multiple times
    //     let states_count = *states_gathered.lock().unwrap();
    //     assert!(
    //         states_count >= 2,
    //         "State should be gathered at least twice, got {}",
    //         states_count
    //     );

    //     // Get activities from the timeline
    //     let activities = timeline.get_activities();

    //     // Verify that at least one activity was added
    //     assert!(
    //         !activities.is_empty(),
    //         "No activities were added to the timeline"
    //     );

    //     // Verify that the activity has the expected process name
    //     let activity = &activities[0];
    //     assert_eq!(
    //         activity.process_name, "activity-reporter",
    //         "Activity should have the expected process name"
    //     );

    //     // Verify that the activity has assets
    //     assert!(!activity.assets.is_empty(), "Activity should have assets");

    //     // In a real test, you might want to verify the content of the assets,
    //     // but for this simple test, we just check that something was added
    // }
}
