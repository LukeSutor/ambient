use tauri::{AppHandle, Manager};
use super::actions::*;
use serde_json::json;

/// Helpers


pub struct ComputerUseEngine {
    app_handle: AppHandle,
    width: i32,
    height: i32,
    cancel_loop: bool
}

impl ComputerUseEngine {
    pub fn new(app_handle: AppHandle) -> Self {
        // Get the screen's physical size to store it
        let mut width: i32 = 0;
        let mut height: i32 = 0;
        if let Some(window) = app_handle.get_webview_window("main") {
            if let Ok(Some(monitor)) = window.current_monitor() {
                let physical_size = monitor.size();
                width = physical_size.width as i32;
                height = physical_size.height as i32;
            }
        }
        if width == 0 || height == 0 {
            log::warn!("Failed to get screen dimensions, using defaults");
        }
        Self {
            app_handle: app_handle.clone(),
            width: width as i32,
            height: height as i32,
            cancel_loop: false
        }
    }

    pub async fn get_model_response(&self, prompt: &str) -> Result<serde_json::Value, String> {
        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| "Missing GEMINI_API_KEY environment variable".to_string())?;
        
        let model = "gemini-2.5-computer-use-preview-10-2025";
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        // Build the parts array
        let parts = vec![
            json!({
                "text": prompt
            })
        ];

        // Build request body
        let request_body = json!({
            "contents": [{
                "role": "user",
                "parts": parts
            }],
            "tools": [{
                "computer_use": {
                    "environment": "ENVIRONMENT_BROWSER"
                }
            }]
        });

        // Make the HTTP request (async)
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        // Get response as raw JSON Value
        let json_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // Check for errors in the response
        if let Some(error) = json_response.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
            let message = error.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error");
            return Err(format!("API Error {}: {}", code, message));
        }

        Ok(json_response)
    }

    fn get_text(&self, candidate: &serde_json::Value) -> Option<String> {
        let content = candidate.get("content")?;
        let parts = content.get("parts")?.as_array()?;
        
        let mut text_parts = Vec::new();
        for part in parts {
            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                if !text.is_empty() {
                    text_parts.push(text);
                }
            }
        }
        
        if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join(" "))
        }
    }

    pub fn extract_function_calls(&self, candidate: &serde_json::Value) -> Vec<serde_json::Value> {
        let content = match candidate.get("content") {
            Some(c) => c,
            None => return Vec::new(),
        };
        let parts = match content.get("parts").and_then(|p| p.as_array()) {
            Some(p) => p,
            None => return Vec::new(),
        };
        
        let mut function_calls = Vec::new();
        for part in parts {
            if let Some(function_call) = part.get("functionCall") {
                function_calls.push(function_call.clone());
            }
        }
        
        function_calls
    }

    pub async fn handle_action(&self, function_call: &serde_json::Value) -> Result<(), String> {
        // Log the function call for debugging
        println!("Handling function call: {}", function_call);
        let name = function_call.get("name")
            .and_then(|n| n.as_str())
            .ok_or("Missing function name")?;
        let args = function_call.get("args")
            .ok_or("Missing function arguments")?;
        
        match name {
            "open_web_browser" => {
                open_web_browser(self.app_handle.clone())
                    .map_err(|_| format!("Failed to open web browser"))?;
                Ok(())
            }
            "wait_5_seconds" => {
                wait_5_seconds().await.map_err(|_| format!("Failed to wait"))?;
                Ok(())
            }
            "go_back" => {
                go_back().map_err(|_| format!("Failed to go back"))?;
                Ok(())
            }
            "go_forward" => {
                go_forward().map_err(|_| format!("Failed to go forward"))?;
                Ok(())
            }
            "search" => {
                search(self.app_handle.clone())
                    .map_err(|_| format!("Failed to perform search"))?;
                Ok(())
            }
            "navigate" => {
                let url = args.get("url").and_then(|u| u.as_str()).ok_or("Missing 'url' argument")?;
                navigate(self.app_handle.clone(), url)
                    .map_err(|_| format!("Failed to navigate to URL"))?;
                Ok(())
            }
            "click_at" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                click_at(actual_x, actual_y).map_err(|_| format!("Failed to click at coordinates"))?;
                Ok(())
            }
            "hover_at" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                hover_at(actual_x, actual_y).map_err(|_| format!("Failed to hover at coordinates"))?;
                Ok(())
            }
            "type_text_at" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let text = args.get("text").and_then(|t| t.as_str()).ok_or("Missing 'text' argument")?;
                let press_enter = args.get("press_enter").and_then(|p| p.as_bool());
                let clear_before_typing = args.get("clear_before_typing").and_then(|c| c.as_bool());
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                type_text_at(actual_x, actual_y, text, press_enter, clear_before_typing)
                    .map_err(|_| format!("Failed to type text at coordinates"))?;
                Ok(())
            }
            "key_combination" => {
                let keys = args.get("keys").and_then(|k| k.as_str()).ok_or("Missing 'keys' argument")?;
                key_combination(keys)
                    .map_err(|_| format!("Failed to perform key combination"))?;
                Ok(())
            }
            "scroll_document" => {
                let direction = args.get("direction").and_then(|d| d.as_str()).ok_or("Missing 'direction' argument")?;
                scroll_document(direction)
                    .map_err(|_| format!("Failed to scroll document"))?;
                Ok(())
            }
            "scroll_at" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let direction = args.get("direction").and_then(|d| d.as_str()).ok_or("Missing 'direction' argument")?;
                let magnitude = args.get("magnitude").and_then(|m| m.as_i64()).map(|m| m as i32);
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                scroll_at(actual_x, actual_y, direction, magnitude)
                    .map_err(|_| format!("Failed to scroll at coordinates"))?;
                Ok(())
            }
            "drag_and_drop" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let destination_x = args.get("destination_x").and_then(|x| x.as_i64()).ok_or("Missing 'destination_x' argument")? as i32;
                let destination_y = args.get("destination_y").and_then(|y| y.as_i64()).ok_or("Missing 'destination_y' argument")? as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                let (actual_dest_x, actual_dest_y) = self.denormalize_coordinates(destination_x, destination_y);
                drag_and_drop(actual_x, actual_y, actual_dest_x, actual_dest_y)
                    .map_err(|_| format!("Failed to drag and drop"))?;
                Ok(())
            }
            _ => {
                Err(format!("Unknown function: {}", name))
            }
        }
    }

    fn denormalize_coordinates(&self, x: i32, y: i32) -> (i32, i32) {
        let actual_x = (x as f64 / 1000.0) * self.width as f64;
        let actual_y = (y as f64 / 1000.0) * self.height as f64;
        (actual_x as i32, actual_y as i32)
    }
}