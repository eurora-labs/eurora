use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;

mod assets;
mod snapshots;

pub use assets::*;
pub use snapshots::*;

#[allow(clippy::enum_variant_names)]
#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize, AsRefStr)]
#[serde(tag = "kind", content = "data")]
pub enum NativeMessage {
    NativeYoutubeAsset,
    NativeArticleAsset,
    NativeTwitterAsset,

    NativeYoutubeSnapshot,
    NativeArticleSnapshot,
    NativeTwitterSnapshot,
}
