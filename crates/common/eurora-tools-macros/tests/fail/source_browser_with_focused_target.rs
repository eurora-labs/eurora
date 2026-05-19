#![allow(unused_imports)]

use eurora_tools::{FocusedOrigin, ToolError, adapter};
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
    async fn boom(
        &self,
        target: &FocusedOrigin,
        args: Empty,
    ) -> Result<Output, ToolError>;
}

fn main() {}
