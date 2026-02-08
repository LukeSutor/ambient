//! Email skill implementation.
//!
//! This skill provides email management capabilities using Gmail API.

use super::ToolCall;
use serde_json::Value;
use crate::auth::storage::get_provider_token;
use crate::auth::auth_flow::refresh_google_token;
use crate::auth::security::HTTP_CLIENT;
use htmd::convert;
use base64::{Engine as _, engine::general_purpose::{URL_SAFE, URL_SAFE_NO_PAD}};

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
    
    let query = call
        .arguments.get("query")
        .and_then(|q| q.as_str())
        .unwrap_or("");

    log::info!("[email] Listing emails (limit: {})", limit);

    let token = match get_provider_token().map_err(|e| e.to_string())? {
        Some(t) => t,
        None => {
            log::info!("[email] No provider token in session, attempting to recover...");
            match refresh_google_token().await {
                Ok(t) => t,
                Err(e) => {
                    log::error!("[email] Failed to recover provider token: {}", e);
                    return Ok(serde_json::json!({
                        "status": "error",
                        "message": "Not authenticated with Google. Please sign in again."
                    }));
                }
            }
        }
    };

    let query_str = if query.is_empty() {
        format!("maxResults={}", limit)
    } else {
        format!("maxResults={}&q={}", limit, query)
    };

    let mut response = call_gmail_api(&token, "me/messages", &query_str).await;

    // Retry once with refresh if unauthorized
    if let Err(ref e) = response {
        if e.contains("401") {
            log::info!("[email] Token expired, refreshing...");
            if let Ok(new_token) = refresh_google_token().await {
                response = call_gmail_api(&new_token, "me/messages", &query_str).await;
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
        None => {
            log::info!("[email] No provider token in session, attempting to recover...");
            match refresh_google_token().await {
                Ok(t) => t,
                Err(e) => {
                    log::error!("[email] Failed to recover provider token: {}", e);
                    return Ok(serde_json::json!({
                        "status": "error",
                        "message": "Not authenticated with Google. Please sign in again."
                    }));
                }
            }
        }
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
                let cleaned_html = clean_html_content(&body_html);
                let markdown = convert(&cleaned_html).unwrap_or(cleaned_html);
                post_process_markdown(&markdown)
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

/// Helper to decode Gmail's base64url data.
fn decode_base64_url(data: &str) -> Result<Vec<u8>, String> {
    let clean_data = data.replace(|c: char| c.is_whitespace(), "");
    
    // Try without padding first (common in Gmail)
    if let Ok(decoded) = URL_SAFE_NO_PAD.decode(&clean_data) {
        return Ok(decoded);
    }
    
    // Try with padding (some clients include it)
    if let Ok(decoded) = URL_SAFE.decode(&clean_data) {
        return Ok(decoded);
    }

    Err("Invalid base64 data".to_string())
}

/// Recursively extract the body from message parts and decode it.
fn extract_body(part: &Value) -> String {
    // 1. Try to get data from this part
    if let Some(data) = part["body"]["data"].as_str() {
        match decode_base64_url(data) {
            Ok(decoded) => {
                let text = String::from_utf8_lossy(&decoded).to_string();
                if !text.is_empty() {
                    return text;
                }
            }
            Err(e) => {
                log::warn!("[email] Failed to decode body data: {}", e);
            }
        }
    }

    // 2. If no data here, look in parts
    if let Some(parts) = part["parts"].as_array() {
        // Look for text/html in any part (recursive)
        for p in parts {
            let mime = p["mimeType"].as_str().unwrap_or("").to_lowercase();
            if mime.starts_with("text/html") {
                let body = extract_body(p);
                if !body.is_empty() { return body; }
            }
        }
        
        // Look for nested multiparts or text/plain
        for p in parts {
            let mime = p["mimeType"].as_str().unwrap_or("").to_lowercase();
            if mime.starts_with("multipart/") || mime.starts_with("text/plain") {
                let body = extract_body(p);
                if !body.is_empty() { return body; }
            }
        }

        // Fallback to any other text part
        for p in parts {
            let mime = p["mimeType"].as_str().unwrap_or("").to_lowercase();
            if mime.starts_with("text/") {
                let body = extract_body(p);
                if !body.is_empty() { return body; }
            }
        }
    }

    "".to_string()
}

/// Clean HTML content by removing styles, scripts, and layout-specific tags.
fn clean_html_content(html: &str) -> String {
    let mut cleaned = html.to_string();

    // Remove style and script tags with their content
    let tags_to_remove = ["style", "script", "head", "title", "meta", "link", "xml"];
    for tag_name in tags_to_remove {
        let start_tag = format!("<{}", tag_name);
        let end_tag = format!("</{}>", tag_name);
        
        while let Some(start_idx) = cleaned.to_lowercase().find(&start_tag) {
            if let Some(end_idx) = cleaned[start_idx..].to_lowercase().find(&end_tag) {
                cleaned.replace_range(start_idx..start_idx + end_idx + end_tag.len(), " ");
            } else {
                // Remove the opening tag and search for the next >
                if let Some(tag_end) = cleaned[start_idx..].find(">") {
                    cleaned.replace_range(start_idx..start_idx + tag_end + 1, " ");
                } else {
                    cleaned.replace_range(start_idx..start_idx + start_tag.len(), " ");
                }
            }
        }
    }

    // Remove other junk tags (images, metadata)
    let junk_tags = ["img", "base", "svg"];
    for tag_name in junk_tags {
        let start_tag = format!("<{}", tag_name);
        while let Some(start_idx) = cleaned.to_lowercase().find(&start_tag) {
            if let Some(tag_end) = cleaned[start_idx..].find(">") {
                cleaned.replace_range(start_idx..start_idx + tag_end + 1, " ");
            } else {
                break;
            }
        }
    }

    // Remove comments
    while let Some(start_idx) = cleaned.find("<!--") {
        if let Some(end_idx) = cleaned[start_idx..].find("-->") {
            cleaned.replace_range(start_idx..start_idx + end_idx + 3, " ");
        } else {
            break;
        }
    }

    // Down-level tables to divs to avoid MarkDown table artifacts (|)
    cleaned = cleaned.replace("<table", "<div");
    cleaned = cleaned.replace("</table", "</div");
    cleaned = cleaned.replace("<tr", "<div");
    cleaned = cleaned.replace("</tr", "</div");
    cleaned = cleaned.replace("<td", "<div");
    cleaned = cleaned.replace("</td", "</div");
    cleaned = cleaned.replace("<th", "<div");
    cleaned = cleaned.replace("</th", "</div");
    cleaned = cleaned.replace("<tbody", "<div");
    cleaned = cleaned.replace("</tbody", "</div");
    cleaned = cleaned.replace("<thead", "<div");
    cleaned = cleaned.replace("</thead", "</div");

    cleaned
}

/// Post-process markdown to remove invisible characters and normalize whitespace.
fn post_process_markdown(markdown: &str) -> String {
    let mut cleaned = markdown.to_string();

    // 1. Remove common invisible/zero-width junk characters used in email templates
    let junk_chars = [
        '\u{200C}', // Zero Width Non-Joiner
        '\u{200B}', // Zero Width Space
        '\u{200D}', // Zero Width Joiner
        '\u{FEFF}', // Zero Width No-Break Space
        '\u{AD}',   // Soft Hyphen
    ];
    for c in junk_chars {
        cleaned = cleaned.replace(c, "");
    }

    // 2. Normalize non-breaking spaces to regular spaces
    cleaned = cleaned.replace('\u{00A0}', " ");

    // 3. Process line by line to collapse whitespace and remove empty lines
    let mut result_lines = Vec::new();
    let mut consecutive_empty = 0;

    for line in cleaned.lines() {
        let trimmed = line.trim();
        
        if trimmed.is_empty() {
            consecutive_empty += 1;
            // Only allow up to 1 consecutive empty line in the output (standard Markdown spacing)
            if consecutive_empty <= 1 {
                result_lines.push("".to_string());
            }
        } else {
            consecutive_empty = 0;
            
            // Collapse multiple horizontal spaces into one
            let collapsed = trimmed
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            
            result_lines.push(collapsed);
        }
    }

    result_lines.join("\n").trim().to_string()
}
