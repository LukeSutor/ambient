use tauri::AppHandle;
use super::computer_use::ComputerUseEngine;

/// Test the computer use engine with a sample prompt
#[tauri::command]
pub async fn start_computer_use(
    app_handle: AppHandle,
    conversation_id: String,
    prompt: String,
) -> Result<String, String> {
    let mut engine = ComputerUseEngine::new(app_handle, conversation_id, prompt.clone());

    match engine.run().await {
        Ok(_) => Ok("Computer use engine ran successfully.".to_string()),
        Err(e) => Err(format!("Error running computer use engine: {}", e)),
    }
}
