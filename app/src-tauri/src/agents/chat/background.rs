//! Background agent runtime for automation tasks.
//!
//! Runs an agentic loop that:
//! - Does NOT save messages to the database
//! - Does NOT emit streaming or UI events
//! - Uses the user's local timezone and session for skill filtering
//! - Resolves the model from the task's model_id (falls back to local)
//!
//! This module exists to avoid duplicating the agent loop in executor.rs.

use crate::models::llm::client::generate;
use crate::models::llm::types::{LlmRequest, LlmResponse};
use crate::models::llm::usage::create_generation_session;
use crate::skills::executor::execute_tools;
use crate::skills::registry::{canonicalize_skill_name, get_filtered_summaries, get_skill_tools, skill_exists};
use crate::skills::types::{AgentRuntimeConfig, SkillSummary, ToolDefinition};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

/// Run an automation task in background mode.
///
/// This is the core agent loop used by [`crate::automations::executor`].
/// It executes the prompt with full tool-use capabilities, but does not
/// persist any messages to the database or emit any UI events.
///
/// # Arguments
/// - `app_handle` – Tauri application handle
/// - `prompt`        – The user-facing prompt to run
/// - `model_id`      – Optional model override (references `models.id`). Falls back to local.
/// - `disabled_skills` – Skill names disabled for this automation
/// - `max_iterations`  – Safety cap for the agentic loop
/// - `timeout_secs`    – Seconds before a timeout cancel signal is set
pub async fn run_background(
    app_handle: &AppHandle,
    prompt: &str,
    model_id: Option<i64>,
    disabled_skills: &[String],
    max_iterations: usize,
    timeout_secs: u64,
) -> Result<String, String> {
    let config = AgentRuntimeConfig {
        local_context_limit: 3,
        cloud_context_limit: 10,
        max_tool_calls_per_turn: 5,
        max_iterations,
        enable_thinking: true,
    };

    // Resolve which model to use
    let (is_local, is_internal, model_key, resolved_model_id) = resolve_model(app_handle, model_id);

    // For internal cloud models, create a generation session ONCE for the entire turn.
    // BYOK models use the user's own API key — no session needed.
    let mut session_token: Option<String> = None;
    if !is_local && is_internal {
        match create_generation_session(&model_key).await {
            Ok(session) => {
                session_token = Some(session.session_token);
                log::info!(
                    "[background_agent] Session created for '{}' (cost: {} credits)",
                    model_key,
                    session.credit_cost
                );
            }
            Err(e) => {
                if e.contains("rate_limit_exceeded") || e.contains("model_not_available") {
                    return Err(e);
                }
                log::warn!("[background_agent] Failed to create session: {}. Continuing.", e);
            }
        }
    }

    // Get skill summaries filtered by auth state (google login, logged-in state, etc.)
    let all_summaries = get_filtered_summaries(app_handle).await;
    let summaries: Vec<SkillSummary> = all_summaries
        .into_iter()
        .filter(|s| !disabled_skills.contains(&s.name))
        .collect();

    // Build system prompt
    let system_prompt = build_system_prompt(&summaries);

    // Shared cancel signal — used by the timeout and the agent loop
    let cancel_signal = Arc::new(AtomicBool::new(false));

    // Spawn a timeout task that fires the cancel signal after `timeout_secs`
    {
        let cancel_for_timeout = cancel_signal.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(timeout_secs)).await;
            cancel_for_timeout.store(true, Ordering::SeqCst);
            log::info!("[background_agent] Timeout fired after {}s", timeout_secs);
        });
    }

    // Build the initial message history (single user message — no DB history)
    let mut messages = vec![crate::db::conversations::Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: String::new(),
        role: crate::db::conversations::Role::User,
        content: prompt.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        message_type: crate::db::conversations::MessageType::Text,
        metadata: None,
        attachments: vec![],
        memory: None,
    }];

    // Track activated skills for progressive disclosure
    let mut active_skills: Vec<String> = Vec::new();

    // Agentic loop
    let result: Result<String, String> = loop {
        if cancel_signal.load(Ordering::SeqCst) {
            break Err("Automation timed out".to_string());
        }

        let iteration = messages.len();
        if iteration > config.max_iterations * 3 {
            // rough guard: each iteration can add up to 3 messages (user/tool_call/tool_result)
            break Err(format!(
                "Max iterations ({}) exceeded",
                config.max_iterations
            ));
        }

        log::info!("[background_agent] Loop step {}", iteration);

        // Build tools list: activate_skill + tools from active skills
        let mut tools = vec![build_activate_skill_tool()];
        for skill_name in &active_skills {
            let mut skill_tools = get_skill_tools(skill_name);
            for t in &mut skill_tools {
                t.skill_name = Some(skill_name.clone());
            }
            tools.extend(skill_tools);
        }

        // Context-limit the message history
        let ctx_limit = if is_local {
            config.local_context_limit
        } else {
            config.cloud_context_limit
        };
        let ctx_messages: Vec<_> = if messages.len() > ctx_limit {
            messages[messages.len() - ctx_limit..].to_vec()
        } else {
            messages.clone()
        };

        let request = LlmRequest::new(String::new())
            .with_system_prompt(Some(system_prompt.clone()))
            .with_messages(Some(ctx_messages))
            .with_internal_tools(Some(tools))
            .with_stream(Some(false))
            .with_cancel_signal(Some(cancel_signal.clone()))
            .with_attempts(Some(2))
            .with_timeout_duration(Some(if is_local { 60 } else { 15 }))
            .with_slot_id(Some(2)) // Use slot 2 to avoid KV-cache conflicts with interactive chat
            .with_session_token(session_token.clone())
            .with_override_model_id(Some(resolved_model_id));

        let response = match generate(app_handle.clone(), request, Some(is_local)).await {
            Ok(r) => r,
            Err(e) => break Err(format!("LLM error: {}", e)),
        };

        match response {
            LlmResponse::Text(text) => break Ok(text),

            LlmResponse::ToolCalls { text, calls } => {
                if calls.len() > config.max_tool_calls_per_turn {
                    break Err(format!("Too many tool calls: {}", calls.len()));
                }

                // Push assistant tool-call message into in-memory history
                let content = text.unwrap_or_default();
                let tool_call_metadatas: Vec<_> = calls
                    .iter()
                    .map(|c| crate::db::conversations::MessageMetadata::ToolCall {
                        call_id: c.id.clone(),
                        skill_name: c.skill_name.clone(),
                        tool_name: c.tool_name.clone(),
                        arguments: c.arguments.clone(),
                        thought_signature: c.thought_signature.clone(),
                    })
                    .collect();

                messages.push(crate::db::conversations::Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    conversation_id: String::new(),
                    role: crate::db::conversations::Role::Assistant,
                    content: content.clone(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    message_type: crate::db::conversations::MessageType::ToolCalls,
                    metadata: Some(tool_call_metadatas),
                    attachments: vec![],
                    memory: None,
                });

                // Handle skill activation calls
                for call in &calls {
                    if call.skill_name == "system" && call.tool_name == "activate_skill" {
                        if let Some(name) =
                            call.arguments.get("skill_name").and_then(|v| v.as_str())
                        {
                            // Normalize skill name: LLMs sometimes use underscores instead of hyphens.
                            let canonical = canonicalize_skill_name(name);
                            if skill_exists(&canonical) && !active_skills.contains(&canonical) {
                                active_skills.push(canonical.clone());
                                log::info!("[background_agent] Activated skill: {}", canonical);
                            }
                        }
                    }
                }

                // Execute tools in parallel
                let results = execute_tools(app_handle, calls.clone()).await;

                // Push tool-result message into in-memory history
                let result_metadatas: Vec<_> = results
                    .iter()
                    .map(|r| crate::db::conversations::MessageMetadata::ToolResult {
                        call_id: r.call_id.clone(),
                        success: r.success,
                        error: r.error.clone(),
                        result: r.result.clone(),
                        screenshot_attachment_id: None,
                    })
                    .collect();

                messages.push(crate::db::conversations::Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    conversation_id: String::new(),
                    role: crate::db::conversations::Role::Tool,
                    content: String::new(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    message_type: crate::db::conversations::MessageType::ToolResults,
                    metadata: Some(result_metadatas),
                    attachments: vec![],
                    memory: None,
                });
            }
        }
    };

    result
}

/// Resolve the model to use for a background automation.
///
/// If `model_id` is provided and the model exists in the DB, use it.
/// Otherwise fall back to the local model.
/// Returns `(is_local, is_internal, model_key, model_db_id)`.
fn resolve_model(app_handle: &AppHandle, model_id: Option<i64>) -> (bool, bool, String, i64) {
    let id = model_id.unwrap_or(1); // 1 == local model (first migration insert)
    match crate::db::models::get_model_by_id(app_handle, id) {
        Ok(entry) => (!entry.is_cloud, entry.is_internal, entry.model, entry.id),
        Err(e) => {
            log::warn!(
                "[background_agent] Could not resolve model id {}: {}. Using local.",
                id,
                e
            );
            (true, true, "qwen3vl-2b".to_string(), 1)
        }
    }
}

/// Build a system prompt for background automation tasks.
fn build_system_prompt(skill_summaries: &[SkillSummary]) -> String {
    let mut prompt = String::from(
        "You are Ambient, an AI assistant running a background automation task. \
         Execute the user's request thoroughly and return a concise text result. \
         You have access to skills that you can activate to gain capabilities. \
         Focus on completing the task and returning a useful result.\n\n",
    );

    if !skill_summaries.is_empty() {
        prompt.push_str("## Available Skills\n");
        prompt.push_str("Activate skills as needed to complete the task:\n\n");
        for s in skill_summaries {
            prompt.push_str(&format!("- **{}**: {}\n", s.name, s.description));
        }
        prompt.push('\n');
    }

    let now = chrono::Local::now();
    prompt.push_str(&format!(
        "Current time: {}. OS: {}.\n",
        now.format("%A, %B %e, %Y at %l:%M %p")
            .to_string()
            .replace("  ", " "),
        std::env::consts::OS
    ));

    prompt
}

/// Build the `activate_skill` tool definition.
fn build_activate_skill_tool() -> ToolDefinition {
    use crate::skills::types::{ParameterType, ToolParameter};

    ToolDefinition {
        skill_name: Some("system".to_string()),
        name: "activate_skill".to_string(),
        description:
            "Activate a skill to gain access to its tools. \
             Use this when you need capabilities not currently available."
                .to_string(),
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
