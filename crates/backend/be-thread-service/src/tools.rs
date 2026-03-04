use std::sync::Arc;

use agent_chain::BaseTool;
use agent_chain::error::{Error, Result};
use agent_chain::tools::tool;

const FIRECRAWL_BASE_URL: &str = "https://firecrawl.inference.nebul.io/v1";

fn firecrawl_api_key() -> Result<String> {
    std::env::var("FIRECRAWL_API_KEY").map_err(|_| {
        Error::ToolException("FIRECRAWL_API_KEY environment variable is not set".into())
    })
}

/// Search the web using Firecrawl and return a list of results with URLs, titles, and snippets.
/// Use firecrawl_scrape to get the full content of any result URL.
#[tool]
async fn firecrawl_search(query: String, limit: Option<u64>) -> Result<String> {
    let api_key = firecrawl_api_key()?;
    let limit = limit.unwrap_or(5);

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{FIRECRAWL_BASE_URL}/search"))
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

/// Scrape a webpage URL and return its content as markdown.
/// Use this after firecrawl_search to get the full content of a specific page.
#[tool]
async fn firecrawl_scrape(url: String) -> Result<String> {
    let api_key = firecrawl_api_key()?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{FIRECRAWL_BASE_URL}/scrape"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "url": url,
            "formats": ["markdown"],
            "onlyMainContent": true,
        }))
        .send()
        .await
        .map_err(|e| Error::ToolException(format!("Request failed: {e}")))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| Error::ToolException(format!("Failed to parse response: {e}")))?;

    // Extract the markdown content from the response if available
    if let Some(markdown) = body.pointer("/data/markdown").and_then(|v| v.as_str()) {
        return Ok(markdown.to_string());
    }

    serde_json::to_string(&body)
        .map_err(|e| Error::ToolException(format!("Failed to serialize response: {e}")))
}

pub fn firecrawl_search_tool() -> Arc<dyn BaseTool> {
    Arc::new(firecrawl_search::tool())
}

pub fn firecrawl_scrape_tool() -> Arc<dyn BaseTool> {
    Arc::new(firecrawl_scrape::tool())
}

pub fn firecrawl_tools() -> Vec<Arc<dyn BaseTool>> {
    vec![firecrawl_search_tool(), firecrawl_scrape_tool()]
}
