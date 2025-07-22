use tauri::{AppHandle, Manager};
use crate::events::{emitter::emit, types::*};
use crate::models::llm::{prompts::get_prompt, schemas::get_schema, server::generate};
use crate::tasks::{TaskService, TaskWithSteps};
use crate::db::{get_latest_activity_summary, insert_activity_summary, DbState};

pub async fn handle_detect_tasks(event: DetectTasksEvent, app_handle: &AppHandle) {
    // Get DB state
    let db_state = app_handle.state::<DbState>();

    // Fetch most recent screen summary
    let summary_text = get_recent_summary_text(&db_state, "[detect_tasks]");

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
    let prompt_template = match get_prompt("detect_tasks") {
        Some(template) => template,
        None => {
            eprintln!("[detect_tasks] Failed to get prompt template for 'detect_tasks'");
            return;
        }
    };
    
    let active_url_str = event.active_url.as_deref().unwrap_or("No active URL");
    let prompt = prompt_template
        .replace("{text}", &event.text)
        .replace("{active_url}", active_url_str)
        .replace("{previous_summary}", &summary_text)
        .replace("{tasks}", &formatted_tasks);

    println!("[detect_tasks] Generated prompt:\n{}", prompt);

    // Get response schema
    let schema = get_schema("detect_tasks").unwrap_or("{}");

    // Generate task updates
    let parsed_response = match generate_and_parse_response(
        app_handle.clone(),
        prompt,
        schema,
        "[detect_tasks]"
    ).await {
        Some(response) => response,
        None => return,
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

pub async fn handle_summarize_screen(event: SummarizeScreenEvent, app_handle: &AppHandle) {
    // Get DB state
    let db_state = app_handle.state::<DbState>();

    // Fetch most recent screen summary
    let summary_text = get_recent_summary_text(&db_state, "[summarize_screen]");

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
    let parsed_response = match generate_and_parse_response(
        app_handle.clone(),
        prompt,
        schema,
        "[summarize_screen]"
    ).await {
        Some(response) => response,
        None => return,
    };

    // Extract summary text
    let summary_value = parsed_response.get("summary")
        .and_then(|s| s.as_str())
        .unwrap_or("No summary generated");

    // Prepare active applications JSON (only include apps with names)
    let active_applications_json = serialize_active_applications(&event.data);

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

// Helper functions

/// Formats tasks with their steps for use in prompts
fn format_tasks(tasks: &[TaskWithSteps]) -> String {
    tasks.iter().map(|task| {
        let steps = task.steps.iter().map(|step| {
            format!("\tStep: {}, ID: {}, Description: {}, Status: {}", step.title, step.id, step.description, step.status)
        }).collect::<Vec<_>>().join("\n");

        format!("Task {},  Description: {}, Steps: [\n{}\n]", task.task.name, task.task.description, steps)
    }).collect::<Vec<_>>().join("\n\n")
}

/// Fetches the most recent activity summary and returns the summary text if it's recent (within 10 minutes)
fn get_recent_summary_text(db_state: &tauri::State<DbState>, log_prefix: &str) -> String {
    let prev_summary = match get_latest_activity_summary(db_state) {
        Ok(summary) => summary,
        Err(e) => {
            eprintln!("{} Failed to fetch latest activity summary: {}", log_prefix, e);
            None
        }
    };

    match prev_summary {
        Some(summary) => {
            let created_at_str = summary.get("created_at")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            
            // Parse the SQLite datetime string (format: "2025-07-21 21:22:24")
            let is_recent = match chrono::NaiveDateTime::parse_from_str(created_at_str, "%Y-%m-%d %H:%M:%S") {
                Ok(naive_dt) => {
                    // Convert to UTC DateTime
                    let created_at_utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive_dt, chrono::Utc);
                    let ten_minutes_ago = chrono::Utc::now() - chrono::Duration::minutes(10);
                    created_at_utc >= ten_minutes_ago
                }
                Err(_) => {
                    eprintln!("{} Failed to parse created_at timestamp: {}", log_prefix, created_at_str);
                    false
                }
            };

            if is_recent {
                summary.get("summary")
                    .and_then(|s| s.as_str())
                    .unwrap_or("No previous summary available")
                    .to_string()
            } else {
                println!("{} Previous summary is older than 10 minutes, treating as stale", log_prefix);
                "No recent summary available".to_string()
            }
        }
        None => {
            println!("{} No previous summary found in database", log_prefix);
            "No previous summary available".to_string()
        }
    }
}

/// Generates a response using the LLM and parses it as JSON
async fn generate_and_parse_response(
    app_handle: AppHandle,
    prompt: String,
    schema: &str,
    log_prefix: &str,
) -> Option<serde_json::Value> {
    // Generate response
    let response = match generate(
        app_handle,
        prompt,
        Some(schema.to_string()),
        None,
        None,
        None,
    ).await {
        Ok(response) => {
            println!("{} Response received: {}", log_prefix, response);
            response
        }
        Err(e) => {
            eprintln!("{} Failed to generate response: {}", log_prefix, e);
            return None;
        }
    };

    // Parse JSON response
    match serde_json::from_str::<serde_json::Value>(&response) {
        Ok(json) => Some(json),
        Err(e) => {
            eprintln!("{} Failed to parse JSON response: {}", log_prefix, e);
            None
        }
    }
}

/// Serializes active applications to JSON string, filtering out apps without names
fn serialize_active_applications(data: &[crate::os_utils::windows::window::ApplicationTextData]) -> String {
    let active_applications: Vec<String> = data.iter()
        .filter_map(|app| app.application_name.as_ref())
        .cloned()
        .collect();

    match serde_json::to_string(&active_applications) {
        Ok(json) => json,
        Err(_) => {
            eprintln!("[serialize_active_applications] Failed to serialize active applications");
            "[]".to_string()
        }
    }
}