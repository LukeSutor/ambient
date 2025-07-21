use tauri::AppHandle;
use chrono;
use crate::events::types::*;
use crate::events::emitter::emit;
use crate::os_utils::windows::window::{get_screen_text, get_brave_url, ApplicationTextData, format_as_markdown};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

// Struct to hold previous screen state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreviousScreenState {
    pub data: Vec<ApplicationTextData>,
    pub active_url: Option<String>,
    pub timestamp: String,
}

// Thread-safe static variable using Lazy and Arc<Mutex<T>>
static PREVIOUS_SCREEN_STATE: Lazy<Arc<Mutex<PreviousScreenState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(PreviousScreenState {
        data: Vec::new(),
        active_url: None,
        timestamp: String::new(),
    }))
});

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

pub async fn handle_get_screen_diff(event: GetScreenDiffEvent, _app_handle: &AppHandle) {
    // Fetch the previous screen state in a thread-safe way
    let previous_state = {
        let state_guard = PREVIOUS_SCREEN_STATE.lock().unwrap();
        state_guard.clone()
    };
    let new_data = event.data.clone();

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
    {
        let mut state_guard = PREVIOUS_SCREEN_STATE.lock().unwrap();
        *state_guard = PreviousScreenState {
            data: new_data.clone(),
            active_url: event.active_url.clone(),
            timestamp: event.timestamp.clone(),
        };
    }

    // Return if there are no changes
    if changes.is_empty() {
        println!("[get_screen_diff] No changes detected");
        return;
    }

    // Format changes as markdown
    let markdown = format_as_markdown(changes);

    println!("[get_screen_diff] Changes detected:\n{}", markdown);
    
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Emit detect tasks event
    if let Err(e) = emit(DETECT_TASKS, DetectTasksEvent { text: markdown.clone(), active_url: event.active_url.clone(), timestamp: timestamp.clone() }) {
        eprintln!("[get_screen_diff] Failed to emit DETECT_TASKS event: {}", e);
    }
    println!("[capture_screen] Emitted DETECT_TASKS event");

    // Emit summarize screen event
    if let Err(e) = emit(SUMMARIZE_SCREEN, SummarizeScreenEvent { text: markdown, data: event.data, active_url: event.active_url, timestamp }) {
        eprintln!("[get_screen_diff] Failed to emit SUMMARIZE_SCREEN event: {}", e);
    }
    println!("[capture_screen] Emitted SUMMARIZE_SCREEN event");
}