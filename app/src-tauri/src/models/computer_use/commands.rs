use tauri::AppHandle;
use super::computer_use::ComputerUseEngine;

/// Test the computer use engine with a sample prompt
#[tauri::command]
pub async fn test_computer_use(
    app_handle: AppHandle,
    prompt: String,
) -> Result<String, String> {
    let engine = ComputerUseEngine::new(app_handle);
    let response = engine.get_model_response(&prompt).await?;

    // Log response
    log::info!("[test_computer_use] Model response: {:?}", response);

    // Extract the first candidate and call the functions
    if let Some(candidates) = response.get("candidates").and_then(|c| c.as_array()) {
        if let Some(first_candidate) = candidates.first() {
            let function_calls = engine.extract_function_calls(first_candidate);
            log::info!("[test_computer_use] Extracted function calls: {:?}", function_calls);
            for function_call in function_calls {
                log::info!("[test_computer_use] Handling function call: {:?}", function_call);
                if let Err(e) = engine.handle_action(&function_call).await {
                    eprintln!("Error handling action: {}", e);
                }
            }
        }
    }
    
    // Response is already a serde_json::Value, just serialize it to pretty JSON string
    serde_json::to_string_pretty(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))
}
