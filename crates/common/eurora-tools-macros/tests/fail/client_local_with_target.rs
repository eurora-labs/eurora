#![allow(unused_imports)]

use eurora_tools::{BrowserOrigin, ToolError, adapter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Default)]
pub struct Empty {}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Output {}

#[adapter(namespace = "client::math")]
pub trait MathAdapter: Send + Sync {
    /// Description.
    #[tool(timeout_ms = 100, source = "client_local")]
    async fn boom(
        &self,
        target: &BrowserOrigin,
        args: Empty,
    ) -> Result<Output, ToolError>;
}

fn main() {}
