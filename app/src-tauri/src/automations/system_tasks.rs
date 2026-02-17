//! Built-in system automation tasks.
//!
//! These are pre-defined automation templates that ship with the app.
//! They are inserted as disabled tasks on first database initialization.

use super::db;
use super::types::*;
use tauri::AppHandle;

/// Definition of a system task template.
struct SystemTaskDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    task_type: TaskType,
    prompt: &'static str,
    schedule_type: Option<ScheduleType>,
    schedule_value: Option<&'static str>,
}

/// Built-in system task definitions.
const SYSTEM_TASKS: &[SystemTaskDef] = &[
    SystemTaskDef {
        id: "system_daily_summary",
        name: "Daily Summary",
        description: "Generate a summary of your day's activities and conversations",
        task_type: TaskType::Scheduled,
        prompt: "Review today's conversations and activities. Provide a brief summary of key topics discussed, decisions made, and any action items.",
        schedule_type: Some(ScheduleType::Daily),
        schedule_value: Some("18:00"),
    },
    SystemTaskDef {
        id: "system_weekly_review",
        name: "Weekly Review",
        description: "Compile a weekly review of your productivity and key insights",
        task_type: TaskType::Scheduled,
        prompt: "Review the past week's conversations and activities. Highlight key themes, insights, completed tasks, and provide suggestions for the coming week.",
        schedule_type: Some(ScheduleType::SpecificDays),
        schedule_value: Some("fri|17:00"),
    },
];

/// Initialize system tasks into the database.
/// Uses INSERT OR IGNORE so each task is only created once.
pub fn initialize_system_tasks(app_handle: &AppHandle) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();

    for def in SYSTEM_TASKS {
        let task = AutomationTask {
            id: def.id.to_string(),
            name: def.name.to_string(),
            description: def.description.to_string(),
            task_type: def.task_type.as_str().to_string(),
            is_enabled: false, // System tasks start disabled
            is_system: true,
            prompt_template: def.prompt.to_string(),
            model_id: None,
            disabled_skills: vec![],
            notify_on_complete: true,
            notify_on_error: true,
            max_iterations: 10,
            timeout_seconds: 120,
            schedule_type: def.schedule_type.as_ref().map(|s| s.as_str().to_string()),
            schedule_value: def.schedule_value.map(|s| s.to_string()),
            schedule_timezone: "local".to_string(),
            trigger_type: None,
            trigger_config: None,
            last_run_at: None,
            next_run_at: None,
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        if let Err(e) = db::create_system_task(app_handle, &task) {
            log::warn!(
                "[automations] Failed to initialize system task '{}': {}",
                def.name,
                e
            );
        }
    }

    log::info!(
        "[automations] System tasks initialized ({} templates)",
        SYSTEM_TASKS.len()
    );
    Ok(())
}
