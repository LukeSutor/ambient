//! Tauri commands for the browser-use runtime.

use tauri::AppHandle;
use super::actions::execute_action;
use super::runtime::BrowserUseRuntime;
use super::state::BrowserUseState;
use super::webview::{create_browser_webview, destroy_browser_webview, extract_snapshot};

/// Start a browser-use session.
#[tauri::command]
pub async fn start_browser_use(
    app_handle: AppHandle,
    state: tauri::State<'_, BrowserUseState>,
    conversation_id: String,
    assistant_message_id: String,
    prompt: String,
    message_id: Option<String>,
) -> Result<String, String> {
    // Mark session as started
    state
        .start_session(&conversation_id)
        .await
        .map_err(|e| e.to_string())?;

    let cancel_signal = state.get_stop_signal();
    let cancel_notify = state.get_cancel_notify();

    let runtime = BrowserUseRuntime::new(
        app_handle.clone(),
        conversation_id,
        assistant_message_id,
        cancel_signal,
        cancel_notify,
    )
    .await
    .map_err(|e| e.to_string())?;

    let result = runtime.run(prompt, message_id).await;

    // Mark session as finished
    state.finish_session().await;

    result.map_err(|e| e.to_string())
}

// =============================================================================
// Dev/test commands for the browser-use WebView
// =============================================================================

/// Create a browser-use WebView for testing.
///
/// Must be async so it runs on a worker thread — WebView creation dispatches
/// to the main thread internally, which requires the event loop to be running.
/// A sync command would block the main thread, preventing WebView2 initialization.
#[tauri::command]
pub async fn browser_test_create(app_handle: AppHandle, url: String) -> Result<String, String> {
    create_browser_webview(&app_handle, &url)
}

/// Extract a DOM snapshot from the browser-use WebView.
#[tauri::command]
pub async fn browser_test_snapshot(app_handle: AppHandle) -> Result<String, String> {
    extract_snapshot(&app_handle, 10).await
}

/// Execute a browser action on the test WebView.
#[tauri::command]
pub async fn browser_test_action(
    app_handle: AppHandle,
    action: String,
    arguments: serde_json::Value,
) -> Result<String, String> {
    execute_action(&app_handle, &action, &arguments).await
}

/// Destroy the browser-use WebView.
#[tauri::command]
pub async fn browser_test_destroy(app_handle: AppHandle) -> Result<(), String> {
    destroy_browser_webview(&app_handle);
    Ok(())
}
