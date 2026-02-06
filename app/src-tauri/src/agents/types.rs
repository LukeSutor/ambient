use ts_rs::TS;
use serde::{Serialize, Deserialize};
use serde_json::Value;

// ============================================================================
// Event Types for Agentic Runtime
// ============================================================================

/// Event emitted when a tool execution starts.
pub const TOOL_EXECUTION_STARTED: &str = "tool_execution_started";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "events.ts")]
pub struct ToolExecutionStartedEvent {
    pub tool_call_id: String,
    pub message_id: String,
    pub skill_name: String,
    pub tool_name: String,
    pub content: String,
    #[ts(type = "any")]
    pub arguments: Value,
    pub timestamp: String,
}

/// Event emitted when a tool execution completes.
pub const TOOL_EXECUTION_COMPLETED: &str = "tool_execution_completed";

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "events.ts")]
pub struct ToolExecutionCompletedEvent {
    pub tool_call_id: String,
    pub message_id: String,
    pub skill_name: String,
    pub tool_name: String,
    pub success: bool,
    #[ts(type = "any")]
    pub result: Option<Value>,
    pub error: Option<String>,
    pub timestamp: String,
}
