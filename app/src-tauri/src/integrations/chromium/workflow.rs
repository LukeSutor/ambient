use crate::DbState;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
  pub event_type: String,
  pub payload: serde_json::Value,
  pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
  pub url: String,
  pub steps: Vec<WorkflowStep>,
  pub recording_start: i64,
  pub recording_end: Option<i64>,
}

// Global workflows map: url -> Workflow
lazy_static::lazy_static! {
    pub static ref WORKFLOWS: DashMap<String, Workflow> = DashMap::new();
}

// Start a new workflow for a URL
pub fn start_workflow(url: &str, open_event: WorkflowStep) {
  let wf = Workflow {
    url: url.to_string(),
    steps: vec![open_event.clone()],
    recording_start: open_event.timestamp,
    recording_end: None,
  };
  WORKFLOWS.insert(url.to_string(), wf);
  log::info!("[chromium/workflow] Started workflow for {}", url);
}

// Append a step to the workflow for a URL
pub fn append_step(url: &str, step: WorkflowStep) {
  if let Some(mut wf) = WORKFLOWS.get_mut(url) {
    log::debug!(
      "[chromium/workflow] Appending step to workflow for {}: {:?}",
      url, step
    );
    wf.steps.push(step);
  } else {
    log::warn!(
      "[chromium/workflow] Tried to append step for {}, but no workflow exists",
      url
    );
  }
}

// Save workflow to DB and remove from memory
pub fn save_workflow(url: &str, db_state: tauri::State<DbState>) -> Result<(), String> {
  if let Some((_key, mut wf)) = WORKFLOWS.remove(url) {
    log::info!("[chromium/workflow] Saving workflow for {}", url);
    wf.recording_end = wf.steps.last().map(|s| s.timestamp);
    let steps_json = serde_json::to_string(&wf.steps).map_err(|e| e.to_string())?;
    let now = wf.recording_end.unwrap_or(wf.recording_start);

    crate::db::insert_workflow(
      db_state,
      format!("Workflow for {}", url),
      Some(format!("Recorded on {}", url)),
      url.to_string(),
      steps_json,
      wf.recording_start,
      now,
      now,
    )?;

    Ok(())
  } else {
    log::warn!(
      "[chromium/workflow] No workflow found to save for {} (already removed?)",
      url
    );
    Ok(())
  }
}

// Remove workflow from memory (on page_closed)
pub fn remove_workflow(url: &str) {
  if WORKFLOWS.remove(url).is_some() {
    log::debug!("[chromium/workflow] Removed workflow for {}", url);
  }
}
