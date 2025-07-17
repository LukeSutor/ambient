use tauri::{AppHandle, Manager, Listener};
use crate::db::DbState;
use super::types::*;
use crate::os_utils::handlers::handle_capture_screen;
use crate::models::llm::handlers::handle_detect_tasks;

pub fn initialize_event_listeners(app_handle: AppHandle) {
    let _db_state = app_handle.state::<DbState>();
    
    // Set all listeners with their handler functions
    let app_handle_clone1 = app_handle.clone();
    app_handle.listen(CAPTURE_SCREEN, move |event| {
        let payload_str = event.payload();
        match serde_json::from_str::<CaptureScreenEvent>(payload_str) {
            Ok(event_data) => {
                println!("[events] Capture screen event received");
                // For async function, we need to spawn a task
                let app_handle_clone = app_handle_clone1.clone();
                tauri::async_runtime::spawn(async move {
                    handle_capture_screen(event_data, &app_handle_clone).await;
                });
            }
            Err(e) => {
                eprintln!("[events] Failed to parse capture screen event: {}", e);
            }
        }
    });

    let app_handle_clone2 = app_handle.clone();
    app_handle.listen(DETECT_TASKS, move |event| {
        let payload_str = event.payload();
        match serde_json::from_str::<DetectTasksEvent>(payload_str) {
            Ok(event_data) => {
                println!("[events] Detect tasks event received");
                // For async function, we need to spawn a task
                let app_handle_clone = app_handle_clone2.clone();
                tauri::async_runtime::spawn(async move {
                    handle_detect_tasks(event_data, &app_handle_clone).await;
                });
            }
            Err(e) => {
                eprintln!("[events] Failed to parse detect tasks event: {}", e);
            }
        }
    });
}