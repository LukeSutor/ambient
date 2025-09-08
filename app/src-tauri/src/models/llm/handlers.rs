use crate::db::activity::{get_latest_activity_summary, insert_activity_summary};
use crate::db::core::DbState;
use crate::events::{emitter::emit, types::*};
use crate::models::llm::{prompts::get_prompt, schemas::get_schema, client::generate};
use crate::tasks::{TaskService, TaskWithSteps};
use tauri::{AppHandle, Manager};

pub async fn handle_detect_tasks(event: DetectTasksEvent, app_handle: &AppHandle) {
  // Get DB state
  let db_state = app_handle.state::<DbState>();

  // Fetch most recent screen summary
  let summary_text = get_recent_summary_text(&db_state, "[detect_tasks]");

  // Fetch all active tasks
  let active_tasks = TaskService::get_active_tasks(&db_state);
  if active_tasks.is_err() {
    log::error!(
      "[detect_tasks] Failed to fetch active tasks: {}",
      active_tasks.err().unwrap()
    );
    return;
  }

  // Return if no active tasks
  let tasks = active_tasks.unwrap();
  if tasks.is_empty() {
    log::info!("[detect_tasks] No active tasks found");
    return;
  }

  // Format tasks for prompt
  let formatted_tasks = format_tasks(&tasks);

  // Create prompt
  let prompt_template = match get_prompt("detect_tasks") {
    Some(template) => template,
    None => {
      log::error!("[detect_tasks] Failed to get prompt template for 'detect_tasks'");
      return;
    }
  };

  let active_url_str = event.active_url.as_deref().unwrap_or("No active URL");
  let prompt = prompt_template
    .replace("{text}", &event.text)
    .replace("{active_url}", active_url_str)
    .replace("{previous_summary}", &summary_text)
    .replace("{tasks}", &formatted_tasks);

  log::debug!("[detect_tasks] Generated prompt:\n{}", prompt);

  // Get response schema
  let schema = get_schema("detect_tasks").unwrap_or("{}");

  // Generate task updates
  let parsed_response =
    match generate_and_parse_response(app_handle.clone(), prompt, schema, "[detect_tasks]").await {
      Some(response) => response,
      None => return,
    };

  // Loop through response and update step statuses
  if let Some(completed_ids) = parsed_response.get("completed").and_then(|c| c.as_array()) {
    for step_id_value in completed_ids {
      if let Some(_step_id) = step_id_value.as_u64() {
        // Update step status implementation would go here
      }
    }
  } else {
    log::info!("[detect_tasks] No completed step IDs found in response");
  }

  // Emit update tasks event
  let update_event = UpdateTasksEvent {
    timestamp: chrono::Utc::now().to_string(),
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
      log::error!("[summarize_screen] Failed to get prompt template for 'summarize_screen'");
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
      log::error!("[summarize_screen] Failed to get schema for 'summarize_screen'");
      return;
    }
  };

  // Generate summary
  let parsed_response =
    match generate_and_parse_response(app_handle.clone(), prompt, schema, "[summarize_screen]")
      .await
    {
      Some(response) => response,
      None => return,
    };

  // Extract summary text
  let summary_value = parsed_response
    .get("summary")
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
      log::info!("[summarize_screen] Saved activity summary with ID: {}", id);

      // Update the screen state buffer with the generated summary
      {
        use crate::os_utils::handlers::SCREEN_STATE_BUFFER;
        let mut buffer_guard = SCREEN_STATE_BUFFER.lock().unwrap();
        buffer_guard.update_latest_summary(summary_value.to_string());
      }
    }
    Err(e) => {
      log::error!("[summarize_screen] Failed to save activity summary: {}", e);
    }
  }
}

#[tauri::command]
pub async fn handle_hud_chat(app_handle: AppHandle, event: HudChatEvent) -> Result<String, String> {
  // Create prompt
  let system_prompt_template = match get_prompt("hud_chat") {
    Some(template) => template,
    None => {
      log::error!("[hud_chat] Failed to get prompt template for 'hud_chat'");
      return Err("Failed to get prompt template for 'hud_chat'".into());
    }
  };

  // Get the current date time YYYY-MM-DD format
  let current_date_time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

  let system_prompt = system_prompt_template
    .replace("{currentDateTime}", &current_date_time);

  log::debug!("[hud_chat] Generated system prompt:\n{}", system_prompt);

  // Combine OCR responses into a single string
  let mut ocr_text = String::new();
  if !event.ocr_responses.is_empty() {
    for (i, ocr_response) in event.ocr_responses.iter().enumerate() {
      ocr_text.push_str(&format!("<cap_{}>{}</cap_{}>\n", i + 1, ocr_response.text, i + 1));
    }
    if !ocr_text.is_empty() {
      ocr_text = format!("\nHere's text captured from my screen as context:\n{}\n", ocr_text);
    }
  }

  // Create user prompt with ocr data
  let user_prompt = format!("{}\n{}", event.text, ocr_text).trim().to_string();

  log::debug!("[hud_chat] Generated user prompt:\n{}", user_prompt);

  // Generate response
  let response = match generate(
    app_handle.clone(),
    user_prompt,
    Some(system_prompt),
    None,
    event.conv_id,
    Some(false),
    Some(true),
    None,
  )
  .await
  {
    Ok(response) => {
      log::debug!("[hud_chat] response: {}", response);
      response
    }
    Err(e) => {
      log::error!("[hud_chat] Failed to generate response: {}", e);
      return Err("Failed to generate response".into());
    }
  };

  // Emit extract memory event
  //TODO: chat messages must be saved here to get an ID to link memory to
  let extract_event = ExtractInteractiveMemoryEvent {
    message: event.text,
    message_id: uuid::Uuid::new_v4().to_string(),
    timestamp: chrono::Utc::now().to_string(),
  };

  // Return response
  Ok(response)
}

// Helper functions

/// Formats tasks with their steps for use in prompts
pub fn format_tasks(tasks: &[TaskWithSteps]) -> String {
  tasks
    .iter()
    .map(|task| {
      let steps = task
        .steps
        .iter()
        .map(|step| {
          format!(
            "\tStep: {}, ID: {}, Description: {}, Status: {}",
            step.title, step.id, step.description, step.status
          )
        })
        .collect::<Vec<_>>()
        .join("\n");

      format!(
        "Task {},  Description: {}, Steps: [\n{}\n]",
        task.task.name, task.task.description, steps
      )
    })
    .collect::<Vec<_>>()
    .join("\n\n")
}

/// Fetches the most recent activity summary and returns the summary text if it's recent (within 10 minutes)
fn get_recent_summary_text(db_state: &tauri::State<DbState>, log_prefix: &str) -> String {
  let prev_summary = match get_latest_activity_summary(db_state) {
    Ok(summary) => summary,
    Err(e) => {
      log::error!(
        "{} Failed to fetch latest activity summary: {}",
        log_prefix,
        e
      );
      return "No previous summary available".to_string();
    }
  };

  match prev_summary {
    Some(summary) => {
      let summary_text = summary
        .get("summary")
        .and_then(|s| s.as_str())
        .unwrap_or("No summary text found");

      // Check if summary is recent (within 10 minutes)
      if let Some(timestamp_str) = summary.get("timestamp").and_then(|t| t.as_str()) {
        if let Ok(timestamp) = chrono::DateTime::parse_from_rfc3339(timestamp_str) {
          let now = chrono::Utc::now();
          let duration = now.signed_duration_since(timestamp);

          if duration.num_minutes() <= 10 {
            return summary_text.to_string();
          } else {
            log::debug!(
              "{} Previous summary is too old ({} minutes), using default",
              log_prefix,
              duration.num_minutes()
            );
          }
        }
      }

      "No recent summary available".to_string()
    }
    None => {
      log::debug!("{} No previous summary found in database", log_prefix);
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
    None,
    Some(schema.to_string()),
    None,
    None,
    None,
    Some(true),
  )
  .await
  {
    Ok(response) => {
      log::debug!("{} LLM response: {}", log_prefix, response);
      response
    }
    Err(e) => {
      log::error!("{} Failed to generate LLM response: {}", log_prefix, e);
      return None;
    }
  };

  // Parse JSON response
  match serde_json::from_str::<serde_json::Value>(&response) {
    Ok(json) => Some(json),
    Err(e) => {
      log::error!("{} Failed to parse LLM response as JSON: {}", log_prefix, e);
      None
    }
  }
}

/// Serializes active applications to JSON string, filtering out apps without names
fn serialize_active_applications(
  data: &[crate::os_utils::windows::window::ApplicationTextData],
) -> String {
  let active_applications: Vec<String> = data
    .iter()
    .filter_map(|app| app.application_name.as_ref())
    .cloned()
    .collect();

  match serde_json::to_string(&active_applications) {
    Ok(json) => json,
    Err(_) => "[]".to_string(),
  }
}
