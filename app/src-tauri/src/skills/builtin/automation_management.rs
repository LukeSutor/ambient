//! Builtin automation management skill.
//!
//! Allows the agent to list, propose creation of, and run automation tasks.
//! Creating, toggling, and deleting automations is always confirmed by the user
//! through the dashboard — the agent only proposes, not commits.

use super::ToolCall;
use serde_json::Value;
use tauri::AppHandle;

/// Entry point — routes by tool_name within the skill.
pub async fn execute(app_handle: &AppHandle, call: &ToolCall) -> Result<Value, String> {
    match call.tool_name.as_str() {
        "list_automations" => list_automations(app_handle).await,
        "create_automation" => propose_automation(app_handle, call).await,
        "run_automation" => run_automation(app_handle, call).await,
        _ => Err(format!("Unknown tool: {}", call.tool_name)),
    }
}

async fn list_automations(app_handle: &AppHandle) -> Result<Value, String> {
    let tasks = crate::automations::db::get_all_tasks(app_handle)?;

    let task_list: Vec<Value> = tasks
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "description": t.description,
                "type": t.task_type,
                "enabled": t.is_enabled,
                "system": t.is_system,
                "schedule_type": t.schedule_type,
                "schedule_value": t.schedule_value,
                "trigger_type": t.trigger_type,
                "last_run_at": t.last_run_at,
                "next_run_at": t.next_run_at,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "automations": task_list,
        "count": task_list.len(),
    }))
}

async fn propose_automation(_app_handle: &AppHandle, call: &ToolCall) -> Result<Value, String> {
    let name = call
        .arguments
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: name")?
        .to_string();

    let task_type = call
        .arguments
        .get("task_type")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: task_type")?
        .to_string();

    let prompt = call
        .arguments
        .get("prompt")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: prompt")?
        .to_string();

    let description = call
        .arguments
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let schedule_type = call
        .arguments
        .get("schedule_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let schedule_value = call
        .arguments
        .get("schedule_value")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let trigger_type = call
        .arguments
        .get("trigger_type")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let trigger_config = call
        .arguments
        .get("trigger_config")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Emit a proposal event so the frontend opens the Create Automation dialog
    // pre-filled with these values.  The user can review, edit, and confirm.
    let proposal = crate::automations::events::AutomationProposalEvent {
        name: name.clone(),
        description,
        task_type: task_type.clone(),
        prompt_template: prompt,
        schedule_type,
        schedule_value,
        trigger_type,
        trigger_config,
    };

    let _ = crate::events::emitter::emit(
        crate::automations::events::AUTOMATION_PROPOSE_CREATION,
        proposal,
    );

    Ok(serde_json::json!({
        "success": true,
        "message": format!(
            "I've pre-filled the Create Automation form with a proposed '{}' automation. \
             Please review the details in the Automations dashboard and click 'Create' to confirm, \
             or adjust any settings before saving.",
            name
        ),
    }))
}

async fn run_automation(app_handle: &AppHandle, call: &ToolCall) -> Result<Value, String> {
    let task_id = call
        .arguments
        .get("task_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing required parameter: task_id")?
        .to_string();

    // Verify the task exists
    let task = crate::automations::db::get_task_by_id(app_handle, &task_id)?;

    // We can't call execute_automation directly here because it creates a
    // type cycle (skill → executor → skill). Instead, invoke the Tauri command
    // which handles execution independently.
    // For now, just acknowledge the request. The user can manually trigger from the dashboard.

    Ok(serde_json::json!({
        "success": true,
        "task_id": task.id,
        "name": task.name,
        "message": format!("To run '{}' immediately, use the 'Run Now' button in the Automations dashboard, or enable it to run on its schedule.", task.name),
    }))
}
