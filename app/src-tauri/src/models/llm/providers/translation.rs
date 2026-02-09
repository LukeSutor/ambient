//! Tool format translation layer.
//!
//! Converts between the unified internal tool format and
//! provider-specific formats for OpenAI (local) and Gemini (cloud).
//!
//! This module provides bidirectional translation:
//! - **Internal → Provider**: Converts tool definitions to provider format
//! - **Provider → Internal**: Parses tool calls from provider responses

use crate::db::conversations::{Message, MessageType, MessageMetadata, Role};
use crate::skills::types::{ToolDefinition, ToolCall};
use crate::skills::registry::get_skill;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use base64::{Engine as _, engine::general_purpose};
use std::fs;

const MAX_RECENT_ATTACHMENTS: usize = 3;

/// Translates tool definitions to OpenAI function calling format.
///
/// Used for local models (llama.cpp with OpenAI-compatible API).
/// OpenAI format uses a `function` type wrapping the function details.
pub fn tools_to_openai_format(tools: &[ToolDefinition]) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            let mut properties = json!({});
            let mut required = Vec::new();

            for param in &tool.parameters {
                let param_schema = json!({
                    "type": param.param_type.as_json_schema(),
                    "description": param.description,
                });
                properties[&param.name] = param_schema;

                if param.required {
                    required.push(param.name.clone());
                }
            }

            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": {
                        "type": "object",
                        "properties": properties,
                        "required": required,
                    }
                }
            })
        })
        .collect()
}

/// Translates tool definitions to Gemini function calling format.
///
/// Used for cloud models via Cloudflare worker (Gemini API).
/// Gemini uses uppercase type names and `functionDeclarations` structure.
pub fn tools_to_gemini_format(tools: &[ToolDefinition]) -> Value {
    let function_declarations: Vec<Value> = tools
        .iter()
        .map(|tool| {
            let mut properties = json!({});
            let mut required = Vec::new();

            for param in &tool.parameters {
                let param_schema = json!({
                    "type": param.param_type.as_gemini_type(),
                    "description": param.description,
                });
                properties[&param.name] = param_schema;

                if param.required {
                    required.push(param.name.clone());
                }
            }
            json!({
                "name": tool.name,
                "description": tool.description,
                "parameters": {
                    "type": "OBJECT",
                    "properties": properties,
                    "required": required,
                }
            })
        })
        .collect();

    json!({
        "functionDeclarations": function_declarations
    })
}

/// Resolves a tool name to its skill and tool name components.
///
/// Handles names with dots (e.g., "web-search.search_web"),
/// system tools, and performs lookups in available tools if needed.
pub fn resolve_tool_call(name: &str, available_tools: Option<&[ToolDefinition]>) -> (String, String) {
    if name.contains('.') {
        let parts: Vec<&str> = name.splitn(2, '.').collect();
        (parts[0].to_string(), parts[1].to_string())
    } else if name == "activate_skill" {
        ("system".to_string(), name.to_string())
    } else {
        // Try to find which skill owns this tool by looking at available tools
        let mut found_skill = "unknown".to_string();
        
        if let Some(tools) = available_tools {
            for tool in tools {
                if tool.name == name {
                    if let Some(s) = &tool.skill_name {
                        found_skill = s.clone();
                        break;
                    }
                }
            }
        }
        
        (found_skill, name.to_string())
    }
}

/// Parses tool calls from OpenAI format response.
///
/// Extracts tool calls from OpenAI's response structure,
/// which uses `tool_calls` array with `function` objects.
pub fn parse_openai_tool_calls(response: &Value, available_tools: Option<&[ToolDefinition]>) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    if let Some(choices) = response.get("choices").and_then(|c| c.as_array()) {
        if let Some(choice) = choices.first() {
            if let Some(message) = choice.get("message") {
                if let Some(tool_calls) = message.get("tool_calls").and_then(|t| t.as_array()) {
                    for tc in tool_calls {
                        let id = tc
                            .get("id")
                            .and_then(|i| i.as_str())
                            .unwrap_or("")
                            .to_string();

                        if let Some(function) = tc.get("function") {
                            let name = function
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("")
                                .to_string();

                            let arguments: Value = function
                                .get("arguments")
                                .and_then(|a| a.as_str())
                                .and_then(|s| serde_json::from_str(s).ok())
                                .unwrap_or(json!({}));

                            let (skill_name, tool_name) = resolve_tool_call(&name, available_tools);

                            calls.push(ToolCall {
                                id,
                                skill_name,
                                tool_name,
                                arguments,
                                thought_signature: None,
                            });
                        }
                    }
                }
            }
        }
    }

    calls
}

/// Parses tool calls from Gemini format response.
///
/// Extracts tool calls from Gemini's response structure,
/// which uses `functionCall` in the `parts` array.
pub fn parse_gemini_tool_calls(response: &Value, available_tools: Option<&[ToolDefinition]>) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    if let Some(candidates) = response.get("candidates").and_then(|c| c.as_array()) {
        if let Some(candidate) = candidates.first() {
            if let Some(content) = candidate.get("content") {
                if let Some(parts) = content.get("parts").and_then(|p| p.as_array()) {
                    for part in parts {
                        if let Some(function_call) = part.get("functionCall") {
                            let name = function_call
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("")
                                .to_string();

                            let arguments = function_call
                                .get("args")
                                .cloned()
                                .unwrap_or(json!({}));

                            // Extract thought signature if present
                            let thought_signature = part
                                .get("thoughtSignature")
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string());

                            // Generate unique ID for this call (Gemini doesn't provide one)
                            let id = uuid::Uuid::new_v4().to_string();

                            let (skill_name, tool_name) = resolve_tool_call(&name, available_tools);

                            calls.push(ToolCall {
                                id,
                                skill_name,
                                tool_name,
                                arguments,
                                thought_signature,
                            });
                        }
                    }
                }
            }
        }
    }

    calls
}

/// Format conversation messages for OpenAI-compatible API according to the spec.
///
/// This properly formats:
/// - Assistant messages with tool calls (using `tool_calls` array)
/// - Tool result messages (using `tool_call_id`)
/// - Regular text messages
/// - Skips "Thinking" messages as they are internal state
pub fn format_messages_for_openai(app_handle: &AppHandle, msgs: &[Message]) -> Vec<Value> {
    let mut formatted = Vec::new();

    // Collect IDs of most recent images/pdfs across all messages
    let mut valid_attachments = Vec::new();
    for msg in msgs.iter().rev() {
        for attachment in msg.attachments.iter().rev() {
            if valid_attachments.len() < MAX_RECENT_ATTACHMENTS {
                valid_attachments.push(attachment.id.clone());
            }
        }
    }

    for msg in msgs {
        match msg.message_type {
            MessageType::ToolCalls => {
                let mut tool_calls = Vec::new();

                // Add tool calls from metadata array
                if let Some(metadata_vec) = &msg.metadata {
                    for meta in metadata_vec {
                        if let MessageMetadata::ToolCall { call_id, tool_name, arguments, thought_signature: _, skill_name: _ } = meta {
                            tool_calls.push(json!({
                                "id": call_id,
                                "type": "function",
                                "function": {
                                    "name": tool_name,
                                    "arguments": serde_json::to_string(arguments).unwrap_or_else(|_| "{}".to_string())
                                }
                            }));
                        }
                    }
                }

                formatted.push(json!({
                    "role": "assistant",
                    "content": if msg.content.is_empty() { Value::Null } else { json!(msg.content) },
                    "tool_calls": tool_calls
                }));
            }

            MessageType::ToolResults => {
                // Format tool results with tool_call_id
                if let Some(metadata_vec) = &msg.metadata {
                    for meta in metadata_vec {
                        if let MessageMetadata::ToolResult { call_id, result, success, error, screenshot_attachment_id } = meta {
                            let mut response_obj = if *success {
                                result.clone().unwrap_or_else(|| json!({"status": "success"}))
                            } else {
                                json!({"error": error.as_deref().unwrap_or("Unknown error")})
                            };
        
                            // Enrichment: If this is a skill activation, inject the skill instructions
                            // but don't save them to the database. This allows the LLM to get the
                            // instructions immediately without bloating the database records.
                            if *success {
                                if let Some(res_val) = result {
                                    if res_val.get("status").and_then(|s| s.as_str()) == Some("skill_activated") {
                                        if let Some(skill_name) = res_val.get("skill_name").and_then(|s| s.as_str()) {
                                            if let Some(skill) = get_skill(skill_name) {
                                                if let Some(obj) = response_obj.as_object_mut() {
                                                    obj.insert("instructions".to_string(), json!(skill.instructions));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
        
                            // Build content - can be array with image for computer-use
                            let mut content_parts = vec![json!({
                                "type": "text",
                                "text": serde_json::to_string(&response_obj).unwrap_or_else(|_| "{}".to_string())
                            })];
        
                            // If there's a screenshot attachment, add it as an image
                            if let Some(screenshot_id) = screenshot_attachment_id {
                                if !valid_attachments.contains(screenshot_id) {
                                    // Skip if not a valid attachment
                                } else {
                                    if let Some(attachment) = msg.attachments.iter().find(|a| &a.id == screenshot_id) {
                                        if attachment.file_type.starts_with("image/") {
                                            if let Some(rel_path) = &attachment.file_path {
                                                if let Ok(app_data) = app_handle.path().app_data_dir() {
                                                    let full_path = app_data.join(rel_path);
                                                    if full_path.exists() {
                                                        if let Ok(bytes) = fs::read(&full_path) {
                                                            let base64_data = general_purpose::STANDARD.encode(bytes);
                                                            content_parts.push(json!({
                                                                "type": "image_url",
                                                                "image_url": {
                                                                    "url": format!("data:{};base64,{}", attachment.file_type, base64_data)
                                                                }
                                                            }));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
        
                            formatted.push(json!({
                                "role": "tool",
                                "tool_call_id": call_id,
                                "content": content_parts
                            }));
                        }
                    }
                }
            }

            MessageType::Text => {
                let mut content_blocks = Vec::new();

                // Add attachments if any (multimodal support)
                for attachment in &msg.attachments {
                    if !valid_attachments.contains(&attachment.id) {
                        continue;
                    }

                    if attachment.file_type.starts_with("image/") {
                        // Attach image as base64 data URL
                        if let Some(rel_path) = &attachment.file_path {
                            if let Ok(app_data) = app_handle.path().app_data_dir() {
                                let full_path = app_data.join(rel_path);
                                if full_path.exists() {
                                    if let Ok(bytes) = fs::read(&full_path) {
                                        let base64_image = general_purpose::STANDARD.encode(bytes);
                                        content_blocks.push(json!({
                                            "type": "image_url",
                                            "image_url": {
                                                "url": format!("data:{};base64,{}", attachment.file_type, base64_image)
                                            }
                                        }));
                                    }
                                }
                            }
                        }
                    } else if attachment.file_type == "application/pdf" {
                        // Extract text from PDF and attach to prompt
                        if let Some(rel_path) = &attachment.file_path {
                            if let Ok(app_data) = app_handle.path().app_data_dir() {
                                let full_path = app_data.join(rel_path);
                                if full_path.exists() {
                                    if let Ok(bytes) = fs::read(&full_path) {
                                        if let Ok(pdf_text) = pdf_extract::extract_text_from_mem(&bytes) {
                                            content_blocks.push(json!({
                                                "type": "text",
                                                "text": format!("Extracted text from {}:\n{}", attachment.file_name, pdf_text)
                                            }));
                                        }
                                    }
                                }
                            }
                        }
                    } else if attachment.file_type == "ambient/ocr" {
                        // Attach OCR text
                        if let Some(extracted_text) = &attachment.extracted_text {
                            content_blocks.push(json!({
                                "type": "text",
                                "text": format!("Extracted text from user's screen:\n{}", extracted_text)
                            }));
                        }
                    }
                }

                // Add text content last
                content_blocks.push(json!({
                    "type": "text",
                    "text": msg.content
                }));

                // Regular text message
                let role = match msg.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                };

                let content_value = if content_blocks.len() == 1 && content_blocks[0].get("type").and_then(|v| v.as_str()) == Some("text") {
                    content_blocks[0].get("text").cloned().unwrap_or_else(|| json!(msg.content))
                } else {
                    json!(content_blocks)
                };

                formatted.push(json!({
                    "role": role,
                    "content": content_value
                }));
            }
        }
    }

    formatted
}

/// Format conversation messages for Gemini API.
///
/// This properly formats:
/// - Text messages with multimodal attachments (images, PDFs)
/// - Tool call messages (using `functionCall` part)
/// - Tool result messages (using `functionResponse` part)
/// - Thinking messages (using text parts with tags)
/// - Merges consecutive parts with the same role (required by Gemini API)
pub fn format_messages_for_gemini(app_handle: &AppHandle, msgs: &[Message]) -> Vec<Value> {
    let mut formatted = Vec::new();

    // Collect IDs of most recent images/pdfs across all messages
    let mut valid_attachments = Vec::new();
    for msg in msgs.iter().rev() {
        for attachment in msg.attachments.iter().rev() {
            if valid_attachments.len() < MAX_RECENT_ATTACHMENTS {
                valid_attachments.push(attachment.id.clone());
            }
        }
    }

    for msg in msgs {
        match msg.message_type {
            MessageType::ToolCalls => {
                let mut parts = Vec::new();

                // Add text content first
                if !msg.content.is_empty() {
                    parts.push(json!({"text": msg.content}));
                }

                // Add tool calls
                if let Some(metadata_vec) = &msg.metadata {
                    for meta in metadata_vec {
                        if let MessageMetadata::ToolCall { tool_name, arguments, thought_signature, skill_name: _, .. } = meta {
                            parts.push(json!({
                                "functionCall": {
                                    "name": tool_name,
                                    "args": arguments
                                }
                            }));
        
                            // Re-attach thought signature if we have one
                            if let Some(signature) = thought_signature {
                                parts.last_mut().unwrap()["thoughtSignature"] = json!(signature);
                            }                    
                        }
                    }
                }
                if !parts.is_empty() {
                    formatted.push(json!({
                        "role": "model",
                        "parts": parts
                    }));
                }
            }

            MessageType::ToolResults => {
                let mut parts = Vec::new();

                if let Some(metadata_vec) = &msg.metadata {
                    for meta in metadata_vec {
                        if let MessageMetadata::ToolResult { call_id, result, success, error, screenshot_attachment_id } = meta {
                            let mut tool_name = "unknown".to_string();
                            
                            // Try to find the matching tool call to get the name
                            if let Some(call_msg) = msgs.iter().find(|m| {
                                if let Some(m_meta_vec) = &m.metadata {
                                    m_meta_vec.iter().any(|m_meta| {
                                        if let MessageMetadata::ToolCall { call_id: cid, .. } = m_meta {
                                            cid == call_id
                                        } else {
                                            false
                                        }
                                    })
                                } else {
                                    false
                                }
                            }) {
                                if let Some(call_meta_vec) = &call_msg.metadata {
                                    for call_meta in call_meta_vec {
                                        if let MessageMetadata::ToolCall { call_id: cid, tool_name: tn, .. } = call_meta {
                                            if cid == call_id {
                                                tool_name = tn.clone();
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
        
                            let mut response_obj = if *success {
                                result.clone().unwrap_or_else(|| json!({"status": "success"}))
                            } else {
                                json!({"error": error.as_deref().unwrap_or("Unknown error")})
                            };
        
                            // Ensure response is an object as Gemini expects a Struct
                            if !response_obj.is_object() {
                                response_obj = json!({ "output": response_obj });
                            }
        
                            // Enrichment: If this is a skill activation, inject the skill instructions
                            // but don't save them to the database. This allows the LLM to get the
                            // instructions immediately without bloating the database records.
                            if *success {
                                if let Some(res_val) = result {
                                    if res_val.get("status").and_then(|s| s.as_str()) == Some("skill_activated") {
                                        if let Some(skill_name) = res_val.get("skill_name").and_then(|s| s.as_str()) {
                                            if let Some(skill) = get_skill(skill_name) {
                                                if let Some(obj) = response_obj.as_object_mut() {
                                                    obj.insert("instructions".to_string(), json!(skill.instructions));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
        
                            // Build functionResponse with optional screenshot parts
                            let mut func_response = json!({
                                "functionResponse": {
                                    "name": tool_name,
                                    "response": response_obj
                                }
                            });
        
                            // If there's a screenshot attachment, add it as parts with inlineData
                            if let Some(screenshot_id) = screenshot_attachment_id {
                                if !valid_attachments.contains(screenshot_id) {
                                    // Skip if not a valid attachment
                                } else {
                                    // Find the attachment in the message
                                    if let Some(attachment) = msg.attachments.iter().find(|a| &a.id == screenshot_id) {
                                        if attachment.file_type.starts_with("image/") {
                                            if let Some(rel_path) = &attachment.file_path {
                                                if let Ok(app_data) = app_handle.path().app_data_dir() {
                                                    let full_path = app_data.join(rel_path);
                                                    if full_path.exists() {
                                                        if let Ok(bytes) = fs::read(&full_path) {
                                                            let base64_data = general_purpose::STANDARD.encode(bytes);
                                                            // Add parts array with inlineData for Gemini computer-use
                                                            func_response["functionResponse"]["parts"] = json!([{
                                                                "inlineData": {
                                                                    "mimeType": attachment.file_type,
                                                                    "data": base64_data,
                                                                }
                                                            }]);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            parts.push(func_response);
                        }
                    }
                }
                if !parts.is_empty() {
                    formatted.push(json!({
                        "role": "user",
                        "parts": parts
                    }));
                }
            }

            MessageType::Text => {
                let role = match msg.role {
                    Role::Assistant => "model",
                    _ => "user",
                };

                let mut parts = Vec::new();
                
                // Add text content first
                if !msg.content.is_empty() {
                    parts.push(json!({"text": msg.content}));
                }
                
                // Add attachments
                for attachment in &msg.attachments {
                    if !valid_attachments.contains(&attachment.id) {
                        continue;
                    }

                    if attachment.file_type.starts_with("image/") || attachment.file_type == "application/pdf" {
                        if let Some(rel_path) = &attachment.file_path {
                            if let Ok(app_data) = app_handle.path().app_data_dir() {
                                let full_path = app_data.join(rel_path);
                                if full_path.exists() {
                                    if let Ok(bytes) = fs::read(&full_path) {
                                        let base64_data = general_purpose::STANDARD.encode(bytes);
                                        parts.push(json!({
                                            "inlineData": {
                                                "mimeType": attachment.file_type,
                                                "data": base64_data,
                                            },
                                        }));
                                    }
                                }
                            }
                        }
                    } else if attachment.file_type == "ambient/ocr" {
                         if let Some(extracted_text) = &attachment.extracted_text {
                            parts.push(json!({
                                "text": format!("Extracted text from user's screen:\n{}", extracted_text)
                            }));
                        }
                    }
                }
                if !parts.is_empty() {
                    formatted.push(json!({
                        "role": role,
                        "parts": parts
                    }));
                }
            }
        };
    }

    formatted
}

/// Checks if an OpenAI response contains tool calls.
pub fn has_tool_calls_openai(response: &Value) -> bool {
    response
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("tool_calls"))
        .and_then(|t| t.as_array())
        .map(|a| !a.is_empty())
        .unwrap_or(false)
}

/// Checks if a Gemini response contains tool calls.
pub fn has_tool_calls_gemini(response: &Value) -> bool {
    response
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .map(|parts| parts.iter().any(|p| p.get("functionCall").is_some()))
        .unwrap_or(false)
}

/// Extracts text content from an OpenAI response.
///
/// Returns the assistant's text response when no tool calls are present.
pub fn extract_text_openai(response: &Value) -> Option<String> {
    response
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .map(|s| s.to_string())
}

/// Extracts text content from a Gemini response.
///
/// Returns the assistant's text response when no tool calls are present.
pub fn extract_text_gemini(response: &Value) -> Option<String> {
    response
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .and_then(|parts| {
            let mut full_text = String::new();
            for part in parts {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    full_text.push_str(text);
                }
            }
            if full_text.is_empty() {
                None
            } else {
                Some(full_text)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_openai_format() {
        let tool = ToolDefinition {
            name: "test_tool".to_string(),
            skill_name: Some("test".to_string()),
            description: "A test tool".to_string(),
            parameters: vec![],
            returns: None,
        };

        let result = tools_to_openai_format(&[tool]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "function");
        assert_eq!(result[0]["function"]["name"], "test_tool");
    }

    #[test]
    fn test_parse_openai_tool_calls() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_123",
                        "function": {
                            "name": "test.skill.search",
                            "arguments": "{\"query\":\"test\"}"
                        }
                    }]
                }
            }]
        });

        let calls = parse_openai_tool_calls(&response, Some(&[ToolDefinition {
            name: "test.skill.search".to_string(),
            skill_name: Some("test".to_string()),
            description: "A test tool".to_string(),
            parameters: vec![],
            returns: None,
        }]));
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].skill_name, "test");
        assert_eq!(calls[0].tool_name, "skill.search");
    }

    #[test]
    fn test_gemini_format() {
        let tool = ToolDefinition {
            name: "test_tool".to_string(),
            skill_name: Some("test".to_string()),
            description: "A test tool".to_string(),
            parameters: vec![],
            returns: None,
        };

        let result = tools_to_gemini_format(&[tool]);
        assert!(result["functionDeclarations"].is_array());
        assert_eq!(result["functionDeclarations"][0]["name"], "test_tool");
    }

    #[test]
    fn test_has_tool_calls() {
        let openai_response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{"id": "1", "function": {"name": "test"}}]
                }
            }]
        });

        let gemini_response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {"name": "test"}
                    }]
                }
            }]
        });

        assert!(has_tool_calls_openai(&openai_response));
        assert!(has_tool_calls_gemini(&gemini_response));
    }
}
