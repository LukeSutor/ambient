//! WebView lifecycle management for browser-use sessions.
//!
//! Creates and manages a persistent Tauri WebView window for the browser-use
//! runtime. Handles navigation interception for the `browsersnapshot://` custom
//! scheme, chunked data assembly, and snapshot extraction coordination.
//!
//! # Security
//! - Execution tokens prevent duplicate or stale extractions
//! - Request IDs isolate concurrent operations
//! - Navigation to `browsersnapshot://` is always blocked (data-only scheme)

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::oneshot;

use super::snapshot::{get_snapshot_script, SNAPSHOT_SCHEME};

/// Window label for the browser-use WebView.
const BROWSER_WINDOW_LABEL: &str = "browser-use-webview";

/// Counter for unique request IDs.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Counter for unique execution tokens (prevents duplicate extractions).
static EXEC_TOKEN_COUNTER: AtomicU64 = AtomicU64::new(0);

type PendingSnapshotSender = Arc<Mutex<Option<oneshot::Sender<Result<String, String>>>>>;

/// Represents the state of a pending snapshot extraction.
struct PendingSnapshot {
    request_id: String,
    execution_token: String,
    sender: PendingSnapshotSender,
    total_chunks: Option<usize>,
    received_chunks: usize,
    chunks: Vec<Option<String>>,
    extraction_started: bool,
}

/// Global pending snapshot state.
static PENDING_SNAPSHOT: Lazy<Mutex<Option<PendingSnapshot>>> = Lazy::new(|| Mutex::new(None));

/// RAII guard to clear pending snapshot state on drop.
struct SnapshotStateGuard {
    request_id: String,
}

impl Drop for SnapshotStateGuard {
    fn drop(&mut self) {
        if let Ok(mut slot) = PENDING_SNAPSHOT.lock() {
            if let Some(pending) = slot.as_ref() {
                if pending.request_id == self.request_id {
                    *slot = None;
                    log::debug!("[browser_use] PENDING_SNAPSHOT cleared by guard for {}", self.request_id);
                }
            }
        }
    }
}

fn generate_request_id() -> String {
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("browser_snapshot_{}", id)
}

fn generate_execution_token() -> String {
    let id = EXEC_TOKEN_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("bexec_{}", id)
}

fn resolve_pending_snapshot(request_id: &str, result: Result<String, String>) {
    if let Ok(mut slot) = PENDING_SNAPSHOT.lock() {
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

/// Add a received chunk to the pending snapshot operation.
fn add_snapshot_chunk(
    request_id: &str,
    execution_token: &str,
    chunk_index: usize,
    total_chunks: usize,
    chunk: &str,
) -> Result<(), String> {
    if let Ok(mut slot) = PENDING_SNAPSHOT.lock() {
        let Some(pending) = slot.as_mut() else {
            return Err("No pending snapshot is active".to_string());
        };

        if pending.request_id != request_id {
            return Err(format!(
                "Request ID mismatch: expected '{}', got '{}'",
                pending.request_id, request_id
            ));
        }

        // Validate execution token
        if pending.execution_token != execution_token {
            if !pending.extraction_started {
                pending.execution_token = execution_token.to_string();
            } else {
                return Err(format!(
                    "Execution token mismatch (active: '{}')",
                    pending.execution_token
                ));
            }
        }

        pending.extraction_started = true;

        if pending.total_chunks.is_none() {
            log::debug!("[browser_use] Expecting {} snapshot chunks", total_chunks);
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

        if pending.chunks[chunk_index].is_some() {
            return Ok(()); // Duplicate chunk, ignore
        }

        let decoded = urlencoding::decode(chunk)
            .unwrap_or_else(|_| chunk.into())
            .to_string();

        pending.chunks[chunk_index] = Some(decoded);
        pending.received_chunks += 1;

        if pending.received_chunks == total_chunks {
            log::debug!("[browser_use] All snapshot chunks received, assembling");
            let encoded = pending
                .chunks
                .iter()
                .map(|c| c.as_deref().unwrap_or_default())
                .collect::<String>();
            drop(slot);

            // URL-decode the assembled content
            let text = urlencoding::decode(&encoded)
                .unwrap_or_else(|_| encoded.clone().into())
                .to_string();

            resolve_pending_snapshot(request_id, Ok(text));
        }

        Ok(())
    } else {
        Err("Failed to acquire snapshot lock".to_string())
    }
}

fn handle_snapshot_error(request_id: &str, execution_token: &str, encoded_error: &str) {
    if let Ok(slot) = PENDING_SNAPSHOT.lock() {
        if let Some(pending) = slot.as_ref() {
            if pending.execution_token != execution_token && pending.extraction_started {
                log::warn!(
                    "[browser_use] Ignoring error from stale execution token '{}'",
                    execution_token
                );
                return;
            }
        }
    }

    let error_msg = urlencoding::decode(encoded_error)
        .unwrap_or_else(|_| "Unknown error".into())
        .to_string();
    log::error!("[browser_use] Snapshot extraction error: {}", error_msg);
    resolve_pending_snapshot(request_id, Err(error_msg));
}

/// Create the browser-use WebView window.
///
/// Creates a hidden WebView with navigation interception for the
/// `browsersnapshot://` custom scheme. The WebView is persistent
/// and reused throughout the browser-use session.
pub fn create_browser_webview(app_handle: &AppHandle, start_url: &str) -> Result<String, String> {
    // Close existing window if any
    if let Some(existing) = app_handle.get_webview_window(BROWSER_WINDOW_LABEL) {
        let _ = existing.destroy();
        log::info!("[browser_use] Destroyed existing browser WebView");
    }

    let url = WebviewUrl::External(
        start_url
            .parse()
            .map_err(|e| format!("Invalid start URL '{}': {}", start_url, e))?,
    );

    let _webview_window = WebviewWindowBuilder::new(
        app_handle,
        BROWSER_WINDOW_LABEL,
        url,
    )
    .title("Ambient Browser")
    .inner_size(1280.0, 900.0)
    .visible(true)
    .focused(false)
    .skip_taskbar(true)
    .on_navigation(move |url| {
        let url_str = url.as_str();

        if url_str.starts_with(SNAPSHOT_SCHEME) {
            // Parse the URL to extract data
            let path = &url_str[SNAPSHOT_SCHEME.len() + 3..]; // Skip "browsersnapshot://"

            if path.starts_with("data/") {
                // Format: data/{request_id}/{exec_token}/{index}/{total}/{chunk}
                let parts: Vec<&str> = path[5..].splitn(5, '/').collect();
                if parts.len() == 5 {
                    let request_id = urlencoding::decode(parts[0]).unwrap_or_default().to_string();
                    let exec_token = urlencoding::decode(parts[1]).unwrap_or_default().to_string();
                    let chunk_index: usize = parts[2].parse().unwrap_or(0);
                    let total_chunks: usize = parts[3].parse().unwrap_or(1);
                    let chunk_data = parts[4];

                    if let Err(e) = add_snapshot_chunk(
                        &request_id,
                        &exec_token,
                        chunk_index,
                        total_chunks,
                        chunk_data,
                    ) {
                        log::warn!("[browser_use] Failed to add snapshot chunk: {}", e);
                    }
                }
            } else if path.starts_with("error/") {
                let parts: Vec<&str> = path[6..].splitn(3, '/').collect();
                if parts.len() == 3 {
                    let request_id = urlencoding::decode(parts[0]).unwrap_or_default().to_string();
                    let exec_token = urlencoding::decode(parts[1]).unwrap_or_default().to_string();
                    handle_snapshot_error(&request_id, &exec_token, parts[2]);
                }
            }

            return false; // Block navigation to custom scheme
        }

        true // Allow all other navigation
    })
    .build()
    .map_err(|e| format!("Failed to create browser WebView: {}", e))?;

    log::info!("[browser_use] Browser WebView created, navigating to {}", start_url);
    Ok(BROWSER_WINDOW_LABEL.to_string())
}

/// Extract a DOM snapshot from the browser WebView.
///
/// Injects the snapshot JavaScript, waits for chunked response via
/// the `browsersnapshot://` navigation scheme, and returns the assembled
/// snapshot text.
pub async fn extract_snapshot(
    app_handle: &AppHandle,
    timeout_secs: u64,
) -> Result<String, String> {
    let webview = app_handle
        .get_webview_window(BROWSER_WINDOW_LABEL)
        .ok_or_else(|| "Browser WebView not found".to_string())?;

    let request_id = generate_request_id();
    let execution_token = generate_execution_token();

    // Set up pending snapshot state
    let (tx, rx) = oneshot::channel::<Result<String, String>>();
    {
        let mut slot = PENDING_SNAPSHOT
            .lock()
            .map_err(|_| "Failed to acquire snapshot lock".to_string())?;
        *slot = Some(PendingSnapshot {
            request_id: request_id.clone(),
            execution_token: execution_token.clone(),
            sender: Arc::new(Mutex::new(Some(tx))),
            total_chunks: None,
            received_chunks: 0,
            chunks: Vec::new(),
            extraction_started: false,
        });
    }

    // RAII guard ensures cleanup
    let _guard = SnapshotStateGuard {
        request_id: request_id.clone(),
    };

    // Inject snapshot extraction script
    let script = get_snapshot_script(&request_id, &execution_token);
    webview
        .eval(&script)
        .map_err(|e| format!("Failed to inject snapshot script: {}", e))?;

    // Wait for result with timeout
    match tokio::time::timeout(Duration::from_secs(timeout_secs), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("Snapshot channel closed unexpectedly".to_string()),
        Err(_) => {
            log::warn!(
                "[browser_use] Snapshot extraction timed out after {}s",
                timeout_secs
            );
            Err(format!(
                "Snapshot extraction timed out after {}s",
                timeout_secs
            ))
        }
    }
}

/// Destroy the browser-use WebView window.
pub fn destroy_browser_webview(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window(BROWSER_WINDOW_LABEL) {
        if let Err(e) = window.destroy() {
            log::error!("[browser_use] Failed to destroy browser WebView: {}", e);
        } else {
            log::info!("[browser_use] Browser WebView destroyed");
        }
    }

    // Clear any pending snapshot state
    if let Ok(mut slot) = PENDING_SNAPSHOT.lock() {
        *slot = None;
    }
}

/// Get the browser WebView window label.
pub fn get_browser_window_label() -> &'static str {
    BROWSER_WINDOW_LABEL
}
