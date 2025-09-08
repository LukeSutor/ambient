use super::types::*;
use crate::db::core::DbState;
use crate::models::llm::handlers::{handle_detect_tasks, handle_summarize_screen};
use crate::os_utils::handlers::{handle_capture_screen, handle_get_screen_diff};
use tauri::{AppHandle, Listener, Manager};

pub fn initialize_event_listeners(app_handle: AppHandle) {
  let _db_state = app_handle.state::<DbState>();

  // Set all listeners with their handler functions
  let app_handle_clone1 = app_handle.clone();
  app_handle.listen(CAPTURE_SCREEN, move |event| {
    let payload_str = event.payload();
    match serde_json::from_str::<CaptureScreenEvent>(payload_str) {
      Ok(event_data) => {
        log::info!("[events] Capture screen event received");
        // For async function, we need to spawn a task
        let app_handle_clone = app_handle_clone1.clone();
        tauri::async_runtime::spawn(async move {
          handle_capture_screen(event_data, &app_handle_clone).await;
        });
      }
      Err(e) => {
        log::error!("[events] Failed to parse capture screen event: {}", e);
      }
    }
  });

  let app_handle_clone2 = app_handle.clone();
  app_handle.listen(GET_SCREEN_DIFF, move |event| {
    let payload_str = event.payload();
    match serde_json::from_str::<GetScreenDiffEvent>(payload_str) {
      Ok(event_data) => {
        log::info!("[events] Get screen diff event received");
        // For async function, we need to spawn a task
        let app_handle_clone = app_handle_clone2.clone();
        tauri::async_runtime::spawn(async move {
          handle_get_screen_diff(event_data, &app_handle_clone).await;
        });
      }
      Err(e) => {
        log::error!("[events] Failed to parse get screen diff event: {}", e);
      }
    }
  });

  let app_handle_clone3 = app_handle.clone();
  app_handle.listen(DETECT_TASKS, move |event| {
    let payload_str = event.payload();
    match serde_json::from_str::<DetectTasksEvent>(payload_str) {
      Ok(event_data) => {
        log::info!("[events] Detect tasks event received");
        // For async function, we need to spawn a task
        let app_handle_clone = app_handle_clone3.clone();
        tauri::async_runtime::spawn(async move {
          handle_detect_tasks(event_data, &app_handle_clone).await;
        });
      }
      Err(e) => {
        log::error!("[events] Failed to parse detect tasks event: {}", e);
      }
    }
  });

  let app_handle_clone4 = app_handle.clone();
  app_handle.listen(SUMMARIZE_SCREEN, move |event| {
    let payload_str = event.payload();
    match serde_json::from_str::<SummarizeScreenEvent>(payload_str) {
      Ok(event_data) => {
        log::info!("[events] Summarize screen event received");
        // For async function, we need to spawn a task
        let app_handle_clone = app_handle_clone4.clone();
        tauri::async_runtime::spawn(async move {
          handle_summarize_screen(event_data, &app_handle_clone).await;
        });
      }
      Err(e) => {
        log::error!("[events] Failed to parse summarize screen event: {}", e);
      }
    }
  });

  let app_handle_clone5 = app_handle.clone();
  app_handle.listen(EXTRACT_INTERACTIVE_MEMORY, move |event| {
    let payload_str = event.payload();
    match serde_json::from_str::<ExtractInteractiveMemoryEvent>(payload_str) {
      Ok(event_data) => {
        log::info!("[events] Extract interactive memory event received");
        // For async function, we need to spawn a task
        let app_handle_clone = app_handle_clone5.clone();
        tauri::async_runtime::spawn(async move {
          handle_extract_interactive_memory(event_data, &app_handle_clone).await;
        });
      }
      Err(e) => {
        log::error!("[events] Failed to parse extract interactive memory event: {}", e);
      }
    }
  });
}
