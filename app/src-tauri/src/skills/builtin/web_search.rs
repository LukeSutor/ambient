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
fn get_extraction_script() -> &'static str {
    r#"
    (function() {
        // Prevent multiple executions
        if (window.__scrapeExecuted) return;
        window.__scrapeExecuted = true;
        console.log("[scraper] Starting HTML extraction");
        
        function extractAndSend() {
            try {
                // Get the full HTML content
                const html = document.documentElement.outerHTML;
                console.log({html});
                
                // Encode as base64 to safely pass through URL
                // Use encodeURIComponent first to handle Unicode properly
                const encoded = btoa(unescape(encodeURIComponent(html)));
                
                // Navigate to our custom URL scheme
                // The on_navigation handler will intercept this
                window.location.href = 'scraperesult://data/' + encoded;
                console.log("[scraper] HTML extraction sent: " + encoded.length + " bytes, navigating to scraperesult://data/");
            } catch (e) {
                // Send error
                window.location.href = 'scraperesult://error/' + encodeURIComponent(e.toString());
            }
        }
        
        // Check document ready state
        if (document.readyState === 'complete') {
            // Small delay to let any final scripts run
            extractAndSend();
        } else {
            // Wait for full load
            window.addEventListener('load', function() {
                extractAndSend();
            });
            
            // Fallback: also listen for DOMContentLoaded with longer delay
            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', function() {
                    setTimeout(extractAndSend, 1500);
                });
            }
            
            // Ultimate fallback timeout
            setTimeout(extractAndSend, 4000);
        }
    })();
    "#
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
    let url_string = url.to_string();

    log::debug!(
        "[web_search] Creating WebView window: {} for URL: {}",
        window_label,
        url
    );

    // Channel to receive the scraped HTML
    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_for_nav = tx.clone();

    // Create the WebView window with navigation interception
    let window = WebviewWindowBuilder::new(
        app_handle,
        &window_label,
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
                // Extract the base64 encoded HTML
                let encoded = &path[1..]; // Skip "/"

                match base64_decode_html(encoded) {
                    Ok(html) => {
                        log::debug!("[web_search] Received scraped HTML ({} bytes)", html.len());
                        if let Ok(mut guard) = tx_for_nav.lock() {
                            if let Some(tx) = guard.take() {
                                let _ = tx.send(Ok(html));
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("[web_search] Failed to decode HTML: {}", e);
                        if let Ok(mut guard) = tx_for_nav.lock() {
                            if let Some(tx) = guard.take() {
                                let _ = tx.send(Err(e));
                            }
                        }
                    }
                }
            } else if host == Some("error") {
                let encoded_error = &path[1..]; // Skip "/"
                let error_msg = urlencoding::decode(encoded_error)
                    .unwrap_or_else(|_| "Unknown error".into())
                    .to_string();
                log::error!("[web_search] Scraping error: {}", error_msg);
                if let Ok(mut guard) = tx_for_nav.lock() {
                    if let Some(tx) = guard.take() {
                        let _ = tx.send(Err(error_msg));
                    }
                }
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
    let extraction_script = get_extraction_script();

    tokio::spawn(async move {
        // Wait a moment to ensure the page is loaded
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Inject the extraction script
        match window_clone.eval(extraction_script) {
            Ok(_) => {
                log::debug!("[web_search] Successfully injected extraction script (first attempt)");
                return;
            }
            Err(e) => log::warn!("[web_search] Failed to inject extraction script (first attempt): {}", e),
        }
        // Retry injection after a delay in case the first one was too early
        tokio::time::sleep(Duration::from_millis(1000)).await;
        match window_clone.eval(extraction_script) {
            Ok(_) => log::debug!("[web_search] Successfully injected extraction script (retry attempt)"),
            Err(e) => log::warn!("[web_search] Failed to inject extraction script (retry attempt): {}", e),
        }
    });

    // Wait for result with timeout
    let result = tokio::time::timeout(Duration::from_secs(timeout_secs), rx).await;

    // Clean up: close the window
    let _ = window.destroy();

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
