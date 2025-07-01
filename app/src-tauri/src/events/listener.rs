use tauri::{AppHandle, Manager};
use crate::db::DbState;
use super::types::*;
use super::constants::*;
use os_utils::windows::
use models::llm::handlers::*;

pub fn initialize_event_listeners(app_handle: AppHandle) {
    let db_state = app_handle.state::<DbState>();
    
    // Set all listeners with their handler functions
    app_handle.listen(CAPTURE_SCREEN, move |event| {
        if let Some(event) = event.payload::<CaptureScreenEvent>() {
            println!("[events] Capture screen event received: {:?}", event);
            // Handle capture screen logic here
        }
    });

    app_handle.listen(ANALYZE_SCREEN, move |event| {
        if let Some(event) = event.payload::<AnalyzeScreenEvent>() {
            println!("[events] Analyze screen event received: {:?}", event);
            handle_screen_analysis(event, &app_handle);
        }
    });

}