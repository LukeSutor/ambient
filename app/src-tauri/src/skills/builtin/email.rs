//! Email skill implementation.
//!
//! This skill provides email management capabilities using Gmail API.

use super::ToolCall;
use serde_json::Value;
use crate::auth::storage::get_provider_token;
use crate::auth::auth_flow::refresh_google_token;
use crate::auth::security::HTTP_CLIENT;
use htmd::convert;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

/// Execute an email tool.
pub async fn execute(
    _app_handle: &tauri::AppHandle,
    call: &ToolCall,
) -> Result<Value, String> {
    match call.tool_name.as_str() {
        "list_emails" => list_emails(call).await,
        "get_email_details" => get_email_details(call).await,
        _ => Err(format!("Unknown tool: {}", call.tool_name)),
    }
}

/// List recent emails (previews only).
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
            let mut previews = Vec::new();
            
            // For each message, fetch metadata (From, Date, Subject, Snippet)
            let auth_token = if let Ok(Some(t)) = get_provider_token() { t } else { token.clone() };
            
            for msg in messages.iter().take(limit as usize) {
                if let Some(id) = msg["id"].as_str() {
                    // Fetch metadata headers we need
                    let query = "format=metadata&metadataHeaders=From&metadataHeaders=Date&metadataHeaders=Subject";
                    if let Ok(details) = call_gmail_api(&auth_token, &format!("me/messages/{}", id), query).await {
                        previews.push(format_email_preview(&details));
                    }
                }
            }

            Ok(serde_json::json!({
                "status": "success",
                "emails": previews
            }))
        },
        Err(e) => Ok(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
}

/// Get full details for a specific email.
async fn get_email_details(call: &ToolCall) -> Result<Value, String> {
    let id = call
        .arguments
        .get("id")
        .and_then(|i| i.as_str())
        .ok_or_else(|| "Missing 'id' argument".to_string())?;

    log::info!("[email] Fetching full details for email: {}", id);

    let token = match get_provider_token().map_err(|e| e.to_string())? {
        Some(t) => t,
        None => return Ok(serde_json::json!({
            "status": "error",
            "message": "Not authenticated with Google"
        })),
    };

    let mut response = call_gmail_api(&token, &format!("me/messages/{}", id), "format=full").await;

    // Retry once with refresh if unauthorized
    if let Err(ref e) = response {
        if e.contains("401") {
            log::info!("[email] Token expired, refreshing...");
            if let Ok(new_token) = refresh_google_token().await {
                response = call_gmail_api(&new_token, &format!("me/messages/{}", id), "format=full").await;
            }
        }
    }

    match response {
        Ok(details) => {
            let from = extract_header(&details, "From");
            let date = extract_header(&details, "Date");
            let subject = extract_header(&details, "Subject");
            let body_html = extract_body(&details["payload"]);
            
            // Convert to markdown if it's HTML/text
            let content = if !body_html.is_empty() {
                convert(&body_html).unwrap_or(body_html)
            } else {
                details["snippet"].as_str().unwrap_or("No content").to_string()
            };

            Ok(serde_json::json!({
                "status": "success",
                "id": id,
                "from": from,
                "date": date,
                "subject": subject,
                "content": content
            }))
        },
        Err(e) => Ok(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
}

/// Helper to call Gmail API.
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
        let error_body = response.text().await.unwrap_or_else(|_| "Could not read error body".to_string());
        log::error!("[email] Gmail API Error: {} - {}", status, error_body);
        return Err(format!("Gmail API error: {} - {}", status, error_body));
    }
    
    let json: Value = response.json().await.map_err(|e| format!("JSON error: {}", e))?;
    Ok(json)
}

/// Format a single message into a preview object for the model to ingest.
fn format_email_preview(msg: &Value) -> Value {
    serde_json::json!({
        "id": msg["id"].as_str().unwrap_or_default(),
        "from": extract_header(msg, "From"),
        "date": extract_header(msg, "Date"),
        "subject": extract_header(msg, "Subject"),
        "snippet": msg["snippet"].as_str().unwrap_or_default()
    })
}

/// Extract a specific header value from a message.
fn extract_header(msg: &Value, name: &str) -> String {
    msg["payload"]["headers"]
        .as_array()
        .and_then(|headers| {
            headers.iter().find(|h| {
                h["name"]
                    .as_str()
                    .map(|n| n.to_lowercase() == name.to_lowercase())
                    .unwrap_or(false)
            })
        })
        .and_then(|h| h["value"].as_str())
        .unwrap_or("Unknown")
        .to_string()
}

/// Recursively extract the body from message parts and decode it.
fn extract_body(part: &Value) -> String {
    // If this part has a body with data, use it
    if let Some(data) = part["body"]["data"].as_str() {
        if let Ok(decoded) = URL_SAFE_NO_PAD.decode(data) {
            return String::from_utf8_lossy(&decoded).to_string();
        }
    }

    // Otherwise, check sub-parts (common in multipart emails)
    if let Some(parts) = part["parts"].as_array() {
        // Prefer HTML parts if they exist
        for p in parts {
            if p["mimeType"].as_str() == Some("text/html") {
                return extract_body(p);
            }
        }
        // Fall back to first part (often text/plain)
        if let Some(first_part) = parts.first() {
            return extract_body(first_part);
        }
    }

    "".to_string()
}