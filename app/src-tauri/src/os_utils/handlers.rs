use tauri::AppHandle;
use chrono;
use crate::events::types::*;
use crate::events::emitter::emit;
use crate::os_utils::windows::window::{get_all_text_from_focused_app, get_brave_url};

pub fn handle_capture_screen(event: CaptureScreenEvent, app_handle: &AppHandle) {
    let text = get_all_text_from_focused_app().map_err(|e| {
        eprintln!("[capture_screen] Failed to capture text: {}", e);
        e
    })?;

    let url = get_brave_url().map_err(|e| {
        eprintln!("[capture_screen] Failed to get browser URL: {}", e);
        e
    })?;
    
    println!("[capture_screen] Captured text");
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Emit detect tasks event
    emit(app_handle, DETECT_TASKS, DetectTasksEvent { text, active_url: url, timestamp })
        .map_err(|e| {
            eprintln!("[capture_screen] Failed to emit DETECT_TASKS event: {}", e);
            e
        })?;
}