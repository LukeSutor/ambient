use tauri::{AppHandle, Manager, Listener};
use tokio::sync::oneshot;
use base64::{Engine as _, engine::general_purpose};
use super::actions::*;
use super::types::ActionResponse;
use serde_json::json;
use crate::images::take_screenshot;
use crate::events::{emitter::emit, types::*};
use crate::db::conversations::add_message;
use crate::windows::{open_main_window, close_main_window, open_computer_use_window, close_computer_use_window};
use chrono;

fn transform_function_call(function_name: String, args: Vec<String>) -> (String, String) {
    let mut message_content = String::new();
    let mut toast_content = String::new();
    match function_name.as_str() {
        "open_web_browser" | "search" => {
            message_content = "Opening web browser".to_string();
            toast_content = message_content.clone();
        },
        "wait_5_seconds" => {
            message_content = "Waiting for 5 seconds".to_string();
            toast_content = message_content.clone();
        },
        "go_back" => {
            message_content = "Going back".to_string();
            toast_content = message_content.clone();
        },
        "go_forward" => {
            message_content = "Going forward".to_string();
            toast_content = message_content.clone();
        },
        "navigate" => {
            message_content = format!("Navigating to {}", args[0]);
            toast_content = "Navigating to new URL".to_string();
        },
        "click_at" => {
            message_content = format!("Clicking at ({}, {})", args[0], args[1]);
            toast_content = message_content.clone();
        },
        "hover_at" => {
            message_content = format!("Hovering at ({}, {})", args[0], args[1]);
            toast_content = message_content.clone();
        },
        "type_text_at" => {
            message_content = format!("Typing '{}' at ({}, {})", args[2], args[0], args[1]);
            toast_content = "Typing text".to_string();
        },
        "key_combination" => {
            message_content = format!("Pressing keys '{}'", args[0]);
            toast_content = message_content.clone();
        },
        "scroll_document" => {
            message_content = format!("Scrolling {}", args[0]);
            toast_content = message_content.clone();
        },
        "scroll_at" => {
            message_content = format!("Scrolling {} at ({}, {})", args[2], args[0], args[1]);
            toast_content = format!("Scrolling {}", args[2]);
        },
        "drag_and_drop" => {
            message_content = format!("Dragging and dropping from ({}, {}) to ({}, {})", args[0], args[1], args[2], args[3]);
            toast_content = "Dragging and dropping".to_string();
        },
        _ => {}
    }
    (message_content, toast_content)
}

pub struct ComputerUseEngine {
    app_handle: AppHandle,
    prompt: String,
    conversation_id: String,
    width: i32,
    height: i32,
    final_response: String,
    contents: Vec<serde_json::Value>,
}

impl ComputerUseEngine {
    pub fn new(app_handle: AppHandle, conversation_id: String, prompt: String) -> Self {
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

        // Build the initial contents with the prompt
        let initial_content = json!({
            "role": "user",
            "parts": [{
                "text": prompt
            }]
        });
        let contents = vec![initial_content];
        Self {
            app_handle: app_handle.clone(),
            prompt,
            width: width as i32,
            height: height as i32,
            final_response: String::new(),
            contents,
            conversation_id,
        }
    }

    async fn get_model_response(&self) -> Result<serde_json::Value, String> {
        let api_key = std::env::var("GEMINI_API_KEY")
            .map_err(|_| "Missing GEMINI_API_KEY environment variable".to_string())?;
        
        let model = "gemini-2.5-computer-use-preview-10-2025";
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        // Build request body
        let request_body = json!({
            "contents": self.contents,
            "tools": [{
                "computer_use": {
                    "environment": "ENVIRONMENT_BROWSER"
                }
            }],
            "generationConfig": {
                "temperature": 1,
                "topP": 0.95,
                "topK": 40,
                "maxOutputTokens": 8192,
            }
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
            log::error!("[computer_use] API Error {}: {}", code, message);
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

    fn extract_function_calls(&self, candidate: &serde_json::Value) -> Vec<serde_json::Value> {
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

    // Returns base64 png data of screen after action
    async fn handle_action(&self, function_call: &serde_json::Value) -> Result<ActionResponse, String> {
        // Log the function call for debugging
        println!("Handling function call: {}", function_call);
        let name = function_call.get("name")
            .and_then(|n| n.as_str())
            .ok_or("Missing function name")?;
        let args = function_call.get("args")
            .ok_or("Missing function arguments")?;
        
        let response = match name {
            "open_web_browser" => {
                open_web_browser(self.app_handle.clone())
                    .map_err(|_| format!("Failed to open web browser"))?
            }
            "wait_5_seconds" => {
                wait_5_seconds().await.map_err(|_| format!("Failed to wait"))?
            }
            "go_back" => {
                go_back().map_err(|_| format!("Failed to go back"))?
            }
            "go_forward" => {
                go_forward().map_err(|_| format!("Failed to go forward"))?
            }
            "search" => {
                search(self.app_handle.clone())
                    .map_err(|_| format!("Failed to perform search"))?
            }
            "navigate" => {
                let url = args.get("url").and_then(|u| u.as_str()).ok_or("Missing 'url' argument")?;
                navigate(self.app_handle.clone(), url)
                    .map_err(|_| format!("Failed to navigate to URL"))?
            }
            "click_at" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                click_at(actual_x, actual_y).map_err(|_| format!("Failed to click at coordinates"))?
            }
            "hover_at" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                hover_at(actual_x, actual_y).map_err(|_| format!("Failed to hover at coordinates"))?
            }
            "type_text_at" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let text = args.get("text").and_then(|t| t.as_str()).ok_or("Missing 'text' argument")?;
                let press_enter = args.get("press_enter").and_then(|p| p.as_bool());
                let clear_before_typing = args.get("clear_before_typing").and_then(|c| c.as_bool());
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                type_text_at(actual_x, actual_y, text, press_enter, clear_before_typing)
                    .map_err(|_| format!("Failed to type text at coordinates"))?
            }
            "key_combination" => {
                let keys = args.get("keys").and_then(|k| k.as_str()).ok_or("Missing 'keys' argument")?;
                key_combination(keys)
                    .map_err(|_| format!("Failed to perform key combination"))?
            }
            "scroll_document" => {
                let direction = args.get("direction").and_then(|d| d.as_str()).ok_or("Missing 'direction' argument")?;
                scroll_document(direction)
                    .map_err(|_| format!("Failed to scroll document"))?
            }
            "scroll_at" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let direction = args.get("direction").and_then(|d| d.as_str()).ok_or("Missing 'direction' argument")?;
                let magnitude = args.get("magnitude").and_then(|m| m.as_i64()).map(|m| m as i32);
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                scroll_at(actual_x, actual_y, direction, magnitude)
                    .map_err(|_| format!("Failed to scroll at coordinates"))?
            }
            "drag_and_drop" => {
                let x = args.get("x").and_then(|x| x.as_i64()).ok_or("Missing 'x' argument")? as i32;
                let y = args.get("y").and_then(|y| y.as_i64()).ok_or("Missing 'y' argument")? as i32;
                let destination_x = args.get("destination_x").and_then(|x| x.as_i64()).ok_or("Missing 'destination_x' argument")? as i32;
                let destination_y = args.get("destination_y").and_then(|y| y.as_i64()).ok_or("Missing 'destination_y' argument")? as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                let (actual_dest_x, actual_dest_y) = self.denormalize_coordinates(destination_x, destination_y);
                drag_and_drop(actual_x, actual_y, actual_dest_x, actual_dest_y)
                    .map_err(|_| format!("Failed to drag and drop"))?
            }
            _ => {
                return Err(format!("Unknown function: {}", name))
            }
        };
        Ok(response)
    }

    fn denormalize_coordinates(&self, x: i32, y: i32) -> (i32, i32) {
        let actual_x = (x as f64 / 1000.0) * self.width as f64;
        let actual_y = (y as f64 / 1000.0) * self.height as f64;
        (actual_x as i32, actual_y as i32)
    }

    async fn get_safety_confirmation(&self, safety: &serde_json::Value) -> Result<bool, String> {
        log::info!("[computer_use] Safety confirmation required");
        // Ensure the toast window is open
        let _ = open_computer_use_window(self.app_handle.clone()).await;

        let safety_confirmation_event = SafetyConfirmationEvent {
            reason: safety.get("explanation").and_then(|e| e.as_str()).unwrap_or("No explanation provided").to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let _ = emit(GET_SAFETY_CONFIRMATION, safety_confirmation_event);

        // Wait for response
        let (tx, rx) = oneshot::channel();
        self.app_handle.once(SAFETY_CONFIRMATION_RESPONSE, move |event| {
            let payload = event.payload();
            if let Ok(res) = serde_json::from_str::<SafetyConfirmationResponseEvent>(payload) {
                let _ = tx.send(res.user_confirmed);
            }
        });

        // Re-close main window and continue
        let _ = close_main_window(self.app_handle.clone()).await;

        match rx.await {
            Ok(true) => Ok(true),
            Ok(false) => Ok(false),
            Err(_) => Err("Safety confirmation failed or timed out".to_string()),
        }
    }

    async fn save_user_message(&self, content: String) -> Result<(), String> {
        let user_message = add_message(
            self.app_handle.clone(),
            self.conversation_id.clone(),
            "user".to_string(),
            content,
        ).await;

        match user_message {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to save user message: {}", e)),
        }
    }

    async fn save_and_emit_reasoning_message(&self, reasoning: String) -> Result<(), String> {
        if reasoning.is_empty() {
            return Ok(());
        }

        let reasoning_message = add_message(
            self.app_handle.clone(),
            self.conversation_id.clone(),
            "assistant".to_string(),
            reasoning.clone(),
        ).await;

        match reasoning_message {
            Ok(msg) => {
                let reasoning_update = ComputerUseUpdateEvent {
                    status: "in_progress".to_string(),
                    message: msg,
                };
                let _ = emit(COMPUTER_USE_UPDATE, reasoning_update);
            },
            Err(e) => {
                log::error!("[computer_use] Failed to save reasoning message: {}", e);
            }
        }
        Ok(())
    }

    async fn save_and_emit_function_message(&self, function_name: String, args: Vec<String>) -> Result<(), String> {
        let (function_call_message, function_call_toast) = transform_function_call(function_name.clone(), args.clone());

        // Emit toast event
        let toast_event = ComputerUseToastEvent {
            message: function_call_toast,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let _ = emit(COMPUTER_USE_TOAST, toast_event);

        let func_message = add_message(
            self.app_handle.clone(),
            self.conversation_id.clone(),
            "functioncall".to_string(),
            function_call_message.clone(),
        ).await;

        match func_message {
            Ok(msg) => {
                let func_update = ComputerUseUpdateEvent {
                    status: "in_progress".to_string(),
                    message: msg,
                };
                let _ = emit(COMPUTER_USE_UPDATE, func_update);
            },
            Err(e) => {
                log::error!("[computer_use] Failed to save function call message: {}", e);
            }
        }
        Ok(())
    }

    // Returns true if iteration is done, false to continue
    async fn run_one_iteration(&mut self) -> Result<bool, String> {
        log::info!("[computer_use] Running one iteration of computer use engine");

        // Get model response
        let response = self.get_model_response().await?;
        log::info!("[computer_use] Model response: {}", response);

        // Check for blocked response
        if let Some(prompt_feedback) = response.get("promptFeedback") {
            if let Some(block_reason) = prompt_feedback.get("blockReason") {
                log::warn!("[computer_use] Model response blocked due to: {}", block_reason);
                self.final_response = "For safety reasons, the model is unable to complete this request.".to_string();
                return Ok(true);
            }
        }

        // Extract the candidate and append it to the contents
        let mut reasoning = String::new();
        let mut function_calls = Vec::new();
        if let Some(candidates) = response.get("candidates").and_then(|c| c.as_array()) {
            if let Some(first_candidate) = candidates.first() {
                if let Some(text) = self.get_text(first_candidate) {
                    reasoning = text;
                }
                function_calls = self.extract_function_calls(first_candidate);
                if let Some(content) = first_candidate.get("content") {
                    self.contents.push(content.clone());
                }
            }
        } else {
            return Ok(false);
        }

        // Check for malformed function calls and retry if necessary
        if function_calls.is_empty() && reasoning.is_empty() {
            log::warn!("[computer_use] No function calls or final text extracted, retrying iteration");
            return Ok(false);
        }

        // Check for final response
        if function_calls.is_empty() {
            log::info!("[computer_use] No function calls found, treating as final response");
            self.final_response = reasoning;
            return Ok(true);
        }
        
        // Emit reasoning and save to db
        let _ = self.save_and_emit_reasoning_message(reasoning.clone()).await;
        
        // Handle function calls
        let mut function_names = Vec::new();
        let mut args = Vec::new();
        let mut parts = Vec::new();
        for function_call in function_calls {
            log::info!("[computer_use] Handling function call: {}", function_call);

            // Check for safety
            let mut safety_required = false;
            if let Some(safety) = function_call.get("args").and_then(|a| a.get("safety_decision")) {
                if safety.get("decision").and_then(|d| d.as_str()) == Some("require_confirmation") {
                    safety_required = true;
                    let user_confirmed = self.get_safety_confirmation(safety).await?;
                    if !user_confirmed {
                        log::warn!("[computer_use] Safety confirmation denied by user, stopping execution");
                        return Ok(true);
                    }
                }
            }
            let action_result = self.handle_action(&function_call).await;
            if action_result.is_err() {
                log::error!("[computer_use] Error handling action: {}", action_result.err().unwrap());
                return Ok(false);
            }
            let action_response = action_result.unwrap();
            function_names.push(action_response.function_name.clone());
            args.push(action_response.args.clone());

            // Emit and save function call message
            let _ = self.save_and_emit_function_message(
                action_response.function_name.clone(),
                action_response.args.clone()
            ).await;

            // Wait 1 second for UI to update
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let screenshot_vec = take_screenshot();
            let screenshot_data = general_purpose::STANDARD.encode(&screenshot_vec);

            // Create part with function call result
            let mut func_result = json!({
                "functionResponse": {
                    "name": function_call.get("name").and_then(|n| n.as_str()).unwrap_or("unknown"),
                    "response": {
                        "url": "current_url"
                    },
                    "parts": [
                        {
                            "inlineData": {
                                "mimeType": "image/png",
                                "data": screenshot_data
                            }
                        }
                    ]
                }
            });

            // Add safety confirmation info if applicable
            if safety_required {
                func_result["functionResponse"]["response"]["safety_acknowledgement"] = json!(true);
            }

            parts.push(func_result);
        }

        // Emit thinking toast
        let toast_event = ComputerUseToastEvent {
            message: "Thinking".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let _ = emit(COMPUTER_USE_TOAST, toast_event);

        // Create new content entry with function results
        let new_content = json!({
            "role": "user",
            "parts": parts
        });
        self.contents.push(new_content);

        // Only keep screenshots from the last 3 turns
        let mut screenshot_count = 0;
        for content in self.contents.iter_mut().rev() {
            if content.get("role").and_then(|r| r.as_str()) == Some("user") && content.get("parts").is_some() {
                let mut has_screenshot = false;
                if let Some(parts) = content.get_mut("parts").and_then(|p| p.as_array_mut()) {
                    for part in parts.iter() {
                        if let Some(fr) = part.get("functionResponse") {
                            if fr.get("parts").is_some() {
                                has_screenshot = true;
                                break;
                            }
                        }
                    }
                    if has_screenshot {
                        screenshot_count += 1;
                        if screenshot_count > 3 {
                            for part in parts.iter_mut() {
                                if let Some(fr) = part.get_mut("functionResponse") {
                                    if let Some(fr_obj) = fr.as_object_mut() {
                                        fr_obj.remove("parts");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Print contents besides png data for debugging
        // let debug_contents: Vec<_> = self.contents.iter().map(|content| {
        //     let mut debug_content = content.clone();
        //     if let Some(parts) = debug_content.get_mut("parts").and_then(|p| p.as_array_mut()) {
        //         for part in parts.iter_mut() {
        //             if let Some(fr) = part.get_mut("functionResponse") {
        //                 if let Some(fr_parts) = fr.get_mut("parts").and_then(|p| p.as_array_mut()) {
        //                     for fr_part in fr_parts.iter_mut() {
        //                         if let Some(inline_data) = fr_part.get_mut("inlineData") {
        //                             *inline_data = json!("[PNG data omitted]");
        //                         }
        //                     }
        //                 }
        //             }
        //         }
        //     }
        //     debug_content
        // }).collect();
        // log::info!("[computer_use] Completed iteration. New contents: {:?}", debug_contents);
        Ok(false)
    }

    pub async fn run(&mut self) -> Result<(), String> {
        // Save user message
        let _ = self.save_user_message(self.prompt.clone()).await;

        // Close main window and open computer use toast before starting
        let _ = open_computer_use_window(self.app_handle.clone()).await;
        let _ = close_main_window(self.app_handle.clone()).await;

        loop {
            let done = self.run_one_iteration().await?;
            log::info!("[computer_use] Iteration complete. Done: {}", done);
            if done {
                break;
            }
        }

        // Reopen main window and close computer use toast once finished
        let _ = open_main_window(self.app_handle.clone()).await;
        let _ = close_computer_use_window(self.app_handle.clone()).await;

        // Emit final update event
        let final_message = add_message(
            self.app_handle.clone(),
            self.conversation_id.clone(),
            "assistant".to_string(),
            self.final_response.clone(),
        ).await;

        match final_message {
            Ok(msg) => {
                let final_update = ComputerUseUpdateEvent {
                    status: "completed".to_string(),
                    message: msg,
                };
                let _ = emit(COMPUTER_USE_UPDATE, final_update);
            },
            Err(e) => {
                log::error!("[computer_use] Failed to save final message: {}", e);
            }
        }

        Ok(())
    }
}