use tauri::AppHandle;
use chrono;
use crate::events::types::*;
use crate::events::emitter::emit;
use crate::os_utils::windows::window::{get_screen_text, get_brave_url, ApplicationTextData, format_as_markdown};
use std::collections::HashSet;

// Struct to hold previous screen state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreviousScreenState {
    pub data: Vec<ApplicationTextData>,
    pub active_url: Option<String>,
    pub timestamp: String,
}

// Initialize this as a static variable
static mut PREVIOUS_SCREEN_STATE: PreviousScreenState = PreviousScreenState {
    data: Vec::new(),
    active_url: None,
    timestamp: String::new(),
};

pub async fn handle_capture_screen(_event: CaptureScreenEvent, app_handle: &AppHandle) {
    let data = match get_screen_text(app_handle.clone()).await {
        Ok(data) => data,
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
    
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Emit get screen diff event
    if let Err(e) = emit(GET_SCREEN_DIFF, GetScreenDiffEvent { data, active_url: Some(url), timestamp }) {
        eprintln!("[get_screen_diff] Failed to emit GET_SCREEN_DIFF event: {}", e);
    }
    println!("[capture_screen] Emitted GET_SCREEN_DIFF event with text and URL");
}

pub async fn handle_get_screen_diff(event: GetScreenDiffEvent, app_handle: &AppHandle) {
    // Fetch the previous and current screen state
    let previous_state = unsafe { PREVIOUS_SCREEN_STATE.clone() };
    let new_data = event.data;

    // Iterate over the new data and compare with previous state
    let mut changes: Vec<ApplicationTextData> = Vec::new();
    for new_app in &new_data {
        // Check if this application was in the previous state
        if let Some(prev_app) = previous_state.data.iter().find(|&app| app.process_id == new_app.process_id) {
            // Convert previous text lines to a HashSet for O(1) lookups
            let prev_lines: HashSet<&String> = prev_app.text_content.iter().collect();

            // Check for new lines not in previous state
            let new_lines: Vec<String> = new_app.text_content.iter()
                .filter(|line| !prev_lines.contains(line))
                .cloned()
                .collect();
            
            if !new_lines.is_empty() {
                changes.push(ApplicationTextData {
                    process_id: new_app.process_id,
                    process_name: new_app.process_name.clone(),
                    application_name: new_app.application_name.clone(),
                    text_content: new_lines,
                });
            }
        } else {
            changes.push(ApplicationTextData {
                process_id: new_app.process_id,
                process_name: new_app.process_name.clone(),
                application_name: new_app.application_name.clone(),
                text_content: new_app.text_content.clone(),
            });
        }
    }

    // Update the previous state with the current data
    unsafe {
        PREVIOUS_SCREEN_STATE = PreviousScreenState {
            data: new_data,
            active_url: event.active_url.clone(),
            timestamp: event.timestamp.clone(),
        };
    }

    // Return if there are no changes
    if changes.is_empty() {
        println!("[get_screen_diff] No changes detected");
        return;
    }

    let timestamp = chrono::Utc::now().to_rfc3339();

    // Format changes as markdown
    let markdown = format_as_markdown(changes);
    println!("[get_screen_diff] Changes detected:\n{}", markdown);
    
    // Emit detect tasks event
    if let Err(e) = emit(DETECT_TASKS, DetectTasksEvent { text: markdown, active_url: event.active_url.clone(), timestamp }) {
        eprintln!("[get_screen_diff] Failed to emit DETECT_TASKS event: {}", e);
    }
    println!("[capture_screen] Emitted DETECT_TASKS event with text and URL");
}