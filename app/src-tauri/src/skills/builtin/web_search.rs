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
use tauri::{AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;
use url::Url;
use uuid::Uuid;

/// Window label prefix for search scraper windows
const SEARCH_WINDOW_PREFIX: &str = "search_scraper_";

/// Timeout for search operations
const SEARCH_TIMEOUT_SECS: u64 = 600;

/// Timeout for page fetch operations
const FETCH_TIMEOUT_SECS: u64 = 30;

/// Scraper shell window label
const SCRAPER_SHELL_WINDOW_LABEL: &str = "webview-scraper";

/// Scraper shell route (app page)
const SCRAPER_SHELL_PATH: &str = "/webview-scraper";

/// Event names for scraper IPC
const EVENT_SCRAPER_LOAD: &str = "webview_scraper_load";
const EVENT_SCRAPER_RESULT: &str = "webview_scraper_result";
const EVENT_SCRAPER_ERROR: &str = "webview_scraper_error";
const EVENT_SCRAPER_SET_HTML: &str = "webview_scraper_set_html";

/// Counter for unique window IDs
static WINDOW_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Search result returned to the caller
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScrapeLoadPayload {
    request_id: String,
    url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScrapeSetHtmlPayload {
    request_id: String,
    html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScrapeResultPayload {
    request_id: String,
    html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScrapeErrorPayload {
    request_id: String,
    error: String,
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
    // let id = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
    // format!("{}{}", SEARCH_WINDOW_PREFIX, id)
    "search-scraper".to_string()
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

/// JavaScript code to extract HTML and send it via Tauri IPC.
///
/// This script:
/// 1. Waits for the page to fully load
/// 2. Extracts the full HTML content
/// 3. Emits the payload via Tauri event IPC (no URL size limits)
fn get_extraction_script(request_id: &str) -> String {
    let request_id_json = serde_json::to_string(request_id).unwrap_or_else(|_| "\"\"".into());

    format!(
        r#"
    (function() {{
        // Prevent multiple executions
        if (window.__scrapeExecuted) return;
        window.__scrapeExecuted = true;

        const requestId = {request_id_json};

        function emitResult(html) {{
            try {{
                const tauri = window.__TAURI__;
                if (tauri && tauri.event && tauri.event.emit) {{
                    tauri.event.emit("{EVENT_SCRAPER_RESULT}", {{ requestId, html }});
                }} else {{
                    console.error("[scraper] Tauri event emitter not available");
                }}
            }} catch (e) {{
                console.error("[scraper] Failed to emit result", e);
            }}
        }}

        function emitError(error) {{
            try {{
                const tauri = window.__TAURI__;
                if (tauri && tauri.event && tauri.event.emit) {{
                    tauri.event.emit("{EVENT_SCRAPER_ERROR}", {{ requestId, error }});
                }} else {{
                    console.error("[scraper] Tauri event emitter not available");
                }}
            }} catch (e) {{
                console.error("[scraper] Failed to emit error", e);
            }}
        }}

        function extractAndSend() {{
            try {{
                const html = document.documentElement.outerHTML;
                emitResult(html);
            }} catch (e) {{
                emitError(e && e.toString ? e.toString() : String(e));
            }}
        }}

        // Check document ready state
        if (document.readyState === 'complete') {{
            // Small delay to let any final scripts run
            setTimeout(extractAndSend, 800);
        }} else {{
            // Wait for full load
            window.addEventListener('load', function() {{
                setTimeout(extractAndSend, 800);
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
    "#,
    )
}

/// Scrape a URL using a hidden WebView window.
///
/// This creates a temporary, hidden WebView that navigates to the target URL,
/// waits for the page to load, extracts the HTML content via JavaScript,
/// and returns it. The WebView provides authentic browser fingerprinting.
///
/// A separate visible "scraper shell" window is used to host an iframe that
/// displays the fetched page and can be manipulated by the app UI.
///
/// # Bot Detection Bypass
///
/// - Uses real browser engine (WebView2/WebKit) with authentic TLS fingerprint
/// - Full JavaScript execution for dynamic pages
/// - Proper cookie and session handling
/// - All standard browser headers sent automatically
/// - Uses IPC for large payloads (no URL length limits)
async fn scrape_url_with_webview(
    app_handle: &AppHandle,
    url: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    let request_id = Uuid::new_v4().to_string();
    let window_label = generate_window_label();
    let url_string = url.to_string();

    log::debug!(
        "[web_search] Creating WebView window: {} for URL: {}",
        window_label,
        url
    );

    // Ensure the scraper shell window exists (app-controlled UI with iframe)
    let shell_window = if let Some(window) = app_handle.get_webview_window(SCRAPER_SHELL_WINDOW_LABEL) {
        window
    } else {
        WebviewWindowBuilder::new(
            app_handle,
            SCRAPER_SHELL_WINDOW_LABEL,
            WebviewUrl::App(SCRAPER_SHELL_PATH.into()),
        )
        .title("Webview Scraper")
        .inner_size(1280.0, 800.0)
        .visible(true)
        .focused(false)
        .skip_taskbar(true)
        .build()
        .map_err(|e| format!("Failed to create scraper shell window: {}", e))?
    };

    let _ = shell_window.show();
    let _ = shell_window.emit(
        EVENT_SCRAPER_LOAD,
        ScrapeLoadPayload {
            request_id: request_id.clone(),
            url: url_string.clone(),
        },
    );

    // Channel to receive the scraped HTML
    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let tx_for_result = tx.clone();
    let request_id_for_result = request_id.clone();
    let result_listener_id = app_handle.listen(EVENT_SCRAPER_RESULT, move |event| {
        let payload = event.payload();
        let parsed: ScrapeResultPayload = match serde_json::from_str(payload) {
            Ok(parsed) => parsed,
            Err(_) => return,
        };
        if parsed.request_id != request_id_for_result {
            return;
        }
        if let Ok(mut guard) = tx_for_result.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(Ok(parsed.html));
            }
        }
    });

    let tx_for_error = tx.clone();
    let request_id_for_error = request_id.clone();
    let error_listener_id = app_handle.listen(EVENT_SCRAPER_ERROR, move |event| {
        let payload = event.payload();
        let parsed: ScrapeErrorPayload = match serde_json::from_str(payload) {
            Ok(parsed) => parsed,
            Err(_) => return,
        };
        if parsed.request_id != request_id_for_error {
            return;
        }
        if let Ok(mut guard) = tx_for_error.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(Err(parsed.error));
            }
        }
    });

    // Create the hidden WebView window for scraping
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
    .visible(true)
    .focused(false)
    .skip_taskbar(true)
    .build()
    .map_err(|e| format!("Failed to create WebView window: {}", e))?;

    // Wait for initial page load, then inject the extraction script
    let window_clone = window.clone();
    let extraction_script = get_extraction_script(&request_id);

    tokio::spawn(async move {
        // Wait a moment to ensure the page is loaded
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Inject the extraction script
        match window_clone.eval(extraction_script.clone()) {
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

    // Clean up event listeners and window
    app_handle.unlisten(result_listener_id);
    app_handle.unlisten(error_listener_id);
    let _ = window.close();

    match result {
        Ok(Ok(Ok(html))) => {
            log::debug!("[web_search] Successfully scraped {} bytes", html.len());
            let _ = shell_window.emit(
                EVENT_SCRAPER_SET_HTML,
                ScrapeSetHtmlPayload {
                    request_id: request_id.clone(),
                    html: html.clone(),
                },
            );
            Ok(html)
        }
        Ok(Ok(Err(e))) => {
            let _ = shell_window.emit(
                EVENT_SCRAPER_ERROR,
                ScrapeErrorPayload {
                    request_id: request_id.clone(),
                    error: e.clone(),
                },
            );
            Err(e)
        }
        Ok(Err(_)) => Err("Channel closed unexpectedly".to_string()),
        Err(_) => {
            let error = format!("Scraping timed out after {} seconds", timeout_secs);
            let _ = shell_window.emit(
                EVENT_SCRAPER_ERROR,
                ScrapeErrorPayload {
                    request_id: request_id.clone(),
                    error: error.clone(),
                },
            );
            Err(error)
        }
    }
}
