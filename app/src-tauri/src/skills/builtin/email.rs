//! Email skill implementation.
//!
//! This skill provides email management capabilities using Gmail API.

use super::ToolCall;
use serde_json::Value;
use crate::auth::storage::get_provider_token;
use crate::auth::auth_flow::refresh_google_token;
use crate::auth::security::HTTP_CLIENT;

/// Execute an email tool.
pub async fn execute(
    _app_handle: &tauri::AppHandle,
    call: &ToolCall,
) -> Result<Value, String> {
    match call.tool_name.as_str() {
        "send_email" => send_email(call).await,
        "list_emails" => list_emails(call).await,
        _ => Err(format!("Unknown tool: {}", call.tool_name)),
    }
}

/// Send an email (Not implemented in this version, focused on context retrieval).
async fn send_email(_call: &ToolCall) -> Result<Value, String> {
    Ok(serde_json::json!({
        "status": "error",
        "message": "Sending emails is not yet supported. This skill currently only supports retrieving emails for context."
    }))
}

/// List recent emails.
async fn list_emails(call: &ToolCall) -> Result<Value, String> {
    let limit = call
        .arguments
        .get("limit")
        .and_then(|l| l.as_u64())
        .unwrap_or(5);

    log::info!("[email] Listing emails (limit: {})", limit);

    let token = match get_provider_token().map_err(|e| e.to_string())? {
        Some(t) => t,
        None => return Ok(serde_json::json!({
            "status": "error",
            "message": "Not authenticated with Google"
        })),
    };

    let mut response = call_gmail_api(&token, "me/messages", &format!("maxResults={}", limit)).await;

    // Retry once with refresh if unauthorized
    if let Err(ref e) = response {
        if e.contains("401") {
            log::info!("[email] Token expired, refreshing...");
            if let Ok(new_token) = refresh_google_token().await {
                response = call_gmail_api(&new_token, "me/messages", &format!("maxResults={}", limit)).await;
            }
        }
    }

    match response {
        Ok(data) => {
            let messages = data["messages"].as_array().cloned().unwrap_or_default();
            let mut detailed_messages = Vec::new();
            
            // For each message, fetch details (snappy)
            let auth_token = if let Ok(Some(t)) = get_provider_token() { t } else { token.clone() };
            
            for msg in messages.iter().take(limit as usize) {
                if let Some(id) = msg["id"].as_str() {
                    if let Ok(details) = call_gmail_api(&auth_token, &format!("me/messages/{}", id), "format=full").await {
                        detailed_messages.push(details);
                    }
                }
            }

            Ok(serde_json::json!({
                "status": "success",
                "emails": detailed_messages
            }))
        },
        Err(e) => Ok(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
}

async fn call_gmail_api(token: &str, path: &str, query: &str) -> Result<Value, String> {
    let url = format!("https://gmail.googleapis.com/gmail/v1/users/{}?{}", path, query);
    
    let response = HTTP_CLIENT
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Gmail API error: {}", status));
    }
    
    let json: Value = response.json().await.map_err(|e| format!("JSON error: {}", e))?;
    Ok(json)
}
