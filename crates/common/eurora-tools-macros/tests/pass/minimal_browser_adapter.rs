//! Minimal `bridge(browser)` adapter — the canonical happy path.

use eurora_tools::{BrowserOrigin, ToolError, adapter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Default)]
pub struct Empty {}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct CurrentTimestamp {
    pub video_id: String,
    pub current_time: f64,
}

/// Tools for the YouTube video the user is currently watching.
#[adapter(namespace = "browser::youtube", version = 1)]
pub trait YoutubeAdapter: Send + Sync {
    /// Return the user's current playback position.
    #[tool(
        timeout_ms = 2_000,
        source = "bridge(browser)",
        requires_context = "youtube::watch_page",
    )]
    async fn get_current_timestamp(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<CurrentTimestamp, ToolError>;
}

fn main() {
    // Reference the emitted symbols so the linker keeps them.
    let _ = &YOUTUBE_DESCRIPTORS[..];
    fn _is_dispatcher<T>()
    where
        T: eurora_tools::Dispatcher,
    {
    }
    struct StubImpl;
    impl YoutubeAdapter for StubImpl {
        async fn get_current_timestamp(
            &self,
            _target: &BrowserOrigin,
            _args: Empty,
        ) -> Result<CurrentTimestamp, ToolError> {
            Ok(CurrentTimestamp {
                video_id: String::new(),
                current_time: 0.0,
            })
        }
    }
    _is_dispatcher::<YoutubeDispatcher<StubImpl>>();
}
