#![allow(unused_imports)]

use eurora_tools::{BrowserOrigin, adapter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Default)]
pub struct Empty {}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Output {}

#[adapter(namespace = "browser::youtube")]
pub trait YoutubeAdapter: Send + Sync {
    /// Description.
    #[tool(timeout_ms = 100, source = "bridge(browser)")]
    async fn boom(&self, target: &BrowserOrigin, args: Empty) -> Output;
}

fn main() {}
