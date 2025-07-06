use tauri::{AppHandle, Manager, Listener};
use crate::db::DbState;
use super::types::*;
use crate::models::llm::handlers::handle_screen_analysis;

pub fn initialize_event_listeners(app_handle: AppHandle) {
    let _db_state = app_handle.state::<DbState>();
    
    // Set all listeners with their handler functions
    app_handle.listen(CAPTURE_SCREEN, move |event| {
        let payload_str = event.payload();
        match serde_json::from_str::<CaptureScreenEvent>(payload_str) {
            Ok(event_data) => {
                println!("[events] Capture screen event received: {:?}", event_data);
                // Handle capture screen logic here
            }
            Err(e) => {
                eprintln!("[events] Failed to parse capture screen event: {}", e);
            }
        }
    });

    let app_handle_clone = app_handle.clone();
    app_handle.listen(ANALYZE_SCREEN, move |event| {
        let payload_str = event.payload();
        match serde_json::from_str::<AnalyzeScreenEvent>(payload_str) {
            Ok(event_data) => {
                println!("[events] Analyze screen event received: {:?}", event_data);
                handle_screen_analysis(event_data, &app_handle_clone);
            }
            Err(e) => {
                eprintln!("[events] Failed to parse analyze screen event: {}", e);
            }
        }
    });
}