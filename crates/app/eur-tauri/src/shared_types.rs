use eur_settings::AppSettings;
use std::sync::Arc;

use async_mutex::Mutex;
use eur_prompt_kit::PromptKitService;
use eur_timeline::Timeline;
pub type SharedPromptKitService = Arc<Mutex<Option<PromptKitService>>>;
pub type SharedTimeline = Arc<Timeline>;
// pub type SharedAppSettings = Arc<Mutex<AppSettings>>;
pub fn create_shared_timeline() -> SharedTimeline {
    // Create a timeline that collects state every 3 seconds and keeps 1 hour of history
    Arc::new(eur_timeline::create_default_timeline())
}
