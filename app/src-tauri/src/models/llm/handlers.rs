use tauri::{AppHandle, Manager};
use crate::events::{emitter::emit, types::*};
use crate::models::llm::{prompts::get_prompt, schemas::get_schema, server::generate};
use crate::tasks::{TaskService, TaskWithSteps};
use crate::db::DbState;

pub async fn handle_detect_tasks(event: DetectTasksEvent, app_handle: &AppHandle) {
    // Get DB state
    let db_state = app_handle.state::<DbState>();

    // Fetch all active tasks
    let active_tasks = TaskService::get_active_tasks(&db_state);
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
    let prompt_template = get_prompt("detect_tasks")
        .ok_or_else(|| {
            eprintln!("[detect_tasks] Failed to get prompt template for 'detect_tasks'");
            return;
        });
    
    let prompt_template = match prompt_template {
        Ok(template) => template,
        Err(_) => return,
    };
    
    let active_url_str = event.active_url.as_deref().unwrap_or("No active URL");
    let prompt = prompt_template
        .replace("{text}", &event.text)
        .replace("{active_url}", active_url_str)
        .replace("{tasks}", &formatted_tasks);

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

    // Handle response and parse JSON
    let parsed_response = match task_updates {
        Ok(response) => {
            println!("[detect_tasks] Response received: {}", response);
            serde_json::from_str(&response)
                .unwrap_or_else(|_| serde_json::json!({}))
        }
        Err(e) => {
            eprintln!("[detect_tasks] Failed to generate task updates: {}", e);
            return;
        }
    };

    // Loop through response and update step statuses
    if let Some(completed_ids) = parsed_response.get("completed").and_then(|c| c.as_array()) {
        for step_id_value in completed_ids {
            if let Some(step_id) = step_id_value.as_u64() {
                println!("[detect_tasks] Updating step {} to status: completed", step_id);
                // Update step status in database
                // Don't actually update for now
                // if let Err(e) = TaskService::update_step_status(&db_state, step_id as i64, StepStatus::Completed) {
                //     eprintln!("[detect_tasks] Failed to update step {}: {}", step_id, e);
                // }
            } else {
                eprintln!("[detect_tasks] Invalid step_id format in completed array: {:?}", step_id_value);
            }
        }
    } else {
        println!("[detect_tasks] No completed step IDs found in response");
    }

    // Emit update tasks event
    let update_event = UpdateTasksEvent {
        timestamp: chrono::Utc::now().to_string()
    };
    let _ = emit(UPDATE_TASKS, update_event);
}

fn format_tasks(tasks: &[TaskWithSteps]) -> String {
    tasks.iter().map(|task| {
        let steps = task.steps.iter().map(|step| {
            format!("\tStep: {}, ID: {}, Description: {}, Status: {}", step.title, step.id, step.description, step.status)
        }).collect::<Vec<_>>().join("\n");

        format!("Task {},  Description: {}, Steps: [\n{}\n]", task.task.name, task.task.description, steps)
    }).collect::<Vec<_>>().join("\n\n")
}