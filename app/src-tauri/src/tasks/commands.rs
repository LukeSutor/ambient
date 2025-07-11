use crate::tasks::{
    models::*,
    service::TaskService,
    templates::{get_built_in_templates, get_template_by_name, get_categories},
};
use crate::db::DbState;
use tauri::State;

/// Create a new task from a request
#[tauri::command]
pub async fn create_task(
    db_state: State<'_, DbState>,
    request: CreateTaskRequest,
) -> Result<TaskWithSteps, String> {
    TaskService::create_task(&db_state, request)
}

/// Create a task from a template
#[tauri::command]
pub async fn create_task_from_template(
    db_state: State<'_, DbState>,
    template_name: String,
) -> Result<TaskWithSteps, String> {
    let template = get_template_by_name(&template_name)
        .ok_or_else(|| format!("Template '{}' not found", template_name))?;
    
    let request = template.to_create_request();
    TaskService::create_task(&db_state, request)
}

/// Get a task by ID with all its steps
#[tauri::command]
pub async fn get_task(
    db_state: State<'_, DbState>,
    task_id: i64,
) -> Result<TaskWithSteps, String> {
    TaskService::get_task_with_steps(&db_state, task_id)
}

/// Get all active tasks
#[tauri::command]
pub async fn get_active_tasks(
    db_state: State<'_, DbState>,
) -> Result<Vec<TaskWithSteps>, String> {
    TaskService::get_active_tasks(&db_state)
}

/// Get all available task templates
#[tauri::command]
pub async fn get_task_templates() -> Result<Vec<crate::tasks::templates::TaskTemplate>, String> {
    Ok(get_built_in_templates())
}

/// Get available template categories
#[tauri::command]
pub async fn get_template_categories() -> Result<Vec<String>, String> {
    Ok(get_categories())
}

/// Get available task frequencies
#[tauri::command]
pub async fn get_available_frequencies() -> Result<Vec<(String, String)>, String> {
    let frequencies = vec![
        (TaskFrequency::OneTime.as_str(), TaskFrequency::OneTime.description()),
        (TaskFrequency::Daily.as_str(), TaskFrequency::Daily.description()),
        (TaskFrequency::Weekly.as_str(), TaskFrequency::Weekly.description()),
        (TaskFrequency::BiWeekly.as_str(), TaskFrequency::BiWeekly.description()),
        (TaskFrequency::Monthly.as_str(), TaskFrequency::Monthly.description()),
        (TaskFrequency::Quarterly.as_str(), TaskFrequency::Quarterly.description()),
        (TaskFrequency::Yearly.as_str(), TaskFrequency::Yearly.description()),
    ];
    Ok(frequencies)
}

/// Update task status
#[tauri::command]
pub async fn update_task_status(
    db_state: State<'_, DbState>,
    task_id: i64,
    status: String,
) -> Result<(), String> {
    let status_enum = match status.as_str() {
        "pending" => TaskStatus::Pending,
        "in_progress" => TaskStatus::InProgress,
        "completed" => TaskStatus::Completed,
        "paused" => TaskStatus::Paused,
        _ => return Err(format!("Invalid status: {}", status)),
    };
    
    TaskService::update_task_status(&db_state, task_id, status_enum)
}

/// Update step status
#[tauri::command]
pub async fn update_step_status(
    db_state: State<'_, DbState>,
    step_id: i64,
    status: String,
) -> Result<(), String> {
    let status_enum = match status.as_str() {
        "pending" => StepStatus::Pending,
        "in_progress" => StepStatus::InProgress,
        "completed" => StepStatus::Completed,
        "skipped" => StepStatus::Skipped,
        _ => return Err(format!("Invalid status: {}", status)),
    };
    
    TaskService::update_step_status(&db_state, step_id, status_enum)
}

/// Delete a task
#[tauri::command]
pub async fn delete_task(
    db_state: State<'_, DbState>,
    task_id: i64,
) -> Result<(), String> {
    TaskService::delete_task(&db_state, task_id)
}

/// Complete a task and handle recurring tasks
#[tauri::command]
pub async fn complete_task(
    db_state: State<'_, DbState>,
    task_id: i64,
) -> Result<Option<TaskWithSteps>, String> {
    TaskService::complete_task(&db_state, task_id)
}

/// Get overdue tasks
#[tauri::command]
pub async fn get_overdue_tasks(
    db_state: State<'_, DbState>,
) -> Result<Vec<TaskWithSteps>, String> {
    TaskService::get_overdue_tasks(&db_state)
}

/// Get tasks due today
#[tauri::command]
pub async fn get_tasks_due_today(
    db_state: State<'_, DbState>,
) -> Result<Vec<TaskWithSteps>, String> {
    TaskService::get_tasks_due_today(&db_state)
}

/// Get tasks by frequency
#[tauri::command]
pub async fn get_tasks_by_frequency(
    db_state: State<'_, DbState>,
    frequency: String,
) -> Result<Vec<TaskWithSteps>, String> {
    let frequency_enum = TaskFrequency::from_str(&frequency);
    TaskService::get_tasks_by_frequency(&db_state, frequency_enum)
}

/// Analyze current screen for task progress using LLM
#[tauri::command]
pub async fn analyze_current_screen_for_tasks(
    db_state: State<'_, DbState>,
    task_id: i64,
) -> Result<TaskProgressUpdate, String> {
    // Get current screen context
    let screen_text = crate::os_utils::windows::window::get_screen_text_formatted()
        .await
        .map_err(|e| format!("Failed to get screen text: {}", e))?;
    
    let application = "Unknown".to_string(); // You might want to get this from window detection
    
    let screen_context = ScreenContext {
        text: screen_text,
        application,
        window_title: None,
        timestamp: chrono::Utc::now(),
    };

    // Create LLM completion function that calls the existing completion endpoint
    let llm_completion = |prompt: &str| -> Result<String, String> {
        // Use a simpler approach - just return mock data for now
        // You can integrate with the actual LLM service later
        Ok(r#"{"completed_steps": [], "in_progress_steps": []}"#.to_string())
    };

    TaskService::analyze_task_progress(&db_state, task_id, screen_context, llm_completion).await
}

/// Get task progress history
#[tauri::command]
pub async fn get_task_progress_history(
    db_state: State<'_, DbState>,
    task_id: i64,
) -> Result<Vec<TaskProgress>, String> {
    let db_guard = db_state.0.lock().map_err(|e| format!("Database lock error: {}", e))?;
    let conn = db_guard.as_ref().ok_or("Database connection not available")?;

    let mut stmt = conn
        .prepare("SELECT id, task_id, step_id, llm_confidence, evidence, reasoning, timestamp FROM task_progress WHERE task_id = ? ORDER BY timestamp DESC")
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let progress_records = stmt
        .query_map(rusqlite::params![task_id], |row| {
            Ok(TaskProgress {
                id: row.get(0)?,
                task_id: row.get(1)?,
                step_id: row.get(2)?,
                llm_confidence: row.get(3)?,
                evidence: row.get(4)?,
                reasoning: row.get(5)?,
                timestamp: chrono::DateTime::parse_from_str(&row.get::<_, String>(6)?, "%Y-%m-%d %H:%M:%S")
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        })
        .map_err(|e| format!("Failed to query progress: {}", e))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| format!("Failed to collect progress: {}", e))?;

    Ok(progress_records)
}
