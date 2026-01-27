// Re-export main types for easy access
pub use agent_chain_core::BaseMessage;
pub use collector::ActivityEvent;
pub use config::TimelineConfig;
pub use error::{TimelineError, TimelineResult};
// Re-export activity types for convenience
pub use euro_activity::{
    Activity, ActivityAsset, ActivityError, ActivitySnapshot, ActivityStorage,
    ActivityStorageConfig, ActivityStrategy, AssetFunctionality, ContextChip,
};
pub use manager::{TimelineManager, TimelineManagerBuilder};

// Internal modules
mod collector;
mod config;
mod error;
mod manager;
mod storage;
