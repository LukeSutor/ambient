//! Event constants and payload types for the automations system.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ── Event name constants ─────────────────────────────────────────────

pub const AUTOMATION_TASK_CREATED: &str = "automation_task_created";
pub const AUTOMATION_PROPOSE_CREATION: &str = "automation_propose_creation";
pub const AUTOMATION_TASK_UPDATED: &str = "automation_task_updated";
pub const AUTOMATION_TASK_DELETED: &str = "automation_task_deleted";
pub const AUTOMATION_RUN_STARTED: &str = "automation_run_started";
pub const AUTOMATION_RUN_COMPLETED: &str = "automation_run_completed";
pub const AUTOMATION_NOTIFICATION: &str = "automation_notification";

// ── Event payload types ──────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct AutomationTaskEvent {
    pub task_id: String,
    pub task_name: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct AutomationTaskDeletedEvent {
    pub task_id: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct AutomationRunEvent {
    pub run_id: String,
    pub task_id: String,
    pub timestamp: String,
}

/// Emitted by the automation-management skill to ask the frontend to open the
/// Create Automation dialog pre-filled with a proposed automation.
#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct AutomationProposalEvent {
    pub name: String,
    pub description: Option<String>,
    pub task_type: String,
    pub prompt_template: String,
    pub schedule_type: Option<String>,
    pub schedule_value: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, TS)]
#[ts(export, export_to = "events.ts")]
pub struct AutomationRunCompletedEvent {
    pub run_id: String,
    pub task_id: String,
    pub status: String,
    pub result_summary: Option<String>,
    pub timestamp: String,
}
