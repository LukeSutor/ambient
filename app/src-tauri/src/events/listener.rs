use super::types::*;
use crate::db::core::DbState;
use crate::memory::handlers::handle_extract_interactive_memory;
use crate::models::llm::handlers::handle_generate_conversation_name;
use tauri::{AppHandle, Listener, Manager};

pub fn initialize_event_listeners(app_handle: AppHandle) {
  let _db_state = app_handle.state::<DbState>();

  // Set all listeners with their handler functions

  let app_handle_memory = app_handle.clone();
  app_handle.listen(EXTRACT_INTERACTIVE_MEMORY, move |event| {
    let payload_str = event.payload();
    match serde_json::from_str::<ExtractInteractiveMemoryEvent>(payload_str) {
      Ok(event_data) => {
        log::info!("[events] Extract interactive memory event received");
        // For async function, we need to spawn a task
        let app_handle_clone = app_handle_memory.clone();
        tauri::async_runtime::spawn(async move {
          let _ = handle_extract_interactive_memory(&app_handle_clone, event_data).await;
        });
      }
      Err(e) => {
        log::error!(
          "[events] Failed to parse extract interactive memory event: {}",
          e
        );
      }
    }
  });

  let app_handle_conv_name = app_handle.clone();
  app_handle.listen(GENERATE_CONVERSATION_NAME, move |event| {
    let payload_str = event.payload();
    match serde_json::from_str::<GenerateConversationNameEvent>(payload_str) {
      Ok(event_data) => {
        log::info!("[events] Generate conversation name event received");
        // For async function, we need to spawn a task
        let app_handle_clone = app_handle_conv_name.clone();
        tauri::async_runtime::spawn(async move {
          let _ = handle_generate_conversation_name(&app_handle_clone, event_data).await;
        });
      }
      Err(e) => {
        log::error!(
          "[events] Failed to parse generate conversation name event: {}",
          e
        );
      }
    }
  });
}