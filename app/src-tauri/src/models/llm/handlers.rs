use tauri::{AppHandle, Manager};
use crate::events::{emitter::emit, types::*};
use crate::models::llm::{prompts::get_prompt, schemas::get_schema, server::generate};
use crate::tasks::{TaskService, TaskWithSteps};
use crate::db::{get_latest_activity_summary, insert_activity_summary, DbState};

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

pub async fn handle_summarize_screen(event: SummarizeScreenEvent, app_handle: &AppHandle) {
    // Get DB state
    let db_state = app_handle.state::<DbState>();

    // Fetch most recent screen summary
    let prev_summary = match get_latest_activity_summary(&db_state) {
        Ok(summary) => summary,
        Err(e) => {
            eprintln!("[summarize_screen] Failed to fetch latest activity summary: {}", e);
            None
        }
    };

    // Check if summary exists and is recent (within 10 minutes)
    let summary_text = match prev_summary {
        Some(summary) => {
            let created_at_str = summary.get("created_at")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            
            // Parse the SQLite datetime string (ISO 8601 format)
            let is_recent = match chrono::DateTime::parse_from_rfc3339(created_at_str) {
                Ok(created_at) => {
                    let ten_minutes_ago = chrono::Utc::now() - chrono::Duration::minutes(10);
                    created_at.with_timezone(&chrono::Utc) >= ten_minutes_ago
                }
                Err(_) => {
                    eprintln!("[summarize_screen] Failed to parse created_at timestamp: {}", created_at_str);
                    false
                }
            };

            if is_recent {
                summary.get("summary")
                    .and_then(|s| s.as_str())
                    .unwrap_or("No previous summary available")
                    .to_string()
            } else {
                println!("[summarize_screen] Previous summary is older than 10 minutes, treating as stale");
                "No recent summary available".to_string()
            }
        }
        None => {
            println!("[summarize_screen] No previous summary found in database");
            "No previous summary available".to_string()
        }
    };

    println!("[summarize_screen] Using previous summary: {}", summary_text);

    // Get prompt template
    let prompt_template = match get_prompt("summarize_screen") {
        Some(template) => template,
        None => {
            eprintln!("[summarize_screen] Failed to get prompt template for 'summarize_screen'");
            return;
        }
    };

    // Build prompt with replacements
    let active_url_str = event.active_url.as_deref().unwrap_or("No active URL");
    let prompt = prompt_template
        .replace("{text}", &event.text)
        .replace("{active_url}", active_url_str)
        .replace("{previous_summary}", &summary_text);

    // Get response schema
    let schema = match get_schema("summarize_screen") {
        Some(schema) => schema,
        None => {
            eprintln!("[summarize_screen] Failed to get schema for 'summarize_screen'");
            return;
        }
    };

    // Generate summary
    let summary_response = match generate(
        app_handle.clone(),
        prompt,
        Some(schema.to_string()),
        None,
        None,
        None,
    ).await {
        Ok(response) => {
            println!("[summarize_screen] Response received: {}", response);
            response
        }
        Err(e) => {
            eprintln!("[summarize_screen] Failed to generate summary: {}", e);
            return;
        }
    };

    // Parse JSON response
    let parsed_response = match serde_json::from_str::<serde_json::Value>(&summary_response) {
        Ok(json) => json,
        Err(e) => {
            eprintln!("[summarize_screen] Failed to parse JSON response: {}", e);
            return;
        }
    };

    // Extract summary text
    let summary_value = parsed_response.get("summary")
        .and_then(|s| s.as_str())
        .unwrap_or("No summary generated");

    // Prepare active applications JSON (only include apps with names)
    let active_applications: Vec<String> = event.data.iter()
        .filter_map(|app| app.application_name.as_ref())
        .cloned()
        .collect();

    let active_applications_json = match serde_json::to_string(&active_applications) {
        Ok(json) => json,
        Err(_) => {
            eprintln!("[summarize_screen] Failed to serialize active applications");
            "[]".to_string()
        }
    };

    // Save summary to database
    match insert_activity_summary(
        db_state.clone(),
        summary_value.to_string(),
        event.active_url,
        Some(active_applications_json),
    ) {
        Ok(id) => {
            println!("[summarize_screen] Summary saved to database successfully with ID: {}", id);
        }
        Err(e) => {
            eprintln!("[summarize_screen] Failed to save summary to database: {}", e);
        }
    }
}