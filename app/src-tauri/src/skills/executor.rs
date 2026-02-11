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

use super::types::{ToolCall, ToolResult};
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

    // Handle system tools
    if call.skill_name == "system" && call.tool_name == "activate_skill" {
        // This is handled by the runtime, we just return the status and skill name
        let skill_name = call.arguments.get("skill_name").and_then(|v| v.as_str()).unwrap_or("<unknown>");
        log::info!(
            "[executor] Skill activation request for skill: {}",
            skill_name
        );
        return ToolResult::success(
            call.id,
            serde_json::json!({
                "status": "skill_activated",
                "skill_name": skill_name
            })
        );
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
            //TODO: remove this log for prod to not leak info
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

/// Executes a single skill tool from a Tauri command.
///
/// This is used for testing and direct tool execution from the frontend.
#[tauri::command]
pub async fn execute_skill_tool(
    app_handle: AppHandle,
    skill_name: String,
    tool_name: String,
    arguments: serde_json::Value,
) -> Result<ToolResult, String> {
    let call = ToolCall::new(skill_name, tool_name, arguments);
    Ok(execute_single_tool(app_handle, call).await)
}

/// Executes multiple tool calls in parallel.
///
/// Takes a vector of tool calls and executes them concurrently.
/// Loads disabled skills from settings and Google auth state internally
/// to ensure the most up-to-date access control is applied.
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
    // Load current settings to get disabled skills
    let disabled_skills = match crate::settings::service::load_user_settings(app_handle.clone()).await {
        Ok(settings) => settings.disabled_skills,
        Err(e) => {
            log::warn!("[executor] Failed to load settings for access control: {}", e);
            Vec::new()
        }
    };

    // Check Google auth state
    let is_google_authed = crate::auth::commands::is_google_authenticated().await;

    // Execute each tool call in parallel, checking access first
    let futures: Vec<_> = tool_calls
        .iter()
        .map(|call| {
            let app = app_handle.clone();
            let c = call.clone();
            let disabled = disabled_skills.clone();
            async move {
                // Check if skill is disabled by user
                if disabled.contains(&c.skill_name) {
                    log::warn!("[executor] Blocked call to disabled skill: {}", c.skill_name);
                    return ToolResult::error(
                        c.id,
                        format!(
                            "{{\"error\": \"skill_disabled\", \"message\": \"The skill '{}' has been disabled by the user. Do not attempt to use this skill again.\"}}",
                            c.skill_name
                        ),
                    );
                }
                // Check if skill requires Google auth using skill metadata
                if super::registry::skill_requires_google_auth(&c.skill_name) && !is_google_authed {
                    log::warn!("[executor] Blocked call to Google-auth skill without auth: {}", c.skill_name);
                    return ToolResult::error(
                        c.id,
                        format!(
                            "{{\"error\": \"auth_required\", \"message\": \"The skill '{}' requires Google authentication. The user has not signed in with Google.\"}}",
                            c.skill_name
                        ),
                    );
                }
                execute_single_tool(app, c).await
            }
        })
        .collect();

    // Wait for all tool calls to complete
    join_all(futures).await
}
