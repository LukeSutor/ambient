//! Browser-use agentic runtime.
//!
//! Implements the main loop for browser agent sessions:
//! 1. Creates a persistent WebView
//! 2. Takes a DOM snapshot
//! 3. Sends snapshot + history to the LLM
//! 4. Executes the returned action
//! 5. Loops until `done` is called or max iterations reached

use crate::db::conversations::{
    add_message, get_conversation_history, get_or_refresh_prompt_time,
    MessageMetadata, MessageType, Role,
};
use crate::events::{emitter::emit, types::{ChatStreamEvent, CHAT_STREAM}};
use crate::models::llm::client::generate;
use crate::models::llm::prompts::get_prompt;
use crate::models::llm::types::{LlmRequest, LlmResponse};
use crate::settings::service::load_user_settings;
use crate::settings::types::ModelSelection;
use crate::skills::types::{AgentError, ToolCall, ToolDefinition, ToolParameter, ParameterType};
use chrono::Local;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

use super::actions::execute_action;
use super::types::BrowserUseConfig;
use super::webview::{create_browser_webview, destroy_browser_webview, extract_snapshot};

/// Browser-use agent runtime.
pub struct BrowserUseRuntime {
    app_handle: AppHandle,
    conv_id: String,
    assistant_message_id: String,
    config: BrowserUseConfig,
    is_local: bool,
    iteration: usize,
    cancel_signal: Arc<AtomicBool>,
    cancel_notify: Arc<tokio::sync::Notify>,
}

impl BrowserUseRuntime {
    /// Creates a new browser-use runtime.
    pub async fn new(
        app_handle: AppHandle,
        conv_id: String,
        assistant_message_id: String,
        cancel_signal: Arc<AtomicBool>,
        cancel_notify: Arc<tokio::sync::Notify>,
    ) -> Result<Self, AgentError> {
        let settings = load_user_settings(app_handle.clone())
            .await
            .map_err(|e| AgentError::RuntimeError(format!("Failed to load settings: {}", e)))?;

        let is_local = matches!(settings.model_selection, ModelSelection::Local);
        let config = BrowserUseConfig::default();

        Ok(Self {
            app_handle,
            conv_id,
            assistant_message_id,
            config,
            is_local,
            iteration: 0,
            cancel_signal,
            cancel_notify,
        })
    }

    /// Runs the browser-use loop until task completion or cancellation.
    pub async fn run(
        mut self,
        user_message: String,
        message_id: Option<String>,
    ) -> Result<String, AgentError> {
        // Save user message
        let msg_id = message_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        add_message(
            &self.app_handle,
            self.conv_id.clone(),
            Role::User,
            user_message.clone(),
            Some(MessageType::Text),
            None,
            Some(msg_id),
        )
        .await?;

        // Create browser WebView
        create_browser_webview(&self.app_handle, &self.config.start_url)
            .map_err(|e| AgentError::RuntimeError(format!("Failed to create browser: {}", e)))?;

        // Build system prompt once (for KV cache stability)
        let system_prompt = self.build_system_prompt().await;

        // Wait for initial page load
        tokio::time::sleep(tokio::time::Duration::from_millis(self.config.navigation_delay_ms)).await;

        let result = self.run_loop(&system_prompt).await;

        // Cleanup
        destroy_browser_webview(&self.app_handle);

        result
    }

    /// Main browser-use loop.
    async fn run_loop(&mut self, system_prompt: &str) -> Result<String, AgentError> {
        loop {
            // Check cancellation
            if self.cancel_signal.load(Ordering::SeqCst) {
                log::info!("[browser_use] Session cancelled by user");
                let text = "*Browser session cancelled by you*".to_string();
                self.emit_final_response(&text).await?;
                return Ok(text);
            }

            self.iteration += 1;
            if self.iteration > self.config.max_iterations {
                let text = "*Browser session reached maximum iterations*".to_string();
                self.emit_final_response(&text).await?;
                return Ok(text);
            }

            log::info!(
                "[browser_use] Iteration {}/{}",
                self.iteration,
                self.config.max_iterations
            );

            // Extract DOM snapshot
            let snapshot = match extract_snapshot(
                &self.app_handle,
                self.config.snapshot_timeout_secs,
            )
            .await
            {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("[browser_use] Snapshot failed: {}", e);
                    // Save snapshot error as tool result and continue
                    let error_msg = format!("Failed to extract page snapshot: {}", e);
                    self.save_snapshot_message(&error_msg).await?;
                    continue;
                }
            };

            log::info!(
                "[browser_use] Snapshot extracted ({} chars)",
                snapshot.len()
            );

            // Save snapshot as a tool result message
            self.save_snapshot_message(&snapshot).await?;

            // Get conversation history (context-limited)
            let messages = get_conversation_history(
                &self.app_handle,
                &self.conv_id,
                self.config.context_limit_for(self.is_local),
            )
            .await?;

            // Build LLM request
            let timeout_duration = if self.is_local { 30 } else { 10 };
            let tools = self.get_browser_tools();

            let request = LlmRequest::new(String::new())
                .with_system_prompt(Some(system_prompt.to_string()))
                .with_messages(Some(messages))
                .with_internal_tools(Some(tools))
                .with_conv_id(Some(self.conv_id.clone()))
                .with_stream(Some(true))
                .with_assistant_message_id(Some(self.assistant_message_id.clone()))
                .with_cancel_signal(Some(self.cancel_signal.clone()))
                .with_cancel_notify(Some(self.cancel_notify.clone()))
                .with_attempts(Some(3))
                .with_timeout_duration(Some(timeout_duration))
                .with_slot_id(Some(0)); // Same slot as agentic chat

            // Generate response
            let response = match generate(
                self.app_handle.clone(),
                request,
                Some(self.is_local),
            )
            .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    if e.contains("cancelled") || e.contains("timed out") {
                        let text = if e.contains("timed out") {
                            "*Browser session timed out*".to_string()
                        } else {
                            "*Browser session cancelled by you*".to_string()
                        };
                        self.emit_final_response(&text).await?;
                        return Ok(text);
                    }
                    return Err(AgentError::LlmError(e));
                }
            };

            // Handle response
            match response {
                LlmResponse::Text(text) => {
                    // Model responded with text (task complete or message to user)
                    log::info!("[browser_use] Final text response received");
                    self.save_assistant_message(&text).await?;
                    return Ok(text);
                }
                LlmResponse::ToolCalls { text, calls } => {
                    if calls.len() > self.config.max_tool_calls_per_turn {
                        log::warn!(
                            "[browser_use] Too many tool calls ({}/{}), truncating",
                            calls.len(),
                            self.config.max_tool_calls_per_turn
                        );
                    }

                    let calls_to_execute: Vec<ToolCall> = calls
                        .into_iter()
                        .take(self.config.max_tool_calls_per_turn)
                        .collect();

                    // Save tool calls message
                    let tool_call_metadatas: Vec<MessageMetadata> = calls_to_execute
                        .iter()
                        .map(|call| MessageMetadata::ToolCall {
                            call_id: call.id.clone(),
                            skill_name: call.skill_name.clone(),
                            tool_name: call.tool_name.clone(),
                            arguments: call.arguments.clone(),
                            thought_signature: call.thought_signature.clone(),
                        })
                        .collect();

                    let content = text.unwrap_or_default();
                    add_message(
                        &self.app_handle,
                        self.conv_id.clone(),
                        Role::Assistant,
                        content.clone(),
                        Some(MessageType::ToolCalls),
                        Some(tool_call_metadatas),
                        Some(uuid::Uuid::new_v4().to_string()),
                    )
                    .await?;

                    // Check if model called "done"
                    let done_call = calls_to_execute
                        .iter()
                        .find(|c| c.tool_name == "done");

                    if let Some(done) = done_call {
                        let summary = done
                            .arguments
                            .get("summary")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Task completed.")
                            .to_string();

                        log::info!("[browser_use] Task done: {}", summary);

                        // Save done result
                        let result_metadata = vec![MessageMetadata::ToolResult {
                            call_id: done.id.clone(),
                            success: true,
                            error: None,
                            result: Some(serde_json::json!({ "status": "completed", "summary": summary })),
                            screenshot_attachment_id: None,
                        }];

                        add_message(
                            &self.app_handle,
                            self.conv_id.clone(),
                            Role::Tool,
                            String::new(),
                            Some(MessageType::ToolResults),
                            Some(result_metadata),
                            None,
                        )
                        .await?;

                        // Save final assistant message
                        self.emit_final_response(&summary).await?;
                        return Ok(summary);
                    }

                    // Execute non-done actions sequentially
                    let mut result_metadatas = Vec::new();
                    for call in &calls_to_execute {
                        if call.tool_name == "done" {
                            continue;
                        }

                        let action_result = execute_action(
                            &self.app_handle,
                            &call.tool_name,
                            &call.arguments,
                        )
                        .await;

                        let metadata = match &action_result {
                            Ok(msg) => MessageMetadata::ToolResult {
                                call_id: call.id.clone(),
                                success: true,
                                error: None,
                                result: Some(serde_json::json!({ "result": msg })),
                                screenshot_attachment_id: None,
                            },
                            Err(e) => MessageMetadata::ToolResult {
                                call_id: call.id.clone(),
                                success: false,
                                error: Some(e.clone()),
                                result: None,
                                screenshot_attachment_id: None,
                            },
                        };
                        result_metadatas.push(metadata);

                        if let Err(e) = &action_result {
                            log::warn!("[browser_use] Action '{}' failed: {}", call.tool_name, e);
                        }
                    }

                    // Save tool results
                    if !result_metadatas.is_empty() {
                        add_message(
                            &self.app_handle,
                            self.conv_id.clone(),
                            Role::Tool,
                            String::new(),
                            Some(MessageType::ToolResults),
                            Some(result_metadatas),
                            None,
                        )
                        .await?;
                    }

                    // Wait for action effects to settle before next snapshot
                    let delay = if calls_to_execute.iter().any(|c| c.tool_name == "navigate") {
                        self.config.navigation_delay_ms
                    } else {
                        self.config.action_delay_ms
                    };
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }

    /// Build the system prompt with dynamic context.
    async fn build_system_prompt(&self) -> String {
        let context = self.build_context().await;
        let template = get_prompt("browser_use").unwrap_or_default();
        template.replace("{context}", &context)
    }

    /// Build dynamic context (date, user, timezone, OS).
    async fn build_context(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        match get_or_refresh_prompt_time(&self.app_handle, &self.conv_id).await {
            Ok(time_str) => parts.push(format!("Today is {}.", time_str)),
            Err(_) => {
                let fallback = Local::now()
                    .format("%A, %B %e, %Y at %l:%M %p")
                    .to_string()
                    .replace("  ", " ");
                parts.push(format!("Today is {}.", fallback));
            }
        }

        if let Ok(Some(auth_state)) = crate::auth::storage::retrieve_auth_state() {
            if let Some(ref meta) = auth_state.session.user.user_metadata {
                if let Some(ref name) = meta.full_name {
                    if !name.is_empty() {
                        parts.push(format!("The user's name is {}.", name));
                    }
                }
            }
        }

        if let Ok(tz) = iana_time_zone::get_timezone() {
            parts.push(format!("Timezone: {}.", tz));
        }

        parts.push(format!("OS: {}.", std::env::consts::OS));
        parts.join(" ")
    }

    /// Save a DOM snapshot as a tool result message.
    async fn save_snapshot_message(&self, snapshot: &str) -> Result<(), AgentError> {
        let metadata = vec![MessageMetadata::ToolResult {
            call_id: format!("snapshot_{}", self.iteration),
            success: true,
            error: None,
            result: Some(serde_json::json!({ "snapshot": snapshot })),
            screenshot_attachment_id: None,
        }];

        add_message(
            &self.app_handle,
            self.conv_id.clone(),
            Role::Tool,
            snapshot.to_string(),
            Some(MessageType::ToolResults),
            Some(metadata),
            None,
        )
        .await?;

        Ok(())
    }

    /// Save an assistant text message.
    async fn save_assistant_message(&self, content: &str) -> Result<(), AgentError> {
        add_message(
            &self.app_handle,
            self.conv_id.clone(),
            Role::Assistant,
            content.to_string(),
            Some(MessageType::Text),
            None,
            Some(self.assistant_message_id.clone()),
        )
        .await?;
        Ok(())
    }

    /// Emit the final chat stream event and save the response.
    async fn emit_final_response(&self, text: &str) -> Result<(), AgentError> {
        // Save assistant message
        self.save_assistant_message(text).await?;

        // Emit stream event so the UI updates
        let stream_data = ChatStreamEvent {
            delta: String::new(),
            is_finished: true,
            full_response: text.to_string(),
            conv_id: Some(self.conv_id.clone()),
            message_id: Some(self.assistant_message_id.clone()),
        };
        let _ = emit(CHAT_STREAM, stream_data);

        Ok(())
    }

    /// Get the browser action tool definitions.
    fn get_browser_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                skill_name: Some("browser".to_string()),
                name: "navigate".to_string(),
                description: "Navigate to a URL.".to_string(),
                parameters: vec![ToolParameter {
                    name: "url".to_string(),
                    param_type: ParameterType::String,
                    description: "The full URL to navigate to.".to_string(),
                    required: true,
                    default: None,
                }],
                returns: None,
            },
            ToolDefinition {
                skill_name: Some("browser".to_string()),
                name: "click".to_string(),
                description: "Click an element by its [ID] number from the snapshot.".to_string(),
                parameters: vec![ToolParameter {
                    name: "element_id".to_string(),
                    param_type: ParameterType::Integer,
                    description: "The element ID number from the page snapshot.".to_string(),
                    required: true,
                    default: None,
                }],
                returns: None,
            },
            ToolDefinition {
                skill_name: Some("browser".to_string()),
                name: "type_text".to_string(),
                description: "Type text into an input element.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "element_id".to_string(),
                        param_type: ParameterType::Integer,
                        description: "The element ID number of the input field.".to_string(),
                        required: true,
                        default: None,
                    },
                    ToolParameter {
                        name: "text".to_string(),
                        param_type: ParameterType::String,
                        description: "The text to type.".to_string(),
                        required: true,
                        default: None,
                    },
                    ToolParameter {
                        name: "press_enter".to_string(),
                        param_type: ParameterType::Boolean,
                        description: "Whether to press Enter after typing. Defaults to false.".to_string(),
                        required: false,
                        default: Some(serde_json::json!(false)),
                    },
                ],
                returns: None,
            },
            ToolDefinition {
                skill_name: Some("browser".to_string()),
                name: "select_option".to_string(),
                description: "Select an option from a dropdown <select> element.".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "element_id".to_string(),
                        param_type: ParameterType::Integer,
                        description: "The element ID of the select dropdown.".to_string(),
                        required: true,
                        default: None,
                    },
                    ToolParameter {
                        name: "value".to_string(),
                        param_type: ParameterType::String,
                        description: "The option value or text to select.".to_string(),
                        required: true,
                        default: None,
                    },
                ],
                returns: None,
            },
            ToolDefinition {
                skill_name: Some("browser".to_string()),
                name: "scroll".to_string(),
                description: "Scroll the page up or down.".to_string(),
                parameters: vec![ToolParameter {
                    name: "direction".to_string(),
                    param_type: ParameterType::String,
                    description: "Scroll direction: 'up' or 'down'.".to_string(),
                    required: true,
                    default: None,
                }],
                returns: None,
            },
            ToolDefinition {
                skill_name: Some("browser".to_string()),
                name: "go_back".to_string(),
                description: "Go back to the previous page.".to_string(),
                parameters: vec![],
                returns: None,
            },
            ToolDefinition {
                skill_name: Some("browser".to_string()),
                name: "wait".to_string(),
                description: "Wait for the page to update. Use after actions that trigger loading.".to_string(),
                parameters: vec![ToolParameter {
                    name: "seconds".to_string(),
                    param_type: ParameterType::Integer,
                    description: "Seconds to wait (1-10). Defaults to 2.".to_string(),
                    required: false,
                    default: Some(serde_json::json!(2)),
                }],
                returns: None,
            },
            ToolDefinition {
                skill_name: Some("browser".to_string()),
                name: "done".to_string(),
                description: "Call this when the task is complete. Provide a summary of what was accomplished.".to_string(),
                parameters: vec![ToolParameter {
                    name: "summary".to_string(),
                    param_type: ParameterType::String,
                    description: "A brief summary of what was accomplished.".to_string(),
                    required: true,
                    default: None,
                }],
                returns: None,
            },
        ]
    }
}
