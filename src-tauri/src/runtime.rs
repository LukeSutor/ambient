use crate::server::infer;
use crate::control::{click_mouse, move_mouse, type_string};


#[tauri::command]
pub async fn handle_request(prompt: String, image: String) -> Result<String, String> {
    // Get the response from the model
    let response = infer(prompt, image).await?;
    println!("{}", response.to_string());

    // Extract the value of the "action" field
    let response_json: serde_json::Value = serde_json::from_str(&response).map_err(|e| format!("Failed to parse JSON: {}", e))?;
    let action = response_json["action"].as_str().ok_or("Failed to extract 'action' field")?.to_string();

    // Switch statement to call the appropriate function
    match action.as_str() {
        "CLICK" => {
            let button = response_json["mouse_button"].as_str().ok_or("Failed to extract 'button' field")?.to_string();
            click_mouse(button);
        },
        "MOVE" => {
            let x = response_json["x"].as_i64().ok_or("Failed to extract 'x' field")? as i32;
            let y = response_json["y"].as_i64().ok_or("Failed to extract 'y' field")? as i32;
            move_mouse(x, y);
        },
        "TYPE" => {
            let text = response_json["text"].as_str().ok_or("Failed to extract 'text' field")?.to_string();
            type_string(text);
        },
        _ => return Err("Unknown action".to_string()),
    }

    Ok(format!("Successfully handled request. Action: {}", action))
}