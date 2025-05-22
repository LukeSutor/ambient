use serde::{Serialize, Deserialize};
use dashmap::DashMap;
use crate::DbState;


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
}

// Append a step to the workflow for a URL
pub fn append_step(url: &str, step: WorkflowStep) {
    if let Some(mut wf) = WORKFLOWS.get_mut(url) {
        println!("[chromium/workflow] Appending step to workflow for {}: {:?}", url, step);
        wf.steps.push(step);
    }
}

// Save workflow to DB and remove from memory
pub fn save_workflow(url: &str, db_state: tauri::State<DbState>) -> Result<(), String> {
    if let Some((_key, mut wf)) = WORKFLOWS.remove(url) {
        println!("[chromium/workflow] Saving workflow for {}: {:?}", url, wf);
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
    }
    Ok(())
}

// Remove workflow from memory (on page_closed)
pub fn remove_workflow(url: &str) {
    WORKFLOWS.remove(url);
}
