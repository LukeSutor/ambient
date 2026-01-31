//! Computer Use Runtime - Agentic runtime for computer control tasks.
//!
//! This module implements a runtime for computer use that follows the same
//! patterns as the main agentic chat runtime. It:
//! 1. Uses the unified LlmRequest/LlmResponse types
//! 2. Stores messages and attachments in the conversation database
//! 3. Handles coordinate denormalization for both Gemini (0-1000 scale) and local models
//! 4. Saves screenshots as function responses (per Gemini computer-use spec)
//! 5. Emits agentic runtime events for UI updates
//! 6. Resizes screenshots to 1000x1000 for local models

use crate::db::conversations::{
    add_message, create_attachments, add_attachments, get_conversation_history,
    MessageMetadata, MessageType, Role,
};
use crate::events::{emitter::emit, types::*};
use crate::images::take_screenshot;
use crate::models::computer_use::actions::*;
use crate::models::computer_use::tools::{get_local_computer_use_tools, is_gemini_computer_use_function};
use crate::models::llm::client::generate;
use crate::models::llm::runtime::{
    ToolExecutionStartedEvent, ToolExecutionCompletedEvent,
    TOOL_EXECUTION_STARTED, TOOL_EXECUTION_COMPLETED,
};
use crate::models::llm::types::{LlmRequest, LlmResponse};
use crate::settings::service::load_user_settings;
use crate::settings::types::ModelSelection;
use crate::skills::types::{ToolCall, ToolResult};
use crate::windows::{open_main_window, close_main_window, open_computer_use_window, close_computer_use_window};
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use image::ImageFormat;
use serde_json::json;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, Listener};
use tokio::sync::oneshot;
use uuid::Uuid;

/// Configuration for the computer use runtime.
#[derive(Debug, Clone)]
pub struct ComputerUseConfig {
    /// Maximum iterations before stopping.
    pub max_iterations: usize,
    /// Maximum screenshots to keep in context (to limit token usage).
    pub max_screenshots_in_context: usize,
}

impl Default for ComputerUseConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50,
            max_screenshots_in_context: 3,
        }
    }
}

/// Computer Use Runtime - manages the agentic loop for computer control.
pub struct ComputerUseRuntime {
    app_handle: AppHandle,
    conversation_id: String,
    config: ComputerUseConfig,
    is_local: bool,
    screen_width: i32,
    screen_height: i32,
    iteration: usize,
    cancel_signal: Arc<AtomicBool>,
    /// Last screenshot bytes for attaching to function responses
    last_screenshot_bytes: Option<Vec<u8>>,
}

impl ComputerUseRuntime {
    /// Creates a new computer use runtime.
    pub async fn new(
        app_handle: AppHandle,
        conversation_id: String,
        cancel_signal: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        // Load settings to determine model type
        let settings = load_user_settings(app_handle.clone())
            .await
            .map_err(|e| format!("Failed to load settings: {}", e))?;

        let is_local = matches!(settings.model_selection, ModelSelection::Local);

        // Get screen dimensions
        let (screen_width, screen_height) = Self::get_screen_dimensions(&app_handle);

        log::info!(
            "[computer_use_runtime] Created runtime: is_local={}, screen={}x{}",
            is_local, screen_width, screen_height
        );

        Ok(Self {
            app_handle,
            conversation_id,
            config: ComputerUseConfig::default(),
            is_local,
            screen_width,
            screen_height,
            iteration: 0,
            cancel_signal,
            last_screenshot_bytes: None,
        })
    }

    /// Get screen dimensions from the main window's monitor.
    fn get_screen_dimensions(app_handle: &AppHandle) -> (i32, i32) {
        if let Some(window) = app_handle.get_webview_window("main") {
            if let Ok(Some(monitor)) = window.current_monitor() {
                let physical_size = monitor.size();
                return (physical_size.width as i32, physical_size.height as i32);
            }
        }
        log::warn!("[computer_use_runtime] Failed to get screen dimensions, using defaults");
        (1920, 1080)
    }

    /// Resize PNG image bytes to 1000x1000 for local model.
    /// Note: This is used for local models to ensure consistent coordinate output.
    #[allow(dead_code)]
    fn resize_screenshot_for_local(png_bytes: &[u8]) -> Result<Vec<u8>, String> {
        let img = image::load_from_memory_with_format(png_bytes, ImageFormat::Png)
            .map_err(|e| format!("Failed to load screenshot: {}", e))?;
        
        let resized = img.resize_exact(1000, 1000, image::imageops::FilterType::Lanczos3);
        
        let mut output = Cursor::new(Vec::new());
        resized.write_to(&mut output, ImageFormat::Png)
            .map_err(|e| format!("Failed to encode resized screenshot: {}", e))?;
        
        Ok(output.into_inner())
    }

    /// Run the computer use session.
    pub async fn run(&mut self, prompt: String) -> Result<String, String> {
        log::info!("[computer_use_runtime] Starting session with prompt: {}", prompt);

        // Close main window and open computer use toast
        let _ = open_computer_use_window(self.app_handle.clone()).await;
        let _ = close_main_window(self.app_handle.clone()).await;

        // Emit initial toast
        self.emit_toast("Starting computer use session").await;

        // Save user message with initial screenshot
        self.save_user_message_with_screenshot(&prompt).await?;

        // Main loop
        let final_response: String;

        'main_loop: loop {
            // Check cancellation
            if self.cancel_signal.load(Ordering::SeqCst) {
                log::info!("[computer_use_runtime] Cancelled by user");
                final_response = "*Computer use session cancelled by user*".to_string();
                break;
            }

            self.iteration += 1;
            if self.iteration > self.config.max_iterations {
                log::warn!("[computer_use_runtime] Max iterations exceeded");
                final_response = "Maximum iterations exceeded. Please try a simpler task.".to_string();
                break;
            }

            log::info!("[computer_use_runtime] Iteration {}/{}", self.iteration, self.config.max_iterations);
            self.emit_toast("Analyzing screen...").await;

            // Get conversation history
            let messages = get_conversation_history(
                &self.app_handle,
                &self.conversation_id,
                if self.is_local { 3 } else { 10 },
            ).await?;

            // Build LLM request
            let request = self.build_llm_request(&messages).await?;

            // Generate response
            let response = match generate(
                self.app_handle.clone(),
                request,
                Some(self.is_local),
            ).await {
                Ok(resp) => resp,
                Err(e) => {
                    if e.contains("cancelled") {
                        final_response = "*Computer use session cancelled by user*".to_string();
                        break;
                    }
                    return Err(format!("LLM generation failed: {}", e));
                }
            };

            // Handle response
            match response {
                LlmResponse::Text(text) => {
                    // Final response - no more actions needed
                    log::info!("[computer_use_runtime] Final response received");
                    final_response = text.clone();
                    self.save_assistant_message(&text, MessageType::Text, None).await?;
                    break;
                }

                LlmResponse::ToolCalls { text, calls } => {
                    // Save reasoning if present
                    if let Some(reasoning) = text {
                        if !reasoning.is_empty() {
                            self.save_thinking_message(&reasoning).await?;
                        }
                    }

                    if calls.is_empty() {
                        log::warn!("[computer_use_runtime] Empty tool calls, treating as final");
                        final_response = "*Task completed*".to_string();
                        self.save_assistant_message(&final_response, MessageType::Text, None).await?;
                        break;
                    }

                    // Save the tool calls as assistant message first
                    for call in &calls {
                        self.save_tool_call_message(call).await?;
                    }

                    // Execute each tool call and save function responses with screenshots
                    for call in &calls {
                        // Check for safety confirmation
                        if let Some(safety) = call.arguments.get("safety_decision") {
                            if safety.get("decision").and_then(|d| d.as_str()) == Some("require_confirmation") {
                                let confirmed = self.get_safety_confirmation(safety).await?;
                                if !confirmed {
                                    final_response = "*Safety confirmation denied. Session stopped*".to_string();
                                    self.save_assistant_message(&final_response, MessageType::Text, None).await?;
                                    break 'main_loop;
                                }
                            }
                        }

                        // Execute the action
                        let result = self.execute_computer_action(call).await;

                        // Wait for UI to update
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                        // Take new screenshot after action
                        let screenshot_bytes = take_screenshot();
                        self.last_screenshot_bytes = Some(screenshot_bytes.clone());

                        // Save function response with screenshot (per Gemini computer-use spec)
                        self.save_tool_result_with_screenshot(call, &result, &screenshot_bytes).await?;
                    }
                }
            }
        }

        // Reopen main window and close toast
        let _ = open_main_window(self.app_handle.clone()).await;
        let _ = close_computer_use_window(self.app_handle.clone()).await;

        // Emit final update
        let final_message = self.save_assistant_message(&final_response, MessageType::Text, None).await?;
        let final_update = ComputerUseUpdateEvent {
            status: "completed".to_string(),
            message: final_message,
        };
        let _ = emit(COMPUTER_USE_UPDATE, final_update);

        Ok(final_response)
    }

    /// Build LLM request based on model type.
    async fn build_llm_request(
        &self,
        messages: &[crate::db::conversations::Message],
    ) -> Result<LlmRequest, String> {
        let system_prompt = if self.is_local {
            "You are a computer use assistant. You can see the screen and interact with it using the provided tools. \
             Look at the screenshot carefully and determine what actions to take to accomplish the user's goal. \
             The screenshot is 1000x1000 pixels, so coordinates should be in the range 0-999. \
             When you're done, respond with a summary of what you accomplished.".to_string()
        } else {
            // Gemini computer-use has its own system prompt
            String::new()
        };

        let tools = if self.is_local {
            Some(get_local_computer_use_tools())
        } else {
            // Gemini computer-use model has built-in tools
            None
        };

        // Use computer-use model type for cloud models (overrides any model selection in settings)
        let model_type = if self.is_local {
            None
        } else {
            Some("computer-use".to_string())
        };

        Ok(LlmRequest::new(String::new())
            .with_system_prompt(Some(system_prompt))
            .with_messages(Some(messages.to_vec()))
            .with_internal_tools(tools)
            .with_conv_id(Some(self.conversation_id.clone()))
            .with_stream(Some(false))  // Computer use works better non-streaming
            .with_cancel_signal(Some(self.cancel_signal.clone()))
            .with_model_type(model_type))
    }

    /// Save the initial user message with a screenshot attachment.
    async fn save_user_message_with_screenshot(&mut self, prompt: &str) -> Result<(), String> {
        // Take screenshot
        let screenshot_bytes = take_screenshot();
        self.last_screenshot_bytes = Some(screenshot_bytes.clone());

        // Create user message first
        let message_id = Uuid::new_v4().to_string();
        let message = add_message(
            &self.app_handle,
            self.conversation_id.clone(),
            Role::User,
            prompt.to_string(),
            Some(MessageType::Text),
            None,
            Some(message_id.clone()),
        ).await?;

        // Create and attach screenshot (save full resolution)
        let timestamp = Utc::now().to_rfc3339();
        let filename = format!("screenshot_{}.png", timestamp.replace(":", "-"));
        let screenshot_base64 = general_purpose::STANDARD.encode(&screenshot_bytes);
        
        let attachment_data = AttachmentData {
            name: filename,
            file_type: "image/png".to_string(),
            data: screenshot_base64,
        };

        let attachments = create_attachments(
            &self.app_handle,
            message_id.clone(),
            vec![attachment_data],
        ).await?;

        // Emit attachment created event so UI updates
        if !attachments.is_empty() {
            let attachments_event = AttachmentsCreatedEvent {
                message_id: message_id.clone(),
                attachments: attachments.clone(),
                timestamp: Utc::now().to_rfc3339(),
            };
            let _ = emit(ATTACHMENTS_CREATED, attachments_event);
        }

        // Link attachments to message
        if !attachments.is_empty() {
            add_attachments(&self.app_handle, message.id.clone(), attachments).await?;
        }

        Ok(())
    }

    /// Execute a computer action based on the tool call.
    async fn execute_computer_action(&self, call: &ToolCall) -> ToolResult {
        let tool_name = if call.skill_name == "computer-use" {
            call.tool_name.as_str()
        } else if is_gemini_computer_use_function(&call.tool_name) {
            call.tool_name.as_str()
        } else {
            return ToolResult::error(call.id.clone(), format!("Unknown tool: {}", call.tool_name));
        };

        // Emit tool execution started
        let started_event = ToolExecutionStartedEvent {
            tool_call_id: call.id.clone(),
            message_id: String::new(),
            skill_name: "computer-use".to_string(),
            tool_name: tool_name.to_string(),
            arguments: call.arguments.clone(),
            timestamp: Utc::now().to_rfc3339(),
        };
        let _ = emit(TOOL_EXECUTION_STARTED, started_event);

        // Toast the action
        let toast_msg = self.format_action_toast(tool_name, &call.arguments);
        self.emit_toast(&toast_msg).await;

        let result: Result<serde_json::Value, String> = match tool_name {
            // Local model tools (coordinates are 0-999 because they see 1000x1000 image)
            // Need to denormalize to actual screen pixels
            "click" => {
                let x = call.arguments.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = call.arguments.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                click_at(actual_x, actual_y).map(|r| json!({"status": "clicked", "action": r.function_name, "x": actual_x, "y": actual_y}))
            }
            "type_text" => {
                let x = call.arguments.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = call.arguments.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let text = call.arguments.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let press_enter = call.arguments.get("press_enter").and_then(|v| v.as_bool());
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                type_text_at(actual_x, actual_y, text, press_enter, Some(true))
                    .map(|r| json!({"status": "typed", "action": r.function_name, "text": text}))
            }
            "scroll" => {
                let direction = call.arguments.get("direction").and_then(|v| v.as_str()).unwrap_or("down");
                scroll_document(direction).map(|r| json!({"status": "scrolled", "action": r.function_name, "direction": direction}))
            }
            "navigate" => {
                let url = call.arguments.get("url").and_then(|v| v.as_str()).unwrap_or("https://google.com");
                navigate(self.app_handle.clone(), url).map(|r| json!({"status": "navigated", "action": r.function_name, "url": url}))
            }
            "wait" => {
                wait_5_seconds().await.map(|r| json!({"status": "waited", "action": r.function_name}))
            }

            // Gemini computer-use tools (normalized coordinates 0-1000)
            "open_web_browser" => {
                open_web_browser(self.app_handle.clone()).map(|r| json!({"status": "browser_opened", "action": r.function_name}))
            }
            "wait_5_seconds" => {
                wait_5_seconds().await.map(|r| json!({"status": "waited", "action": r.function_name}))
            }
            "go_back" => {
                go_back().map(|r| json!({"status": "went_back", "action": r.function_name}))
            }
            "go_forward" => {
                go_forward().map(|r| json!({"status": "went_forward", "action": r.function_name}))
            }
            "search" => {
                search(self.app_handle.clone()).map(|r| json!({"status": "search_opened", "action": r.function_name}))
            }
            "click_at" => {
                let x = call.arguments.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = call.arguments.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                click_at(actual_x, actual_y).map(|r| json!({"status": "clicked", "action": r.function_name, "x": actual_x, "y": actual_y}))
            }
            "hover_at" => {
                let x = call.arguments.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = call.arguments.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                hover_at(actual_x, actual_y).map(|r| json!({"status": "hovered", "action": r.function_name, "x": actual_x, "y": actual_y}))
            }
            "type_text_at" => {
                let x = call.arguments.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = call.arguments.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let text = call.arguments.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let press_enter = call.arguments.get("press_enter").and_then(|v| v.as_bool());
                let clear = call.arguments.get("clear_before_typing").and_then(|v| v.as_bool());
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                type_text_at(actual_x, actual_y, text, press_enter, clear)
                    .map(|r| json!({"status": "typed", "action": r.function_name, "text": text}))
            }
            "key_combination" => {
                let keys = call.arguments.get("keys").and_then(|v| v.as_str()).unwrap_or("");
                key_combination(keys).map(|r| json!({"status": "keys_pressed", "action": r.function_name, "keys": keys}))
            }
            "scroll_document" => {
                let direction = call.arguments.get("direction").and_then(|v| v.as_str()).unwrap_or("down");
                scroll_document(direction).map(|r| json!({"status": "scrolled", "action": r.function_name, "direction": direction}))
            }
            "scroll_at" => {
                let x = call.arguments.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = call.arguments.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let direction = call.arguments.get("direction").and_then(|v| v.as_str()).unwrap_or("down");
                let magnitude = call.arguments.get("magnitude").and_then(|v| v.as_i64()).map(|m| m as i32);
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                scroll_at(actual_x, actual_y, direction, magnitude)
                    .map(|r| json!({"status": "scrolled", "action": r.function_name, "direction": direction}))
            }
            "drag_and_drop" => {
                let x = call.arguments.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = call.arguments.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dest_x = call.arguments.get("destination_x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dest_y = call.arguments.get("destination_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let (actual_x, actual_y) = self.denormalize_coordinates(x, y);
                let (actual_dest_x, actual_dest_y) = self.denormalize_coordinates(dest_x, dest_y);
                drag_and_drop(actual_x, actual_y, actual_dest_x, actual_dest_y)
                    .map(|r| json!({"status": "dragged", "action": r.function_name}))
            }

            _ => Err(format!("Unknown action: {}", tool_name)),
        };

        let tool_result = match result {
            Ok(value) => ToolResult::success(call.id.clone(), value),
            Err(e) => ToolResult::error(call.id.clone(), e),
        };

        // Emit tool execution completed
        let completed_event = ToolExecutionCompletedEvent {
            tool_call_id: call.id.clone(),
            message_id: String::new(),
            skill_name: "computer-use".to_string(),
            tool_name: tool_name.to_string(),
            success: tool_result.success,
            result: tool_result.result.clone(),
            error: tool_result.error.clone(),
            timestamp: Utc::now().to_rfc3339(),
        };
        let _ = emit(TOOL_EXECUTION_COMPLETED, completed_event);

        tool_result
    }

    /// Denormalize coordinates from 0-1000 scale to actual pixels.
    fn denormalize_coordinates(&self, x: i32, y: i32) -> (i32, i32) {
        let actual_x = (x as f64 / 1000.0 * self.screen_width as f64) as i32;
        let actual_y = (y as f64 / 1000.0 * self.screen_height as f64) as i32;
        (actual_x, actual_y)
    }

    /// Format a toast message for an action.
    fn format_action_toast(&self, tool_name: &str, args: &serde_json::Value) -> String {
        match tool_name {
            "click" | "click_at" => {
                let x = args.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                let y = args.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
                format!("Clicking at ({}, {})", x, y)
            }
            "type_text" | "type_text_at" => "Typing text...".to_string(),
            "scroll" | "scroll_document" | "scroll_at" => {
                let dir = args.get("direction").and_then(|v| v.as_str()).unwrap_or("down");
                format!("Scrolling {}", dir)
            }
            "navigate" => {
                let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("");
                format!("Navigating to {}", url)
            }
            "wait" | "wait_5_seconds" => "Waiting...".to_string(),
            "open_web_browser" | "search" => "Opening browser...".to_string(),
            "go_back" => "Going back...".to_string(),
            "go_forward" => "Going forward...".to_string(),
            "hover_at" => "Hovering...".to_string(),
            "key_combination" => "Pressing keys...".to_string(),
            "drag_and_drop" => "Dragging...".to_string(),
            _ => format!("Executing {}...", tool_name),
        }
    }

    /// Emit a toast message.
    async fn emit_toast(&self, message: &str) {
        let toast_event = ComputerUseToastEvent {
            message: message.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        let _ = emit(COMPUTER_USE_TOAST, toast_event);
    }

    /// Get safety confirmation from user.
    async fn get_safety_confirmation(&self, safety: &serde_json::Value) -> Result<bool, String> {
        log::info!("[computer_use_runtime] Safety confirmation required");

        let reason = safety.get("explanation")
            .and_then(|e| e.as_str())
            .unwrap_or("No explanation provided");

        let safety_event = SafetyConfirmationEvent {
            reason: reason.to_string(),
            timestamp: Utc::now().to_rfc3339(),
        };
        let _ = emit(GET_SAFETY_CONFIRMATION, safety_event);

        // Wait for response
        let (tx, rx) = oneshot::channel();
        self.app_handle.once(SAFETY_CONFIRMATION_RESPONSE, move |event| {
            let payload = event.payload();
            if let Ok(res) = serde_json::from_str::<SafetyConfirmationResponseEvent>(payload) {
                let _ = tx.send(res.user_confirmed);
            }
        });

        // Keep main window closed
        let _ = close_main_window(self.app_handle.clone()).await;

        match rx.await {
            Ok(confirmed) => Ok(confirmed),
            Err(_) => Err("Safety confirmation failed or timed out".to_string()),
        }
    }

    /// Save tool call message (assistant requesting action).
    async fn save_tool_call_message(&self, call: &ToolCall) -> Result<(), String> {
        let call_metadata = MessageMetadata::ToolCall {
            call_id: call.id.clone(),
            skill_name: "computer-use".to_string(),
            tool_name: call.tool_name.clone(),
            arguments: call.arguments.clone(),
            thought_signature: call.thought_signature.clone(),
        };

        let call_content = format!(
            "{}: {}",
            call.tool_name,
            serde_json::to_string_pretty(&call.arguments).unwrap_or_default()
        );

        self.save_assistant_message(&call_content, MessageType::ToolCall, Some(call_metadata)).await?;
        Ok(())
    }

    /// Save assistant message.
    async fn save_assistant_message(
        &self,
        content: &str,
        message_type: MessageType,
        metadata: Option<MessageMetadata>,
    ) -> Result<crate::db::conversations::Message, String> {
        let message = add_message(
            &self.app_handle,
            self.conversation_id.clone(),
            Role::Assistant,
            content.to_string(),
            Some(message_type.clone()),
            metadata,
            None,
        ).await?;

        // Emit update event
        let update_event = ComputerUseUpdateEvent {
            status: "in_progress".to_string(),
            message: message.clone(),
        };
        let _ = emit(COMPUTER_USE_UPDATE, update_event);

        Ok(message)
    }

    /// Save thinking message.
    async fn save_thinking_message(&self, reasoning: &str) -> Result<(), String> {
        let metadata = MessageMetadata::Thinking {
            stage: "Reasoning".to_string(),
        };
        self.save_assistant_message(reasoning, MessageType::Thinking, Some(metadata)).await?;
        Ok(())
    }

    /// Save tool result with screenshot as function response (per Gemini spec).
    async fn save_tool_result_with_screenshot(
        &self,
        _call: &ToolCall,
        result: &ToolResult,
        screenshot_bytes: &[u8],
    ) -> Result<(), String> {
        // Create the tool result message
        let message_id = Uuid::new_v4().to_string();
        
        // Save full-resolution screenshot to disk
        let timestamp = Utc::now().to_rfc3339();
        let filename = format!("screenshot_{}.png", timestamp.replace(":", "-"));
        let screenshot_base64 = general_purpose::STANDARD.encode(screenshot_bytes);
        
        let attachment_data = AttachmentData {
            name: filename,
            file_type: "image/png".to_string(),
            data: screenshot_base64,
        };

        let attachments = create_attachments(
            &self.app_handle,
            message_id.clone(),
            vec![attachment_data],
        ).await?;

        // Emit attachment created event so UI updates
        if !attachments.is_empty() {
            let attachments_event = AttachmentsCreatedEvent {
                message_id: message_id.clone(),
                attachments: attachments.clone(),
                timestamp: Utc::now().to_rfc3339(),
            };
            let _ = emit(ATTACHMENTS_CREATED, attachments_event);
        }

        let screenshot_attachment_id = attachments.first().map(|a| a.id.clone());

        // Create tool result metadata with screenshot reference
        let result_metadata = MessageMetadata::ToolResult {
            call_id: result.call_id.clone(),
            success: result.success,
            error: result.error.clone(),
            result: result.result.clone(),
            screenshot_attachment_id,
        };

        let result_content = if result.success {
            format!("Success: {:?}", result.result)
        } else {
            format!("Error: {:?}", result.error)
        };

        let message = add_message(
            &self.app_handle,
            self.conversation_id.clone(),
            Role::Tool,
            result_content,
            Some(MessageType::ToolResult),
            Some(result_metadata),
            Some(message_id.clone()),
        ).await?;

        // Link attachments to message
        if !attachments.is_empty() {
            add_attachments(&self.app_handle, message.id.clone(), attachments).await?;
        }

        Ok(())
    }
}
