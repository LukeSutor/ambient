//! Tool execution engine.
//!
//! Provides parallel execution of tool calls with routing to
//! appropriate skill implementations and database persistence.
//!
//! # Features
//!
//! - **Parallel Execution**: Multiple tools execute concurrently using futures
//! - **Skill Routing**: Routes tool calls to appropriate skill handler
//! - **Persistence**: Saves tool call records to database
//! - **Error Handling**: Captures and returns errors for each tool call

use super::types::{ToolCall, ToolResult, AgentError};
use futures::future::join_all;
use tauri::AppHandle;

// ============================================================================
// Internal Functions
// ============================================================================

/// Executes a single tool call.
///
/// Routes to the appropriate skill handler based on tool name.
/// Returns ToolResult containing either success value or error message.
async fn execute_single_tool(
    app_handle: AppHandle,
    call: ToolCall,
) -> ToolResult {
    log::info!(
        "[executor] Executing {}.{} with args: {:?}",
        call.skill_name,
        call.tool_name,
        call.arguments
    );

    // Handle system tools (like activate_skill) - these are handled by runtime, not here
    if call.skill_name == "system" && call.tool_name == "activate_skill" {
        // This is handled by the runtime, not here
        log::info!(
            "[executor] Skill activation request for skill: {}",
            call.arguments.get("skill_name").and_then(|v| v.as_str()).unwrap_or("<unknown>")
        );
        return ToolResult::success(call.id, serde_json::json!({"status": "skill_activated"}));
    }

    // Route to appropriate skill executor
    // TODO: Implement actual skill handlers in builtin module
    let result = match call.skill_name.as_str() {
        "web-search" => execute_builtin(
            app_handle.clone(),
            call.clone(),
            "web-search",
            |h, c| async move { super::builtin::web_search::execute(&h, &c).await },
        )
        .await,
        "memory-search" => execute_builtin(
            app_handle.clone(),
            call.clone(),
            "memory-search",
            |h, c| async move { super::builtin::memory_search::execute(&h, &c).await },
        )
        .await,
        "code-execution" => execute_builtin(
            app_handle.clone(),
            call.clone(),
            "code-execution",
            |h, c| async move { super::builtin::code_execution::execute(&h, &c).await },
        )
        .await,
        "calendar" => execute_builtin(
            app_handle.clone(),
            call.clone(),
            "calendar",
            |h, c| async move { super::builtin::calendar::execute(&h, &c).await },
        )
        .await,
        "email" => execute_builtin(
            app_handle.clone(),
            call.clone(),
            "email",
            |h, c| async move { super::builtin::email::execute(&h, &c).await },
        )
        .await,
        "computer-use" => execute_builtin(
            app_handle.clone(),
            call.clone(),
            "computer-use",
            |h, c| async move { super::builtin::computer_use::execute(&h, &c).await },
        )
        .await,
        _ => Err(format!("Unknown skill: {}", call.skill_name)),
    };

    match result {
        Ok(value) => {
            log::info!(
                "[executor] Tool {} succeeded: {}",
                call.qualified_name(),
                value
            );
            ToolResult::success(call.id, value)
        }
        Err(e) => {
            log::error!("[executor] Tool {} failed: {}", call.qualified_name(), e);
            ToolResult::error(call.id, e)
        }
    }
}

/// Executes a builtin skill function.
///
/// Generic wrapper that calls skill's execute function and
/// handles any panics gracefully via tokio::spawn.
async fn execute_builtin<F, Fut>(
    app_handle: AppHandle,
    call: ToolCall,
    skill_name: &'static str,
    f: F,
) -> Result<serde_json::Value, String>
where
    F: FnOnce(AppHandle, ToolCall) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'static,
{
    let skill_name = skill_name.to_string();

    let join_handle = tokio::spawn(async move {
        f(app_handle, call).await
    });

    join_handle.await.unwrap_or_else(|e| {
        if e.is_panic() {
            Err(format!("Skill '{}' panicked", skill_name))
        } else {
            Err(format!("Skill '{}' execution error: {}", skill_name, e))
        }
    })
}

// ============================================================================
// Public API
// ============================================================================

/// Executes multiple tool calls in parallel.
///
/// Takes a vector of tool calls and executes them concurrently.
/// Returns a vector of results in the same order as input calls.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle for database and resource access
/// * `tool_calls` - Tool calls to execute
///
/// # Returns
///
/// Vector of `ToolResult` containing either success results or error messages
pub async fn execute_tools(
    app_handle: &AppHandle,
    tool_calls: Vec<ToolCall>,
) -> Vec<ToolResult> {
    log::info!("[executor] Executing {} tool calls in parallel", tool_calls.len());

    // Execute each tool call in parallel
    let futures: Vec<_> = tool_calls
        .iter()
        .map(|call| execute_single_tool(app_handle.clone(), call.clone()))
        .collect();

    // Wait for all tool calls to complete
    join_all(futures).await
}

/// Saves a tool call record to the database.
///
/// Creates a pending tool call entry that can later be
/// updated with results.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle for database access
/// * `message_id` - ID of the message requesting this tool call
/// * `conversation_id` - ID of the conversation
/// * `call` - The tool call to record
///
/// # Returns
///
/// `Ok(())` on success, or `Err(String)` on database error
pub async fn save_tool_call_record(
    app_handle: &AppHandle,
    message_id: &str,
    conversation_id: &str,
    call: &ToolCall,
) -> Result<(), AgentError> {
    use crate::db::core::DbState;
    use tauri::Manager;
    use chrono::Utc;

    let state = app_handle.state::<DbState>();
    let conn_guard = state
        .0
        .lock()
        .map_err(|_| AgentError::DatabaseError("Failed to acquire DB lock".to_string()))?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| AgentError::DatabaseError("Database connection not available".to_string()))?;

    conn
        .execute(
            "INSERT INTO tool_calls
             (id, message_id, conversation_id, skill_name, tool_name, arguments, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                &call.id,
                message_id,
                conversation_id,
                &call.skill_name,
                &call.tool_name,
                serde_json::to_string(&call.arguments).unwrap_or_else(|_| "{}".to_string()),
                "pending",
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(|e| AgentError::DatabaseError(format!("Insert failed: {}", e)))?;

    Ok(())
}

/// Updates a tool call record with execution result.
///
/// Changes status to 'completed' or 'failed' and stores result.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle for database access
/// * `call_id` - ID of the tool call to update
/// * `result` - The result of tool execution
///
/// # Returns
///
/// `Ok(())` on success, or `Err(String)` on database error
pub async fn update_tool_call_result(
    app_handle: &AppHandle,
    call_id: &str,
    result: &ToolResult,
) -> Result<(), AgentError> {
    use crate::db::core::DbState;
    use tauri::Manager;
    use chrono::Utc;

    let state = app_handle.state::<DbState>();
    let conn_guard = state
        .0
        .lock()
        .map_err(|_| AgentError::DatabaseError("Failed to acquire DB lock".to_string()))?;
    let conn = conn_guard
        .as_ref()
        .ok_or_else(|| AgentError::DatabaseError("Database connection not available".to_string()))?;

    let status = if result.success {
        "completed"
    } else {
        "failed"
    };

    let result_json = result
        .result
        .as_ref()
        .and_then(|r| serde_json::to_string(r).ok());

    conn
        .execute(
            "UPDATE tool_calls
             SET status = ?1, result = ?2, completed_at = ?3
             WHERE id = ?4",
            rusqlite::params![
                status,
                result_json,
                Utc::now().to_rfc3339(),
                call_id
            ],
        )
        .map_err(|e| AgentError::DatabaseError(format!("Update failed: {}", e)))?;

    Ok(())
}
