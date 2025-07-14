use tauri::AppHandle;
use chrono;
use crate::events::types::*;
use crate::events::emitter::emit;
use crate::os_utils::windows::window::{get_all_text_from_focused_app, get_brave_url};

pub fn handle_capture_screen(_event: CaptureScreenEvent, _app_handle: &AppHandle) {
    let text = match get_all_text_from_focused_app() {
        Ok(text) => text,
        Err(e) => {
            eprintln!("[capture_screen] Failed to capture text: {}", e);
            return;
        }
    };

    let url = match get_brave_url() {
        Ok(url) => url,
        Err(e) => {
            eprintln!("[capture_screen] Failed to get browser URL: {}", e);
            return;
        }
    };
    
    println!("[capture_screen] Captured text");
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Emit detect tasks event
    if let Err(e) = emit(DETECT_TASKS, DetectTasksEvent { text, active_url: Some(url), timestamp }) {
        eprintln!("[capture_screen] Failed to emit DETECT_TASKS event: {}", e);
    }
}