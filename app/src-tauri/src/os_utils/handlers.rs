use tauri::AppHandle;
use crate::events::types::*;
use crate::os_utils::windows::window::get_all_text_from_focused_app;

pub fn handle_capture_screen(event: CaptureScreenEvent, app_handle: &AppHandle) {
    match get_all_text_from_focused_app() {
        Ok(text) => {
            println!("Captured text: {}", text);
            // You can emit an event or handle the text as needed here
        }
        Err(e) => {
            eprintln!("Failed to capture text: {}", e);
            // Optionally, emit an error event or handle the error
        }
    }
    // Here you would implement the logic to capture the screen
    // For example, using a screenshot library or OS-specific API
    // This is just a placeholder for demonstration purposes
    let timestamp = event.timestamp.clone();
}