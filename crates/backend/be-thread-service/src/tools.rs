use std::collections::HashMap;
use std::sync::Arc;

use agent_chain::BaseTool;
use agent_chain::tools::{StructuredTool, create_args_schema};
use serde_json::Value;

const FIRECRAWL_SEARCH_URL: &str = "https://firecrawl.inference.nebul.io/v1/search";

pub fn firecrawl_search_tool() -> Arc<dyn BaseTool> {
    let schema = create_args_schema(
        "firecrawl_search",
        HashMap::from([
            (
                "query".to_string(),
                serde_json::json!({"type": "string", "description": "The search query"}),
            ),
            (
                "limit".to_string(),
                serde_json::json!({"type": "integer", "description": "Maximum number of results to return"}),
            ),
        ]),
        vec!["query".to_string()],
        Some("Search the web using Firecrawl and return results"),
    );

    let api_key = std::env::var("FIRECRAWL_API_KEY").unwrap_or_default();

    Arc::new(StructuredTool::from_function_with_async(
        |_args| {
            Err(agent_chain::error::Error::NotImplemented(
                "use async".into(),
            ))
        },
        move |args: HashMap<String, Value>| {
            let api_key = api_key.clone();
            async move {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(5);

                let client = reqwest::Client::new();
                let resp = client
                    .post(FIRECRAWL_SEARCH_URL)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .json(&serde_json::json!({
                        "query": query,
                        "limit": limit,
                    }))
                    .send()
                    .await
                    .map_err(|e| {
                        agent_chain::error::Error::ToolInvocation(format!(
                            "firecrawl_search request failed: {e}"
                        ))
                    })?;

                let body: Value = resp.json().await.map_err(|e| {
                    agent_chain::error::Error::ToolInvocation(format!(
                        "firecrawl_search response parse failed: {e}"
                    ))
                })?;

                Ok(body)
            }
        },
        "firecrawl_search",
        "Search the web using Firecrawl and return results",
        schema,
    ))
}
