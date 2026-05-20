use eurora_tools_youtube::CapturedFrame;
use serde::{Deserialize, Serialize};
use specta::Type;

mod article;
mod metadata;
mod twitter;
mod youtube;

pub use article::*;
pub use metadata::*;
pub use twitter::*;
pub use youtube::*;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeImage {
    pub base64: String,
    pub mime_type: String,
}

/// Envelope for every payload the browser native-messaging host
/// exchanges with the desktop bridge. Externally tagged on `kind` with
/// the inner payload under `data` so the JSON shape matches what the
/// browser extension already constructs.
///
/// The YouTube snapshot variant carries an [`eurora_tools_youtube::CapturedFrame`]
/// — the single canonical YouTube-frame shape, also returned by the
/// `browser::youtube::get_current_frame` tool. The legacy
/// `NativeYoutubeSnapshot` wrapper was dropped in favour of this unified
/// type; consumers compose around `CapturedFrame` directly.
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "data")]
pub enum NativeMessage {
    NativeYoutubeAsset(NativeYoutubeAsset),
    NativeArticleAsset(NativeArticleAsset),
    NativeTwitterAsset(NativeTwitterAsset),

    NativeYoutubeSnapshot(CapturedFrame),
    NativeArticleSnapshot(NativeArticleSnapshot),

    NativeMetadata(NativeMetadata),
}
