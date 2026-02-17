//! Core types for the automations system.
//!
//! Defines automation tasks, triggers, runs, and configuration types.
//! Types with `#[ts(export)]` are auto-generated to TypeScript.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ============================================================================
// Enums
// ============================================================================

/// Type of automation task.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "automations.ts")]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Scheduled,
    Semantic,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::Scheduled => "scheduled",
            TaskType::Semantic => "semantic",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "scheduled" => Ok(TaskType::Scheduled),
            "semantic" => Ok(TaskType::Semantic),
            _ => Err(format!("Unknown task type: {}", s)),
        }
    }
}

/// Schedule type for time-based automations.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "automations.ts")]
#[serde(rename_all = "snake_case")]
pub enum ScheduleType {
    /// Every N minutes
    Interval,
    /// Specific time each day
    Daily,
    /// Specific day and time each week
    Weekly,
    /// One-time execution at a specific datetime
    Once,
}

impl ScheduleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleType::Interval => "interval",
            ScheduleType::Daily => "daily",
            ScheduleType::Weekly => "weekly",
            ScheduleType::Once => "once",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "interval" => Ok(ScheduleType::Interval),
            "daily" => Ok(ScheduleType::Daily),
            "weekly" => Ok(ScheduleType::Weekly),
            "once" => Ok(ScheduleType::Once),
            _ => Err(format!("Unknown schedule type: {}", s)),
        }
    }
}

/// Trigger type for event-based automations.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "automations.ts")]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// Trigger when specific text appears on screen (OCR)
    ScreenContent,
    /// Trigger when specific app gains focus
    AppFocus,
    /// Trigger when visiting specific URLs
    UrlVisit,
}

impl TriggerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerType::ScreenContent => "screen_content",
            TriggerType::AppFocus => "app_focus",
            TriggerType::UrlVisit => "url_visit",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "screen_content" => Ok(TriggerType::ScreenContent),
            "app_focus" => Ok(TriggerType::AppFocus),
            "url_visit" => Ok(TriggerType::UrlVisit),
            _ => Err(format!("Unknown trigger type: {}", s)),
        }
    }
}

/// Status of an automation run.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "automations.ts")]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Running => "running",
            RunStatus::Completed => "completed",
            RunStatus::Failed => "failed",
            RunStatus::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "running" => Ok(RunStatus::Running),
            "completed" => Ok(RunStatus::Completed),
            "failed" => Ok(RunStatus::Failed),
            "cancelled" => Ok(RunStatus::Cancelled),
            _ => Err(format!("Unknown run status: {}", s)),
        }
    }
}

/// Notification type for automation results.
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export, export_to = "automations.ts")]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    Success,
    Error,
    Warning,
    Info,
}

// ============================================================================
// Data Models
// ============================================================================

/// An automation task definition (maps to `automation_tasks` table).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "automations.ts")]
pub struct AutomationTask {
    pub id: String,
    pub name: String,
    pub description: String,
    pub task_type: String,
    pub is_enabled: bool,
    pub is_system: bool,
    /// The prompt template sent to the agentic runtime. May contain
    /// `{{parameter_name}}` placeholders filled in at execution time.
    pub prompt_template: String,
    /// Optional model override (references `models.id`).
    pub model_id: Option<i64>,
    /// JSON array of skill names disabled for this automation.
    pub disabled_skills: Vec<String>,
    pub notify_on_complete: bool,
    pub notify_on_error: bool,
    pub max_iterations: i64,
    pub timeout_seconds: i64,

    // Schedule fields (for scheduled tasks)
    pub schedule_type: Option<String>,
    pub schedule_value: Option<String>,
    pub schedule_timezone: String,

    // Trigger fields (for semantic tasks)
    pub trigger_type: Option<String>,
    pub trigger_config: Option<String>,

    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Parameters for creating a new automation task.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "automations.ts")]
pub struct CreateAutomationParams {
    pub name: String,
    pub description: Option<String>,
    pub task_type: String,
    pub prompt_template: String,
    pub model_id: Option<i64>,
    pub disabled_skills: Option<Vec<String>>,
    pub notify_on_complete: Option<bool>,
    pub notify_on_error: Option<bool>,
    pub max_iterations: Option<i64>,
    pub timeout_seconds: Option<i64>,
    pub schedule_type: Option<String>,
    pub schedule_value: Option<String>,
    pub schedule_timezone: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<String>,
}

/// Parameters for updating an existing automation task.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "automations.ts")]
pub struct UpdateAutomationParams {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub prompt_template: Option<String>,
    pub model_id: Option<i64>,
    pub disabled_skills: Option<Vec<String>>,
    pub notify_on_complete: Option<bool>,
    pub notify_on_error: Option<bool>,
    pub max_iterations: Option<i64>,
    pub timeout_seconds: Option<i64>,
    pub schedule_type: Option<String>,
    pub schedule_value: Option<String>,
    pub schedule_timezone: Option<String>,
    pub trigger_type: Option<String>,
    pub trigger_config: Option<String>,
    pub is_enabled: Option<bool>,
}

/// An automation run record (maps to `automation_runs` table).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "automations.ts")]
pub struct AutomationRun {
    pub id: String,
    pub task_id: String,
    pub status: String,
    pub result_text: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub credits_used: f64,
}

/// A semantic trigger definition (maps to `automation_triggers` table).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "automations.ts")]
pub struct AutomationTrigger {
    pub id: String,
    pub task_id: String,
    pub trigger_type: String,
    pub trigger_config: String,
    pub is_enabled: bool,
    pub created_at: String,
}

/// Notification payload emitted to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "automations.ts")]
pub struct AutomationNotification {
    pub id: String,
    pub task_id: String,
    pub task_name: String,
    pub notification_type: NotificationType,
    pub title: String,
    pub body: String,
    pub timestamp: String,
}

/// Background execution mode configuration.
/// Controls whether the agent runtime writes to DB, emits events, etc.
#[derive(Debug, Clone)]
pub struct ExecutionMode {
    /// True when running as a background automation.
    pub is_background: bool,
    /// Which llama.cpp KV cache slot to use (0 = interactive, 1+ = background).
    pub slot_id: i32,
    /// Whether to emit frontend events (streaming, tool execution, etc.).
    pub emit_events: bool,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self {
            is_background: false,
            slot_id: 0,
            emit_events: true,
        }
    }
}

impl ExecutionMode {
    /// Create a background execution mode for automations.
    pub fn background() -> Self {
        Self {
            is_background: true,
            slot_id: 2,
            emit_events: false,
        }
    }
}
