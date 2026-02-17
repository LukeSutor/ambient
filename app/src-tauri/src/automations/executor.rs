//! Automation task execution engine.
//!
//! Runs automation tasks using the existing agent runtime in background mode.
//! The executor creates a run record, prepares the prompt, invokes the agent,
//! and records the result.

use super::db;
use super::types::*;
use crate::models::llm::client::generate;
use crate::models::llm::types::{LlmRequest, LlmResponse};
use crate::models::llm::usage::create_generation_session;
use crate::skills::executor::execute_tools;
use crate::skills::registry::{get_filtered_summaries, get_skill_tools, skill_exists};
use crate::skills::types::{AgentRuntimeConfig, SkillSummary, ToolDefinition};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

/// Execute an automation task and return the run record.
///
/// This is the main entry point called by both the scheduler and manual triggers.
/// It creates a run record, executes the agent in background mode, and records results.
pub async fn execute_automation(
    app_handle: &AppHandle,
    task: &AutomationTask,
) -> Result<AutomationRun, String> {
    log::info!(
        "[executor] Starting automation '{}' ({})",
        task.name,
        task.id
    );

    // Create a run record
    let run = db::create_run(app_handle, &task.id)?;

    // Update last_run_at on the task
    let now = chrono::Utc::now().to_rfc3339();
    let _ = db::update_task_run_times(app_handle, &task.id, Some(&now), None);

    // Emit run started event
    let _ = crate::events::emitter::emit(
        super::events::AUTOMATION_RUN_STARTED,
        super::events::AutomationRunEvent {
            run_id: run.id.clone(),
            task_id: task.id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    // Execute the background agent
    let result = run_background_agent(app_handle, task, &run.id).await;

    match result {
        Ok(result_text) => {
            // Mark run as completed
            db::complete_run(
                app_handle,
                &run.id,
                "completed",
                Some(&result_text),
                None,
                0.0,
            )?;

            // Send notification if configured
            if task.notify_on_complete {
                super::notifications::send_completion_notification(
                    app_handle,
                    task,
                    &result_text,
                );
            }

            // Emit run completed event
            let _ = crate::events::emitter::emit(
                super::events::AUTOMATION_RUN_COMPLETED,
                super::events::AutomationRunCompletedEvent {
                    run_id: run.id.clone(),
                    task_id: task.id.clone(),
                    status: "completed".to_string(),
                    result_summary: Some(result_text.clone()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
            );

            Ok(AutomationRun {
                id: run.id,
                task_id: task.id.clone(),
                status: "completed".to_string(),
                result_text: Some(result_text),
                error_message: None,
                started_at: run.started_at,
                completed_at: Some(chrono::Utc::now().to_rfc3339()),
                credits_used: 0.0,
            })
        }
        Err(error) => {
            log::error!(
                "[executor] Automation '{}' failed: {}",
                task.id,
                error
            );

            // Mark run as failed
            db::complete_run(
                app_handle,
                &run.id,
                "failed",
                None,
                Some(&error),
                0.0,
            )?;

            // Send error notification if configured
            if task.notify_on_error {
                super::notifications::send_error_notification(app_handle, task, &error);
            }

            // Emit run completed event with error
            let _ = crate::events::emitter::emit(
                super::events::AUTOMATION_RUN_COMPLETED,
                super::events::AutomationRunCompletedEvent {
                    run_id: run.id.clone(),
                    task_id: task.id.clone(),
                    status: "failed".to_string(),
                    result_summary: Some(error.clone()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
            );

            Ok(AutomationRun {
                id: run.id,
                task_id: task.id.clone(),
                status: "failed".to_string(),
                result_text: None,
                error_message: Some(error),
                started_at: run.started_at,
                completed_at: Some(chrono::Utc::now().to_rfc3339()),
                credits_used: 0.0,
            })
        }
    }
}

/// Run the agent in background mode for an automation task.
///
/// This is a simplified version of the interactive agentic loop that:
/// - Does NOT save messages to the DB
/// - Does NOT emit streaming events
/// - Uses llama.cpp slot 2 to avoid KV cache conflicts
/// - Returns the final text result
async fn run_background_agent(
    app_handle: &AppHandle,
    task: &AutomationTask,
    _run_id: &str,
) -> Result<String, String> {
    let config = AgentRuntimeConfig {
        local_context_limit: 3,
        cloud_context_limit: 10,
        max_tool_calls_per_turn: 5,
        max_iterations: task.max_iterations as usize,
        enable_thinking: true,
    };

    // Resolve which model to use
    let model_id = task.model_id;
    let (is_local, is_internal, model_key) = resolve_automation_model(app_handle, model_id)?;

    // For internal cloud models, create a generation session
    let mut session_token: Option<String> = None;
    if !is_local && is_internal {
        match create_generation_session(&model_key).await {
            Ok(session) => {
                session_token = Some(session.session_token);
                log::info!(
                    "[executor] Generation session created for automation (cost: {} credits)",
                    session.credit_cost
                );
            }
            Err(e) => {
                if e.contains("rate_limit_exceeded") || e.contains("model_not_available") {
                    return Err(e);
                }
                log::warn!(
                    "[executor] Failed to create session, continuing without: {}",
                    e
                );
            }
        }
    }

    // Get skill summaries, filtering out disabled skills for this automation
    let all_summaries = get_filtered_summaries(app_handle).await;
    let summaries: Vec<SkillSummary> = all_summaries
        .into_iter()
        .filter(|s| !task.disabled_skills.contains(&s.name))
        .collect();

    // Build the system prompt
    let system_prompt = build_automation_system_prompt(&summaries);

    // Prepare the user prompt from the template
    let user_prompt = task.prompt_template.clone();

    // Build initial messages
    let mut messages = vec![crate::db::conversations::Message {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: String::new(),
        role: crate::db::conversations::Role::User,
        content: user_prompt,
        timestamp: chrono::Utc::now().to_rfc3339(),
        message_type: crate::db::conversations::MessageType::Text,
        metadata: None,
        attachments: vec![],
        memory: None,
    }];

    let cancel_signal = Arc::new(AtomicBool::new(false));

    // Set up a timeout
    let timeout_duration = task.timeout_seconds as u64;
    let timeout_cancel = cancel_signal.clone();
    let timeout_handle = tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(timeout_duration)).await;
        timeout_cancel.store(true, Ordering::SeqCst);
    });

    // Track active skills for progressive disclosure
    let mut active_skills: Vec<String> = Vec::new();

    // Agentic loop
    let mut iteration = 0;
    let result = loop {
        if cancel_signal.load(Ordering::SeqCst) {
            break Err("Automation timed out".to_string());
        }

        iteration += 1;
        if iteration > config.max_iterations {
            break Err(format!(
                "Max iterations ({}) exceeded for automation",
                config.max_iterations
            ));
        }

        log::info!(
            "[executor] Automation '{}' iteration {}/{}",
            task.id,
            iteration,
            config.max_iterations
        );

        // Build tools list: always include activate_skill + active skill tools
        let mut tools = Vec::new();
        tools.push(build_activate_skill_tool());
        for skill_name in &active_skills {
            let mut skill_tools = get_skill_tools(skill_name);
            for tool in &mut skill_tools {
                tool.skill_name = Some(skill_name.clone());
            }
            tools.extend(skill_tools);
        }

        // Context-limit messages
        let context_limit = if is_local {
            config.local_context_limit
        } else {
            config.cloud_context_limit
        };
        let context_messages: Vec<_> = if messages.len() > context_limit {
            messages[messages.len() - context_limit..].to_vec()
        } else {
            messages.clone()
        };

        let request = LlmRequest::new(String::new())
            .with_system_prompt(Some(system_prompt.clone()))
            .with_messages(Some(context_messages))
            .with_internal_tools(Some(tools))
            .with_stream(Some(false)) // No streaming for background
            .with_cancel_signal(Some(cancel_signal.clone()))
            .with_attempts(Some(2))
            .with_timeout_duration(Some(if is_local { 30 } else { 10 }))
            .with_slot_id(Some(2)) // Use slot 2 for background tasks
            .with_session_token(session_token.clone());

        let response = match generate(app_handle.clone(), request, Some(is_local)).await {
            Ok(resp) => resp,
            Err(e) => break Err(format!("LLM error: {}", e)),
        };

        match response {
            LlmResponse::Text(text) => {
                break Ok(text);
            }
            LlmResponse::ToolCalls { text, calls } => {
                if calls.len() > config.max_tool_calls_per_turn {
                    break Err(format!("Too many tool calls: {}", calls.len()));
                }

                // Save tool calls as an assistant message in the in-memory history
                let tool_call_content = text.unwrap_or_default();

                let mut tool_call_metadatas = Vec::new();
                for call in &calls {
                    tool_call_metadatas.push(crate::db::conversations::MessageMetadata::ToolCall {
                        call_id: call.id.clone(),
                        skill_name: call.skill_name.clone(),
                        tool_name: call.tool_name.clone(),
                        arguments: call.arguments.clone(),
                        thought_signature: call.thought_signature.clone(),
                    });
                }

                messages.push(crate::db::conversations::Message {
                    id: uuid::Uuid::new_v4().to_string(),
                    conversation_id: String::new(),
                    role: crate::db::conversations::Role::Assistant,
                    content: tool_call_content.clone(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    message_type: crate::db::conversations::MessageType::ToolCalls,
                    metadata: Some(tool_call_metadatas),
                    attachments: vec![],
                    memory: None,
                });

                // Handle skill activation
                for call in &calls {
                    if call.skill_name == "system" && call.tool_name == "activate_skill" {
                        if let Some(skill_name) = call.arguments.get("skill_name").and_then(|v| v.as_str()) {
                            if skill_exists(skill_name) && !active_skills.contains(&skill_name.to_string()) {
                                active_skills.push(skill_name.to_string());
                                log::info!(
                                    "[executor] Automation activated skill: {}",
                                    skill_name
                                );
                            }
                        }
                    }
                }

                // Execute tools
                let results = execute_tools(app_handle, calls.clone()).await;

                // Build tool result metadatas
                let mut result_metadatas = Vec::new();
                for result in &results {
                    result_metadatas.push(crate::db::conversations::MessageMetadata::ToolResult {
                        call_id: result.call_id.clone(),
                        success: result.success,
                        error: result.error.clone(),
                        result: result.result.clone(),
                        screenshot_attachment_id: None,
                    });
                }

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

                continue;
            }
        }
    };

    // Cancel the timeout
    timeout_handle.abort();

    result
}

/// Resolve which model to use for an automation.
/// If model_id is set, use that; otherwise use the user's default model.
fn resolve_automation_model(
    app_handle: &AppHandle,
    model_id: Option<i64>,
) -> Result<(bool, bool, String), String> {
    let model_id = match model_id {
        Some(id) => id,
        None => {
            // Use default model from settings
            // We can't use async here easily, so just default to local model
            1 // Default to model id 1 (local)
        }
    };

    match crate::db::models::get_model_by_id(app_handle, model_id) {
        Ok(entry) => Ok((!entry.is_cloud, entry.is_internal, entry.model)),
        Err(e) => {
            log::warn!(
                "[executor] Could not find model {}: {}. Falling back to local.",
                model_id,
                e
            );
            Ok((true, true, "qwen3vl-2b".to_string()))
        }
    }
}

/// Build a system prompt for background automation execution.
fn build_automation_system_prompt(skill_summaries: &[SkillSummary]) -> String {
    let mut prompt = String::from(
        "You are Ambient, an AI assistant running a background automation task. \
         Execute the user's request and provide a concise text result. \
         You have access to skills that you can activate to gain capabilities. \
         Be thorough but concise. Focus on completing the task and returning the result.\n\n",
    );

    if !skill_summaries.is_empty() {
        prompt.push_str("## Available Skills\n");
        prompt.push_str("You can activate these skills to gain new capabilities:\n\n");
        for summary in skill_summaries {
            prompt.push_str(&format!("- **{}**: {}\n", summary.name, summary.description));
        }
        prompt.push('\n');
    }

    // Add dynamic context
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

/// Build the activate_skill tool definition (same as in the interactive runtime).
fn build_activate_skill_tool() -> ToolDefinition {
    use crate::skills::types::{ParameterType, ToolParameter};

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
