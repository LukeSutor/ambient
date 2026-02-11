//! Tauri commands for the browser-use runtime.

use tauri::AppHandle;
use super::runtime::BrowserUseRuntime;
use super::state::BrowserUseState;

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
