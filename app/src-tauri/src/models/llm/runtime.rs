//! Agentic runtime for handling tool-using conversations.
//!
//! This module implements the main agentic loop that:
//! 1. Loads conversation history with context limiting
//! 2. Builds system prompts with skill summaries
//! 3. Executes agentic loop: model request → response → tool execution
//! 4. Handles skill activation and tool calling
//! 5. Persists all messages to database
//!
//! # Flow
//!
//! ```
//! User Message
//!     ↓
//! [Check context limit] → [Build system prompt with skill summaries]
//!     ↓
//! Model Generation (Phase 1 - summaries only)
//!     ↓
//! Response: Text | Skill Activation | Tool Calls
//!     ↓
//! Skill Activation? → [Load skill tools] → Continue
//! Tool Calls? → [Execute tools] → [Add results] → Continue
//! Text? → [Save and return]
//! ```
//!
//! The loop continues until:
//! - Model returns plain text (final answer)
//! - Maximum iterations exceeded
//! - Error occurs

use crate::db::conversations::{
    add_message, get_conversation_history, load_conversation_skills,
    save_conversation_skill, MessageMetadata, MessageType, Role,
};
use crate::events::{emitter::emit, types::AttachmentData};
use crate::models::llm::client::generate;
use crate::models::llm::types::{LlmRequest, LlmResponse};
use crate::settings::service::load_user_settings;
use crate::settings::types::ModelSelection;
use crate::skills::executor::{execute_tools, save_tool_call_record, update_tool_call_result};
use crate::skills::registry::{get_all_summaries, get_skill, get_skill_tools, skill_exists};
use crate::skills::types::{
    AgentError, AgentRuntimeConfig,
    SkillActivationRequest, SkillSummary, ToolCall, ToolDefinition, ToolResult,
};
use chrono::Local;
use tauri::AppHandle;
use uuid::Uuid;
use ts_rs::TS;

// ============================================================================
// Event Types for Agentic Runtime
// ============================================================================

/// Event emitted when a skill is activated.
pub const SKILL_ACTIVATED: &str = "skill_activated";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "events.ts")]
pub struct SkillActivatedEvent {
    pub skill_name: String,
    pub conversation_id: String,
    pub timestamp: String,
}

/// Event emitted when a tool execution starts.
pub const TOOL_EXECUTION_STARTED: &str = "tool_execution_started";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "events.ts")]
pub struct ToolExecutionStartedEvent {
    pub tool_call_id: String,
    pub skill_name: String,
    pub tool_name: String,
    pub timestamp: String,
}

/// Event emitted when a tool execution completes.
pub const TOOL_EXECUTION_COMPLETED: &str = "tool_execution_completed";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, TS)]
#[ts(export, export_to = "events.ts")]
pub struct ToolExecutionCompletedEvent {
    pub tool_call_id: String,
    pub skill_name: String,
    pub tool_name: String,
    pub success: bool,
    pub timestamp: String,
}

// ============================================================================
// Agentic Runtime
// ============================================================================

/// Main entry point for agentic chat.
///
/// This command handles a user message in a conversation with
/// full agentic capabilities including skill activation and tool calling.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle
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
    conv_id: String,
    message_id: String,
    user_message: String,
    attachments: Vec<AttachmentData>,
) -> Result<String, AgentError> {
    log::info!(
        "[agent] Starting agentic chat for conversation {}",
        conv_id
    );

    // Create runtime and run
    let runtime = AgentRuntime::new(app_handle.clone(), conv_id, message_id).await?;
    runtime.run(user_message, attachments).await
}

/// Agentic runtime managing the tool-using conversation loop.
pub struct AgentRuntime {
    /// Tauri app handle for database and event access.
    app_handle: AppHandle,

    /// Conversation ID being processed.
    conv_id: String,

    /// Message ID of the current user message.
    message_id: String,

    /// Runtime configuration.
    config: AgentRuntimeConfig,

    /// Whether using local model (vs cloud).
    is_local: bool,

    /// Currently activated skills for this conversation.
    active_skills: Vec<String>,

    /// Current iteration count (for safety).
    iteration: usize,
}

impl AgentRuntime {
    /// Creates a new agentic runtime instance.
    ///
    /// Loads settings to determine model type and loads
    /// previously activated skills for the conversation.
    async fn new(
        app_handle: AppHandle,
        conv_id: String,
        message_id: String,
    ) -> Result<Self, AgentError> {
        // Load settings to determine model type
        let settings = load_user_settings(app_handle.clone())
            .await
            .map_err(|e| AgentError::DatabaseError(format!("Failed to load settings: {}", e)))?;

        let is_local = matches!(
            settings.model_selection,
            ModelSelection::Local
        );

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
            config,
            is_local,
            active_skills,
            iteration: 0,
        })
    }

    /// Runs the agentic loop until a final response is received.
    ///
    /// This is the main execution method that:
    /// 1. Saves the user message
    /// 2. Builds the system prompt with skill summaries
    /// 3. Gets conversation history (context-limited)
    /// 4. Enters the agentic loop
    async fn run(
        mut self,
        user_message: String,
        attachments: Vec<AttachmentData>,
    ) -> Result<String, AgentError> {
        // Save user message to database
        self.save_user_message(&user_message, &attachments).await?;

        // Get skill summaries for system prompt
        let skill_summaries = get_all_summaries();

        // Build system prompt
        let system_prompt = self.build_system_prompt(&skill_summaries);

        // Main agentic loop
        loop {
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

            // Build LLM request
            let request = LlmRequest::new(String::new())
                .with_system_prompt(Some(system_prompt.clone()))
                .with_messages(Some(messages.clone()))
                .with_internal_tools(Some(available_tools))
                .with_current_message_id(Some(self.message_id.clone()))
                .with_conv_id(Some(self.conv_id.clone()))
                .with_stream(Some(true));

            // Generate response from LLM
            let response = generate(
                self.app_handle.clone(),
                request,
                Some(self.is_local),
            )
            .await
            .map_err(|e| AgentError::LlmError(e))?;

            log::info!("[agent] Received response from model: {:?}", response);

            // Handle response
            match response {
                LlmResponse::Text(text) => {
                    // Final response - save and return
                    log::info!("[agent] Final response received, saving and returning");
                    self.save_assistant_message(&text, MessageType::Text, None).await?;
                    return Ok(text);
                }

                LlmResponse::SkillActivation(activation) => {
                    // Model wants to activate a skill
                    log::info!("[agent] Skill activation requested: {:?}", activation);
                    self.handle_skill_activation(activation).await?;

                    // Continue loop with skill now active
                    continue;
                }

                LlmResponse::ToolCalls(tool_calls) => {
                    // Model wants to execute tools
                    log::info!("[agent] Tool calls requested: {:?}", tool_calls);
                    // Check if we have too many tool calls
                    if tool_calls.len() > self.config.max_tool_calls_per_turn {
                        return Err(AgentError::TooManyToolCalls(
                            tool_calls.len(),
                            self.config.max_tool_calls_per_turn,
                        ));
                    }

                    // Save tool calls as messages
                    for call in &tool_calls {
                        let metadata = MessageMetadata::ToolCall {
                            call_id: call.id.clone(),
                            skill_name: call.skill_name.clone(),
                            tool_name: call.tool_name.clone(),
                            arguments: call.arguments.clone(),
                        };

                        let content = format!(
                            "Calling {}.{} with: {}",
                            call.skill_name,
                            call.tool_name,
                            serde_json::to_string_pretty(&call.arguments).unwrap_or_default()
                        );

                        self.save_assistant_message(&content, MessageType::ToolCall, Some(metadata))
                            .await?;
                    }

                    // Execute tools in parallel
                    let results = self.execute_tool_calls(tool_calls).await?;

                    // Add results to context and continue
                    for result in &results {
                        let metadata = MessageMetadata::ToolResult {
                            call_id: result.call_id.clone(),
                            success: result.success,
                            error: result.error.clone(),
                            result: result.result.clone(),
                        };

                        let content = if result.success {
                            format!(
                                "Tool result: {}",
                                result.result
                                    .as_ref()
                                    .map(|r| serde_json::to_string_pretty(r).unwrap_or_default())
                                    .unwrap_or_else(|| "Success".to_string())
                            )
                        } else {
                            format!(
                                "Tool error: {}",
                                result.error.as_deref().unwrap_or("Unknown error")
                            )
                        };

                        self.save_tool_result_message(&content, metadata).await?;
                    }

                    continue;
                }
            }
        }
    }

    /// Builds the system prompt with skill information.
    ///
    /// Includes skill summaries and active skill instructions.
    fn build_system_prompt(&self, skill_summaries: &[SkillSummary]) -> String {
        let skills_section = self.format_skill_summaries(skill_summaries);

        let base_prompt = format!(
            r#"You are Ambient, a helpful AI assistant. Today is {date}.

{skills_section}

## Skill Activation
When you need capabilities from a skill:
1. Call the `activate_skill` function with the skill name
2. After activation, the skill's tools will become available
3. Use the tools to complete the user's request

## Guidelines
- Only activate skills when necessary for the task
- Use available tools efficiently
- Provide clear, helpful responses
- Cite sources when using web search"#,
            date = Local::now().format("%Y-%m-%d %H:%M:%S"),
            skills_section = skills_section,
        );

        // Add active skill instructions
        let mut prompt = base_prompt;
        for skill_name in &self.active_skills {
            if let Some(skill) = get_skill(skill_name) {
                prompt.push_str(&format!(
                    "\n\n## Active Skill: {}\n{}",
                    skill.name,
                    skill.instructions
                ));
            }
        }

        prompt
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
            let skill_tools = get_skill_tools(skill_name);
            tools.extend(skill_tools);
        }

        tools
    }

    /// Gets the skill activation tool definition.
    fn get_activate_skill_tool(&self) -> ToolDefinition {
        use crate::skills::types::{ToolDefinition, ToolParameter, ParameterType};

        ToolDefinition {
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
    /// to database, and saves an activation message.
    async fn handle_skill_activation(
        &mut self,
        activation: SkillActivationRequest,
    ) -> Result<(), AgentError> {
        log::info!(
            "[agent] Activating skill '{}': {}",
            activation.skill_name,
            activation.reason
        );

        // Verify skill exists
        if !skill_exists(&activation.skill_name) {
            return Err(AgentError::SkillNotFound(activation.skill_name.clone()));
        }

        // Add to active skills if not already active
        if !self.active_skills.contains(&activation.skill_name) {
            self.active_skills.push(activation.skill_name.clone());

            // Persist to database
            save_conversation_skill(&self.app_handle, &self.conv_id, &activation.skill_name)
                .await?;

            // Emit skill activated event
            let event = SkillActivatedEvent {
                skill_name: activation.skill_name.clone(),
                conversation_id: self.conv_id.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let _ = emit(SKILL_ACTIVATED, event);
        }

        // Save activation as a thinking message
        let metadata = MessageMetadata::SkillActivation {
            skill_name: activation.skill_name.clone(),
            reason: activation.reason.clone(),
        };

        let content = format!(
            "Activating {} skill: {}",
            activation.skill_name, activation.reason
        );

        self.save_assistant_message(&content, MessageType::SkillActivation, Some(metadata))
            .await?;

        Ok(())
    }

    /// Executes a set of tool calls.
    ///
    /// Saves tool call records, executes them in parallel, and updates
    /// records with results.
    async fn execute_tool_calls(
        &self,
        tool_calls: Vec<ToolCall>,
    ) -> Result<Vec<ToolResult>, AgentError> {
        log::info!("[agent] Executing {} tool calls", tool_calls.len());

        // Save tool call records
        let message_id = Uuid::new_v4().to_string();
        for call in &tool_calls {
            save_tool_call_record(&self.app_handle, &message_id, &self.conv_id, call).await?;

            // Emit tool execution started event
            let started_event = ToolExecutionStartedEvent {
                tool_call_id: call.id.clone(),
                skill_name: call.skill_name.clone(),
                tool_name: call.tool_name.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let _ = emit(TOOL_EXECUTION_STARTED, started_event);
        }

        // Execute tools in parallel
        let results = execute_tools(&self.app_handle, tool_calls.clone()).await;

        // Update records with results and emit completion events
        for result in &results {
            update_tool_call_result(&self.app_handle, &result.call_id, result).await?;

            let completed_event = ToolExecutionCompletedEvent {
                tool_call_id: result.call_id.clone(),
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
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            let _ = emit(TOOL_EXECUTION_COMPLETED, completed_event);
        }

        Ok(results)
    }

    /// Saves a user message to the database.
    async fn save_user_message(
        &self,
        content: &str,
        attachments: &[AttachmentData],
    ) -> Result<(), AgentError> {
        use crate::db::conversations::create_attachments;
        use crate::db::conversations::add_attachments;

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

            add_attachments(&self.app_handle, self.message_id.clone(), atts)
                .await
                .map_err(|e| AgentError::DatabaseError(format!("Failed to add attachments: {}", e)))?;
        }

        Ok(())
    }

    /// Saves an assistant message to the database.
    async fn save_assistant_message(
        &self,
        content: &str,
        message_type: MessageType,
        metadata: Option<MessageMetadata>,
    ) -> Result<(), AgentError> {
        let _ = add_message(
            &self.app_handle,
            self.conv_id.clone(),
            Role::Assistant,
            content.to_string(),
            Some(message_type),
            metadata,
            None,
        )
        .await?;

        Ok(())
    }

    /// Saves a tool result message to the database.
    async fn save_tool_result_message(
        &self,
        content: &str,
        metadata: MessageMetadata,
    ) -> Result<(), AgentError> {
        let _ = add_message(
            &self.app_handle,
            self.conv_id.clone(),
            Role::Tool,
            content.to_string(),
            Some(MessageType::ToolResult),
            Some(metadata),
            None,
        )
        .await?;

        Ok(())
    }
}
