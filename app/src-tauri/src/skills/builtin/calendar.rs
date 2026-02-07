//! Calendar skill implementation.
//!
//! This skill provides calendar event management capabilities using Google Calendar API.

use super::ToolCall;
use serde_json::Value;
use crate::auth::storage::get_provider_token;
use crate::auth::auth_flow::refresh_google_token;
use crate::auth::security::HTTP_CLIENT;

/// Execute a calendar tool.
pub async fn execute(
    _app_handle: &tauri::AppHandle,
    call: &ToolCall,
) -> Result<Value, String> {
    match call.tool_name.as_str() {
        "create_event" => create_event(call).await,
        "list_events" => list_events(call).await,
        _ => Err(format!("Unknown tool: {}", call.tool_name)),
    }
}

/// Create a new calendar event.
async fn create_event(call: &ToolCall) -> Result<Value, String> {
    let title = call
        .arguments
        .get("title")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "Missing 'title' argument".to_string())?;

    let start_time = call
        .arguments
        .get("start_time")
        .and_then(|t| t.as_str())
        .ok_or_else(|| "Missing 'start_time' argument".to_string())?;

    let end_time = call
        .arguments
        .get("end_time")
        .and_then(|t| t.as_str());

    let description = call
        .arguments
        .get("description")
        .and_then(|t| t.as_str());

    let token = match get_provider_token().map_err(|e| e.to_string())? {
        Some(t) => t,
        None => return Ok(serde_json::json!({
            "status": "error",
            "message": "Not authenticated with Google"
        })),
    };

    let event_body = serde_json::json!({
        "summary": title,
        "description": description.unwrap_or_default(),
        "start": { "dateTime": start_time },
        "end": { "dateTime": end_time.unwrap_or(start_time) }
    });

    log::info!("[calendar] Creating event: {} at {}", title, start_time);

    let mut response = post_google_calendar_api(&token, "primary", event_body.clone()).await;

    // Retry once with refresh if unauthorized
    if let Err(ref e) = response {
        if e.contains("401") {
            log::info!("[calendar] Token expired, refreshing...");
            if let Ok(new_token) = refresh_google_token().await {
                response = post_google_calendar_api(&new_token, "primary", event_body).await;
            }
        }
    }

    match response {
        Ok(event) => Ok(serde_json::json!({
            "status": "success",
            "event": event
        })),
        Err(e) => Ok(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
}

/// List events in a date range.
async fn list_events(call: &ToolCall) -> Result<Value, String> {
    let start_time = call
        .arguments
        .get("start")
        .and_then(|s| s.as_str())
        .map(|s| format!("timeMin={}", urlencoding::encode(s)))
        .unwrap_or_else(|| "timeMin=".to_string());

    let end_time = call
        .arguments
        .get("end")
        .and_then(|e| e.as_str())
        .map(|e| format!("&timeMax={}", urlencoding::encode(e)))
        .unwrap_or_default();

    log::info!("[calendar] Listing events with query: {}{}", start_time, end_time);

    let token = match get_provider_token().map_err(|e| e.to_string())? {
        Some(t) => t,
        None => return Ok(serde_json::json!({
            "status": "error",
            "message": "Not authenticated with Google"
        })),
    };

    let query = format!("{}&singleEvents=true&orderBy=startTime{}", start_time, end_time);
    let mut response = call_google_calendar_api(&token, "primary", &query).await;

    // Retry once with refresh if unauthorized
    if let Err(ref e) = response {
        if e.contains("401") {
            log::info!("[calendar] Token expired, refreshing...");
            if let Ok(new_token) = refresh_google_token().await {
                response = call_google_calendar_api(&new_token, "primary", &query).await;
            }
        }
    }

    match response {
        Ok(events) => Ok(serde_json::json!({
            "status": "success",
            "events": events["items"]
        })),
        Err(e) => Ok(serde_json::json!({
            "status": "error",
            "message": e
        })),
    }
}

async fn call_google_calendar_api(token: &str, calendar_id: &str, query: &str) -> Result<Value, String> {
    let url = format!("https://www.googleapis.com/calendar/v3/calendars/{}/events?{}", calendar_id, query);
    
    let response = HTTP_CLIENT
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Google API error: {}", status));
    }
    
    let json: Value = response.json().await.map_err(|e| format!("JSON error: {}", e))?;
    Ok(json)
}

async fn post_google_calendar_api(token: &str, calendar_id: &str, body: Value) -> Result<Value, String> {
    let url = format!("https://www.googleapis.com/calendar/v3/calendars/{}/events", calendar_id);
    
    let response = HTTP_CLIENT
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;
    
    let status = response.status();
    if !status.is_success() {
        return Err(format!("Google API error: {}", status));
    }
    
    let json: Value = response.json().await.map_err(|e| format!("JSON error: {}", e))?;
    Ok(json)
}
