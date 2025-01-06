// Contains high level functions for features

use crate::control::{click_mouse, move_mouse, type_string};
use crate::data::take_screenshot;
use crate::sidecar::infer;

#[tauri::command]
pub async fn handle_request(prompt: String, include_image: bool, app_handle: tauri::AppHandle) -> Result<String, String> {
    let image_path = if include_image {
        take_screenshot(app_handle.clone())
    } else {
        "".to_string()
    };

    // Get the response from the model
    let response = infer(prompt, image_path, app_handle).await?;
    println!("{}", response.to_string());

    // Extract the value of the "action" field
    let response_json: serde_json::Value =
        serde_json::from_str(&response).map_err(|e| format!("Failed to parse JSON: {}", e))?;
    let action = response_json["action"]
        .as_str()
        .ok_or("Failed to extract 'action' field")?
        .to_string();

    // Switch statement to perform the appropriate action
    match action.as_str() {
        "HOVER" => {
            let x = response_json["x"]
                .as_f64()
                .ok_or("Failed to extract 'x' field")? as f64;
            let y = response_json["y"]
                .as_f64()
                .ok_or("Failed to extract 'y' field")? as f64;
            move_mouse(x, y);
        }
        "CLICK" => {
            let x = response_json["x"]
                .as_f64()
                .ok_or("Failed to extract 'x' field")? as f64;
            let y = response_json["y"]
                .as_f64()
                .ok_or("Failed to extract 'y' field")? as f64;
            let button = response_json["mouse_button"]
                .as_str()
                .ok_or("Failed to extract 'button' field")?
                .to_string();
            move_mouse(x, y);
            click_mouse(button);
        }
        "TYPE" => {
            let x = response_json["x"]
                .as_f64()
                .ok_or("Failed to extract 'x' field")? as f64;
            let y = response_json["y"]
                .as_f64()
                .ok_or("Failed to extract 'y' field")? as f64;
            let text = response_json["text"]
                .as_str()
                .ok_or("Failed to extract 'text' field")?
                .to_string();
            move_mouse(x, y);
            click_mouse("LEFT".to_string());
            type_string(text);
        }
        _ => return Err("Unknown action".to_string()),
    }

    Ok(format!("Successfully handled request. Action: {}", action))
}
