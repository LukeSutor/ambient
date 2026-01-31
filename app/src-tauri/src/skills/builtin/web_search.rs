//! Web search skill implementation using Tauri WebView.
//!
//! This skill provides web searching and webpage fetching capabilities using
//! a real browser engine (WebView2 on Windows, WebKit on macOS) for authentic
//! browser fingerprinting that bypasses bot detection.
//!
//! # Tools
//!
//! - `search_web`: Perform a web search and return relevant results
//! - `fetch_webpage`: Fetch and extract main content from a specific URL
//!
//! # Bot Detection Bypass Strategies
//!
//! 1. **Real Browser Engine**: Uses system WebView with authentic TLS fingerprint
//! 2. **JavaScript Execution**: Full JS support for dynamic content
//! 3. **Authentic Headers**: Browser sends all proper headers automatically
//! 4. **Cookie/Session Support**: Proper session handling via WebView
//! 5. **Natural Timing**: Random delays to mimic human behavior
//! 6. **Navigation Interception**: Uses custom URL scheme to get data back from external pages

use super::ToolCall;
use once_cell::sync::Lazy;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;
use url::Url;

/// Window label prefix for search scraper windows
const SEARCH_WINDOW_PREFIX: &str = "search_scraper_";

/// Timeout for search operations
const SEARCH_TIMEOUT_SECS: u64 = 600;

/// Timeout for page fetch operations
const FETCH_TIMEOUT_SECS: u64 = 30;

/// Custom URL scheme for receiving scraped data
const SCRAPE_RESULT_SCHEME: &str = "scraperesult";

/// Counter for unique window IDs
static WINDOW_COUNTER: AtomicU64 = AtomicU64::new(0);

type PendingScrapeSender = Arc<Mutex<Option<oneshot::Sender<Result<String, String>>>>>;

struct PendingScrape {
    request_id: String,
    sender: PendingScrapeSender,
    total_chunks: Option<usize>,
    received_chunks: usize,
    chunks: Vec<Option<String>>,
}

static PENDING_SCRAPE: Lazy<Mutex<Option<PendingScrape>>> = Lazy::new(|| Mutex::new(None));

/// Search result returned to the caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Execute a web search tool.
///
/// Routes to the appropriate tool handler based on tool name.
pub async fn execute(app_handle: &AppHandle, call: &ToolCall) -> Result<Value, String> {
    match call.tool_name.as_str() {
        "search_web" => search_web(app_handle, call).await,
        "fetch_webpage" => fetch_webpage(app_handle, call).await,
        _ => Err(format!("Unknown tool: {}", call.tool_name)),
    }
}

/// Generate a unique window label for each scraper instance
fn generate_window_label() -> String {
    let id = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("{}{}", SEARCH_WINDOW_PREFIX, id)
}

fn resolve_pending_scrape(request_id: &str, result: Result<String, String>) {
    if let Ok(mut slot) = PENDING_SCRAPE.lock() {
        if let Some(pending) = slot.take() {
            if pending.request_id == request_id {
                if let Ok(mut guard) = pending.sender.lock() {
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(result);
                    }
                }
            } else {
                *slot = Some(pending);
            }
        }
    }
}

fn add_scrape_chunk(
    request_id: &str,
    chunk_index: usize,
    total_chunks: usize,
    chunk: &str,
) -> Result<(), String> {
    if let Ok(mut slot) = PENDING_SCRAPE.lock() {
        let Some(pending) = slot.as_mut() else {
            return Err("No pending scrape is active".to_string());
        };

        if pending.request_id != request_id {
            return Err("Scrape request id does not match active scrape".to_string());
        }

        if pending.total_chunks.is_none() {
            pending.total_chunks = Some(total_chunks);
            pending.chunks = vec![None; total_chunks];
        } else if pending.total_chunks != Some(total_chunks) {
            return Err("Chunk total does not match active scrape".to_string());
        }

        if chunk_index >= total_chunks {
            return Err("Chunk index out of range".to_string());
        }

        if pending.chunks[chunk_index].is_none() {
            pending.received_chunks += 1;
        }

        let decoded_chunk = urlencoding::decode(chunk)
            .unwrap_or_else(|_| chunk.into())
            .to_string();

        pending.chunks[chunk_index] = Some(decoded_chunk);

        if pending.received_chunks == total_chunks {
            let encoded = pending
                .chunks
                .iter()
                .map(|c| c.as_deref().unwrap_or_default())
                .collect::<String>();
            drop(slot);
            let html = base64_decode_html(&encoded)?;
            log::debug!("html: {}", html);
            resolve_pending_scrape(request_id, Ok(html));
        }

        Ok(())
    } else {
        Err("Failed to acquire scrape lock".to_string())
    }
}

pub fn handle_scrape_error(request_id: &str, encoded_error: &str) {
    let error_msg = urlencoding::decode(encoded_error)
        .unwrap_or_else(|_| "Unknown error".into())
        .to_string();
    resolve_pending_scrape(request_id, Err(error_msg));
}

/// Perform a web search using DuckDuckGo via WebView.
///
/// Uses a hidden WebView window to navigate to DuckDuckGo's HTML interface,
/// which provides authentic browser fingerprinting to bypass bot detection.
async fn search_web(app_handle: &AppHandle, call: &ToolCall) -> Result<Value, String> {
    let query = call
        .arguments
        .get("query")
        .and_then(|q| q.as_str())
        .ok_or_else(|| "Missing 'query' argument".to_string())?;

    log::info!("[web_search] Searching for: {} (via WebView)", query);

    // Build the DuckDuckGo search URL
    let search_url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding::encode(query)
    );

    // Scrape the page using WebView
    let html = scrape_url_with_webview(app_handle, &search_url, SEARCH_TIMEOUT_SECS).await?;

    // Parse the results
    let results = parse_ddg_results(&html)?;

    log::info!(
        "[web_search] Found {} results for query: {}",
        results.len(),
        query
    );

    Ok(serde_json::json!({
        "results": results,
        "query": query
    }))
}

/// Parse DuckDuckGo HTML search results.
fn parse_ddg_results(html: &str) -> Result<Vec<SearchResult>, String> {
    let document = Html::parse_document(html);

    // Check for bot detection / CAPTCHA page
    if html.contains("anomaly-modal") || html.contains("select all squares containing") {
        log::warn!("[web_search] Bot detection triggered - CAPTCHA page detected");
        return Err(
            "DuckDuckGo bot detection triggered. Please try again in a few minutes.".to_string(),
        );
    }

    let result_selector =
        Selector::parse(".result").map_err(|_| "Failed to parse result selector")?;
    let title_selector =
        Selector::parse(".result__a").map_err(|_| "Failed to parse title selector")?;
    let snippet_selector =
        Selector::parse(".result__snippet").map_err(|_| "Failed to parse snippet selector")?;

    let mut results = Vec::new();

    for element in document.select(&result_selector) {
        // Skip ads
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

            // Skip empty results
            let title = title_node.text().collect::<String>().trim().to_string();
            let snippet = snippet_node.text().collect::<String>().trim().to_string();

            if !title.is_empty() && !clean_url.is_empty() {
                results.push(SearchResult {
                    title,
                    url: clean_url,
                    snippet,
                });
            }
        }

        // Limit to 5 results
        if results.len() >= 5 {
            break;
        }
    }

    if results.is_empty() {
        log::warn!(
            "[web_search] No results parsed from HTML (length: {})",
            html.len()
        );
    }

    Ok(results)
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

/// Fetch and extract main content from a specific URL using WebView.
async fn fetch_webpage(app_handle: &AppHandle, call: &ToolCall) -> Result<Value, String> {
    let url = call
        .arguments
        .get("url")
        .and_then(|u| u.as_str())
        .ok_or_else(|| "Missing 'url' argument".to_string())?;

    log::info!("[web_search] Fetching webpage via WebView: {}", url);

    // Scrape the page using WebView
    let html = scrape_url_with_webview(app_handle, url, FETCH_TIMEOUT_SECS).await?;

    // Extract main content as markdown
    let document = Html::parse_document(&html);
    let markdown = extract_content_as_markdown(&document)?;

    Ok(serde_json::json!({
        "url": url,
        "content": markdown
    }))
}

/// Extract main content from HTML document as markdown.
///
/// Uses dom-content-extraction for intelligent content extraction,
/// with fallback to basic text extraction.
fn extract_content_as_markdown(document: &Html) -> Result<String, String> {
    use dom_content_extraction::{extract_content_as_markdown as extract_md, DensityTree};

    match DensityTree::from_document(document) {
        Ok(dtree) => extract_md(&dtree, document)
            .map_err(|_| "Failed to convert content to markdown".to_string()),
        Err(_) => {
            // Fallback: extract all text from body
            let body_selector = Selector::parse("body").map_err(|_| "Failed to parse selector")?;
            let text = document
                .select(&body_selector)
                .next()
                .map(|body| body.text().collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            Ok(text.trim().to_string())
        }
    }
}

/// JavaScript code to extract HTML and send it via navigation.
///
/// This script:
/// 1. Waits for the page to fully load
/// 2. Extracts the full HTML content
/// 3. Encodes it as base64
/// 4. Navigates to a custom URL scheme that we intercept
fn get_extraction_script(request_id: &str) -> String {
    format!(
        r#"
    (function() {{
        // Prevent multiple executions
        if (window.__scrapeExecuted) return;
        window.__scrapeExecuted = true;
        console.log("[scraper] Starting HTML extraction");
        const requestId = "{request_id}";
        const chunkSize = 1800;
        const baseUrl = "scraperesult://data";
        const errorUrl = "scraperesult://error";

        function sendChunk(index, total, chunk) {{
            const encodedChunk = encodeURIComponent(chunk);
            const url = `${{baseUrl}}/${{encodeURIComponent(requestId)}}/${{index}}/${{total}}/${{encodedChunk}}`;
            window.location.href = url;
        }}

        function sendError(err) {{
            const encodedError = encodeURIComponent(err);
            window.location.href = `${{errorUrl}}/${{encodeURIComponent(requestId)}}/${{encodedError}}`;
        }}

        function extractAndSend() {{
            try {{
                // Get the full HTML content
                const html = document.documentElement.outerHTML;
                console.log({{html}});

                // Encode as base64 to safely pass through URL
                // Use encodeURIComponent first to handle Unicode properly
                const encoded = btoa(unescape(encodeURIComponent(html)));
                const total = Math.ceil(encoded.length / chunkSize);

                let i = 0;
                function sendNext() {{
                    const start = i * chunkSize;
                    const chunk = encoded.slice(start, start + chunkSize);
                    sendChunk(i, total, chunk);
                    i += 1;
                    if (i < total) {{
                        setTimeout(sendNext, 5);
                    }}
                }}

                sendNext();
            }} catch (e) {{
                sendError(e.toString());
            }}
        }}

        // Check document ready state
        if (document.readyState === 'complete') {{
            // Small delay to let any final scripts run
            extractAndSend();
        }} else {{
            // Wait for full load
            window.addEventListener('load', function() {{
                extractAndSend();
            }});

            // Fallback: also listen for DOMContentLoaded with longer delay
            if (document.readyState === 'loading') {{
                document.addEventListener('DOMContentLoaded', function() {{
                    setTimeout(extractAndSend, 1500);
                }});
            }}

            // Ultimate fallback timeout
            setTimeout(extractAndSend, 4000);
        }}
    }})();
    "#
    )
}

/// Scrape a URL using a hidden WebView window.
///
/// This creates a temporary, invisible WebView that navigates to the target URL,
/// waits for the page to load, extracts the HTML content via JavaScript,
/// and returns it. The WebView provides authentic browser fingerprinting.
///
/// # Bot Detection Bypass
///
/// - Uses real browser engine (WebView2/WebKit) with authentic TLS fingerprint
/// - Full JavaScript execution for dynamic pages
/// - Proper cookie and session handling
/// - All standard browser headers sent automatically
/// - Uses navigation interception to get data back (works on external domains)
async fn scrape_url_with_webview(
    app_handle: &AppHandle,
    url: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    let window_label = generate_window_label();
    let request_id = window_label.clone();
    let url_string = url.to_string();

    log::debug!(
        "[web_search] Creating WebView window for URL: {}",
        url
    );

    // Channel to receive the scraped HTML
    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    if let Ok(mut slot) = PENDING_SCRAPE.lock() {
        if slot.is_some() {
            return Err("Another web scrape is already in progress".to_string());
        }
        *slot = Some(PendingScrape {
            request_id: request_id.clone(),
            sender: tx.clone(),
            total_chunks: None,
            received_chunks: 0,
            chunks: Vec::new(),
        });
    }

    // Create the WebView window with navigation interception
    let window = WebviewWindowBuilder::new(
        app_handle,
        "web-search-scraper",
        WebviewUrl::External(
            url_string
                .parse()
                .map_err(|e| format!("Invalid URL: {}", e))?,
        ),
    )
    .title("Web Search")
    .inner_size(1280.0, 800.0)
    .visible(true) // Hidden window
    .focused(false)
    .skip_taskbar(true)
    .on_navigation(move |nav_url| {
        // Check if this is our custom scheme with scraped data
        if nav_url.scheme() == SCRAPE_RESULT_SCHEME {
            log::debug!("[web_search] Intercepted scraper result navigation");
            let host = nav_url.host_str();
            let path = nav_url.path();

            if host == Some("data") {
                let mut parts = path.trim_start_matches('/').splitn(4, '/');
                let Some(req_id) = parts.next() else { return false; };
                let Some(index_str) = parts.next() else { return false; };
                let Some(total_str) = parts.next() else { return false; };
                let Some(chunk) = parts.next() else { return false; };

                let index = index_str.parse::<usize>().unwrap_or(usize::MAX);
                let total = total_str.parse::<usize>().unwrap_or(0);

                if index == usize::MAX || total == 0 {
                    log::error!("[web_search] Invalid chunk metadata: {}/{}", index_str, total_str);
                    return false;
                }

                log::debug!(
                    "[web_search] Received scrape chunk {}/{} for request {}",
                    index + 1,
                    total,
                    req_id
                );

                if let Err(e) = add_scrape_chunk(req_id, index, total, chunk) {
                    log::error!("[web_search] Failed to add scrape chunk: {}", e);
                }
            } else if host == Some("error") {
                let mut parts = path.trim_start_matches('/').splitn(2, '/');
                let Some(req_id) = parts.next() else { return false; };
                let Some(encoded_error) = parts.next() else { return false; };
                log::error!("[web_search] Scraping error received");
                handle_scrape_error(req_id, encoded_error);
            }

            // Don't navigate to our custom scheme
            return false;
        }

        // Allow all other navigations
        true
    })
    .build()
    .map_err(|e| format!("Failed to create WebView window: {}", e))?;

    // Wait for initial page load, then inject the extraction script
    let window_clone = window.clone();
    let extraction_script = get_extraction_script(&request_id);

    tokio::spawn(async move {
        // Wait a moment to ensure the page is loaded
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Inject the extraction script
        match window_clone.eval(&extraction_script) {
            Ok(_) => {
                log::debug!("[web_search] Successfully injected extraction script (first attempt)");
                return;
            }
            Err(e) => log::warn!("[web_search] Failed to inject extraction script (first attempt): {}", e),
        }
        // Retry injection after a delay in case the first one was too early
        tokio::time::sleep(Duration::from_millis(1000)).await;
        match window_clone.eval(&extraction_script) {
            Ok(_) => log::debug!("[web_search] Successfully injected extraction script (retry attempt)"),
            Err(e) => log::warn!("[web_search] Failed to inject extraction script (retry attempt): {}", e),
        }
    });

    // Wait for result with timeout
    let result = tokio::time::timeout(Duration::from_secs(timeout_secs), rx).await;

    // Clean up: close the window
    let _ = window.destroy();

    if let Ok(mut slot) = PENDING_SCRAPE.lock() {
        if let Some(pending) = slot.as_ref() {
            if pending.request_id == request_id {
                *slot = None;
            }
        }
    }

    match result {
        Ok(Ok(Ok(html))) => {
            log::debug!("[web_search] Successfully scraped {} bytes", html.len());
            Ok(html)
        }
        Ok(Ok(Err(e))) => Err(e),
        Ok(Err(_)) => Err("Channel closed unexpectedly".to_string()),
        Err(_) => Err(format!(
            "Scraping timed out after {} seconds",
            timeout_secs
        )),
    }
}

/// Decode base64-encoded HTML from the navigation URL.
fn base64_decode_html(encoded: &str) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let decoded_bytes = STANDARD
        .decode(encoded)
        .map_err(|e| format!("Base64 decode error: {}", e))?;

    let decoded_str =
        String::from_utf8(decoded_bytes).map_err(|e| format!("UTF-8 decode error: {}", e))?;

    // The JS does encodeURIComponent before btoa, so we need to decode that
    let html = urlencoding::decode(&decoded_str)
        .map_err(|e| format!("URL decode error: {}", e))?
        .to_string();

    Ok(html)
}
