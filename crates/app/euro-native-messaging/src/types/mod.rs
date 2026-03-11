use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;

mod article;
mod metadata;
pub mod proto;
mod twitter;
mod youtube;

pub use article::*;
pub use metadata::*;
pub use twitter::*;
pub use youtube::*;

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

    NativeMetadata,
}
