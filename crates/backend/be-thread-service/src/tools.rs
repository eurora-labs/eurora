use std::sync::Arc;

use agent_chain::BaseTool;
use agent_chain::error::{Error, Result};
use agent_chain::tools::tool;

const FIRECRAWL_SEARCH_URL: &str = "https://firecrawl.inference.nebul.io/v1/search";

/// Search the web using Firecrawl and return results.
#[tool]
async fn firecrawl_search(query: String, limit: Option<u64>) -> Result<String> {
    let api_key = std::env::var("FIRECRAWL_API_KEY").map_err(|_| {
        Error::ToolException("FIRECRAWL_API_KEY environment variable is not set".into())
    })?;
    let limit = limit.unwrap_or(5);

    let client = reqwest::Client::new();
    let response = client
        .post(FIRECRAWL_SEARCH_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "query": query,
            "limit": limit,
        }))
        .send()
        .await
        .map_err(|e| Error::ToolException(format!("Request failed: {e}")))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| Error::ToolException(format!("Failed to parse response: {e}")))?;

    serde_json::to_string(&body)
        .map_err(|e| Error::ToolException(format!("Failed to serialize response: {e}")))
}

pub fn firecrawl_search_tool() -> Arc<dyn BaseTool> {
    Arc::new(firecrawl_search::tool())
}
