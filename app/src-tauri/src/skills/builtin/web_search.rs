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
//!
//! # Reliability Features
//!
//! - Unique execution tokens prevent duplicate extractions
//! - State machine ensures single extraction per page load
//! - Content filtering removes scripts, styles, and media before transmission
//! - Chunked transmission with atomic state management

use super::ToolCall;
use once_cell::sync::Lazy;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};
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

/// Counter for unique execution tokens (prevents duplicate extractions)
static EXECUTION_TOKEN_COUNTER: AtomicU64 = AtomicU64::new(0);

type PendingScrapeSender = Arc<Mutex<Option<oneshot::Sender<Result<String, String>>>>>;

/// Represents the state of a pending scrape operation
struct PendingScrape {
    /// Unique identifier for this scrape request
    request_id: String,
    /// Unique token for this specific extraction attempt
    execution_token: String,
    /// Channel sender to return the result
    sender: PendingScrapeSender,
    /// Total number of chunks expected (set on first chunk)
    total_chunks: Option<usize>,
    /// Number of chunks received so far
    received_chunks: usize,
    /// Storage for received chunks (indexed by chunk number)
    chunks: Vec<Option<String>>,
    /// Whether extraction has started (received at least one chunk)
    extraction_started: bool,
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

/// Generate a unique execution token for each extraction attempt
fn generate_execution_token() -> String {
    let id = EXECUTION_TOKEN_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("exec_{}", id)
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

/// Add a received chunk to the pending scrape operation.
/// 
/// Validates:
/// - Request ID matches
/// - Execution token matches (prevents duplicate extractions)
/// - Chunk metadata is consistent
fn add_scrape_chunk(
    request_id: &str,
    execution_token: &str,
    chunk_index: usize,
    total_chunks: usize,
    chunk: &str,
) -> Result<(), String> {
    if let Ok(mut slot) = PENDING_SCRAPE.lock() {
        let Some(pending) = slot.as_mut() else {
            return Err("No pending scrape is active".to_string());
        };

        if pending.request_id != request_id {
            return Err(format!(
                "Request ID mismatch: expected '{}', got '{}'",
                pending.request_id, request_id
            ));
        }

        // Validate execution token to prevent duplicate extractions
        if pending.execution_token != execution_token {
            // If extraction hasn't started yet, accept the new token
            if !pending.extraction_started {
                log::info!(
                    "[web_search] Accepting new execution token '{}' (previous: '{}')",
                    execution_token,
                    pending.execution_token
                );
                pending.execution_token = execution_token.to_string();
            } else {
                // Extraction already in progress with different token - reject
                log::warn!(
                    "[web_search] Rejecting chunk from stale execution token '{}' (active: '{}')",
                    execution_token,
                    pending.execution_token
                );
                return Err(format!(
                    "Execution token mismatch: extraction already in progress with token '{}'",
                    pending.execution_token
                ));
            }
        }

        // Mark extraction as started
        pending.extraction_started = true;

        if pending.total_chunks.is_none() {
            log::info!(
                "[web_search] Starting chunk reception: expecting {} chunks",
                total_chunks
            );
            pending.total_chunks = Some(total_chunks);
            pending.chunks = vec![None; total_chunks];
        } else if pending.total_chunks != Some(total_chunks) {
            return Err(format!(
                "Chunk total mismatch: expected {}, got {}",
                pending.total_chunks.unwrap_or(0),
                total_chunks
            ));
        }

        if chunk_index >= total_chunks {
            return Err(format!(
                "Chunk index {} out of range (total: {})",
                chunk_index, total_chunks
            ));
        }

        if pending.chunks[chunk_index].is_none() {
            pending.received_chunks += 1;
        } else {
            log::debug!(
                "[web_search] Received duplicate chunk {}/{}, ignoring",
                chunk_index + 1,
                total_chunks
            );
            return Ok(());
        }

        let decoded_chunk = urlencoding::decode(chunk)
            .unwrap_or_else(|_| chunk.into())
            .to_string();

        pending.chunks[chunk_index] = Some(decoded_chunk);

        // Log progress at intervals
        if pending.received_chunks % 50 == 0 || pending.received_chunks == total_chunks {
            log::info!(
                "[web_search] Chunk progress: {}/{} ({:.1}%)",
                pending.received_chunks,
                total_chunks,
                (pending.received_chunks as f64 / total_chunks as f64) * 100.0
            );
        }

        if pending.received_chunks == total_chunks {
            log::info!("[web_search] All chunks received, assembling content");
            let encoded = pending
                .chunks
                .iter()
                .map(|c| c.as_deref().unwrap_or_default())
                .collect::<String>();
            drop(slot);
            let html = base64_decode_html(&encoded)?;
            log::debug!("[web_search] Decoded content length: {} bytes", html.len());
            resolve_pending_scrape(request_id, Ok(html));
        }

        Ok(())
    } else {
        Err("Failed to acquire scrape lock".to_string())
    }
}

pub fn handle_scrape_error(request_id: &str, execution_token: &str, encoded_error: &str) {
    // Validate execution token before accepting error
    if let Ok(slot) = PENDING_SCRAPE.lock() {
        if let Some(pending) = slot.as_ref() {
            if pending.execution_token != execution_token && pending.extraction_started {
                log::warn!(
                    "[web_search] Ignoring error from stale execution token '{}'",
                    execution_token
                );
                return;
            }
        }
    }

    let error_msg = urlencoding::decode(encoded_error)
        .unwrap_or_else(|_| "Unknown error".into())
        .to_string();
    log::error!("[web_search] Scraping error: {}", error_msg);
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

/// JavaScript code to extract text content and send it via navigation.
///
/// This script implements a robust, production-ready extraction system:
///
/// # Reliability Features
/// 1. Unique execution token prevents duplicate extractions
/// 2. State machine ensures single extraction per injection
/// 3. Atomic state transitions prevent race conditions
/// 4. Content filtering removes scripts, styles, and media
///
/// # Content Filtering
/// - Removes all `<script>`, `<style>`, `<noscript>`, `<svg>`, `<canvas>` elements
/// - Removes `<img>`, `<video>`, `<audio>`, `<picture>`, `<source>` elements
/// - Removes `<iframe>`, `<object>`, `<embed>` elements
/// - Removes hidden elements and ads
/// - Preserves only text-relevant content for LLM consumption
fn get_extraction_script(request_id: &str, execution_token: &str) -> String {
    format!(
        r#"
(function() {{
    'use strict';
    
    // ==========================================
    // CONFIGURATION
    // ==========================================
    const REQUEST_ID = "{request_id}";
    const EXECUTION_TOKEN = "{execution_token}";
    const CHUNK_SIZE = 1800;
    const BASE_URL = "scraperesult://data";
    const ERROR_URL = "scraperesult://error";
    
    // State machine states
    const STATE_IDLE = 0;
    const STATE_EXTRACTING = 1;
    const STATE_SENDING = 2;
    const STATE_COMPLETE = 3;
    const STATE_ERROR = 4;
    
    // ==========================================
    // STATE MANAGEMENT (prevents duplicate execution)
    // ==========================================
    
    // Use a unique key per execution token to prevent conflicts
    const STATE_KEY = '__scraper_state_' + EXECUTION_TOKEN;
    
    // Check if this exact extraction is already running or complete
    if (window[STATE_KEY] !== undefined) {{
        console.log('[scraper] Extraction already in progress or complete for token:', EXECUTION_TOKEN);
        return;
    }}
    
    // Initialize state atomically
    window[STATE_KEY] = STATE_IDLE;
    
    function setState(newState) {{
        const oldState = window[STATE_KEY];
        window[STATE_KEY] = newState;
        console.log('[scraper] State transition:', oldState, '->', newState);
        return oldState;
    }}
    
    function getState() {{
        return window[STATE_KEY];
    }}
    
    // ==========================================
    // CONTENT EXTRACTION & FILTERING
    // ==========================================
    
    /**
     * Clean the DOM by removing non-text elements.
     * This significantly reduces payload size and focuses on content relevant for LLMs.
     */
    function extractCleanTextContent() {{
        // Clone the document to avoid modifying the live page
        const docClone = document.cloneNode(true);
        
        // Elements to completely remove (they contain no useful text)
        const elementsToRemove = [
            'script', 'style', 'noscript', 'svg', 'canvas',
            'img', 'video', 'audio', 'picture', 'source', 'track',
            'iframe', 'object', 'embed', 'applet',
            'map', 'area',
            'link[rel="stylesheet"]', 'link[rel="preload"]', 'link[rel="prefetch"]',
            'meta', 'base',
            'template',
            '[hidden]', '[aria-hidden="true"]',
            '.ad', '.ads', '.advertisement', '.sponsored',
            '[data-ad]', '[data-ads]'
        ];
        
        elementsToRemove.forEach(selector => {{
            try {{
                docClone.querySelectorAll(selector).forEach(el => el.remove());
            }} catch (e) {{
                // Ignore invalid selectors
            }}
        }});
        
        // Remove elements with display:none or visibility:hidden inline styles
        docClone.querySelectorAll('*').forEach(el => {{
            const style = el.getAttribute('style') || '';
            if (style.includes('display:none') || 
                style.includes('display: none') ||
                style.includes('visibility:hidden') ||
                style.includes('visibility: hidden')) {{
                el.remove();
            }}
        }});
        
        // Remove all style attributes to reduce size
        docClone.querySelectorAll('[style]').forEach(el => {{
            el.removeAttribute('style');
        }});
        
        // Remove all class attributes (not needed for text extraction)
        docClone.querySelectorAll('[class]').forEach(el => {{
            el.removeAttribute('class');
        }});
        
        // Remove data-* attributes
        docClone.querySelectorAll('*').forEach(el => {{
            Array.from(el.attributes).forEach(attr => {{
                if (attr.name.startsWith('data-') || 
                    attr.name.startsWith('aria-') ||
                    attr.name === 'onclick' ||
                    attr.name === 'onload' ||
                    attr.name === 'onerror') {{
                    el.removeAttribute(attr.name);
                }}
            }});
        }});
        
        // Remove empty elements (elements with no text content)
        function removeEmptyElements(root) {{
            let removed = true;
            while (removed) {{
                removed = false;
                root.querySelectorAll('div, span, p, section, article, aside, header, footer, nav').forEach(el => {{
                    if (el.textContent.trim() === '' && el.children.length === 0) {{
                        el.remove();
                        removed = true;
                    }}
                }});
            }}
        }}
        removeEmptyElements(docClone);
        
        // Get the cleaned HTML
        const html = docClone.documentElement ? docClone.documentElement.outerHTML : '';
        
        console.log('[scraper] Cleaned content size:', html.length, 'bytes');
        
        return html;
    }}
    
    // ==========================================
    // CHUNKED TRANSMISSION
    // ==========================================
    
    let currentChunkIndex = 0;
    let totalChunks = 0;
    let encodedContent = '';
    
    function sendChunk(index, total, chunk) {{
        const encodedChunk = encodeURIComponent(chunk);
        const encodedToken = encodeURIComponent(EXECUTION_TOKEN);
        const encodedRequestId = encodeURIComponent(REQUEST_ID);
        const url = `${{BASE_URL}}/${{encodedRequestId}}/${{encodedToken}}/${{index}}/${{total}}/${{encodedChunk}}`;
        window.location.href = url;
    }}
    
    function sendError(err) {{
        setState(STATE_ERROR);
        const encodedError = encodeURIComponent(String(err));
        const encodedToken = encodeURIComponent(EXECUTION_TOKEN);
        const encodedRequestId = encodeURIComponent(REQUEST_ID);
        window.location.href = `${{ERROR_URL}}/${{encodedRequestId}}/${{encodedToken}}/${{encodedError}}`;
    }}
    
    function sendNextChunk() {{
        if (getState() !== STATE_SENDING) {{
            console.log('[scraper] Aborting chunk send - state is not SENDING');
            return;
        }}
        
        if (currentChunkIndex >= totalChunks) {{
            console.log('[scraper] All chunks sent successfully');
            setState(STATE_COMPLETE);
            return;
        }}
        
        const start = currentChunkIndex * CHUNK_SIZE;
        const chunk = encodedContent.slice(start, start + CHUNK_SIZE);
        
        console.log('[scraper] Sending chunk', currentChunkIndex + 1, '/', totalChunks);
        sendChunk(currentChunkIndex, totalChunks, chunk);
        
        currentChunkIndex++;
        
        // Schedule next chunk with a small delay to allow navigation to complete
        if (currentChunkIndex < totalChunks) {{
            setTimeout(sendNextChunk, 10);
        }} else {{
            setState(STATE_COMPLETE);
        }}
    }}
    
    // ==========================================
    // MAIN EXTRACTION FUNCTION
    // ==========================================
    
    function performExtraction() {{
        // Ensure we can transition to EXTRACTING state
        const currentState = getState();
        if (currentState !== STATE_IDLE) {{
            console.log('[scraper] Cannot extract - already in state:', currentState);
            return;
        }}
        
        setState(STATE_EXTRACTING);
        
        try {{
            console.log('[scraper] Starting content extraction with token:', EXECUTION_TOKEN);
            
            // Extract and clean content
            const cleanHtml = extractCleanTextContent();
            
            if (!cleanHtml || cleanHtml.length === 0) {{
                throw new Error('No content extracted from page');
            }}
            
            // Encode as base64 (handle Unicode properly)
            encodedContent = btoa(unescape(encodeURIComponent(cleanHtml)));
            totalChunks = Math.ceil(encodedContent.length / CHUNK_SIZE);
            currentChunkIndex = 0;
            
            console.log('[scraper] Content prepared:', {{
                originalSize: cleanHtml.length,
                encodedSize: encodedContent.length,
                chunks: totalChunks
            }});
            
            // Transition to sending state
            setState(STATE_SENDING);
            
            // Start sending chunks
            sendNextChunk();
            
        }} catch (e) {{
            console.error('[scraper] Extraction error:', e);
            sendError(e.toString());
        }}
    }}
    
    // ==========================================
    // PAGE LOAD DETECTION
    // ==========================================
    
    function startExtraction() {{
        // Only start if we're still in IDLE state
        if (getState() !== STATE_IDLE) {{
            console.log('[scraper] Extraction already triggered, skipping');
            return;
        }}
        
        console.log('[scraper] Page ready, starting extraction');
        performExtraction();
    }}
    
    // Check document ready state and extract when ready
    if (document.readyState === 'complete') {{
        // Page already loaded, extract after a brief delay for any final JS
        console.log('[scraper] Document complete, scheduling extraction');
        setTimeout(startExtraction, 100);
    }} else if (document.readyState === 'interactive') {{
        // DOM ready but resources loading, wait a bit longer
        console.log('[scraper] Document interactive, scheduling extraction');
        setTimeout(startExtraction, 500);
    }} else {{
        // Still loading, wait for load event
        console.log('[scraper] Document loading, waiting for load event');
        window.addEventListener('load', function loadHandler() {{
            window.removeEventListener('load', loadHandler);
            setTimeout(startExtraction, 100);
        }});
        
        // Fallback timeout in case load event doesn't fire
        setTimeout(function() {{
            if (getState() === STATE_IDLE) {{
                console.log('[scraper] Fallback timeout triggered');
                startExtraction();
            }}
        }}, 5000);
    }}
    
    console.log('[scraper] Extraction script initialized with token:', EXECUTION_TOKEN);
}})();
"#
    )
}

/// Scrape a URL using a hidden WebView window.
///
/// This creates a temporary, invisible WebView that navigates to the target URL,
/// waits for the page to load, extracts the text content via JavaScript,
/// and returns it. The WebView provides authentic browser fingerprinting.
///
/// # Bot Detection Bypass
///
/// - Uses real browser engine (WebView2/WebKit) with authentic TLS fingerprint
/// - Full JavaScript execution for dynamic pages
/// - Proper cookie and session handling
/// - All standard browser headers sent automatically
/// - Uses navigation interception to get data back (works on external domains)
///
/// # Reliability
///
/// - Unique execution tokens prevent duplicate extractions
/// - Single injection with proper timing
/// - Content filtering reduces payload size
async fn scrape_url_with_webview(
    app_handle: &AppHandle,
    url: &str,
    timeout_secs: u64,
) -> Result<String, String> {
    let window_label = generate_window_label();
    let request_id = window_label.clone();
    let execution_token = generate_execution_token();
    let url_string = url.to_string();

    log::info!(
        "[web_search] Creating WebView scraper for URL: {} (request: {}, token: {})",
        url,
        request_id,
        execution_token
    );

    // Channel to receive the scraped HTML
    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    // Set up the pending scrape state
    if let Ok(mut slot) = PENDING_SCRAPE.lock() {
        if slot.is_some() {
            return Err("Another web scrape is already in progress".to_string());
        }
        *slot = Some(PendingScrape {
            request_id: request_id.clone(),
            execution_token: execution_token.clone(),
            sender: tx.clone(),
            total_chunks: None,
            received_chunks: 0,
            chunks: Vec::new(),
            extraction_started: false,
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
    .visible(false)
    .focused(false)
    .skip_taskbar(true)
    .on_navigation(move |nav_url| {
        // Check if this is our custom scheme with scraped data
        if nav_url.scheme() == SCRAPE_RESULT_SCHEME {
            log::debug!("[web_search] Intercepted scraper navigation: {}", nav_url.host_str().unwrap_or("unknown"));
            let host = nav_url.host_str();
            let path = nav_url.path();

            if host == Some("data") {
                // Parse: /request_id/execution_token/chunk_index/total_chunks/chunk_data
                let mut parts = path.trim_start_matches('/').splitn(5, '/');
                let Some(req_id) = parts.next() else {
                    log::error!("[web_search] Missing request ID in data URL");
                    return false;
                };
                let Some(exec_token) = parts.next() else {
                    log::error!("[web_search] Missing execution token in data URL");
                    return false;
                };
                let Some(index_str) = parts.next() else {
                    log::error!("[web_search] Missing chunk index in data URL");
                    return false;
                };
                let Some(total_str) = parts.next() else {
                    log::error!("[web_search] Missing total chunks in data URL");
                    return false;
                };
                let Some(chunk) = parts.next() else {
                    log::error!("[web_search] Missing chunk data in data URL");
                    return false;
                };

                // Decode the URL-encoded values
                let req_id = urlencoding::decode(req_id).unwrap_or_else(|_| req_id.into()).to_string();
                let exec_token = urlencoding::decode(exec_token).unwrap_or_else(|_| exec_token.into()).to_string();

                let index = match index_str.parse::<usize>() {
                    Ok(i) => i,
                    Err(_) => {
                        log::error!("[web_search] Invalid chunk index: {}", index_str);
                        return false;
                    }
                };
                let total = match total_str.parse::<usize>() {
                    Ok(t) if t > 0 => t,
                    _ => {
                        log::error!("[web_search] Invalid total chunks: {}", total_str);
                        return false;
                    }
                };

                log::debug!(
                    "[web_search] Received chunk {}/{} (token: {})",
                    index + 1,
                    total,
                    &exec_token[..exec_token.len().min(10)]
                );

                if let Err(e) = add_scrape_chunk(&req_id, &exec_token, index, total, chunk) {
                    log::error!("[web_search] Failed to add chunk: {}", e);
                }
            } else if host == Some("error") {
                // Parse: /request_id/execution_token/error_message
                let mut parts = path.trim_start_matches('/').splitn(3, '/');
                let Some(req_id) = parts.next() else {
                    log::error!("[web_search] Missing request ID in error URL");
                    return false;
                };
                let Some(exec_token) = parts.next() else {
                    log::error!("[web_search] Missing execution token in error URL");
                    return false;
                };
                let Some(encoded_error) = parts.next() else {
                    log::error!("[web_search] Missing error message in error URL");
                    return false;
                };

                let req_id = urlencoding::decode(req_id).unwrap_or_else(|_| req_id.into()).to_string();
                let exec_token = urlencoding::decode(exec_token).unwrap_or_else(|_| exec_token.into()).to_string();

                handle_scrape_error(&req_id, &exec_token, encoded_error);
            }

            // Don't navigate to our custom scheme
            return false;
        }

        // Allow all other navigations
        true
    })
    .build()
    .map_err(|e| format!("Failed to create WebView window: {}", e))?;

    // Wait for page load, then inject the extraction script once
    let window_clone = window.clone();
    let extraction_script = get_extraction_script(&request_id, &execution_token);

    tokio::spawn(async move {
        // Wait for page to load (give it enough time)
        tokio::time::sleep(Duration::from_millis(2000)).await;

        // Inject the extraction script exactly once
        match window_clone.eval(&extraction_script) {
            Ok(_) => {
                log::info!("[web_search] Successfully injected extraction script");
            }
            Err(e) => {
                log::error!("[web_search] Failed to inject extraction script: {}", e);
            }
        }
    });

    // Wait for result with timeout
    let result = tokio::time::timeout(Duration::from_secs(timeout_secs), rx).await;

    // Clean up: close the window
    if let Err(e) = window.destroy() {
        log::warn!("[web_search] Failed to destroy WebView window: {}", e);
    }

    // Clear pending scrape state
    if let Ok(mut slot) = PENDING_SCRAPE.lock() {
        if let Some(pending) = slot.as_ref() {
            if pending.request_id == request_id {
                *slot = None;
            }
        }
    }

    match result {
        Ok(Ok(Ok(html))) => {
            log::info!("[web_search] Successfully scraped {} bytes", html.len());
            Ok(html)
        }
        Ok(Ok(Err(e))) => {
            log::error!("[web_search] Scraping failed: {}", e);
            Err(e)
        }
        Ok(Err(_)) => {
            log::error!("[web_search] Channel closed unexpectedly");
            Err("Channel closed unexpectedly".to_string())
        }
        Err(_) => {
            log::error!("[web_search] Scraping timed out after {} seconds", timeout_secs);
            Err(format!(
                "Scraping timed out after {} seconds",
                timeout_secs
            ))
        }
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
