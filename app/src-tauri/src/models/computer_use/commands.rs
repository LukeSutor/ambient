use tauri::AppHandle;
use super::computer_use::ComputerUseEngine;
use super::actions;
use super::types::{ActionResponse, ComputerAction};

/// Test the computer use engine with a sample prompt
#[tauri::command]
pub async fn start_computer_use(
    app_handle: AppHandle,
    conversation_id: String,
    prompt: String,
) -> Result<String, String> {
    let mut engine = ComputerUseEngine::new(app_handle, conversation_id, prompt.clone()).await;

    match engine.run().await {
        Ok(_) => Ok("Computer use engine ran successfully.".to_string()),
        Err(e) => Err(format!("Error running computer use engine: {}", e)),
    }
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
