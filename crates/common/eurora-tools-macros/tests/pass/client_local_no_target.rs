//! `client_local` adapter with no target parameter.

use eurora_tools::{ToolError, adapter};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Query {
    pub q: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Answer {
    pub a: String,
}

/// Local tools that run inside the client app process.
#[adapter(namespace = "client::math")]
pub trait MathAdapter: Send + Sync {
    /// Echo the query verbatim.
    #[tool(timeout_ms = 100, source = "client_local")]
    async fn echo(&self, args: Query) -> Result<Answer, ToolError>;
}

fn main() {
    let _ = &MATH_DESCRIPTORS[..];
}
