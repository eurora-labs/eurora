pub mod config;
pub mod error;
pub mod storage;
pub mod strategies;
pub mod tool_backend;
pub mod types;
mod utils;

pub use config::{
    ActivityConfig, ActivityConfigBuilder, ApplicationConfig, GlobalConfig, PrivacyConfig,
    SnapshotFrequency, StrategyConfig,
};
pub use error::{ActivityError, ActivityResult};
pub use storage::ActivityStorage;
pub use strategies::ActivityStrategy;
pub use strategies::{
    ActivityReport, BrowserStrategy, DefaultStrategy, NoStrategy, PreviewStrategy,
};
pub use tool_backend::ActivityToolBackend;
pub use types::{Activity, ContextChip};
