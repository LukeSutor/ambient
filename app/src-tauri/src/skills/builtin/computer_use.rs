//! Computer use skill implementation.
//!
//! This skill provides computer control capabilities via mouse
//! and keyboard interaction.
//!
//! # Tools
//!
//! - `start_computer_use`: Start a computer use session with a goal
//!
//! # Status
//!
//! **TODO**: This is a stub implementation. Should delegate to
//! the existing ComputerUseEngine in the future.

use super::ToolCall;
use serde_json::Value;
use tauri::AppHandle;

/// Execute a computer use tool.
pub async fn execute(
    _app_handle: &AppHandle,
    call: &ToolCall,
) -> Result<Value, String> {
    match call.tool_name.as_str() {
        "start_computer_use" => start_computer_use(call).await,
        _ => Err(format!("Unknown tool: {}", call.tool_name)),
    }
}

/// Start a computer use session with a specific goal.
async fn start_computer_use(call: &ToolCall) -> Result<Value, String> {
    let goal = call
        .arguments
        .get("goal")
        .and_then(|g| g.as_str())
        .ok_or_else(|| "Missing 'goal' argument".to_string())?;

    log::info!("[computer_use] Starting computer use with goal: {}", goal);

    // TODO: Delegate to the existing ComputerUseEngine
    // For now, return a placeholder response
    Ok(serde_json::json!({
        "status": "not_implemented",
        "message": "Computer use integration should delegate to ComputerUseEngine. This is a stub placeholder.",
        "goal": goal
    }))
}
