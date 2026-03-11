use agent_chain::error::{Error, Result};
use agent_chain::tools::tool;
use agent_chain::BaseTool;


#[tool]
async fn get_text() -> Result<String> {
    todo!()
}

#[tool]
async fn get_highlighted_text() -> Result<String> {
    todo!()
}

#[tool]
async fn get_url() -> Result<String> {
    todo!()
}

// async fn firecrawl_search(query: String, limit: Option<u64>) -> Result<String> {
//     let api_key = firecrawl_api_key()?;
//     let limit = limit.unwrap_or(5);

//     let (header_name, header_value) = authorization_header(&api_key);
//     let response = HTTP_CLIENT
//         .post(format!("{FIRECRAWL_BASE_URL}/search"))
//         .header(header_name, header_value)
//         .header("Content-Type", "application/json")
//         .json(&serde_json::json!({
//             "query": query,
//             "limit": limit,
//         }))
//         .timeout(REQUEST_TIMEOUT)
//         .send()
//         .await
//         .map_err(|e| Error::ToolException(format!("Search request failed: {e}")))?;

//     let status = response.status();
//     let body: Value = response
//         .json()
//         .await
//         .map_err(|e| Error::ToolException(format!("Failed to parse search response: {e}")))?;

//     check_response_status(status, &body)?;
//     Ok(format_search_results(&body))
// }
