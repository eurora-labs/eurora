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
#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "data")]
pub enum NativeMessage {
    NativeYoutubeAsset(NativeYoutubeAsset),
    NativeArticleAsset(NativeArticleAsset),
    NativeTwitterAsset(NativeTwitterAsset),

    NativeYoutubeSnapshot(NativeYoutubeSnapshot),
    NativeArticleSnapshot(NativeArticleSnapshot),

    NativeMetadata(NativeMetadata),
}
