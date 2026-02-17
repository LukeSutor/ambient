//! Database operations for the automations system.
//!
//! Provides CRUD operations for automation tasks, runs, and triggers
//! following the same pattern as `db::conversations`.

use crate::db::core::DbState;
use super::types::*;
use tauri::Manager;

// ============================================================================
// Task Operations
// ============================================================================

/// Get all automation tasks for the current user.
pub fn get_all_tasks(app_handle: &tauri::AppHandle) -> Result<Vec<AutomationTask>, String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, task_type, is_enabled, is_system, prompt_template,
                    model_id, disabled_skills, notify_on_complete, notify_on_error,
                    max_iterations, timeout_seconds, schedule_type, schedule_value,
                    schedule_timezone, trigger_type, trigger_config, last_run_at,
                    next_run_at, created_at, updated_at
             FROM automation_tasks ORDER BY updated_at DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let tasks = stmt
        .query_map([], |row| {
            let disabled_skills_str: String = row.get(8)?;
            let disabled_skills: Vec<String> =
                serde_json::from_str(&disabled_skills_str).unwrap_or_default();

            Ok(AutomationTask {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                task_type: row.get(3)?,
                is_enabled: row.get(4)?,
                is_system: row.get(5)?,
                prompt_template: row.get(6)?,
                model_id: row.get(7)?,
                disabled_skills,
                notify_on_complete: row.get(9)?,
                notify_on_error: row.get(10)?,
                max_iterations: row.get(11)?,
                timeout_seconds: row.get(12)?,
                schedule_type: row.get(13)?,
                schedule_value: row.get(14)?,
                schedule_timezone: row.get(15)?,
                trigger_type: row.get(16)?,
                trigger_config: row.get(17)?,
                last_run_at: row.get(18)?,
                next_run_at: row.get(19)?,
                created_at: row.get(20)?,
                updated_at: row.get(21)?,
            })
        })
        .map_err(|e| format!("Failed to query tasks: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect tasks: {}", e))?;

    Ok(tasks)
}

/// Get a single automation task by ID.
pub fn get_task_by_id(
    app_handle: &tauri::AppHandle,
    task_id: &str,
) -> Result<AutomationTask, String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    let mut stmt = conn
        .prepare(
            "SELECT id, name, description, task_type, is_enabled, is_system, prompt_template,
                    model_id, disabled_skills, notify_on_complete, notify_on_error,
                    max_iterations, timeout_seconds, schedule_type, schedule_value,
                    schedule_timezone, trigger_type, trigger_config, last_run_at,
                    next_run_at, created_at, updated_at
             FROM automation_tasks WHERE id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let disabled_skills_str: String = stmt
        .query_row(rusqlite::params![task_id], |row| row.get(8))
        .map_err(|e| format!("Task not found: {}", e))?;

    // Re-query to get the full row (simpler than trying to clone from above)
    stmt.query_row(rusqlite::params![task_id], |row| {
        let disabled_skills: Vec<String> =
            serde_json::from_str(&disabled_skills_str).unwrap_or_default();

        Ok(AutomationTask {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            task_type: row.get(3)?,
            is_enabled: row.get(4)?,
            is_system: row.get(5)?,
            prompt_template: row.get(6)?,
            model_id: row.get(7)?,
            disabled_skills,
            notify_on_complete: row.get(9)?,
            notify_on_error: row.get(10)?,
            max_iterations: row.get(11)?,
            timeout_seconds: row.get(12)?,
            schedule_type: row.get(13)?,
            schedule_value: row.get(14)?,
            schedule_timezone: row.get(15)?,
            trigger_type: row.get(16)?,
            trigger_config: row.get(17)?,
            last_run_at: row.get(18)?,
            next_run_at: row.get(19)?,
            created_at: row.get(20)?,
            updated_at: row.get(21)?,
        })
    })
    .map_err(|e| format!("Task not found: {}", e))
}

/// Create a new automation task.
pub fn create_task(
    app_handle: &tauri::AppHandle,
    params: CreateAutomationParams,
) -> Result<AutomationTask, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let disabled_skills = params.disabled_skills.clone().unwrap_or_default();
    let disabled_skills_json = serde_json::to_string(&disabled_skills)
        .map_err(|e| format!("Failed to serialize disabled_skills: {}", e))?;

    {
        let state = app_handle.state::<DbState>();
        let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
        let conn = conn.as_ref().ok_or("Database not available")?;

        conn.execute(
            "INSERT INTO automation_tasks (
                id, name, description, task_type, prompt_template, model_id,
                disabled_skills, notify_on_complete, notify_on_error,
                max_iterations, timeout_seconds, schedule_type, schedule_value,
                schedule_timezone, trigger_type, trigger_config, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            rusqlite::params![
                id,
                params.name,
                params.description.unwrap_or_default(),
                params.task_type,
                params.prompt_template,
                params.model_id,
                disabled_skills_json,
                params.notify_on_complete.unwrap_or(true),
                params.notify_on_error.unwrap_or(true),
                params.max_iterations.unwrap_or(10),
                params.timeout_seconds.unwrap_or(120),
                params.schedule_type,
                params.schedule_value,
                params.schedule_timezone.unwrap_or_else(|| "local".to_string()),
                params.trigger_type,
                params.trigger_config,
                now,
                now,
            ],
        )
        .map_err(|e| format!("Failed to create automation task: {}", e))?;
    } // DB lock released here

    get_task_by_id(app_handle, &id)
}

/// Create a system automation task (upsert: skip if ID already exists).
pub fn create_system_task(
    app_handle: &tauri::AppHandle,
    task: &AutomationTask,
) -> Result<(), String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    let disabled_skills_json = serde_json::to_string(&task.disabled_skills)
        .map_err(|e| format!("Failed to serialize disabled_skills: {}", e))?;

    conn.execute(
        "INSERT OR IGNORE INTO automation_tasks (
            id, name, description, task_type, is_enabled, is_system,
            prompt_template, model_id, disabled_skills, notify_on_complete,
            notify_on_error, max_iterations, timeout_seconds,
            schedule_type, schedule_value, schedule_timezone,
            trigger_type, trigger_config, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        rusqlite::params![
            task.id,
            task.name,
            task.description,
            task.task_type,
            task.is_enabled,
            task.is_system,
            task.prompt_template,
            task.model_id,
            disabled_skills_json,
            task.notify_on_complete,
            task.notify_on_error,
            task.max_iterations,
            task.timeout_seconds,
            task.schedule_type,
            task.schedule_value,
            task.schedule_timezone,
            task.trigger_type,
            task.trigger_config,
            task.created_at,
            task.updated_at,
        ],
    )
    .map_err(|e| format!("Failed to create system automation task: {}", e))?;

    Ok(())
}

/// Update an automation task. Only non-system tasks can be fully updated.
/// System tasks can only toggle `is_enabled`.
pub fn update_task(
    app_handle: &tauri::AppHandle,
    params: UpdateAutomationParams,
) -> Result<AutomationTask, String> {
    // First, check if it's a system task
    let existing = get_task_by_id(app_handle, &params.id)?;
    if existing.is_system {
        // System tasks can only toggle enabled state
        if let Some(enabled) = params.is_enabled {
            {
                let state = app_handle.state::<DbState>();
                let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
                let conn = conn.as_ref().ok_or("Database not available")?;
                let now = chrono::Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE automation_tasks SET is_enabled = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![enabled, now, params.id],
                )
                .map_err(|e| format!("Failed to toggle system task: {}", e))?;
            }
            return get_task_by_id(app_handle, &params.id);
        }
        return Err("System automation tasks cannot be modified".to_string());
    }

    {
        let state = app_handle.state::<DbState>();
        let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
        let conn = conn.as_ref().ok_or("Database not available")?;

        let now = chrono::Utc::now().to_rfc3339();

        // Build dynamic SET clause
        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut param_idx = 2u32;
        // We'll collect params as boxed dyn ToSql
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now.clone())];

        macro_rules! maybe_set {
            ($field:expr, $col:expr) => {
                if let Some(ref val) = $field {
                    sets.push(format!("{} = ?{}", $col, param_idx));
                    params_vec.push(Box::new(val.clone()));
                    param_idx += 1;
                }
            };
        }

        maybe_set!(params.name, "name");
        maybe_set!(params.description, "description");
        maybe_set!(params.prompt_template, "prompt_template");
        maybe_set!(params.schedule_type, "schedule_type");
        maybe_set!(params.schedule_value, "schedule_value");
        maybe_set!(params.schedule_timezone, "schedule_timezone");
        maybe_set!(params.trigger_type, "trigger_type");
        maybe_set!(params.trigger_config, "trigger_config");
        maybe_set!(params.max_iterations, "max_iterations");
        maybe_set!(params.timeout_seconds, "timeout_seconds");
        maybe_set!(params.model_id, "model_id");
        maybe_set!(params.is_enabled, "is_enabled");
        maybe_set!(params.notify_on_complete, "notify_on_complete");
        maybe_set!(params.notify_on_error, "notify_on_error");

        if let Some(ref disabled_skills) = params.disabled_skills {
            let json = serde_json::to_string(disabled_skills)
                .map_err(|e| format!("Failed to serialize disabled_skills: {}", e))?;
            sets.push(format!("disabled_skills = ?{}", param_idx));
            params_vec.push(Box::new(json));
            param_idx += 1;
        }

        // Add the WHERE clause param
        let _ = param_idx; // suppress unused warning
        sets.push(format!("id = id")); // no-op to ensure always valid SQL
        let where_idx = params_vec.len() + 1;
        params_vec.push(Box::new(params.id.clone()));

        let sql = format!(
            "UPDATE automation_tasks SET {} WHERE id = ?{}",
            sets.join(", "),
            where_idx,
        );

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        conn.execute(&sql, params_refs.as_slice())
            .map_err(|e| format!("Failed to update automation task: {}", e))?;
    } // DB lock released here

    get_task_by_id(app_handle, &params.id)
}

/// Delete a non-system automation task.
pub fn delete_task(app_handle: &tauri::AppHandle, task_id: &str) -> Result<(), String> {
    let existing = get_task_by_id(app_handle, task_id)?;
    if existing.is_system {
        return Err("Cannot delete system automation tasks".to_string());
    }

    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    conn.execute(
        "DELETE FROM automation_tasks WHERE id = ?1",
        rusqlite::params![task_id],
    )
    .map_err(|e| format!("Failed to delete automation task: {}", e))?;

    Ok(())
}

/// Update the last_run_at and/or next_run_at timestamps for a task.
/// Only updates fields that are provided (Some). Leaves others unchanged.
pub fn update_task_run_times(
    app_handle: &tauri::AppHandle,
    task_id: &str,
    last_run_at: Option<&str>,
    next_run_at: Option<&str>,
) -> Result<(), String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    let now = chrono::Utc::now().to_rfc3339();
    let mut sets = vec!["updated_at = ?1".to_string()];
    let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];
    let mut idx = 2u32;

    if let Some(lra) = last_run_at {
        sets.push(format!("last_run_at = ?{}", idx));
        params_vec.push(Box::new(lra.to_string()));
        idx += 1;
    }

    if let Some(nra) = next_run_at {
        sets.push(format!("next_run_at = ?{}", idx));
        params_vec.push(Box::new(nra.to_string()));
        idx += 1;
    }

    let sql = format!(
        "UPDATE automation_tasks SET {} WHERE id = ?{}",
        sets.join(", "),
        idx,
    );
    params_vec.push(Box::new(task_id.to_string()));

    let params_refs: Vec<&dyn rusqlite::types::ToSql> =
        params_vec.iter().map(|p| p.as_ref()).collect();
    conn.execute(&sql, params_refs.as_slice())
        .map_err(|e| format!("Failed to update run times: {}", e))?;

    Ok(())
}

/// Get all enabled scheduled tasks.
pub fn get_enabled_scheduled_tasks(
    app_handle: &tauri::AppHandle,
) -> Result<Vec<AutomationTask>, String> {
    let all = get_all_tasks(app_handle)?;
    Ok(all
        .into_iter()
        .filter(|t| t.is_enabled && t.task_type == "scheduled")
        .collect())
}

/// Get all enabled semantic tasks.
pub fn get_enabled_semantic_tasks(
    app_handle: &tauri::AppHandle,
) -> Result<Vec<AutomationTask>, String> {
    let all = get_all_tasks(app_handle)?;
    Ok(all
        .into_iter()
        .filter(|t| t.is_enabled && t.task_type == "semantic")
        .collect())
}

// ============================================================================
// Run Operations
// ============================================================================

/// Create a new automation run record.
pub fn create_run(
    app_handle: &tauri::AppHandle,
    task_id: &str,
) -> Result<AutomationRun, String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO automation_runs (id, task_id, status, started_at) VALUES (?1, ?2, 'running', ?3)",
        rusqlite::params![id, task_id, now],
    )
    .map_err(|e| format!("Failed to create run: {}", e))?;

    Ok(AutomationRun {
        id,
        task_id: task_id.to_string(),
        status: "running".to_string(),
        result_text: None,
        error_message: None,
        started_at: now,
        completed_at: None,
        credits_used: 0.0,
    })
}

/// Complete a run with either success or failure.
pub fn complete_run(
    app_handle: &tauri::AppHandle,
    run_id: &str,
    status: &str,
    result_text: Option<&str>,
    error_message: Option<&str>,
    credits_used: f64,
) -> Result<(), String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE automation_runs SET status = ?1, result_text = ?2, error_message = ?3, completed_at = ?4, credits_used = ?5 WHERE id = ?6",
        rusqlite::params![status, result_text, error_message, now, credits_used, run_id],
    )
    .map_err(|e| format!("Failed to complete run: {}", e))?;

    Ok(())
}

/// Get run history for a task.
pub fn get_runs(
    app_handle: &tauri::AppHandle,
    task_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<AutomationRun>, String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, status, result_text, error_message, started_at, completed_at, credits_used
             FROM automation_runs WHERE task_id = ?1 ORDER BY started_at DESC LIMIT ?2 OFFSET ?3",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let runs = stmt
        .query_map(rusqlite::params![task_id, limit, offset], |row| {
            Ok(AutomationRun {
                id: row.get(0)?,
                task_id: row.get(1)?,
                status: row.get(2)?,
                result_text: row.get(3)?,
                error_message: row.get(4)?,
                started_at: row.get(5)?,
                completed_at: row.get(6)?,
                credits_used: row.get(7)?,
            })
        })
        .map_err(|e| format!("Failed to query runs: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect runs: {}", e))?;

    Ok(runs)
}

/// Get the latest run for a task.
pub fn get_latest_run(
    app_handle: &tauri::AppHandle,
    task_id: &str,
) -> Result<Option<AutomationRun>, String> {
    let runs = get_runs(app_handle, task_id, 1, 0)?;
    Ok(runs.into_iter().next())
}

// ============================================================================
// Trigger Operations
// ============================================================================

/// Get triggers for a task.
pub fn get_triggers(
    app_handle: &tauri::AppHandle,
    task_id: &str,
) -> Result<Vec<AutomationTrigger>, String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    let mut stmt = conn
        .prepare(
            "SELECT id, task_id, trigger_type, trigger_config, is_enabled, created_at
             FROM automation_triggers WHERE task_id = ?1",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let triggers = stmt
        .query_map(rusqlite::params![task_id], |row| {
            Ok(AutomationTrigger {
                id: row.get(0)?,
                task_id: row.get(1)?,
                trigger_type: row.get(2)?,
                trigger_config: row.get(3)?,
                is_enabled: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("Failed to query triggers: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect triggers: {}", e))?;

    Ok(triggers)
}

/// Add a trigger to a task.
pub fn add_trigger(
    app_handle: &tauri::AppHandle,
    task_id: &str,
    trigger_type: &str,
    trigger_config: &str,
) -> Result<AutomationTrigger, String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO automation_triggers (id, task_id, trigger_type, trigger_config, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, task_id, trigger_type, trigger_config, now],
    )
    .map_err(|e| format!("Failed to add trigger: {}", e))?;

    Ok(AutomationTrigger {
        id,
        task_id: task_id.to_string(),
        trigger_type: trigger_type.to_string(),
        trigger_config: trigger_config.to_string(),
        is_enabled: true,
        created_at: now,
    })
}

/// Delete a trigger.
pub fn delete_trigger(
    app_handle: &tauri::AppHandle,
    trigger_id: &str,
) -> Result<(), String> {
    let state = app_handle.state::<DbState>();
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn.as_ref().ok_or("Database not available")?;

    conn.execute(
        "DELETE FROM automation_triggers WHERE id = ?1",
        rusqlite::params![trigger_id],
    )
    .map_err(|e| format!("Failed to delete trigger: {}", e))?;

    Ok(())
}
