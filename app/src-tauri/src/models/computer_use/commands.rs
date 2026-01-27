use tauri::{AppHandle};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use super::computer_use::ComputerUseEngine;
use super::actions;
use super::types::{ActionResponse, ComputerAction};

pub struct ComputerUseState {
    pub is_running: Mutex<bool>,
    pub should_stop: Arc<AtomicBool>,
}

impl Default for ComputerUseState {
    fn default() -> Self {
        Self {
            is_running: Mutex::new(false),
            should_stop: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Test the computer use engine with a sample prompt
#[tauri::command]
pub async fn start_computer_use(
    app_handle: AppHandle,
    state: tauri::State<'_, ComputerUseState>,
    conversation_id: String,
    prompt: String,
) -> Result<String, String> {
    // Check if a session is already running
    {
        let mut is_running = state.is_running.lock().unwrap();
        if *is_running {
            return Err("A computer use session is already running.".to_string());
        }
        *is_running = true;
    }

    // Reset stop signal
    state.should_stop.store(false, Ordering::SeqCst);

    let mut engine = ComputerUseEngine::new(
        app_handle.clone(),
        conversation_id,
        prompt.clone(),
        state.should_stop.clone(),
    ).await;

    let result = engine.run().await;

    // Reset running state
    {
        let mut is_running = state.is_running.lock().unwrap();
        *is_running = false;
    }

    match result {
        Ok(_) => Ok("Computer use engine ran successfully.".to_string()),
        Err(e) => Err(format!("Error running computer use engine: {}", e)),
    }
}

/// Stop the current computer use session
#[tauri::command]
pub async fn stop_computer_use(
    state: tauri::State<'_, ComputerUseState>,
) -> Result<String, String> {
    state.should_stop.store(true, Ordering::SeqCst);
    Ok("Stop signal sent.".to_string())
}

/// Directly execute a computer action for testing
#[tauri::command]
pub async fn execute_computer_action(
    app_handle: AppHandle,
    action: ComputerAction,
) -> Result<ActionResponse, String> {
    log::info!("[computer_use::commands] Executing direct action: {:?}", action);
    match action {
        ComputerAction::OpenWebBrowser => actions::open_web_browser(app_handle),
        ComputerAction::Wait5Seconds => actions::wait_5_seconds().await,
        ComputerAction::GoBack => actions::go_back(),
        ComputerAction::GoForward => actions::go_forward(),
        ComputerAction::Search => actions::search(app_handle),
        ComputerAction::Navigate { url } => actions::navigate(app_handle, &url),
        ComputerAction::ClickAt { x, y } => actions::click_at(x, y),
        ComputerAction::HoverAt { x, y } => actions::hover_at(x, y),
        ComputerAction::TypeTextAt { x, y, text, press_enter, clear_before_typing } => 
            actions::type_text_at(x, y, &text, press_enter, clear_before_typing),
        ComputerAction::KeyCombination { keys } => actions::key_combination(&keys),
        ComputerAction::ScrollDocument { direction } => actions::scroll_document(&direction),
        ComputerAction::ScrollAt { x, y, direction, magnitude } => 
            actions::scroll_at(x, y, &direction, magnitude),
        ComputerAction::DragAndDrop { x, y, destination_x, destination_y } => 
            actions::drag_and_drop(x, y, destination_x, destination_y),
    }
}
