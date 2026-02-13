use crate::utils::convert_svg_to_rgba;
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use enum_dispatch::enum_dispatch;
use focus_tracker::FocusedWindow;
use std::sync::Arc;
use tokio::sync::mpsc;

pub mod browser;
pub mod default;
pub mod no_strategy;
pub mod processes;

pub use browser::BrowserStrategy;
pub use default::DefaultStrategy;
use euro_native_messaging::NativeMetadata;
pub use no_strategy::NoStrategy;

use crate::{
    error::ActivityResult,
    types::{Activity, ActivityAsset, ActivitySnapshot},
};

#[derive(Debug, Clone, Default)]
pub struct StrategyMetadata {
    pub url: Option<String>,
    pub icon: Option<Arc<image::RgbaImage>>,
}

#[derive(Debug, Clone)]
pub enum ActivityReport {
    NewActivity(Activity),
    Snapshots(Vec<ActivitySnapshot>),
    Assets(Vec<ActivityAsset>),
    Stopping,
}

impl From<NativeMetadata> for StrategyMetadata {
    fn from(metadata: NativeMetadata) -> Self {
        let icon = match metadata.icon_base64 {
            Some(icon) => match icon.starts_with("data:image/svg+xml;base64") {
                true => convert_svg_to_rgba(&icon).ok().map(Arc::new),
                false => {
                    let icon = icon.split(',').nth(1).unwrap_or(&icon);
                    let icon_data = BASE64_STANDARD.decode(icon.trim()).ok();

                    image::load_from_memory(&icon_data.unwrap_or_default())
                        .ok()
                        .map(|icon_image| Arc::new(icon_image.to_rgba8()))
                }
            },
            None => None,
        };
        StrategyMetadata {
            url: metadata.url,
            icon,
        }
    }
}

#[enum_dispatch(ActivityStrategyFunctionality)]
#[derive(Clone)]
pub enum ActivityStrategy {
    BrowserStrategy,
    DefaultStrategy,
    NoStrategy,
}

#[async_trait]
#[enum_dispatch]
pub trait ActivityStrategyFunctionality {
    fn can_handle_process(&self, focus_window: &FocusedWindow) -> bool;

    async fn start_tracking(
        &mut self,
        focus_window: &focus_tracker::FocusedWindow,
        sender: mpsc::UnboundedSender<ActivityReport>,
    ) -> ActivityResult<()>;

    async fn handle_process_change(&mut self, focus_window: &FocusedWindow)
    -> ActivityResult<bool>;

    async fn stop_tracking(&mut self) -> ActivityResult<()>;

    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>>;
    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>>;
    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata>;
}

impl ActivityStrategy {
    pub async fn new(process_name: &str) -> ActivityResult<ActivityStrategy> {
        if NoStrategy::get_supported_processes().contains(&process_name) {
            return Ok(ActivityStrategy::NoStrategy(NoStrategy));
        }

        if BrowserStrategy::get_supported_processes().contains(&process_name) {
            return Ok(ActivityStrategy::BrowserStrategy(
                BrowserStrategy::new().await?,
            ));
        }

        Ok(ActivityStrategy::DefaultStrategy(DefaultStrategy))
    }
}

#[async_trait]
pub trait StrategySupport {
    fn get_supported_processes() -> Vec<&'static str>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_supported_processes() {
        let processes = BrowserStrategy::get_supported_processes();
        assert!(!processes.is_empty());
    }
}
