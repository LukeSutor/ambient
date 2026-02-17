//! Automation task execution engine.
//!
//! Orchestrates automation runs: creates the run record, invokes the
//! background agent (via [`crate::agents::chat::background`]), records
//! the result, and sends notifications.
//!
//! The actual agentic loop lives in `agents/chat/background.rs` so it
//! can be maintained alongside the interactive runtime without duplication.

use super::db;
use super::types::*;
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

    // Execute the background agent (no DB writes or streaming events)
    let result = crate::agents::chat::background::run_background(
        app_handle,
        &task.prompt_template,
        task.model_id,
        &task.disabled_skills,
        task.max_iterations as usize,
        task.timeout_seconds as u64,
    )
    .await;

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
