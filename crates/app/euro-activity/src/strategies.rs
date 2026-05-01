use crate::utils::render_svg_bytes;
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use enum_dispatch::enum_dispatch;
use focus_tracker::FocusedWindow;
use std::sync::Arc;
use tokio::sync::mpsc;
use url::Url;

pub mod browser;
pub mod default;
pub mod no_strategy;

pub use browser::BrowserStrategy;
pub use default::DefaultStrategy;
use euro_native_messaging::NativeMetadata;
pub use no_strategy::NoStrategy;

use crate::{
    error::ActivityResult,
    types::{Activity, ActivityAsset, ActivitySnapshot},
};

/// Metadata returned by a strategy about the currently focused target.
///
/// The `url` is stored as a parsed [`Url`] so that the rest of the pipeline
/// cannot accidentally accept malformed or empty URL strings and silently
/// emit an Activity without a domain.
#[derive(Debug, Clone, Default)]
pub struct StrategyMetadata {
    pub url: Option<Url>,
    pub title: Option<String>,
    pub icon: Option<Arc<image::RgbaImage>>,
}

#[derive(Debug, Clone)]
pub enum ActivityReport {
    NewActivity(Activity),
    TitleUpdated { title: String, url: Url },
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
                        .or_else(|| render_svg_bytes(&bytes).ok().map(Arc::new))
                })
            }
            None => None,
        };
        let url = metadata.url.as_deref().and_then(|raw| Url::parse(raw).ok());
        StrategyMetadata {
            url,
            title: metadata.title,
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

impl ActivityStrategy {
    /// Build the strategy responsible for the given focused process.
    ///
    /// Strategies are tried in priority order: [`NoStrategy`] suppresses
    /// tracking for Eurora's own processes, [`BrowserStrategy`] handles
    /// known browsers, and any other process falls through to
    /// [`DefaultStrategy`].
    pub async fn new(process_name: &str) -> ActivityResult<ActivityStrategy> {
        if NoStrategy::matches_process(process_name) {
            return NoStrategy::create().await;
        }
        if BrowserStrategy::matches_process(process_name) {
            return BrowserStrategy::create().await;
        }
        Ok(ActivityStrategy::DefaultStrategy(DefaultStrategy))
    }
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

#[async_trait]
pub trait StrategySupport {
    /// Return `true` if this strategy is responsible for the given focused
    /// process. Implementations must normalize the comparison (e.g.
    /// case-insensitivity on Windows) so dispatch and self-reporting agree.
    fn matches_process(process_name: &str) -> bool;
    async fn create() -> ActivityResult<ActivityStrategy>;
}
