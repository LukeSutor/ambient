use crate::events::emitter::emit;
use crate::events::types::*;
use crate::os_utils::windows::window::{
  format_as_markdown, get_brave_url, get_screen_text, ApplicationTextData,
};
use chrono;
use once_cell::sync::Lazy;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};

// Struct to hold screen state with associated summary
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScreenStateWithSummary {
  pub data: Vec<ApplicationTextData>,
  pub active_url: Option<String>,
  pub timestamp: String,
  pub summary: Option<String>, // Summary generated from this state
}

// Struct to hold evaluation data for capture
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvalData {
  pub timestamp: String,
  pub prev_prev_screen_state: ScreenStateWithSummary,
  pub prev_screen_state: ScreenStateWithSummary,
  pub screen_diff_markdown: String,
  pub formatted_screen_state: String,
  pub prev_prev_summary: String,
  pub active_tasks: Vec<crate::tasks::TaskWithSteps>,
  pub formatted_tasks: String,
  pub ground_truth_completed_step_ids: Vec<i64>, // To be filled manually
}

// Buffer to maintain the last 3 screen states
#[derive(Debug, Clone)]
pub struct ScreenStateBuffer {
  states: VecDeque<ScreenStateWithSummary>,
}

impl ScreenStateBuffer {
  fn new() -> Self {
    Self {
      states: VecDeque::with_capacity(3),
    }
  }

  fn push(&mut self, state: ScreenStateWithSummary) {
    if self.states.len() >= 3 {
      self.states.pop_front();
    }
    self.states.push_back(state);
  }

  fn get_prev_prev(&self) -> Option<&ScreenStateWithSummary> {
    if self.states.len() >= 2 {
      self.states.get(self.states.len() - 2)
    } else {
      None
    }
  }

  fn get_prev(&self) -> Option<&ScreenStateWithSummary> {
    self.states.back()
  }

  pub fn update_latest_summary(&mut self, summary: String) {
    if let Some(latest) = self.states.back_mut() {
      latest.summary = Some(summary);
    }
  }
}

// Thread-safe static variable using Lazy and Arc<Mutex<T>>
pub static SCREEN_STATE_BUFFER: Lazy<Arc<Mutex<ScreenStateBuffer>>> =
  Lazy::new(|| Arc::new(Mutex::new(ScreenStateBuffer::new())));

pub async fn handle_capture_screen(_event: CaptureScreenEvent, app_handle: &AppHandle) {
  let data = match get_screen_text(app_handle.clone()).await {
    Ok(data) => data,
    Err(e) => {
      log::error!("[capture_screen] Failed to capture text: {}", e);
      return;
    }
  };

  let url = match get_brave_url() {
    Ok(url) => url,
    Err(e) => {
      log::error!("[capture_screen] Failed to get browser URL: {}", e);
      return;
    }
  };

  let timestamp = chrono::Utc::now().to_rfc3339();

  // Emit get screen diff event
  if let Err(e) = emit(
    GET_SCREEN_DIFF,
    GetScreenDiffEvent {
      data,
      active_url: Some(url),
      timestamp,
    },
  ) {
    log::error!(
      "[get_screen_diff] Failed to emit GET_SCREEN_DIFF event: {}",
      e
    );
  }
  log::info!("[capture_screen] Emitted GET_SCREEN_DIFF event with text and URL");
}

pub async fn handle_get_screen_diff(event: GetScreenDiffEvent, _app_handle: &AppHandle) {
  // Fetch the previous screen state in a thread-safe way
  let previous_state = {
    let buffer_guard = SCREEN_STATE_BUFFER.lock().unwrap();
    buffer_guard.get_prev().cloned()
  };

  let new_data = event.data.clone();

  // Iterate over the new data and compare with previous state
  let mut changes: Vec<ApplicationTextData> = Vec::new();
  if let Some(prev_state) = &previous_state {
    for new_app in &new_data {
      // Check if this application was in the previous state
      if let Some(prev_app) = prev_state
        .data
        .iter()
        .find(|&app| app.process_id == new_app.process_id)
      {
        // Convert previous text lines to a HashSet for O(1) lookups
        let prev_lines: HashSet<&String> = prev_app.text_content.iter().collect();

        // Check for new lines not in previous state
        let new_lines: Vec<String> = new_app
          .text_content
          .iter()
          .filter(|line| !prev_lines.contains(line))
          .cloned()
          .collect();

        if !new_lines.is_empty() {
          changes.push(ApplicationTextData {
            process_id: new_app.process_id,
            process_name: new_app.process_name.clone(),
            application_name: new_app.application_name.clone(),
            text_content: new_lines,
          });
        }
      } else {
        changes.push(ApplicationTextData {
          process_id: new_app.process_id,
          process_name: new_app.process_name.clone(),
          application_name: new_app.application_name.clone(),
          text_content: new_app.text_content.clone(),
        });
      }
    }
  } else {
    // No previous state, consider all as changes
    changes = new_data.clone();
  }

  // Add current state to buffer (summary will be added later)
  {
    let mut buffer_guard = SCREEN_STATE_BUFFER.lock().unwrap();
    buffer_guard.push(ScreenStateWithSummary {
      data: new_data.clone(),
      active_url: event.active_url.clone(),
      timestamp: event.timestamp.clone(),
      summary: None,
    });
  }

  // Return if there are no changes
  if changes.is_empty() {
    log::info!("[get_screen_diff] No changes detected");
    return;
  }

  // Format changes as markdown
  let markdown = format_as_markdown(changes);

  let timestamp = chrono::Utc::now().to_rfc3339();

  // Emit detect tasks event
  if let Err(e) = emit(
    DETECT_TASKS,
    DetectTasksEvent {
      text: markdown.clone(),
      active_url: event.active_url.clone(),
      timestamp: timestamp.clone(),
    },
  ) {
    log::error!("[get_screen_diff] Failed to emit DETECT_TASKS event: {}", e);
  }
  log::info!("[capture_screen] Emitted DETECT_TASKS event");

  // Emit summarize screen event
  if let Err(e) = emit(
    SUMMARIZE_SCREEN,
    SummarizeScreenEvent {
      text: markdown,
      data: event.data,
      active_url: event.active_url,
      timestamp,
    },
  ) {
    log::error!(
      "[get_screen_diff] Failed to emit SUMMARIZE_SCREEN event: {}",
      e
    );
  }
  log::info!("[capture_screen] Emitted SUMMARIZE_SCREEN event");
}

/// Capture evaluation data for the task detection functionality
/// This saves the previous task detection loop's data for evaluation
#[tauri::command]
pub async fn capture_eval_data(app_handle: AppHandle) -> Result<String, String> {
  use crate::db::core::DbState;
  use crate::models::llm::handlers::format_tasks;
  use crate::tasks::TaskService;

  // Get the current buffer states
  let (prev_prev_state, prev_state) = {
    let buffer_guard = SCREEN_STATE_BUFFER.lock().unwrap();
    let prev_prev = buffer_guard.get_prev_prev().cloned();
    let prev = buffer_guard.get_prev().cloned();
    (prev_prev, prev)
  };

  // Get previous state formatted as markdown
  let formatted_screen_state = if let Some(prev_state) = &prev_state {
    format_as_markdown(prev_state.data.clone())
  } else {
    return Err("No previous screen state available to capture eval data".to_string());
  };

  // Ensure we have the required states
  let prev_prev_state =
    prev_prev_state.ok_or("Need at least 2 screen states to capture eval data")?;
  let prev_state = prev_state.ok_or("Need current screen state to capture eval data")?;

  // Get DB state and fetch active tasks
  let db_state = app_handle.state::<DbState>();
  let active_tasks = TaskService::get_active_tasks(&db_state)
    .map_err(|e| format!("Failed to fetch active tasks: {}", e))?;

  if active_tasks.is_empty() {
    return Err("No active tasks found - cannot create meaningful eval data".to_string());
  }

  // Format tasks for prompt
  let formatted_tasks = format_tasks(&active_tasks);

  // Calculate screen diff between prev_prev and prev states
  let mut changes: Vec<ApplicationTextData> = Vec::new();
  for new_app in &prev_state.data {
    if let Some(prev_app) = prev_prev_state
      .data
      .iter()
      .find(|&app| app.process_id == new_app.process_id)
    {
      let prev_lines: HashSet<&String> = prev_app.text_content.iter().collect();
      let new_lines: Vec<String> = new_app
        .text_content
        .iter()
        .filter(|line| !prev_lines.contains(line))
        .cloned()
        .collect();

      if !new_lines.is_empty() {
        changes.push(ApplicationTextData {
          process_id: new_app.process_id,
          process_name: new_app.process_name.clone(),
          application_name: new_app.application_name.clone(),
          text_content: new_lines,
        });
      }
    } else {
      changes.push(ApplicationTextData {
        process_id: new_app.process_id,
        process_name: new_app.process_name.clone(),
        application_name: new_app.application_name.clone(),
        text_content: new_app.text_content.clone(),
      });
    }
  }

  let screen_diff_markdown = format_as_markdown(changes);

  // Get prev-prev summary from the prev_prev_state
  let prev_prev_summary = if let Some(summary) = &prev_prev_state.summary {
    summary.clone()
  } else {
    return Err("Previous screen state summary is missing".to_string());
  };

  // Create eval data structure
  let eval_data = EvalData {
    timestamp: chrono::Utc::now().to_rfc3339(),
    prev_prev_screen_state: prev_prev_state,
    prev_screen_state: prev_state,
    screen_diff_markdown,
    formatted_screen_state,
    prev_prev_summary,
    active_tasks,
    formatted_tasks,
    ground_truth_completed_step_ids: Vec::new(), // To be filled manually
  };

  // Create filename with timestamp
  let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
  let filename = format!("eval_{}.json", timestamp);

  // Get the project root directory by navigating from the current executable location
  let current_exe =
    std::env::current_exe().map_err(|e| format!("Failed to get current executable path: {}", e))?;

  // Navigate up from the executable to find the project root
  // The executable is typically in target/debug/ or target/release/
  let mut project_root = current_exe;

  // Go up until we find a directory that contains both "app" and "evals" folders
  for _ in 0..10 {
    // Limit iterations to prevent infinite loop
    project_root = project_root
      .parent()
      .ok_or("Could not find project root")?
      .to_path_buf();

    if project_root.join("app").exists() && project_root.join("evals").exists() {
      break;
    }

    // Alternative: look for directories that suggest we're in the right place
    if project_root.join("app").exists() {
      // Create evals directory if it doesn't exist but app does
      break;
    }
  }

  let evals_dir = project_root
    .join("evals")
    .join("task-detection")
    .join("data");

  // Create directory if it doesn't exist
  fs::create_dir_all(&evals_dir).map_err(|e| format!("Failed to create evals directory: {}", e))?;

  let file_path = evals_dir.join(&filename);

  // Serialize and write to file
  let json_content = serde_json::to_string_pretty(&eval_data)
    .map_err(|e| format!("Failed to serialize eval data: {}", e))?;

  fs::write(&file_path, json_content).map_err(|e| format!("Failed to write eval file: {}", e))?;

  let file_path_str = file_path.to_string_lossy().to_string();
  log::info!("[eval] Saved evaluation data to: {}", file_path_str);

  Ok(format!("Evaluation data saved to: {}", filename))
}
