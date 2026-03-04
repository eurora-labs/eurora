use std::sync::{Arc, LazyLock};
use std::time::Duration;

use agent_chain::BaseTool;
use agent_chain::error::{Error, Result};
use agent_chain::tools::tool;
use serde_json::Value;

const FIRECRAWL_BASE_URL: &str = "https://firecrawl.inference.nebul.io/v1";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SCRAPE_TIMEOUT: Duration = Duration::from_secs(60);
const CRAWL_POLL_INTERVAL: Duration = Duration::from_secs(2);
const CRAWL_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_SCRAPE_CHARS: usize = 30_000;
const MAX_CRAWL_CHARS: usize = 50_000;
const MAX_CRAWL_PAGES: u64 = 10;

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client")
});

fn firecrawl_api_key() -> Result<String> {
    std::env::var("FIRECRAWL_API_KEY").map_err(|_| {
        Error::ToolException("FIRECRAWL_API_KEY environment variable is not set".into())
    })
}

fn authorization_header(api_key: &str) -> (&'static str, String) {
    ("Authorization", format!("Bearer {api_key}"))
}

fn check_response_status(status: reqwest::StatusCode, body: &Value) -> Result<()> {
    if status.is_success() {
        return Ok(());
    }

    let message = body
        .get("error")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error");

    match status.as_u16() {
        402 => Err(Error::ToolException(format!(
            "Firecrawl payment required: {message}"
        ))),
        429 => Err(Error::ToolException(format!(
            "Firecrawl rate limit exceeded: {message}"
        ))),
        status => Err(Error::ToolException(format!(
            "Firecrawl request failed (HTTP {status}): {message}"
        ))),
    }
}

fn format_search_results(body: &Value) -> String {
    let results = match body.get("data") {
        Some(Value::Array(arr)) => arr.as_slice(),
        Some(Value::Object(obj)) => match obj.get("web") {
            Some(Value::Array(arr)) => arr.as_slice(),
            _ => return serde_json::to_string(body).unwrap_or_default(),
        },
        _ => return serde_json::to_string(body).unwrap_or_default(),
    };

    if results.is_empty() {
        return "No results found.".to_string();
    }

    let mut output = String::new();
    for (index, result) in results.iter().enumerate() {
        let title = result
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");
        let url = result
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("No URL");
        let description = result
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        output.push_str(&format!("{}. {}\n   URL: {}\n", index + 1, title, url));
        if !description.is_empty() {
            output.push_str(&format!("   {}\n", description));
        }
        output.push('\n');
    }

    output.trim_end().to_string()
}

fn format_map_results(body: &Value) -> String {
    let links = match body.get("links") {
        Some(Value::Array(arr)) => arr,
        _ => return serde_json::to_string(body).unwrap_or_default(),
    };

    if links.is_empty() {
        return "No links found.".to_string();
    }

    let mut output = format!("Found {} URLs:\n\n", links.len());
    for link in links {
        match link {
            // v1 may return plain strings
            Value::String(url) => {
                output.push_str(&format!("- {url}\n"));
            }
            // v2 returns objects with url/title/description
            Value::Object(obj) => {
                let url = obj.get("url").and_then(|v| v.as_str()).unwrap_or("No URL");
                let title = obj.get("title").and_then(|v| v.as_str());
                match title {
                    Some(title) => output.push_str(&format!("- {url} — {title}\n")),
                    None => output.push_str(&format!("- {url}\n")),
                }
            }
            _ => {}
        }
    }

    output.trim_end().to_string()
}

fn format_crawl_results(body: &Value) -> String {
    let data = match body.get("data") {
        Some(Value::Array(arr)) => arr,
        _ => return serde_json::to_string(body).unwrap_or_default(),
    };

    if data.is_empty() {
        return "Crawl returned no pages.".to_string();
    }

    let mut output = format!("Crawled {} pages:\n", data.len());
    let mut total_chars = 0;

    for (index, page) in data.iter().enumerate() {
        let url = page
            .pointer("/metadata/sourceURL")
            .or_else(|| page.pointer("/metadata/url"))
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown URL");
        let title = page
            .pointer("/metadata/title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");

        let header = format!(
            "\n--- Page {} of {}: {} ---\nURL: {}\n\n",
            index + 1,
            data.len(),
            title,
            url
        );
        output.push_str(&header);
        total_chars += header.len();

        if let Some(markdown) = page.get("markdown").and_then(|v| v.as_str()) {
            let remaining = MAX_CRAWL_CHARS.saturating_sub(total_chars);
            if remaining == 0 {
                output.push_str("[Remaining pages omitted — content limit reached]\n");
                break;
            }
            let content = truncate_content(markdown, remaining);
            total_chars += content.len();
            output.push_str(&content);
            output.push('\n');
        }
    }

    output
}

fn truncate_content(content: &str, max_chars: usize) -> String {
    if content.len() <= max_chars {
        return content.to_string();
    }

    let truncated = &content[..max_chars];
    let cut_point = truncated
        .rfind("\n\n")
        .or_else(|| truncated.rfind('\n'))
        .unwrap_or(max_chars);

    format!(
        "{}\n\n[Content truncated — {:.0}% of original shown]",
        &content[..cut_point],
        (cut_point as f64 / content.len() as f64) * 100.0
    )
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

/// Search the web using Firecrawl and return a list of results with URLs, titles, and descriptions.
/// Use firecrawl_scrape to get the full content of a specific result URL, or firecrawl_map to
/// discover all pages on a result's website.
#[tool]
async fn firecrawl_search(query: String, limit: Option<u64>) -> Result<String> {
    let api_key = firecrawl_api_key()?;
    let limit = limit.unwrap_or(5);

    let (header_name, header_value) = authorization_header(&api_key);
    let response = HTTP_CLIENT
        .post(format!("{FIRECRAWL_BASE_URL}/search"))
        .header(header_name, header_value)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "query": query,
            "limit": limit,
        }))
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| Error::ToolException(format!("Search request failed: {e}")))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| Error::ToolException(format!("Failed to parse search response: {e}")))?;

    check_response_status(status, &body)?;
    Ok(format_search_results(&body))
}

/// Scrape a single webpage URL and return its content as markdown.
/// Use this to get the full content of a page found via firecrawl_search or firecrawl_map.
#[tool]
async fn firecrawl_scrape(url: String) -> Result<String> {
    let api_key = firecrawl_api_key()?;

    let (header_name, header_value) = authorization_header(&api_key);
    let response = HTTP_CLIENT
        .post(format!("{FIRECRAWL_BASE_URL}/scrape"))
        .header(header_name, header_value)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "url": url,
            "formats": ["markdown"],
            "onlyMainContent": true,
        }))
        .timeout(SCRAPE_TIMEOUT)
        .send()
        .await
        .map_err(|e| Error::ToolException(format!("Scrape request failed: {e}")))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| Error::ToolException(format!("Failed to parse scrape response: {e}")))?;

    check_response_status(status, &body)?;

    let markdown = body
        .pointer("/data/markdown")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            Error::ToolException("Scrape response did not contain markdown content".into())
        })?;

    Ok(truncate_content(markdown, MAX_SCRAPE_CHARS))
}

/// Discover all URLs on a website. Returns a list of pages found on the site.
/// Use this to understand a website's structure before scraping specific pages.
/// Optionally provide a search query to filter and rank results by relevance.
#[tool]
async fn firecrawl_map(url: String, search: Option<String>, limit: Option<u64>) -> Result<String> {
    let api_key = firecrawl_api_key()?;
    let limit = limit.unwrap_or(100);

    let mut request_body = serde_json::json!({
        "url": url,
        "limit": limit,
    });
    if let Some(search) = search {
        request_body["search"] = Value::String(search);
    }

    let (header_name, header_value) = authorization_header(&api_key);
    let response = HTTP_CLIENT
        .post(format!("{FIRECRAWL_BASE_URL}/map"))
        .header(header_name, header_value)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| Error::ToolException(format!("Map request failed: {e}")))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| Error::ToolException(format!("Failed to parse map response: {e}")))?;

    check_response_status(status, &body)?;
    Ok(format_map_results(&body))
}

/// Crawl a website starting from a URL and return the markdown content of multiple pages.
/// Use this when you need content from several pages on the same site (e.g., documentation sections).
/// The crawl follows links from the starting URL up to the page limit.
#[tool]
async fn firecrawl_crawl(url: String, limit: Option<u64>) -> Result<String> {
    let api_key = firecrawl_api_key()?;
    let limit = limit.unwrap_or(5).min(MAX_CRAWL_PAGES);

    // Start the crawl job
    let (header_name, header_value) = authorization_header(&api_key);
    let response = HTTP_CLIENT
        .post(format!("{FIRECRAWL_BASE_URL}/crawl"))
        .header(header_name, &header_value)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "url": url,
            "limit": limit,
            "scrapeOptions": {
                "formats": ["markdown"],
                "onlyMainContent": true,
            },
        }))
        .timeout(REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| Error::ToolException(format!("Crawl request failed: {e}")))?;

    let status = response.status();
    let body: Value = response
        .json()
        .await
        .map_err(|e| Error::ToolException(format!("Failed to parse crawl response: {e}")))?;

    check_response_status(status, &body)?;

    let job_id = body
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::ToolException("Crawl response did not contain a job ID".into()))?;

    // Poll for completion
    let started = std::time::Instant::now();
    loop {
        if started.elapsed() > CRAWL_TIMEOUT {
            return Err(Error::ToolException(format!(
                "Crawl timed out after {}s. The job '{job_id}' may still be running.",
                CRAWL_TIMEOUT.as_secs()
            )));
        }

        tokio::time::sleep(CRAWL_POLL_INTERVAL).await;

        let (header_name, header_value) = authorization_header(&api_key);
        let poll_response = HTTP_CLIENT
            .get(format!("{FIRECRAWL_BASE_URL}/crawl/{job_id}"))
            .header(header_name, header_value)
            .timeout(REQUEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| Error::ToolException(format!("Crawl status check failed: {e}")))?;

        let poll_status = poll_response.status();
        let poll_body: Value = poll_response
            .json()
            .await
            .map_err(|e| Error::ToolException(format!("Failed to parse crawl status: {e}")))?;

        check_response_status(poll_status, &poll_body)?;

        let crawl_status = poll_body
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match crawl_status {
            "completed" => return Ok(format_crawl_results(&poll_body)),
            "failed" => {
                let error = poll_body
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                return Err(Error::ToolException(format!("Crawl failed: {error}")));
            }
            _ => continue,
        }
    }
}

// ---------------------------------------------------------------------------
// Public constructors
// ---------------------------------------------------------------------------

pub fn firecrawl_tools() -> Vec<Arc<dyn BaseTool>> {
    vec![
        Arc::new(firecrawl_search::tool()),
        Arc::new(firecrawl_scrape::tool()),
        Arc::new(firecrawl_map::tool()),
        Arc::new(firecrawl_crawl::tool()),
    ]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_search_results_v1() {
        let body = serde_json::json!({
            "success": true,
            "data": [
                {
                    "title": "Rust Programming",
                    "url": "https://rust-lang.org",
                    "description": "A systems programming language"
                },
                {
                    "title": "Rust Book",
                    "url": "https://doc.rust-lang.org/book/",
                    "description": "The official Rust book"
                }
            ]
        });

        let formatted = format_search_results(&body);
        assert!(formatted.contains("1. Rust Programming"));
        assert!(formatted.contains("URL: https://rust-lang.org"));
        assert!(formatted.contains("A systems programming language"));
        assert!(formatted.contains("2. Rust Book"));
    }

    #[test]
    fn test_format_search_results_v2() {
        let body = serde_json::json!({
            "success": true,
            "data": {
                "web": [
                    {
                        "title": "Example",
                        "url": "https://example.com",
                        "description": "An example page"
                    }
                ]
            }
        });

        let formatted = format_search_results(&body);
        assert!(formatted.contains("1. Example"));
        assert!(formatted.contains("URL: https://example.com"));
    }

    #[test]
    fn test_format_search_results_empty() {
        let body = serde_json::json!({ "success": true, "data": [] });
        assert_eq!(format_search_results(&body), "No results found.");
    }

    #[test]
    fn test_format_map_results_strings() {
        let body = serde_json::json!({
            "success": true,
            "links": [
                "https://example.com/page1",
                "https://example.com/page2",
            ]
        });

        let formatted = format_map_results(&body);
        assert!(formatted.contains("Found 2 URLs"));
        assert!(formatted.contains("- https://example.com/page1"));
        assert!(formatted.contains("- https://example.com/page2"));
    }

    #[test]
    fn test_format_map_results_objects() {
        let body = serde_json::json!({
            "success": true,
            "links": [
                { "url": "https://example.com", "title": "Home" },
                { "url": "https://example.com/about" }
            ]
        });

        let formatted = format_map_results(&body);
        assert!(formatted.contains("Found 2 URLs"));
        assert!(formatted.contains("- https://example.com — Home"));
        assert!(formatted.contains("- https://example.com/about"));
    }

    #[test]
    fn test_format_map_results_empty() {
        let body = serde_json::json!({ "success": true, "links": [] });
        assert_eq!(format_map_results(&body), "No links found.");
    }

    #[test]
    fn test_format_crawl_results() {
        let body = serde_json::json!({
            "status": "completed",
            "data": [
                {
                    "markdown": "# Page 1\n\nContent of page 1.",
                    "metadata": {
                        "title": "Page One",
                        "sourceURL": "https://example.com/page1"
                    }
                },
                {
                    "markdown": "# Page 2\n\nContent of page 2.",
                    "metadata": {
                        "title": "Page Two",
                        "sourceURL": "https://example.com/page2"
                    }
                }
            ]
        });

        let formatted = format_crawl_results(&body);
        assert!(formatted.contains("Crawled 2 pages"));
        assert!(formatted.contains("Page One"));
        assert!(formatted.contains("https://example.com/page1"));
        assert!(formatted.contains("Content of page 1."));
        assert!(formatted.contains("Page Two"));
    }

    #[test]
    fn test_format_crawl_results_empty() {
        let body = serde_json::json!({ "status": "completed", "data": [] });
        assert_eq!(format_crawl_results(&body), "Crawl returned no pages.");
    }

    #[test]
    fn test_truncate_content_short() {
        let content = "Short content.";
        assert_eq!(truncate_content(content, 100), content);
    }

    #[test]
    fn test_truncate_content_at_paragraph() {
        let content = "First paragraph.\n\nSecond paragraph.\n\nThird very long paragraph that goes on and on.";
        let truncated = truncate_content(content, 40);
        assert!(truncated.starts_with("First paragraph.\n\nSecond paragraph."));
        assert!(truncated.contains("[Content truncated"));
    }

    #[test]
    fn test_check_response_status_success() {
        let body = serde_json::json!({"success": true});
        assert!(check_response_status(reqwest::StatusCode::OK, &body).is_ok());
    }

    #[test]
    fn test_check_response_status_rate_limit() {
        let body = serde_json::json!({"error": "Too many requests"});
        let result = check_response_status(reqwest::StatusCode::TOO_MANY_REQUESTS, &body);
        assert!(result.is_err());
        let error = result.unwrap_err().to_string();
        assert!(error.contains("rate limit"));
        assert!(error.contains("Too many requests"));
    }

    #[test]
    fn test_check_response_status_payment() {
        let body = serde_json::json!({"error": "Credits exhausted"});
        let result = check_response_status(reqwest::StatusCode::PAYMENT_REQUIRED, &body);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("payment required"));
    }
}
