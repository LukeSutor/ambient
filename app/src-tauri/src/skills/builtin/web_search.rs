//! Web search skill implementation.
//!
//! This skill provides web searching and webpage fetching capabilities.
//!
//! # Tools
//!
//! - `search_web`: Perform a web search and return relevant results
//! - `fetch_webpage`: Fetch and extract main content from a specific URL

use super::ToolCall;
use serde_json::Value;
use tauri::AppHandle;
use scraper::{Html, Selector};
use dom_content_extraction::{DensityTree, extract_content_as_markdown};
use url::Url;

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(serde::Serialize)]
pub struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

/// Execute a web search tool.
///
/// Routes to the appropriate tool handler based on tool name.
pub async fn execute(
    _app_handle: &AppHandle,
    call: &ToolCall,
) -> Result<Value, String> {
    match call.tool_name.as_str() {
        "search_web" => search_web(call).await,
        "fetch_webpage" => fetch_webpage(call).await,
        _ => Err(format!("Unknown tool: {}", call.tool_name)),
    }
}

/// Perform a web search using DuckDuckGo HTML version.
async fn search_web(call: &ToolCall) -> Result<Value, String> {
    let query = call
        .arguments
        .get("query")
        .and_then(|q| q.as_str())
        .ok_or_else(|| "Missing 'query' argument".to_string())?;

    log::info!("[web_search] Searching for: {}", query);

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let url = format!("https://html.duckduckgo.com/html/?q={}", urlencoding::encode(query));
    let html = client.get(url).send().await
        .map_err(|e| format!("Search request failed: {}", e))?
        .text().await
        .map_err(|e| format!("Failed to read search response: {}", e))?;

    let document = Html::parse_document(&html);
    let result_selector = Selector::parse(".result").unwrap();
    let title_selector = Selector::parse(".result__a").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();

    // Log the raw HTML for debugging
    log::debug!("[web_search] Search results HTML: {}", html);

    let mut results = Vec::new();

    for element in document.select(&result_selector) {
        // Skip ads: check if the element's class contains "result--ad"
        if let Some(class) = element.value().attr("class") {
            if class.contains("result--ad") {
                continue;
            }
        }

        let title_elem = element.select(&title_selector).next();
        let snippet_elem = element.select(&snippet_selector).next();

        if let (Some(title_node), Some(snippet_node)) = (title_elem, snippet_elem) {
            let raw_url = title_node.value().attr("href").unwrap_or("");
            let clean_url = clean_ddg_url(raw_url);

            results.push(SearchResult {
                title: title_node.text().collect::<String>().trim().to_string(),
                url: clean_url,
                snippet: snippet_node.text().collect::<String>().trim().to_string(),
            });
        }

        // Stop once we have 3 results
        if results.len() >= 3 {
            break;
        }
    }

    Ok(serde_json::json!({
        "results": results,
        "query": query
    }))
}

/// Helper to extract the actual target URL from DuckDuckGo redirection links.
fn clean_ddg_url(raw_url: &str) -> String {
    let mut url_str = raw_url.to_string();
    
    // Handle protocol-relative URLs
    if url_str.starts_with("//") {
        url_str = format!("https:{}", url_str);
    }
    
    // Handle relative URLs starting with /
    if url_str.starts_with('/') && !url_str.starts_with("//") {
        url_str = format!("https://duckduckgo.com{}", url_str);
    }
    
    if let Ok(parsed) = Url::parse(&url_str) {
        // DuckDuckGo HTML puts the real URL in the 'uddg' query parameter
        if let Some((_, actual_url)) = parsed.query_pairs().find(|(k, _)| k == "uddg") {
            return actual_url.to_string();
        }
    }
    
    url_str
}

/// Fetch and extract main content from a specific URL.
async fn fetch_webpage(call: &ToolCall) -> Result<Value, String> {
    let url = call
        .arguments
        .get("url")
        .and_then(|u| u.as_str())
        .ok_or_else(|| "Missing 'url' argument".to_string())?;

    log::info!("[web_search] Fetching webpage: {}", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))?;

    let html = client.get(url).send().await
        .map_err(|e| format!("Fetch request failed: {}", e))?
        .text().await
        .map_err(|e| format!("Failed to read page content: {}", e))?;

    let document = Html::parse_document(&html);
    let dtree = DensityTree::from_document(&document)
        .map_err(|_| "Failed to analyze page structure".to_string())?;
    
    let markdown = extract_content_as_markdown(&dtree, &document)
        .map_err(|_| "Failed to convert content to markdown".to_string())?;

    Ok(serde_json::json!({
        "url": url,
        "content": markdown
    }))
}
