//! Agentic runtime for handling tool-using conversations.
//!
//! This module implements the main agentic loop that:
//! 1. Loads conversation history with context limiting
//! 2. Builds system prompts with skill summaries
//! 3. Executes agentic loop: model request > response > tool execution
//! 4. Handles skill activation and tool calling
//! 5. Persists all messages to database

use crate::db::conversations::{
    add_message, get_conversation_history, get_or_refresh_prompt_time,
    load_conversation_skills, save_conversation_skill,
    MessageMetadata, MessageType, Role,
};
use crate::events::{emitter::emit, types::{AttachmentData, EXTRACT_INTERACTIVE_MEMORY, ChatStreamEvent, CHAT_STREAM, CloudUsageDecrementedEvent, CLOUD_USAGE_DECREMENTED}};
use crate::models::llm::client::generate;
use crate::models::llm::types::{LlmRequest, LlmResponse};
use crate::models::llm::usage::create_generation_session;
use crate::settings::service::load_user_settings;
use crate::skills::executor::{execute_tools};
use crate::skills::registry::{get_filtered_summaries, get_skill_tools, skill_exists};
use crate::skills::types::{
    AgentError, AgentRuntimeConfig,
    SkillSummary, ToolCall, ToolDefinition, ToolResult,
};
use crate::agents::types::{
    ToolExecutionStartedEvent, ToolExecutionCompletedEvent,
    TOOL_EXECUTION_STARTED, TOOL_EXECUTION_COMPLETED,
};
use chrono::Local;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use crate::models::llm::prompts::get_prompt;
use super::state::AgentRuntimeState;

/// Main entry point for agentic chat.
///
/// This command handles a user message in a conversation with
/// full agentic capabilities including skill activation and tool calling.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle
/// * `state` - Agent runtime state for cancellation support
/// * `conv_id` - Conversation ID
/// * `message_id` - Unique message ID for this user message
/// * `user_message` - The user's message text
/// * `attachments` - Any file attachments with the message
///
/// # Returns
///
/// The final assistant response text on success
#[tauri::command]
pub async fn handle_agent_chat(
    app_handle: AppHandle,
    state: tauri::State<'_, AgentRuntimeState>,
    conv_id: String,
    assistant_message_id: String,
    message_id: String,
    user_message: String,
    attachments: Vec<AttachmentData>,
) -> Result<String, AgentError> {
    log::info!(
        "[agent] Starting agentic chat for conversation {}",
        conv_id
    );

    // Mark generation as started and get the cancellation signal
    state.start_generation(&conv_id).await
        .map_err(|e| AgentError::RuntimeError(e))?;

    // Create runtime and run
    let cancel_signal = state.get_stop_signal();
    let cancel_notify = state.get_cancel_notify();
    let runtime = match AgentRuntime::new(app_handle.clone(), conv_id, assistant_message_id, message_id, cancel_signal, cancel_notify).await {
        Ok(r) => r,
        Err(e) => {
            // Clean up state on runtime creation failure
            state.finish_generation().await;
            return Err(e);
        }
    };

    // Spawn the runtime in an isolated task so that:
    // 1. Panics don't propagate and leave is_running stuck
    // 2. finish_generation() ALWAYS runs regardless of outcome
    let task_result = tokio::spawn(async move {
        runtime.run(user_message, attachments).await
    })
    .await;

    // ALWAYS mark generation as finished
    state.finish_generation().await;

    match task_result {
        Ok(result) => result,
        Err(join_err) => {
            log::error!("[agent] Chat runtime task panicked: {}", join_err);
            Err(AgentError::RuntimeError(format!("Chat session failed unexpectedly: {}", join_err)))
        }
    }
}

/// Agentic runtime managing the tool-using conversation loop.
pub struct AgentRuntime {
    /// Tauri app handle for database and event access.
    app_handle: AppHandle,

    /// Conversation ID being processed.
    conv_id: String,

    /// Message ID of the current user message.
    message_id: String,

    /// Assistant message ID for the response.
    assistant_message_id: String,

    /// Runtime configuration.
    config: AgentRuntimeConfig,

    /// Whether using local model (vs cloud).
    is_local: bool,

    /// The selected model key (e.g. "gemini-3-flash").
    model_key: String,

    /// Session token for cloud rate limiting (one per turn).
    session_token: Option<String>,

    /// Currently activated skills for this conversation.
    active_skills: Vec<String>,

    /// Current iteration count (for safety).
    iteration: usize,

    /// Cancellation signal from the runtime state.
    cancel_signal: Arc<AtomicBool>,

    /// Async cancel notification for instant I/O cancellation.
    cancel_notify: Arc<tokio::sync::Notify>,
}

impl AgentRuntime {
    /// Creates a new agentic runtime instance.
    ///
    /// Loads settings to determine model type and loads
    /// previously activated skills for the conversation.
    async fn new(
        app_handle: AppHandle,
        conv_id: String,
        assistant_message_id: String,
        message_id: String,
        cancel_signal: Arc<AtomicBool>,
        cancel_notify: Arc<tokio::sync::Notify>,
    ) -> Result<Self, AgentError> {
        // Load settings to determine model type
        let settings = load_user_settings(app_handle.clone())
            .await
            .map_err(|e| AgentError::DatabaseError(format!("Failed to load settings: {}", e)))?;

        let model_key = settings.model_selection.as_str();
        let is_local = match crate::db::models::get_model_by_key(&app_handle, model_key) {
            Ok(entry) => !entry.is_cloud,
            Err(e) => {
                log::warn!("[agent] Could not look up model '{}': {}. Defaulting to local.", model_key, e);
                true
            }
        };

        // Load runtime config (use defaults for now, could be from settings in future)
        let config = AgentRuntimeConfig::default();

        // Load previously activated skills for this conversation
        let active_skills = load_conversation_skills(&app_handle, &conv_id)
            .await
            .unwrap_or_default();

        log::info!(
            "[agent] Runtime created: active_skills={:?}",
            active_skills
        );

        Ok(Self {
            app_handle,
            conv_id,
            message_id,
            assistant_message_id,
            config,
            is_local,
            model_key: model_key.to_string(),
            session_token: None,
            active_skills,
            iteration: 0,
            cancel_signal,
            cancel_notify,
        })
    }

    /// Runs the agentic loop until a final response is received.
    ///
    /// This is the main execution method that:
    /// 1. Saves the user message
    /// 2. Emits memory save event
    /// 3. Creates a generation session (for cloud models — one per turn)
    /// 4. Builds the system prompt with skill summaries
    /// 5. Gets conversation history (context-limited)
    /// 6. Enters the agentic loop
    async fn run(
        mut self,
        user_message: String,
        attachments: Vec<AttachmentData>,
    ) -> Result<String, AgentError> {
        // Save user message to database
        self.save_user_message(&user_message, &attachments).await?;

        // Emit memory save event
        self.emit_memory_save_event(&user_message).await?;

        // For cloud models, create a generation session before entering the loop.
        // This checks the rate limit and increments usage ONCE for the entire turn.
        // All subsequent LLM calls in the loop use the session token.
        if !self.is_local {
            match create_generation_session(&self.model_key).await {
                Ok(session) => {
                    self.session_token = Some(session.session_token);
                    log::info!("[agent] Generation session created for model '{}'", self.model_key);

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
                        log::info!("[agent] Rate limit reached for model '{}'", self.model_key);
                        return Err(AgentError::LlmError(e));
                    }
                    if e.contains("model_not_available") {
                        log::info!("[agent] Model '{}' not available on user's tier", self.model_key);
                        return Err(AgentError::LlmError(e));
                    }
                    // For other errors, log and continue without session
                    // (the worker will fall back to per-call rate limiting)
                    log::warn!("[agent] Failed to create generation session: {}. Continuing without session.", e);
                }
            }
        }

        // Get skill summaries for system prompt, filtered by auth requirements and disabled skills
        // get_filtered_summaries loads settings and auth state internally
        let skill_summaries = get_filtered_summaries(&self.app_handle).await;

        // Filter active skills to only keep those that are still available
        // (removes skills that were disabled or lost auth since activation)
        let available_skill_names: Vec<String> = skill_summaries.iter().map(|s| s.name.clone()).collect();
        self.active_skills.retain(|s| available_skill_names.contains(s));

        // Build system prompt (includes cached date, user info, timezone, skills)
        let system_prompt = self.build_system_prompt(&skill_summaries).await;

        // Main agentic loop
        loop {
            // Check for cancellation at the start of each iteration
            if self.cancel_signal.load(Ordering::SeqCst) {
                log::info!("[agent] Generation cancelled by user at iteration start");
                let text = "*Request cancelled by you*".to_string();

                // Emit event so UI updates immediately
                let stream_data = ChatStreamEvent {
                    delta: "".to_string(),
                    is_finished: true,
                    full_response: text.clone(),
                    conv_id: Some(self.conv_id.clone()),
                    message_id: Some(self.assistant_message_id.clone()),
                };
                let _ = emit(CHAT_STREAM, stream_data);

                self.save_assistant_message(&text, MessageType::Text, None).await?;
                return Ok(text);
            }

            self.iteration += 1;
            if self.iteration > self.config.max_iterations {
                return Err(AgentError::MaxIterationsExceeded(self.config.max_iterations));
            }

            log::info!("[agent] Iteration {}/{}", self.iteration, self.config.max_iterations);

            // Get context-limited conversation history
            let messages = get_conversation_history(
                &self.app_handle,
                &self.conv_id,
                self.config.context_limit_for(self.is_local),
            )
            .await?;

            // Determine what tools to include in request
            let available_tools = self.get_available_tools();

            // Build LLM request with cancel signal
            let timeout_duration = if self.is_local { 30 } else { 10 };

            let request = LlmRequest::new(String::new())
                .with_system_prompt(Some(system_prompt.clone()))
                .with_messages(Some(messages))
                .with_internal_tools(Some(available_tools))
                .with_conv_id(Some(self.conv_id.clone()))
                .with_stream(Some(true))
                .with_assistant_message_id(Some(self.assistant_message_id.clone()))
                .with_cancel_signal(Some(self.cancel_signal.clone()))
                .with_cancel_notify(Some(self.cancel_notify.clone()))
                .with_attempts(Some(3))
                .with_timeout_duration(Some(timeout_duration))
                .with_slot_id(Some(0))
                .with_session_token(self.session_token.clone());

            // Generate response from LLM
            let response = match generate(
                self.app_handle.clone(),
                request,
                Some(self.is_local),
            )
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    // Check if it's a cancellation/timeout error
                    if e.contains("cancelled") || e.contains("timed out") {
                        let text: String;
                        if e.contains("timed out") {
                            text = "*Request timed out*".to_string();
                            log::info!("[agent] Generation timed out");
                        } else {
                            text = "*Request cancelled by you*".to_string();
                            log::info!("[agent] Generation was cancelled during LLM call");
                        }
                        self.save_assistant_message_with_id(&self.assistant_message_id, &text, MessageType::Text, None).await?;
                        let stream_data = ChatStreamEvent {
                            delta: "".to_string(),
                            is_finished: true,
                            full_response: text.clone(),
                            conv_id: Some(self.conv_id.clone()),
                            message_id: Some(self.assistant_message_id.clone()),
                        };
                        let _ = emit(CHAT_STREAM, stream_data);
                        return Ok(text);
                    }

                    // Rate limit / model unavailable — surface a user-friendly message
                    if e.contains("rate_limit_exceeded") {
                        let text = "*You've reached your daily usage limit for this model. Please try again tomorrow or switch to a different model.*".to_string();
                        self.save_assistant_message_with_id(&self.assistant_message_id, &text, MessageType::Text, None).await?;
                        let stream_data = ChatStreamEvent {
                            delta: "".to_string(),
                            is_finished: true,
                            full_response: text.clone(),
                            conv_id: Some(self.conv_id.clone()),
                            message_id: Some(self.assistant_message_id.clone()),
                        };
                        let _ = emit(CHAT_STREAM, stream_data);
                        return Ok(text);
                    }

                    if e.contains("model_not_available") {
                        let text = "*This model is not available on your current plan. Please upgrade or switch to a different model.*".to_string();
                        self.save_assistant_message_with_id(&self.assistant_message_id, &text, MessageType::Text, None).await?;
                        let stream_data = ChatStreamEvent {
                            delta: "".to_string(),
                            is_finished: true,
                            full_response: text.clone(),
                            conv_id: Some(self.conv_id.clone()),
                            message_id: Some(self.assistant_message_id.clone()),
                        };
                        let _ = emit(CHAT_STREAM, stream_data);
                        return Ok(text);
                    }

                    return Err(AgentError::LlmError(e));
                }
            };

            // Handle response
            match response {
                LlmResponse::Text(text) => {
                    // Final response - save and return
                    log::info!("[agent] Final response received, saving and returning");
                    self.save_assistant_message_with_id(&self.assistant_message_id, &text, MessageType::Text, None).await?;
                    return Ok(text);
                }

                LlmResponse::ToolCalls { text, calls: tool_calls } => {
                    // Model wants to execute tools
                    // Check if we have too many tool calls
                    if tool_calls.len() > self.config.max_tool_calls_per_turn {
                        return Err(AgentError::TooManyToolCalls(
                            tool_calls.len(),
                            self.config.max_tool_calls_per_turn,
                        ));
                    }

                    // Save tool calls as a message
                    let mut tool_call_metadatas = Vec::with_capacity(tool_calls.len());
                    for call in &tool_calls {
                        let metadata = MessageMetadata::ToolCall {
                            call_id: call.id.clone(),
                            skill_name: call.skill_name.clone(),
                            tool_name: call.tool_name.clone(),
                            arguments: call.arguments.clone(),
                            thought_signature: call.thought_signature.clone(),
                        };
                        tool_call_metadatas.push(metadata.clone());
                    }
                    let content = text.unwrap_or_default();
                    let tool_call_msg_id = self.save_assistant_message(&content, MessageType::ToolCalls, Some(tool_call_metadatas)).await?;

                    // Execute tools in parallel
                    let results = self.execute_tool_calls(tool_calls.clone(), &content, tool_call_msg_id).await?;

                    // Add results to context and continue
                    let mut tool_result_metadatas = Vec::with_capacity(results.len());
                    for result in &results {
                        let metadata = MessageMetadata::ToolResult {
                            call_id: result.call_id.clone(),
                            success: result.success,
                            error: result.error.clone(),
                            result: result.result.clone(),
                            screenshot_attachment_id: None,  // Chat runtime doesn't capture screenshots
                        };
                        tool_result_metadatas.push(metadata.clone());
                    }

                    // Save message and emit tool completion events
                    let tool_result_msg_id = self.save_tool_result_message(&content, tool_result_metadatas).await?;
                    
                    for result in &results {
                        let completed_event = ToolExecutionCompletedEvent {
                            tool_call_id: result.call_id.clone(),
                            message_id: tool_result_msg_id.clone(),
                            skill_name: tool_calls
                                .iter()
                                .find(|c| c.id == result.call_id)
                                .map(|c| c.skill_name.clone())
                                .unwrap_or_default(),
                            tool_name: tool_calls
                                .iter()
                                .find(|c| c.id == result.call_id)
                                .map(|c| c.tool_name.clone())
                                .unwrap_or_default(),
                            success: result.success,
                            result: result.result.clone(),
                            error: result.error.clone(),
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        };
                        let _ = emit(TOOL_EXECUTION_COMPLETED, completed_event);
                    }

                    continue;
                }
            }
        }
    }

    /// Builds the system prompt with skill information and dynamic context.
    ///
    /// Dynamic context (date, user name, timezone, OS) is included in the system
    /// prompt itself. The date uses a "sticky" cache: `prompt_cached_at` in the DB
    /// only refreshes when >10 minutes have passed or a new day starts. This keeps
    /// the system prompt identical across messages within the window, enabling
    /// llama.cpp's KV cache to reuse the prompt prefix.
    async fn build_system_prompt(&self, skill_summaries: &[SkillSummary]) -> String {
        let skills_section = self.format_skill_summaries(skill_summaries);
        let context = self.build_context().await;
        let agentic_prompt = get_prompt("agentic_chat").unwrap_or_default();
        
        agentic_prompt
            .replace("{context}", &context)
            .replace("{skills_section}", &skills_section)
    }

    /// Builds the dynamic context string for the system prompt.
    ///
    /// Contains: cached date/time, user's full name, IANA timezone, OS.
    /// The date is read from `prompt_cached_at` in the conversations table
    /// and only refreshes on 10-minute or day-boundary changes.
    async fn build_context(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        // Date/time — uses sticky cache from DB
        match get_or_refresh_prompt_time(&self.app_handle, &self.conv_id).await {
            Ok(time_str) => parts.push(format!("Today is {}.", time_str)),
            Err(e) => {
                log::warn!("[agent] Failed to get prompt time: {}", e);
                // Fallback to current time
                let fallback = Local::now()
                    .format("%A, %B %e, %Y at %l:%M %p")
                    .to_string()
                    .replace("  ", " ");
                parts.push(format!("Today is {}.", fallback));
            }
        }

        // User's full name from auth state
        if let Ok(Some(auth_state)) = crate::auth::storage::retrieve_auth_state() {
            if let Some(ref meta) = auth_state.session.user.user_metadata {
                if let Some(ref name) = meta.full_name {
                    if !name.is_empty() {
                        parts.push(format!("The user's name is {}.", name));
                    }
                }
            }
        }

        // Timezone from IANA
        if let Ok(tz) = iana_time_zone::get_timezone() {
            parts.push(format!("Timezone: {}.", tz));
        }

        // Operating system
        parts.push(format!("OS: {}.", std::env::consts::OS));

        parts.join(" ")
    }

    /// Formats skill summaries for the system prompt.
    fn format_skill_summaries(&self, summaries: &[SkillSummary]) -> String {
        if summaries.is_empty() {
            return String::new();
        }

        let mut section = String::from("## Available Skills\n");
        section.push_str("You can activate these skills to gain new capabilities:\n\n");

        for summary in summaries {
            let status = if self.active_skills.contains(&summary.name) {
                " [ACTIVE]"
            } else {
                ""
            };
            section.push_str(&format!(
                "- **{}**{}: {}\n",
                summary.name, status, summary.description
            ));
        }

        section
    }

    /// Gets available tools for the current request.
    ///
    /// Always includes `activate_skill` tool plus tools from active skills.
    fn get_available_tools(&self) -> Vec<ToolDefinition> {
        let mut tools = Vec::new();

        // Always include skill activation tool
        tools.push(self.get_activate_skill_tool());

        // Add tools from active skills
        for skill_name in &self.active_skills {
            let mut skill_tools = get_skill_tools(skill_name);
            // Set skill name on each tool for mapping back from model responses
            for tool in &mut skill_tools {
                tool.skill_name = Some(skill_name.clone());
            }
            tools.extend(skill_tools);
        }

        tools
    }

    /// Gets the skill activation tool definition.
    fn get_activate_skill_tool(&self) -> ToolDefinition {
        use crate::skills::types::{ToolDefinition, ToolParameter, ParameterType};

        ToolDefinition {
            skill_name: Some("system".to_string()),
            name: "activate_skill".to_string(),
            description: "Activate a skill to gain access to its tools. Use this when you need capabilities not currently available.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "skill_name".to_string(),
                    param_type: ParameterType::String,
                    description: "The name of the skill to activate".to_string(),
                    required: true,
                    default: None,
                },
                ToolParameter {
                    name: "reason".to_string(),
                    param_type: ParameterType::String,
                    description: "Brief explanation of why this skill is needed".to_string(),
                    required: true,
                    default: None,
                },
            ],
            returns: None,
        }
    }

    /// Handles a skill activation request.
    ///
    /// Verifies skill exists, adds to active skills list, persists
    /// to database, and emits activation event.
    async fn activate_skill_internal(
        &mut self,
        skill_name: &str,
    ) -> Result<(), AgentError> {
        log::info!("[agent] Activating skill '{}'", skill_name);

        // Verify skill exists
        if !skill_exists(skill_name) {
            return Err(AgentError::SkillNotFound(skill_name.to_string()));
        }

        // Add to active skills if not already active
        if !self.active_skills.contains(&skill_name.to_string()) {
            self.active_skills.push(skill_name.to_string());

            // Persist to database
            save_conversation_skill(&self.app_handle, &self.conv_id, skill_name)
                .await?;
        }

        Ok(())
    }

    /// Executes a set of tool calls.
    ///
    /// Saves tool call records, executes them in parallel, and updates
    /// records with results.
    async fn execute_tool_calls(
        &mut self,
        tool_calls: Vec<ToolCall>,
        content: &str,
        message_id: String,
    ) -> Result<Vec<ToolResult>, AgentError> {
        log::info!("[agent] Executing {} tool calls", tool_calls.len());

        // Emit tool started events
        for call in tool_calls.iter() {
            let started_event = ToolExecutionStartedEvent {
                tool_call_id: call.id.clone(),
                message_id: message_id.clone(),
                skill_name: call.skill_name.clone(),
                tool_name: call.tool_name.clone(),
                content: content.to_string(),
                arguments: call.arguments.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let _ = emit(TOOL_EXECUTION_STARTED, started_event);
        }

        // Execute tools in parallel
        let results = execute_tools(&self.app_handle, tool_calls.clone()).await;

        // Check for skill activation calls and update state
        for call in &tool_calls {
            if call.skill_name == "system" && call.tool_name == "activate_skill" {
                if let Some(skill_name) = call.arguments.get("skill_name").and_then(|v| v.as_str()) {
                    self.activate_skill_internal(skill_name).await?;
                }
            }
        }

        Ok(results)
    }

    /// Emits an event to save the user message to interactive memory.
    async fn emit_memory_save_event(&self, user_message: &str) -> Result<(), AgentError> {
        use crate::events::types::ExtractInteractiveMemoryEvent;

        // Emit extract memory event
        let extract_event = ExtractInteractiveMemoryEvent {
            message: user_message.to_string(),
            message_id: self.message_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        let _ = emit(EXTRACT_INTERACTIVE_MEMORY, extract_event);

        Ok(())
    }

    /// Saves a user message to the database.
    async fn save_user_message(
        &self,
        content: &str,
        attachments: &[AttachmentData],
    ) -> Result<(), AgentError> {
        use crate::db::conversations::{create_attachments, add_attachments, get_message};

        // Check if message already exists
        if let Ok(_) = get_message(self.app_handle.clone(), self.message_id.clone()).await {
            log::info!("[agent] User message {} already exists, skipping save", self.message_id);
            return Ok(());
        }

        // Save message
        add_message(
            &self.app_handle,
            self.conv_id.clone(),
            Role::User,
            content.to_string(),
            Some(MessageType::Text),
            None,
            Some(self.message_id.clone()),
        )
        .await?;

        // Handle attachments
        if !attachments.is_empty() {
            let atts = create_attachments(
                &self.app_handle,
                self.message_id.clone(),
                attachments.to_vec(),
            )
            .await
            .map_err(|e| AgentError::DatabaseError(format!("Failed to create attachments: {}", e)))?;

            add_attachments(&self.app_handle, atts)
                .await
                .map_err(|e| AgentError::DatabaseError(format!("Failed to add attachments: {}", e)))?;
        }

        Ok(())
    }

    /// Saves an assistant message to the database with a specific ID.
    async fn save_assistant_message_with_id(
        &self,
        id: &str,
        content: &str,
        message_type: MessageType,
        metadata: Option<Vec<MessageMetadata>>,
    ) -> Result<String, AgentError> {
        let message = add_message(
            &self.app_handle,
            self.conv_id.clone(),
            Role::Assistant,
            content.to_string(),
            Some(message_type),
            metadata,
            Some(id.to_string()),
        )
        .await?;

        Ok(message.id)
    }

    /// Saves an assistant message to the database.
    async fn save_assistant_message(
        &self,
        content: &str,
        message_type: MessageType,
        metadata: Option<Vec<MessageMetadata>>,
    ) -> Result<String, AgentError> {
        self.save_assistant_message_with_id(
            &uuid::Uuid::new_v4().to_string(),
            content,
            message_type,
            metadata,
        )
        .await
    }

    /// Saves a tool result message to the database.
    async fn save_tool_result_message(
        &self,
        content: &str,
        metadata: Vec<MessageMetadata>,
    ) -> Result<String, AgentError> {
        let message = add_message(
            &self.app_handle,
            self.conv_id.clone(),
            Role::Tool,
            content.to_string(),
            Some(MessageType::ToolResults),
            Some(metadata),
            None,
        )
        .await?;

        Ok(message.id)
    }
}
