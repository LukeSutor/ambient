//! Browser-use agentic runtime.
//!
//! Implements the main loop for browser agent sessions:
//! 1. Creates a persistent WebView
//! 2. LLM decides on an action (navigate, click, type, etc.)
//! 3. Executes the action and takes a DOM snapshot
//! 4. Returns the snapshot as the tool result
//! 5. Loops until `done` is called or max iterations reached
//!
//! Browser state is attached to tool results (not sent as separate messages).
//! Only the most recent tool result includes the full browser state — older
//! states are stripped before each LLM call to keep context lean.

use crate::agents::types::{
    ToolExecutionCompletedEvent, ToolExecutionStartedEvent,
    TOOL_EXECUTION_COMPLETED, TOOL_EXECUTION_STARTED,
};
use crate::db::conversations::{
    add_message, get_conversation_history, get_message, get_or_refresh_prompt_time,
    Message, MessageMetadata, MessageType, Role,
};
use crate::events::{emitter::emit, types::{ChatStreamEvent, CHAT_STREAM, CloudUsageDecrementedEvent, CLOUD_USAGE_DECREMENTED}};
use crate::models::llm::client::generate;
use crate::models::llm::prompts::get_prompt;
use crate::models::llm::types::{LlmRequest, LlmResponse};
use crate::models::llm::usage::create_generation_session;
use crate::settings::service::load_user_settings;
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
    model_key: String,
    is_local: bool,
    session_token: Option<String>,
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

        let model_selection = settings.model_selection.as_str();
        let model_id: i64 = model_selection.parse().unwrap_or(1);
        let (is_local, model_key) = match crate::db::models::get_model_by_id(&app_handle, model_id) {
            Ok(entry) => (!entry.is_cloud, entry.model),
            Err(e) => {
                log::warn!("[browser_use] Could not look up model id {}: {}. Defaulting to local.", model_id, e);
                (true, "qwen3vl-2b".to_string())
            }
        };
        let config = BrowserUseConfig::default();

        Ok(Self {
            app_handle,
            conv_id,
            assistant_message_id,
            config,
            model_key,
            is_local,
            session_token: None,
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
        // Save user message (skip if already exists — retry/resubmit scenario)
        let msg_id = message_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        if get_message(self.app_handle.clone(), msg_id.clone()).await.is_err() {
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
        }

        // Create browser WebView
        create_browser_webview(&self.app_handle, &self.config.start_url)
            .map_err(|e| AgentError::RuntimeError(format!("Failed to create browser: {}", e)))?;

        // For cloud models, create a generation session before entering the loop.
        // This checks the rate limit and increments usage ONCE for the entire turn.
        if !self.is_local {
            match create_generation_session(&self.model_key).await {
                Ok(session) => {
                    self.session_token = Some(session.session_token);
                    log::info!("[browser_use] Generation session created for model '{}'", self.model_key);

                    // Emit usage decrement event so the frontend updates counters
                    let _ = emit(
                        CLOUD_USAGE_DECREMENTED,
                        CloudUsageDecrementedEvent {
                            model_key: self.model_key.clone(),
                            timestamp: chrono::Local::now().to_rfc3339(),
                        },
                    );
                }
                Err(e) => {
                    if e.contains("rate_limit_exceeded") {
                        log::info!("[browser_use] Rate limit reached for model '{}'", self.model_key);
                        destroy_browser_webview(&self.app_handle);
                        return Err(AgentError::LlmError(e));
                    }
                    if e.contains("model_not_available") {
                        log::info!("[browser_use] Model '{}' not available on user's tier", self.model_key);
                        destroy_browser_webview(&self.app_handle);
                        return Err(AgentError::LlmError(e));
                    }
                    log::warn!("[browser_use] Failed to create generation session: {}. Continuing without session.", e);
                }
            }
        }

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

            // Get conversation history (context-limited)
            let mut messages = get_conversation_history(
                &self.app_handle,
                &self.conv_id,
                self.config.context_limit_for(self.is_local),
            )
            .await?;

            // Strip browser state from all tool results except the most recent
            strip_old_browser_states(&mut messages);

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
                .with_slot_id(Some(0))
                .with_session_token(self.session_token.clone());

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

                    // Rate limit / model unavailable — surface user-friendly message
                    if e.contains("rate_limit_exceeded") {
                        let text = "*You've reached your daily usage limit for this model. Please try again tomorrow or switch to a different model.*".to_string();
                        self.emit_final_response(&text).await?;
                        return Ok(text);
                    }
                    if e.contains("model_not_available") {
                        let text = "*This model is not available on your current plan. Please upgrade or switch to a different model.*".to_string();
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
                    self.emit_final_response(&text).await?;
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

                    // Save tool calls message to DB
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
                    let tool_call_msg_id = uuid::Uuid::new_v4().to_string();
                    add_message(
                        &self.app_handle,
                        self.conv_id.clone(),
                        Role::Assistant,
                        content.clone(),
                        Some(MessageType::ToolCalls),
                        Some(tool_call_metadatas),
                        Some(tool_call_msg_id.clone()),
                    )
                    .await?;

                    // Emit tool_execution_started for each call
                    let timestamp = chrono::Utc::now().to_rfc3339();
                    for call in &calls_to_execute {
                        let _ = emit(
                            TOOL_EXECUTION_STARTED,
                            ToolExecutionStartedEvent {
                                tool_call_id: call.id.clone(),
                                message_id: tool_call_msg_id.clone(),
                                skill_name: call.skill_name.clone(),
                                tool_name: call.tool_name.clone(),
                                content: content.clone(),
                                arguments: call.arguments.clone(),
                                timestamp: timestamp.clone(),
                            },
                        );
                    }

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

                        let result_metadata = vec![MessageMetadata::ToolResult {
                            call_id: done.id.clone(),
                            success: true,
                            error: None,
                            result: Some(serde_json::json!({ "status": "completed", "summary": summary })),
                            screenshot_attachment_id: None,
                        }];

                        let tool_result_msg_id = uuid::Uuid::new_v4().to_string();
                        add_message(
                            &self.app_handle,
                            self.conv_id.clone(),
                            Role::Tool,
                            String::new(),
                            Some(MessageType::ToolResults),
                            Some(result_metadata),
                            Some(tool_result_msg_id.clone()),
                        )
                        .await?;

                        // Emit completion event
                        let _ = emit(
                            TOOL_EXECUTION_COMPLETED,
                            ToolExecutionCompletedEvent {
                                tool_call_id: done.id.clone(),
                                message_id: tool_result_msg_id,
                                skill_name: done.skill_name.clone(),
                                tool_name: done.tool_name.clone(),
                                success: true,
                                result: Some(serde_json::json!({ "status": "completed", "summary": summary })),
                                error: None,
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            },
                        );

                        self.emit_final_response(&summary).await?;
                        return Ok(summary);
                    }

                    // Execute actions sequentially, then take a snapshot to include in results
                    let mut action_results: Vec<(String, String, bool, Option<String>)> = Vec::new(); // (call_id, tool_name, success, result/error)

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

                        match &action_result {
                            Ok(msg) => {
                                action_results.push((call.id.clone(), call.tool_name.clone(), true, Some(msg.clone())));
                            }
                            Err(e) => {
                                log::warn!("[browser_use] Action '{}' failed: {}", call.tool_name, e);
                                action_results.push((call.id.clone(), call.tool_name.clone(), false, Some(e.clone())));
                            }
                        }
                    }

                    // Wait for action effects to settle before snapshot
                    let delay = if calls_to_execute.iter().any(|c| c.tool_name == "navigate") {
                        self.config.navigation_delay_ms
                    } else {
                        self.config.action_delay_ms
                    };
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

                    // Take DOM snapshot after actions
                    let snapshot = match extract_snapshot(
                        &self.app_handle,
                        self.config.snapshot_timeout_secs,
                    )
                    .await
                    {
                        Ok(s) => {
                            log::info!("[browser_use] Post-action snapshot ({} chars)", s.len());
                            Some(s)
                        }
                        Err(e) => {
                            log::warn!("[browser_use] Post-action snapshot failed: {}", e);
                            None
                        }
                    };

                    // Build tool result metadata with browser state attached to each result
                    let result_metadatas: Vec<MessageMetadata> = action_results
                        .iter()
                        .map(|(call_id, _tool_name, success, msg)| {
                            let result_value = if *success {
                                let mut result = serde_json::json!({
                                    "action": msg.as_deref().unwrap_or("OK"),
                                });
                                // Attach browser state to each tool result
                                if let Some(ref snap) = snapshot {
                                    result["browser_state"] = serde_json::json!(snap);
                                }
                                Some(result)
                            } else {
                                None
                            };

                            MessageMetadata::ToolResult {
                                call_id: call_id.clone(),
                                success: *success,
                                error: if *success { None } else { msg.clone() },
                                result: result_value,
                                screenshot_attachment_id: None,
                            }
                        })
                        .collect();

                    // Save tool results to DB
                    let tool_result_msg_id = uuid::Uuid::new_v4().to_string();
                    if !result_metadatas.is_empty() {
                        add_message(
                            &self.app_handle,
                            self.conv_id.clone(),
                            Role::Tool,
                            String::new(),
                            Some(MessageType::ToolResults),
                            Some(result_metadatas.clone()),
                            Some(tool_result_msg_id.clone()),
                        )
                        .await?;
                    }

                    // Emit tool_execution_completed for each result
                    let completed_timestamp = chrono::Utc::now().to_rfc3339();
                    for (call_id, tool_name, success, msg) in &action_results {
                        let skill_name = calls_to_execute
                            .iter()
                            .find(|c| c.id == *call_id)
                            .map(|c| c.skill_name.clone())
                            .unwrap_or_default();

                        // Don't include snapshot in the event payload (too large for events)
                        let event_result = if *success {
                            Some(serde_json::json!({ "action": msg.as_deref().unwrap_or("OK") }))
                        } else {
                            None
                        };

                        let _ = emit(
                            TOOL_EXECUTION_COMPLETED,
                            ToolExecutionCompletedEvent {
                                tool_call_id: call_id.clone(),
                                message_id: tool_result_msg_id.clone(),
                                skill_name,
                                tool_name: tool_name.clone(),
                                success: *success,
                                result: event_result,
                                error: if *success { None } else { msg.clone() },
                                timestamp: completed_timestamp.clone(),
                            },
                        );
                    }
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

/// Strip `browser_state` from all tool result messages except the most recent one.
///
/// This keeps the LLM context lean by only providing the current page state.
/// Historical actions are preserved (success/error), but their browser states
/// are removed since they're no longer relevant.
fn strip_old_browser_states(messages: &mut [Message]) {
    // Find the index of the last ToolResults message
    let last_tool_result_idx = messages
        .iter()
        .rposition(|m| m.message_type == MessageType::ToolResults);

    let Some(last_idx) = last_tool_result_idx else {
        return; // No tool results, nothing to strip
    };

    for (i, msg) in messages.iter_mut().enumerate() {
        if i == last_idx || msg.message_type != MessageType::ToolResults {
            continue;
        }

        // Strip browser_state from each ToolResult metadata entry
        if let Some(ref mut metadata) = msg.metadata {
            for entry in metadata.iter_mut() {
                if let MessageMetadata::ToolResult { result, .. } = entry {
                    if let Some(ref mut val) = result {
                        if val.is_object() {
                            if let Some(obj) = val.as_object_mut() {
                                obj.remove("browser_state");
                            }
                        }
                    }
                }
            }
        }

        // Also clear content if it contained snapshot data
        if !msg.content.is_empty() && msg.role == Role::Tool {
            msg.content.clear();
        }
    }
}