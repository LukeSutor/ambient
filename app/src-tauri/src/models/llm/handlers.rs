use tauri::{AppHandle, Manager};
use crate::events::{emitter::emit, types::*};
use crate::models::llm::{prompts::get_prompt, schemas::get_schema, server::generate};
use crate::tasks::TaskService;
use crate::tasks::TaskWithSteps;

pub fn handle_detect_tasks(event: DetectTasksEvent, app_handle: &AppHandle) {
    // Get DB state
    let db_state = app_handle.state::<DbState>();

    // Fetch all active tasks
    let active_tasks = TaskService::get_active_tasks(db_state);
    if active_tasks.is_err() {
        eprintln!("[detect_tasks] Failed to fetch active tasks: {}", active_tasks.err().unwrap());
        return;
    }

    // Return if no active tasks
    let tasks = active_tasks.unwrap();
    if tasks.is_empty() {
        println!("[detect_tasks] No active tasks found");
        return;
    }

    // Format tasks for prompt
    let formatted_tasks = format_tasks(&tasks);

    // Create prompt
    let prompt = get_prompt("detect_tasks")
        .replace("{text}", &event.text)
        .replace("{active_url}", &event.active_url)
        .replace("{tasks}", &formatted_tasks);

    println!("[detect_tasks] Prompt created:\n{}", prompt);

    // Get response schema
    let schema = get_schema("detect_tasks").unwrap_or("{}");

    // Get task updates
    let task_updates = generate(
        app_handle.clone(),
        prompt,
        Some(schema.to_string()),
        None,
        None,
        None,
    ).await;

    // Handle response
    if let Ok(response) = task_updates {
        println!("[detect_tasks] Task updates generated successfully: {:?}", response);
        
        // Parse JSON response
        let parsed_response: serde_json::Value = serde_json::from_str(&response)
            .unwrap_or_else(|_| serde_json::json!({}));
    } else {
        eprintln!("[detect_tasks] Failed to generate task updates: {}", task_updates.unwrap_err());
    }

    // Loop through response and update step statuses
    if let Some(updates) = parsed_response.get("updates").and_then(|u| u.as_array()) {
        for update in updates {
            if let (Some(step_id), Some(status)) = (
                update.get("step_id").and_then(|id| id.as_u64()),
                update.get("status").and_then(|s| s.as_str())
            ) {
                println!("[detect_tasks] Updating step {} to status: {}", step_id, status);
                // Update step status in database
                if let Ok(step_status) = status.parse::<StepStatus>() {
                    if let Err(e) = TaskService::update_step_status(&db_state, step_id as i64, step_status) {
                        eprintln!("[detect_tasks] Failed to update step {}: {}", step_id, e);
                    }
                } else {
                    eprintln!("[detect_tasks] Invalid status '{}' for step {}", status, step_id);
                }
            }
        }
    } else {
        println!("[detect_tasks] No updates found in response");
    }

    // Emit update tasks event
    let update_event = UpdateTasksEvent {
        timestamp: chrono::Utc::now().to_string()
    };
    emit(UPDATE_TASKS, update_event);
}

fn format_tasks(tasks: &[TaskWithSteps]) -> String {
    tasks.iter().map(|task| {
        let steps = task.steps.iter().map(|step| {
            format!("\tStep {}, ID: {}, Description: {}, Status: {}", step.title, step.id, step.description, step.status)
        }).collect::<Vec<_>>().join("\n");

        format!("Task {}, ID: {},  Description: {}, Steps: [\n{}\n]", task.task.name, task.task.id, task.task.description, steps)
    }).collect::<Vec<_>>().join("\n\n")
}