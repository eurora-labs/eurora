use crate::{strategies::safari::SafariStrategy, utils::convert_svg_to_rgba};
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use enum_dispatch::enum_dispatch;
use focus_tracker::FocusedWindow;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::sync::mpsc;

pub mod browser;
pub mod default;
pub mod no_strategy;
pub mod processes;
pub mod safari;

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
            Some(ref icon) if icon.is_empty() => None,
            Some(ref icon) => {
                let raw = if let Some(pos) = icon.find(',') {
                    &icon[pos + 1..]
                } else {
                    icon.as_str()
                };
                BASE64_STANDARD.decode(raw.trim()).ok().and_then(|bytes| {
                    image::load_from_memory(&bytes)
                        .map(|img| Arc::new(img.to_rgba8()))
                        .ok()
                        .or_else(|| convert_svg_to_rgba(raw).ok().map(Arc::new))
                })
            }
            None => None,
        };
        StrategyMetadata {
            url: metadata.url,
            icon,
        }
    }
}

macro_rules! register_strategies {
    ($($Strategy:ident),+ $(,)?) => {
        static PROCESS_STRATEGY_MAP: LazyLock<HashMap<&'static str, ActivityStrategy>> =
            LazyLock::new(|| {
                let mut map = HashMap::new();
                $(
                    for name in $Strategy::get_supported_processes() {
                        map.insert(name, ActivityStrategy::$Strategy($Strategy::default()));
                    }
                )+
                map
            });

        impl ActivityStrategy {
            pub async fn new(process_name: &str) -> ActivityResult<ActivityStrategy> {
                match PROCESS_STRATEGY_MAP.get(process_name) {
                    $(
                        Some(ActivityStrategy::$Strategy(_)) => $Strategy::create().await,
                    )+
                    _ => Ok(ActivityStrategy::DefaultStrategy(DefaultStrategy)),
                }
            }
        }
    };
}

#[enum_dispatch(ActivityStrategyFunctionality)]
#[derive(Clone)]
pub enum ActivityStrategy {
    SafariStrategy,
    BrowserStrategy,
    DefaultStrategy,
    NoStrategy,
}

register_strategies!(NoStrategy, SafariStrategy, BrowserStrategy);

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

#[async_trait]
pub trait StrategySupport {
    fn get_supported_processes() -> Vec<&'static str>;
    async fn create() -> ActivityResult<ActivityStrategy>;
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
