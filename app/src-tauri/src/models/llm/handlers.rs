use crate::db::activity::{get_latest_activity_summary, insert_activity_summary};
use crate::db::conversations::{add_message, add_message_with_id, update_conversation_name};
use crate::db::core::DbState;
use crate::db::memory::find_similar_memories;
use crate::events::{emitter::emit, types::*};
use crate::models::llm::{client::generate, prompts::get_prompt, schemas::get_schema, types::LlmRequest};
use crate::tasks::{TaskService, TaskWithSteps};
use tauri::{AppHandle, Manager};

pub async fn handle_detect_tasks(app_handle: &AppHandle, event: DetectTasksEvent) -> Result<(), String> {
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
    return Err("Failed to fetch active tasks".into());
  }

  // Return if no active tasks
  let tasks = active_tasks.unwrap();
  if tasks.is_empty() {
    log::info!("[detect_tasks] No active tasks found");
    return Ok(());
  }

  // Format tasks for prompt
  let formatted_tasks = format_tasks(&tasks);

  // Create prompt
  let prompt_template = match get_prompt("detect_tasks") {
    Some(template) => template,
    None => {
      log::error!("[detect_tasks] Failed to get prompt template for 'detect_tasks'");
      return Err("Failed to get prompt template for 'detect_tasks'".into());
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
  let schema = match get_schema("detect_tasks") {
    Some(schema) => schema,
    None => {
      log::error!("[detect_tasks] Failed to get schema for 'detect_tasks'");
      return Err("Failed to get schema for 'detect_tasks'".into());
    }
  };

  // Generate task updates
  let parsed_response =
    match generate_and_parse_response(app_handle.clone(), prompt, schema, "[detect_tasks]").await {
      Some(response) => response,
      None => return Err("Failed to generate and parse response for 'detect_tasks'".into()),
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
    timestamp: chrono::Utc::now().to_rfc3339(),
  };
  let _ = emit(UPDATE_TASKS, update_event);
  Ok(())
}

pub async fn handle_summarize_screen(app_handle: &AppHandle, event: SummarizeScreenEvent) -> Result<(), String> {
  // Get DB state
  let db_state = app_handle.state::<DbState>();

  // Fetch most recent screen summary
  let summary_text = get_recent_summary_text(&db_state, "[summarize_screen]");

  // Get prompt template
  let prompt_template = match get_prompt("summarize_screen") {
    Some(template) => template,
    None => {
      log::error!("[summarize_screen] Failed to get prompt template for 'summarize_screen'");
      return Err("Failed to get prompt template for 'summarize_screen'".into());
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
      return Err("Failed to get schema for 'summarize_screen'".into());
    }
  };

  // Generate summary
  let parsed_response =
    match generate_and_parse_response(app_handle.clone(), prompt, schema, "[summarize_screen]")
      .await
    {
      Some(response) => response,
      None => return Err("Failed to generate and parse response for 'summarize_screen'".into()),
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
      return Err("Failed to save activity summary".into());
    }
  }
  Ok(())
}

#[tauri::command]
pub async fn handle_hud_chat(app_handle: AppHandle, event: HudChatEvent) -> Result<String, String> {
  // Save the user message to the database
  let _user_message = match add_message_with_id(
    app_handle.clone(),
    event.conv_id.clone(),
    "user".to_string(),
    event.text.clone(),
    Some(event.message_id.clone()),
  )
  .await
  {
    Ok(message) => Some(message),
    Err(e) => {
      log::error!("[hud_chat] Failed to save user message: {}", e);
      None
    }
  };

  // Emit extract memory event
  let extract_event = ExtractInteractiveMemoryEvent {
    message: event.text.clone(),
    message_id: event.message_id.clone(),
    timestamp: chrono::Utc::now().to_rfc3339(),
  };
  let _ = emit(EXTRACT_INTERACTIVE_MEMORY, extract_event);

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

  let system_prompt = system_prompt_template.replace("{currentDateTime}", &current_date_time);

  // Get 3 most relevant memories
  let relevant_memories =
    match find_similar_memories(&app_handle.clone(), &event.text, 3, 0.5).await {
      Ok(memories) => memories,
      Err(e) => {
        log::warn!("[hud_chat] Failed to find similar memories: {}", e);
        Vec::new()
      }
    };

  // Create memory context string
  let mut memory_context = String::new();
  if !relevant_memories.is_empty() {
    memory_context.push_str("Here are some relevant memories you have of our past interactions:\n");
    for memory in relevant_memories {
      memory_context.push_str(&format!("- {}\n", memory.text));
    }
    memory_context.push_str("\n");
  }

  // Combine OCR responses into a single string
  let mut ocr_text = String::new();
  if !event.ocr_responses.is_empty() {
    for ocr_response in event.ocr_responses.iter() {
      ocr_text.push_str(&format!("{}\n", ocr_response.text));
    }
    if !ocr_text.is_empty() {
      ocr_text = format!(
        "\nHere's text captured from my screen as context:\n{}\n",
        ocr_text
      );
    }
  }

  // Create user prompt with ocr data and memory context
  let user_prompt = format!(
    "{}{}Task: {}",
    if memory_context.is_empty() {
      ""
    } else {
      &format!("{}\n", memory_context)
    },
    if ocr_text.is_empty() {
      ""
    } else {
      &format!("{}\n", ocr_text)
    },
    event.text
  );

  // Generate response
  let request = LlmRequest::new(user_prompt)
    .with_system_prompt(Some(system_prompt))
    .with_conv_id(Some(event.conv_id.clone()))
    .with_use_thinking(Some(false))
    .with_stream(Some(true))
    .with_current_message_id(Some(event.message_id.clone()));

  let response = match generate(app_handle.clone(), request, None).await {
    Ok(response) => {
      response
    }
    Err(e) => {
      log::error!("[hud_chat] Failed to generate response: {}", e);
      return Err("Failed to generate response".into());
    }
  };

  // Save the assistant response to the database
  if let Err(e) = add_message(
    app_handle.clone(),
    event.conv_id.clone(),
    "assistant".to_string(),
    response.clone(),
  )
  .await
  {
    log::error!("[hud_chat] Failed to save assistant message: {}", e);
  }

  // Return response
  Ok(response)
}

pub async fn handle_generate_conversation_name(
  app_handle: &AppHandle,
  event: GenerateConversationNameEvent,
) -> Result<(), String> {
  log::info!(
    "[generate_conversation_name] Generating name for conversation ID: {}",
    event.conv_id
  );
  
  // Load system prompt
  let system_prompt = match get_prompt("generate_conversation_name") {
    Some(p) => p.to_string(),
    None => {
      log::error!("[generate_conversation_name] Missing system prompt: generate_conversation_name");
      return Err("Missing system prompt: generate_conversation_name".into());
    }
  };

  // Load schema
  let schema = match get_schema("generate_conversation_name") {
    Some(s) => Some(s.to_string()),
    None => {
      log::error!("[generate_conversation_name] Missing schema: generate_conversation_name");
      return Err("Missing schema: generate_conversation_name".into());
    }
  };

  // Generate name
  let request = LlmRequest::new(event.message.clone())
    .with_system_prompt(Some(system_prompt))
    .with_json_schema(schema)
    .with_use_thinking(Some(false))
    .with_stream(Some(false));

  let generated_name = match generate(app_handle.clone(), request, Some(true)).await {
    Ok(generated) => {
      log::info!("[generate_conversation_name] Generated conversation name");
      generated
    }
    Err(e) => {
      log::error!("[generate_conversation_name] Failed to generate conversation name: {}", e);
      return Err("Failed to generate conversation name".into());
    }
  };

  // Extract the name text
  let extracted_name = match serde_json::from_str::<serde_json::Value>(&generated_name) {
    Ok(json) => match json.get("name") {
      Some(name_value) => match name_value.as_str() {
        Some(name_text) => name_text.to_string(),
        None => {
          log::error!("[generate_conversation_name] Name field is not a string");
          return Err("Name field is not a string".into());
        }
      },
      None => {
        log::error!("[generate_conversation_name] No 'name' field found in JSON response");
        return Err("No 'name' field found in JSON response".into());
      }
    },
    Err(e) => {
      log::error!("[generate_conversation_name] Failed to parse JSON response: {}", e);
      return Err("Failed to parse JSON response".into());
    }
  };

  // Skip if extracted name is empty
  if extracted_name.trim().is_empty() {
    log::info!("[generate_conversation_name] Extracted name is empty, skipping save");
    return Ok(());
  }

  // Save to db
  match update_conversation_name(app_handle.clone(), event.conv_id.clone(), extracted_name.clone()).await {
    Ok(_) => {}
    Err(e) => {
      log::error!(
        "[generate_conversation_name] Failed to rename conversation {}: {}",
        event.conv_id,
        e
      );
    }
  };

  // Emit name change event
  let name_event = RenameConversationEvent {
    conv_id: event.conv_id,
    new_name: extracted_name,
    timestamp: chrono::Utc::now().to_rfc3339(),
  };
  let _ = emit(RENAME_CONVERSATION, name_event);
  Ok(())
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
  let request = LlmRequest::new(prompt)
    .with_json_schema(Some(schema.to_string()))
    .with_use_thinking(Some(true));

  let response = match generate(app_handle, request, Some(true)).await {
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
