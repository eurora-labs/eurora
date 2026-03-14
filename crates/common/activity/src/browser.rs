use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};
use strum_macros::AsRefStr;

mod default;
mod twitter;
mod youtube;

#[allow(clippy::enum_variant_names)]
#[enum_dispatch]
#[derive(Debug, Clone, Serialize, Deserialize, AsRefStr)]
#[serde(tag = "kind", content = "data")]
pub enum BrowserMessage {}
