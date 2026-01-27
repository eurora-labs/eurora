// Re-export main types for easy access
pub use agent_chain_core::BaseMessage;
pub use config::TimelineConfig;
pub use error::{TimelineError, TimelineResult};
pub use types::ActivityEvent;
// Re-export activity types for convenience
pub use euro_activity::{
    Activity, ActivityAsset, ActivityError, ActivitySnapshot, ActivityStorage, ActivityStrategy,
    AssetFunctionality, ContextChip,
};
pub use manager::TimelineManager;

// Internal modules
mod collector;
mod config;
mod error;
mod manager;
mod storage;
mod types;
