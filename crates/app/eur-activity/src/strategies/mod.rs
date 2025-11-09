//! Simplified strategy implementations for different activity types

use crate::utils::convert_svg_to_rgba;
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use enum_dispatch::enum_dispatch;

pub mod browser;
pub mod default;
pub mod no_strategy;
pub mod processes;

pub use browser::BrowserStrategy;
pub use default::DefaultStrategy;
use eur_native_messaging::NativeMetadata;
pub use no_strategy::NoStrategy;

use crate::{
    error::ActivityResult,
    types::{ActivityAsset, ActivitySnapshot},
};

#[derive(Debug, Clone, Default)]
pub struct StrategyMetadata {
    pub url: Option<String>,
    pub icon: Option<image::RgbaImage>,
}

impl From<NativeMetadata> for StrategyMetadata {
    fn from(metadata: NativeMetadata) -> Self {
        let icon = match metadata.icon_base64 {
            Some(icon) => match icon.starts_with("data:image/svg+xml;base64") {
                true => convert_svg_to_rgba(&icon).ok(),
                false => {
                    let icon = icon.split(',').nth(1).unwrap_or(&icon);
                    let icon_data = BASE64_STANDARD.decode(icon.trim()).ok();

                    image::load_from_memory(&icon_data.unwrap_or_default())
                        .ok()
                        .map(|icon_image| icon_image.to_rgba8())
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

/// Enum containing all possible activity strategies
#[enum_dispatch(ActivityStrategyFunctionality)]
#[derive(Debug, Clone)]
pub enum ActivityStrategy {
    BrowserStrategy,
    DefaultStrategy,
    NoStrategy,
}

#[async_trait]
#[enum_dispatch]
pub trait ActivityStrategyFunctionality {
    async fn retrieve_assets(&mut self) -> ActivityResult<Vec<ActivityAsset>>;
    async fn retrieve_snapshots(&mut self) -> ActivityResult<Vec<ActivitySnapshot>>;
    async fn get_metadata(&mut self) -> ActivityResult<StrategyMetadata>;
}

impl ActivityStrategy {
    pub async fn new(process_name: &str) -> ActivityResult<ActivityStrategy> {
        if BrowserStrategy::get_supported_processes().contains(&process_name) {
            return Ok(ActivityStrategy::BrowserStrategy(
                BrowserStrategy::new().await?,
            ));
        }

        Ok(ActivityStrategy::DefaultStrategy(DefaultStrategy))
    }
}

/// Trait for strategies to declare which processes they support
#[async_trait]
pub trait StrategySupport {
    /// Returns list of exact process names this strategy supports
    fn get_supported_processes() -> Vec<&'static str>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_supported_processes() {
        let processes = BrowserStrategy::get_supported_processes();
        assert!(!processes.is_empty());
        // Should contain browser process names based on OS
    }
}
