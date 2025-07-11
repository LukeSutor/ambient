use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: String, // Will be converted to/from TaskStatus
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskStep {
    pub id: i64,
    pub task_id: i64,
    pub step_number: i32,
    pub title: String,
    pub description: Option<String>,
    pub status: String, // Will be converted to/from StepStatus
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskProgress {
    pub id: i64,
    pub task_id: i64,
    pub step_id: Option<i64>,
    pub llm_confidence: f64, // How confident the LLM is about completion
    pub evidence: Option<String>, // What the LLM found as evidence
    pub reasoning: Option<String>, // LLM's reasoning for the decision
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Completed => "completed",
            TaskStatus::Paused => "paused",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => TaskStatus::Pending,
            "in_progress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "paused" => TaskStatus::Paused,
            _ => TaskStatus::Pending,
        }
    }
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StepStatus::Pending => "pending",
            StepStatus::InProgress => "in_progress",
            StepStatus::Completed => "completed",
            StepStatus::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => StepStatus::Pending,
            "in_progress" => StepStatus::InProgress,
            "completed" => StepStatus::Completed,
            "skipped" => StepStatus::Skipped,
            _ => StepStatus::Pending,
        }
    }
}

// Task creation structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub priority: i32,
    pub steps: Vec<CreateTaskStepRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskStepRequest {
    pub title: String,
    pub description: Option<String>,
}

// Task update structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressUpdate {
    pub task_id: i64,
    pub step_updates: Vec<StepUpdate>,
    pub overall_status: TaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepUpdate {
    pub step_id: i64,
    pub status: StepStatus,
    pub confidence: f64,
    pub evidence: String,
    pub reasoning: Option<String>,
}

// LLM Detection Result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetectionResult {
    pub completed_steps: Vec<CompletedStepDetection>,
    pub in_progress_steps: Vec<InProgressStepDetection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStepDetection {
    pub step_id: i64,
    pub confidence: f64,
    pub evidence: String,
    pub reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InProgressStepDetection {
    pub step_id: i64,
    pub confidence: f64,
    pub evidence: String,
}

// Screen context for LLM analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenContext {
    pub text: String,
    pub application: String,
    pub window_title: Option<String>,
    pub timestamp: DateTime<Utc>,
}

// Task with complete step information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskWithSteps {
    pub task: Task,
    pub steps: Vec<TaskStep>,
    pub progress_percentage: f64,
}
