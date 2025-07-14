use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub category: Option<String>,
    pub priority: i32,
    pub frequency: String, // Will be converted to/from TaskFrequency
    pub last_completed_at: Option<DateTime<Utc>>,
    pub next_due_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: String, // Will be converted to/from TaskStatus
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub id: i64,
    pub task_id: i64,
    pub step_number: i32,
    pub title: String,
    pub description: String,
    pub status: String, // Will be converted to/from StepStatus
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskFrequency {
    OneTime,     // Task only needs to be completed once
    Daily,       // Every day
    Weekly,      // Every week
    BiWeekly,    // Every two weeks
    Monthly,     // Every month
    Quarterly,   // Every 3 months
    Yearly,      // Every year
    Custom(i32), // Custom frequency in days
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

impl TaskFrequency {
    pub fn as_str(&self) -> String {
        match self {
            TaskFrequency::OneTime => "one_time".to_string(),
            TaskFrequency::Daily => "daily".to_string(),
            TaskFrequency::Weekly => "weekly".to_string(),
            TaskFrequency::BiWeekly => "bi_weekly".to_string(),
            TaskFrequency::Monthly => "monthly".to_string(),
            TaskFrequency::Quarterly => "quarterly".to_string(),
            TaskFrequency::Yearly => "yearly".to_string(),
            TaskFrequency::Custom(days) => format!("custom_{}", days),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "one_time" => TaskFrequency::OneTime,
            "daily" => TaskFrequency::Daily,
            "weekly" => TaskFrequency::Weekly,
            "bi_weekly" => TaskFrequency::BiWeekly,
            "monthly" => TaskFrequency::Monthly,
            "quarterly" => TaskFrequency::Quarterly,
            "yearly" => TaskFrequency::Yearly,
            _ if s.starts_with("custom_") => {
                if let Ok(days) = s.strip_prefix("custom_").unwrap_or("1").parse::<i32>() {
                    TaskFrequency::Custom(days)
                } else {
                    TaskFrequency::OneTime
                }
            }
            _ => TaskFrequency::OneTime,
        }
    }

    /// Calculate the next due date based on the frequency and last completion
    pub fn next_due_date(&self, last_completed: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
        use chrono::{Duration, Datelike};
        
        let base_date = last_completed.unwrap_or(Utc::now());
        
        match self {
            TaskFrequency::OneTime => None, // One-time tasks don't have a next due date
            TaskFrequency::Daily => Some(base_date + Duration::days(1)),
            TaskFrequency::Weekly => Some(base_date + Duration::weeks(1)),
            TaskFrequency::BiWeekly => Some(base_date + Duration::weeks(2)),
            TaskFrequency::Monthly => {
                // Add one month (approximately 30 days, but try to maintain the same day of month)
                let mut next_month = base_date.month() + 1;
                let mut next_year = base_date.year();
                if next_month > 12 {
                    next_month = 1;
                    next_year += 1;
                }
                base_date.with_year(next_year).and_then(|d| d.with_month(next_month))
                    .or_else(|| Some(base_date + Duration::days(30))) // Fallback to 30 days
            },
            TaskFrequency::Quarterly => Some(base_date + Duration::days(90)),
            TaskFrequency::Yearly => {
                base_date.with_year(base_date.year() + 1)
                    .or_else(|| Some(base_date + Duration::days(365))) // Fallback to 365 days
            },
            TaskFrequency::Custom(days) => Some(base_date + Duration::days(*days as i64)),
        }
    }

    /// Check if a task is overdue based on its frequency and last completion
    pub fn is_overdue(&self, last_completed: Option<DateTime<Utc>>) -> bool {
        match self.next_due_date(last_completed) {
            Some(due_date) => Utc::now() > due_date,
            None => false, // One-time tasks can't be overdue
        }
    }

    /// Get a human-readable description of the frequency
    pub fn description(&self) -> String {
        match self {
            TaskFrequency::OneTime => "One time only".to_string(),
            TaskFrequency::Daily => "Every day".to_string(),
            TaskFrequency::Weekly => "Every week".to_string(),
            TaskFrequency::BiWeekly => "Every 2 weeks".to_string(),
            TaskFrequency::Monthly => "Every month".to_string(),
            TaskFrequency::Quarterly => "Every 3 months".to_string(),
            TaskFrequency::Yearly => "Every year".to_string(),
            TaskFrequency::Custom(days) => format!("Every {} day{}", days, if *days == 1 { "" } else { "s" }),
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
    pub frequency: TaskFrequency,
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
