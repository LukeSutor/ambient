//! Tauri commands for the automations system.
//!
//! Exposes CRUD operations for automation tasks, runs, and triggers
//! as well as execution and scheduling commands.

use super::db;
use super::types::*;
use tauri::AppHandle;

// ============================================================================
// Task CRUD Commands
// ============================================================================

/// Get all automation tasks.
#[tauri::command]
pub async fn get_automation_tasks(
    app_handle: AppHandle,
) -> Result<Vec<AutomationTask>, String> {
    db::get_all_tasks(&app_handle)
}

/// Get a single automation task by ID.
#[tauri::command]
pub async fn get_automation_task(
    app_handle: AppHandle,
    task_id: String,
) -> Result<AutomationTask, String> {
    db::get_task_by_id(&app_handle, &task_id)
}

/// Create a new automation task.
#[tauri::command]
pub async fn create_automation_task(
    app_handle: AppHandle,
    params: CreateAutomationParams,
) -> Result<AutomationTask, String> {
    // Validate task type
    TaskType::from_str(&params.task_type)?;

    // Validate schedule type if present
    if let Some(ref st) = params.schedule_type {
        ScheduleType::from_str(st)?;
    }

    // Validate trigger type if present
    if let Some(ref tt) = params.trigger_type {
        TriggerType::from_str(tt)?;
    }

    let task = db::create_task(&app_handle, params)?;

    // Emit event so frontend updates
    let _ = crate::events::emitter::emit(
        super::events::AUTOMATION_TASK_CREATED,
        super::events::AutomationTaskEvent {
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    // If it's a scheduled task, compute next_run_at and register with scheduler
    if task.task_type == "scheduled" {
        // Compute and store next_run_at
        if let (Some(st), Some(sv)) = (task.schedule_type.as_deref(), task.schedule_value.as_deref())
        {
            if let Ok(next) = super::scheduler::calculate_next_run_time(st, sv) {
                let _ = db::update_task_run_times(&app_handle, &task.id, None, Some(&next));
            }
        }

        if task.is_enabled {
            let app_handle_clone = app_handle.clone();
            let task_clone = task.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    super::scheduler::schedule_task(&app_handle_clone, &task_clone).await
                {
                    log::warn!(
                        "[automations] Failed to schedule task {}: {}",
                        task_clone.id,
                        e
                    );
                }
            });
        }
    } else if task.task_type == "semantic" {
        // A new semantic task was added — restart the screen monitor so it picks it up.
        let app_handle_for_monitor = app_handle.clone();
        tokio::spawn(async move {
            super::triggers::restart_screen_monitor(&app_handle_for_monitor).await;
        });
    }

    Ok(task)
}

/// Update an existing automation task.
#[tauri::command]
pub async fn update_automation_task(
    app_handle: AppHandle,
    params: UpdateAutomationParams,
) -> Result<AutomationTask, String> {
    // Validate schedule type if present
    if let Some(ref st) = params.schedule_type {
        ScheduleType::from_str(st)?;
    }

    // Validate trigger type if present
    if let Some(ref tt) = params.trigger_type {
        TriggerType::from_str(tt)?;
    }

    let task_id = params.id.clone();
    let task = db::update_task(&app_handle, params)?;

    // Emit event
    let _ = crate::events::emitter::emit(
        super::events::AUTOMATION_TASK_UPDATED,
        super::events::AutomationTaskEvent {
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    // Reschedule if it's a scheduled task.
    // Unschedule first (awaited in-place to ensure old task is cancelled before
    // scheduling the new one), then schedule if still enabled.
    if task.task_type == "scheduled" {
        super::scheduler::unschedule_task(&task_id).await;
        if task.is_enabled {
            if let Err(e) = super::scheduler::schedule_task(&app_handle, &task).await {
                log::warn!(
                    "[automations] Failed to reschedule task {}: {}",
                    task.id,
                    e
                );
            }
        }
    } else if task.task_type == "semantic" {
        // Semantic task changed — restart screen monitor.
        let app_handle_for_monitor = app_handle.clone();
        tokio::spawn(async move {
            super::triggers::restart_screen_monitor(&app_handle_for_monitor).await;
        });
    }

    Ok(task)
}

/// Delete an automation task (non-system only).
#[tauri::command]
pub async fn delete_automation_task(
    app_handle: AppHandle,
    task_id: String,
) -> Result<(), String> {
    // Unschedule first
    super::scheduler::unschedule_task(&task_id).await;

    db::delete_task(&app_handle, &task_id)?;

    // Emit event
    let _ = crate::events::emitter::emit(
        super::events::AUTOMATION_TASK_DELETED,
        super::events::AutomationTaskDeletedEvent {
            task_id: task_id.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    // Restart screen monitor in case we just removed the last semantic task.
    let app_handle_for_monitor = app_handle.clone();
    tokio::spawn(async move {
        super::triggers::restart_screen_monitor(&app_handle_for_monitor).await;
    });

    Ok(())
}

/// Toggle an automation task's enabled state.
#[tauri::command]
pub async fn toggle_automation_task(
    app_handle: AppHandle,
    task_id: String,
    enabled: bool,
) -> Result<AutomationTask, String> {
    let task = db::update_task(
        &app_handle,
        UpdateAutomationParams {
            id: task_id.clone(),
            is_enabled: Some(enabled),
            name: None,
            description: None,
            prompt_template: None,
            model_id: None,
            disabled_skills: None,
            notify_on_complete: None,
            notify_on_error: None,
            max_iterations: None,
            timeout_seconds: None,
            schedule_type: None,
            schedule_value: None,
            schedule_timezone: None,
            trigger_type: None,
            trigger_config: None,
        },
    )?;

    // Update scheduler
    if task.task_type == "scheduled" {
        super::scheduler::unschedule_task(&task_id).await;
        if task.is_enabled {
            if let Err(e) = super::scheduler::schedule_task(&app_handle, &task).await {
                log::warn!("[automations] Failed to schedule task: {}", e);
            }
        } else {
            // Clear next_run_at when disabled so UI shows no scheduled time
            let _ = db::clear_next_run_at(&app_handle, &task_id);
        }
    }

    // Emit event
    let _ = crate::events::emitter::emit(
        super::events::AUTOMATION_TASK_UPDATED,
        super::events::AutomationTaskEvent {
            task_id: task.id.clone(),
            task_name: task.name.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(task)
}

// ============================================================================
// Run History Commands
// ============================================================================

/// Get run history for a task.
#[tauri::command]
pub async fn get_automation_runs(
    app_handle: AppHandle,
    task_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<AutomationRun>, String> {
    db::get_runs(&app_handle, &task_id, limit.unwrap_or(20), offset.unwrap_or(0))
}

/// Get the latest run for a task.
#[tauri::command]
pub async fn get_latest_automation_run(
    app_handle: AppHandle,
    task_id: String,
) -> Result<Option<AutomationRun>, String> {
    db::get_latest_run(&app_handle, &task_id)
}

// ============================================================================
// Manual Execution Commands
// ============================================================================

/// Manually trigger an automation task to run now.
#[tauri::command]
pub async fn run_automation_task(
    app_handle: AppHandle,
    task_id: String,
) -> Result<AutomationRun, String> {
    let task = db::get_task_by_id(&app_handle, &task_id)?;
    super::executor::execute_automation(&app_handle, &task).await
}

// ============================================================================
// Trigger Commands
// ============================================================================

/// Get triggers for a task.
#[tauri::command]
pub async fn get_automation_triggers(
    app_handle: AppHandle,
    task_id: String,
) -> Result<Vec<AutomationTrigger>, String> {
    db::get_triggers(&app_handle, &task_id)
}

/// Add a trigger to a task.
#[tauri::command]
pub async fn add_automation_trigger(
    app_handle: AppHandle,
    task_id: String,
    trigger_type: String,
    trigger_config: String,
) -> Result<AutomationTrigger, String> {
    TriggerType::from_str(&trigger_type)?;
    db::add_trigger(&app_handle, &task_id, &trigger_type, &trigger_config)
}

/// Delete a trigger.
#[tauri::command]
pub async fn delete_automation_trigger(
    app_handle: AppHandle,
    trigger_id: String,
) -> Result<(), String> {
    db::delete_trigger(&app_handle, &trigger_id)
}
