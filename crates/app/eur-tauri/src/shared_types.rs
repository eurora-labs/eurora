use async_mutex::Mutex;
use eur_prompt_kit::PromptKitService;
use eur_timeline::Timeline;
use std::sync::Arc;
pub type SharedPromptKitService = Arc<Mutex<Option<PromptKitService>>>;
pub type SharedTimeline = Arc<Timeline>;
pub fn create_shared_timeline() -> SharedTimeline {
    // Create a timeline that collects state every 3 seconds and keeps 1 hour of history
    Arc::new(eur_timeline::create_default_timeline())
}
