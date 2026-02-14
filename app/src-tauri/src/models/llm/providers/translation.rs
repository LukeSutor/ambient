//! Tool format translation layer.
//!
//! Converts between the unified internal tool format and
//! provider-specific formats for OpenAI (local), Gemini (cloud),
//! and Anthropic (cloud).
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
        
                            // Build content - can be array with image
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
                                                            // Add parts array with inlineData for Gemini function response
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

// ---------------------------------------------------------------------------
// Anthropic Messages API
// ---------------------------------------------------------------------------

/// Translates tool definitions to Anthropic tool format.
///
/// Anthropic uses `input_schema` (JSON Schema) instead of `parameters`.
/// Type names use lowercase JSON Schema types (same as OpenAI).
pub fn tools_to_anthropic_format(tools: &[ToolDefinition]) -> Vec<Value> {
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
                "name": tool.name,
                "description": tool.description,
                "input_schema": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                }
            })
        })
        .collect()
}

/// Parses tool calls from an Anthropic Messages API response.
///
/// Anthropic returns tool calls as `tool_use` content blocks in the
/// assistant message's `content` array.
pub fn parse_anthropic_tool_calls(response: &Value, available_tools: Option<&[ToolDefinition]>) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    if let Some(content) = response.get("content").and_then(|c| c.as_array()) {
        for block in content {
            if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                let id = block
                    .get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or("")
                    .to_string();

                let name = block
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("")
                    .to_string();

                let arguments = block
                    .get("input")
                    .cloned()
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

    calls
}

/// Format conversation messages for Anthropic Messages API.
///
/// Key differences from OpenAI/Gemini:
/// - System messages are excluded (passed separately via `system` param)
/// - Assistant tool calls use `tool_use` content blocks
/// - Tool results use `tool_result` content blocks in `user` messages
/// - Images use `image` type with `source.type = "base64"`
/// - PDFs use `document` type with `source.type = "base64"`
pub fn format_messages_for_anthropic(app_handle: &AppHandle, msgs: &[Message]) -> Vec<Value> {
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
        // Anthropic does not support system role in messages — system prompts
        // are passed via the top-level `system` parameter.
        if msg.role == Role::System {
            continue;
        }

        match msg.message_type {
            MessageType::ToolCalls => {
                let mut content_blocks = Vec::new();

                // Add text content first (assistant thinking/commentary)
                if !msg.content.is_empty() {
                    content_blocks.push(json!({
                        "type": "text",
                        "text": msg.content
                    }));
                }

                // Add tool_use blocks
                if let Some(metadata_vec) = &msg.metadata {
                    for meta in metadata_vec {
                        if let MessageMetadata::ToolCall { call_id, tool_name, arguments, .. } = meta {
                            content_blocks.push(json!({
                                "type": "tool_use",
                                "id": call_id,
                                "name": tool_name,
                                "input": arguments
                            }));
                        }
                    }
                }

                if !content_blocks.is_empty() {
                    formatted.push(json!({
                        "role": "assistant",
                        "content": content_blocks
                    }));
                }
            }

            MessageType::ToolResults => {
                let mut content_blocks = Vec::new();

                if let Some(metadata_vec) = &msg.metadata {
                    for meta in metadata_vec {
                        if let MessageMetadata::ToolResult { call_id, result, success, error, screenshot_attachment_id } = meta {
                            let mut response_obj = if *success {
                                result.clone().unwrap_or_else(|| json!({"status": "success"}))
                            } else {
                                json!({"error": error.as_deref().unwrap_or("Unknown error")})
                            };

                            // Enrichment: inject skill instructions on activation
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

                            // Build tool_result content — can be string or array of blocks
                            let result_text = serde_json::to_string(&response_obj)
                                .unwrap_or_else(|_| "{}".to_string());

                            let mut result_content: Vec<Value> = vec![json!({
                                "type": "text",
                                "text": result_text
                            })];

                            // If there's a screenshot attachment, add it as an image block
                            if let Some(screenshot_id) = screenshot_attachment_id {
                                if valid_attachments.contains(screenshot_id) {
                                    if let Some(attachment) = msg.attachments.iter().find(|a| &a.id == screenshot_id) {
                                        if attachment.file_type.starts_with("image/") {
                                            if let Some(rel_path) = &attachment.file_path {
                                                if let Ok(app_data) = app_handle.path().app_data_dir() {
                                                    let full_path = app_data.join(rel_path);
                                                    if full_path.exists() {
                                                        if let Ok(bytes) = fs::read(&full_path) {
                                                            let base64_data = general_purpose::STANDARD.encode(bytes);
                                                            result_content.push(json!({
                                                                "type": "image",
                                                                "source": {
                                                                    "type": "base64",
                                                                    "media_type": attachment.file_type,
                                                                    "data": base64_data,
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

                            let mut tool_result = json!({
                                "type": "tool_result",
                                "tool_use_id": call_id,
                                "content": result_content
                            });

                            if !success {
                                tool_result["is_error"] = json!(true);
                            }

                            content_blocks.push(tool_result);
                        }
                    }
                }

                if !content_blocks.is_empty() {
                    formatted.push(json!({
                        "role": "user",
                        "content": content_blocks
                    }));
                }
            }

            MessageType::Text => {
                let role = match msg.role {
                    Role::Assistant => "assistant",
                    _ => "user",
                };

                let mut content_blocks = Vec::new();

                // Add attachments before text (Anthropic recommends images before text)
                for attachment in &msg.attachments {
                    if !valid_attachments.contains(&attachment.id) {
                        continue;
                    }

                    if attachment.file_type.starts_with("image/") {
                        if let Some(rel_path) = &attachment.file_path {
                            if let Ok(app_data) = app_handle.path().app_data_dir() {
                                let full_path = app_data.join(rel_path);
                                if full_path.exists() {
                                    if let Ok(bytes) = fs::read(&full_path) {
                                        let base64_image = general_purpose::STANDARD.encode(bytes);
                                        content_blocks.push(json!({
                                            "type": "image",
                                            "source": {
                                                "type": "base64",
                                                "media_type": attachment.file_type,
                                                "data": base64_image,
                                            }
                                        }));
                                    }
                                }
                            }
                        }
                    } else if attachment.file_type == "application/pdf" {
                        if let Some(rel_path) = &attachment.file_path {
                            if let Ok(app_data) = app_handle.path().app_data_dir() {
                                let full_path = app_data.join(rel_path);
                                if full_path.exists() {
                                    if let Ok(bytes) = fs::read(&full_path) {
                                        let base64_pdf = general_purpose::STANDARD.encode(bytes);
                                        content_blocks.push(json!({
                                            "type": "document",
                                            "source": {
                                                "type": "base64",
                                                "media_type": "application/pdf",
                                                "data": base64_pdf,
                                            }
                                        }));
                                    }
                                }
                            }
                        }
                    } else if attachment.file_type == "ambient/ocr" {
                        if let Some(extracted_text) = &attachment.extracted_text {
                            content_blocks.push(json!({
                                "type": "text",
                                "text": format!("Extracted text from user's screen:\n{}", extracted_text)
                            }));
                        }
                    }
                }

                // Add text content last
                if !msg.content.is_empty() {
                    content_blocks.push(json!({
                        "type": "text",
                        "text": msg.content
                    }));
                }

                if !content_blocks.is_empty() {
                    formatted.push(json!({
                        "role": role,
                        "content": content_blocks
                    }));
                }
            }
        }
    }

    formatted
}

/// Checks if an Anthropic response contains tool calls.
pub fn has_tool_calls_anthropic(response: &Value) -> bool {
    response
        .get("content")
        .and_then(|c| c.as_array())
        .map(|blocks| blocks.iter().any(|b| {
            b.get("type").and_then(|t| t.as_str()) == Some("tool_use")
        }))
        .unwrap_or(false)
}

/// Extracts text content from an Anthropic response.
///
/// Returns the concatenated text from all `text` content blocks.
pub fn extract_text_anthropic(response: &Value) -> Option<String> {
    response
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|blocks| {
            let mut full_text = String::new();
            for block in blocks {
                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        full_text.push_str(text);
                    }
                }
            }
            if full_text.is_empty() {
                None
            } else {
                Some(full_text)
            }
        })
}

// ---------------------------------------------------------------------------
// Token usage extraction
// ---------------------------------------------------------------------------

/// Extracted token counts from a provider response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
}

/// Extracts token usage from an OpenAI response.
///
/// OpenAI provides `usage.prompt_tokens` and `usage.completion_tokens`.
pub fn extract_usage_openai(response: &Value) -> Option<TokenUsage> {
    response.get("usage").map(|usage| TokenUsage {
        prompt_tokens: usage
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        completion_tokens: usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}

/// Extracts token usage from a Gemini response.
///
/// Gemini provides `usageMetadata.promptTokenCount` and
/// `usageMetadata.candidatesTokenCount`.
pub fn extract_usage_gemini(response: &Value) -> Option<TokenUsage> {
    response.get("usageMetadata").map(|usage| TokenUsage {
        prompt_tokens: usage
            .get("promptTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        completion_tokens: usage
            .get("candidatesTokenCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}

/// Extracts token usage from an Anthropic response.
///
/// Anthropic provides `usage.input_tokens` and `usage.output_tokens`.
pub fn extract_usage_anthropic(response: &Value) -> Option<TokenUsage> {
    response.get("usage").map(|usage| TokenUsage {
        prompt_tokens: usage
            .get("input_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        completion_tokens: usage
            .get("output_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    })
}

// ---------------------------------------------------------------------------
// Finish reason extraction
// ---------------------------------------------------------------------------

/// Normalised finish reason across providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    /// Model finished naturally.
    Stop,
    /// Model wants to call one or more tools.
    ToolUse,
    /// Stopped because of max token limit.
    MaxTokens,
    /// Unknown or provider-specific reason.
    Other(String),
}

/// Extracts the finish reason from an OpenAI response.
///
/// Maps `stop` → `Stop`, `tool_calls` → `ToolUse`, `length` → `MaxTokens`.
pub fn extract_finish_reason_openai(response: &Value) -> Option<FinishReason> {
    response
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("finish_reason"))
        .and_then(|r| r.as_str())
        .map(|reason| match reason {
            "stop" => FinishReason::Stop,
            "tool_calls" => FinishReason::ToolUse,
            "length" => FinishReason::MaxTokens,
            other => FinishReason::Other(other.to_string()),
        })
}

/// Extracts the finish reason from a Gemini response.
///
/// Maps `STOP` → `Stop`, `MAX_TOKENS` → `MaxTokens`. Tool calls are detected
/// via content parts rather than finish reason in Gemini.
pub fn extract_finish_reason_gemini(response: &Value) -> Option<FinishReason> {
    response
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("finishReason"))
        .and_then(|r| r.as_str())
        .map(|reason| match reason {
            "STOP" => {
                // Gemini uses STOP even for tool calls — check parts
                if has_tool_calls_gemini(response) {
                    FinishReason::ToolUse
                } else {
                    FinishReason::Stop
                }
            }
            "MAX_TOKENS" => FinishReason::MaxTokens,
            other => FinishReason::Other(other.to_string()),
        })
}

/// Extracts the finish reason from an Anthropic response.
///
/// Maps `end_turn` → `Stop`, `tool_use` → `ToolUse`, `max_tokens` → `MaxTokens`.
pub fn extract_finish_reason_anthropic(response: &Value) -> Option<FinishReason> {
    response
        .get("stop_reason")
        .and_then(|r| r.as_str())
        .map(|reason| match reason {
            "end_turn" => FinishReason::Stop,
            "tool_use" => FinishReason::ToolUse,
            "max_tokens" => FinishReason::MaxTokens,
            other => FinishReason::Other(other.to_string()),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::types::ToolParameter;
    use crate::skills::types::ParameterType;
    use serde_json::json;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_tool(name: &str, description: &str, params: Vec<ToolParameter>) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            skill_name: Some("test-skill".to_string()),
            description: description.to_string(),
            parameters: params,
            returns: None,
        }
    }

    fn make_param(name: &str, param_type: ParameterType, required: bool) -> ToolParameter {
        ToolParameter {
            name: name.to_string(),
            param_type,
            description: format!("{} parameter", name),
            required,
            default: None,
        }
    }

    fn simple_tool() -> ToolDefinition {
        make_tool("search_web", "Search the web", vec![
            make_param("query", ParameterType::String, true),
            make_param("max_results", ParameterType::Integer, false),
        ])
    }

    fn no_params_tool() -> ToolDefinition {
        make_tool("get_time", "Get the current time", vec![])
    }

    fn multi_param_tool() -> ToolDefinition {
        make_tool("calculate", "Perform a calculation", vec![
            make_param("expression", ParameterType::String, true),
            make_param("precision", ParameterType::Integer, false),
            make_param("use_radians", ParameterType::Boolean, false),
            make_param("values", ParameterType::Array, false),
            make_param("options", ParameterType::Object, false),
        ])
    }

    // =======================================================================
    // Tool definition formatting
    // =======================================================================

    // -- OpenAI -------------------------------------------------------------

    #[test]
    fn test_openai_tools_single_with_params() {
        let tools = vec![simple_tool()];
        let result = tools_to_openai_format(&tools);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["type"], "function");
        assert_eq!(result[0]["function"]["name"], "search_web");
        assert_eq!(result[0]["function"]["description"], "Search the web");

        let params = &result[0]["function"]["parameters"];
        assert_eq!(params["type"], "object");
        assert_eq!(params["properties"]["query"]["type"], "string");
        assert_eq!(params["properties"]["max_results"]["type"], "integer");
        assert_eq!(params["required"], json!(["query"]));
    }

    #[test]
    fn test_openai_tools_no_params() {
        let tools = vec![no_params_tool()];
        let result = tools_to_openai_format(&tools);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["function"]["name"], "get_time");
        assert_eq!(result[0]["function"]["parameters"]["required"], json!([]));
        assert!(result[0]["function"]["parameters"]["properties"].as_object().unwrap().is_empty());
    }

    #[test]
    fn test_openai_tools_multiple() {
        let tools = vec![simple_tool(), no_params_tool()];
        let result = tools_to_openai_format(&tools);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["function"]["name"], "search_web");
        assert_eq!(result[1]["function"]["name"], "get_time");
    }

    #[test]
    fn test_openai_tools_all_param_types() {
        let tools = vec![multi_param_tool()];
        let result = tools_to_openai_format(&tools);
        let props = &result[0]["function"]["parameters"]["properties"];

        assert_eq!(props["expression"]["type"], "string");
        assert_eq!(props["precision"]["type"], "integer");
        assert_eq!(props["use_radians"]["type"], "boolean");
        assert_eq!(props["values"]["type"], "array");
        assert_eq!(props["options"]["type"], "object");
    }

    #[test]
    fn test_openai_tools_empty() {
        let result = tools_to_openai_format(&[]);
        assert!(result.is_empty());
    }

    // -- Gemini -------------------------------------------------------------

    #[test]
    fn test_gemini_tools_single_with_params() {
        let tools = vec![simple_tool()];
        let result = tools_to_gemini_format(&tools);

        let decls = result["functionDeclarations"].as_array().unwrap();
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0]["name"], "search_web");
        assert_eq!(decls[0]["description"], "Search the web");

        let params = &decls[0]["parameters"];
        assert_eq!(params["type"], "OBJECT");
        assert_eq!(params["properties"]["query"]["type"], "STRING");
        assert_eq!(params["properties"]["max_results"]["type"], "INTEGER");
        assert_eq!(params["required"], json!(["query"]));
    }

    #[test]
    fn test_gemini_tools_no_params() {
        let tools = vec![no_params_tool()];
        let result = tools_to_gemini_format(&tools);
        let decls = result["functionDeclarations"].as_array().unwrap();

        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0]["name"], "get_time");
        assert_eq!(decls[0]["parameters"]["required"], json!([]));
    }

    #[test]
    fn test_gemini_tools_multiple() {
        let tools = vec![simple_tool(), no_params_tool()];
        let result = tools_to_gemini_format(&tools);
        let decls = result["functionDeclarations"].as_array().unwrap();
        assert_eq!(decls.len(), 2);
        assert_eq!(decls[0]["name"], "search_web");
        assert_eq!(decls[1]["name"], "get_time");
    }

    #[test]
    fn test_gemini_tools_all_param_types() {
        let tools = vec![multi_param_tool()];
        let result = tools_to_gemini_format(&tools);
        let props = &result["functionDeclarations"][0]["parameters"]["properties"];

        assert_eq!(props["expression"]["type"], "STRING");
        assert_eq!(props["precision"]["type"], "INTEGER");
        assert_eq!(props["use_radians"]["type"], "BOOLEAN");
        assert_eq!(props["values"]["type"], "ARRAY");
        assert_eq!(props["options"]["type"], "OBJECT");
    }

    #[test]
    fn test_gemini_tools_empty() {
        let result = tools_to_gemini_format(&[]);
        assert!(result["functionDeclarations"].as_array().unwrap().is_empty());
    }

    // -- Anthropic ----------------------------------------------------------

    #[test]
    fn test_anthropic_tools_single_with_params() {
        let tools = vec![simple_tool()];
        let result = tools_to_anthropic_format(&tools);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "search_web");
        assert_eq!(result[0]["description"], "Search the web");

        let schema = &result[0]["input_schema"];
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["query"]["type"], "string");
        assert_eq!(schema["properties"]["query"]["description"], "query parameter");
        assert_eq!(schema["properties"]["max_results"]["type"], "integer");
        assert_eq!(schema["required"], json!(["query"]));
    }

    #[test]
    fn test_anthropic_tools_no_params() {
        let tools = vec![no_params_tool()];
        let result = tools_to_anthropic_format(&tools);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0]["name"], "get_time");
        assert_eq!(result[0]["input_schema"]["required"], json!([]));
        assert!(result[0]["input_schema"]["properties"].as_object().unwrap().is_empty());
    }

    #[test]
    fn test_anthropic_tools_multiple() {
        let tools = vec![simple_tool(), no_params_tool()];
        let result = tools_to_anthropic_format(&tools);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["name"], "search_web");
        assert_eq!(result[1]["name"], "get_time");
    }

    #[test]
    fn test_anthropic_tools_all_param_types() {
        let tools = vec![multi_param_tool()];
        let result = tools_to_anthropic_format(&tools);
        let props = &result[0]["input_schema"]["properties"];

        // Anthropic uses lowercase JSON Schema types (same as OpenAI)
        assert_eq!(props["expression"]["type"], "string");
        assert_eq!(props["precision"]["type"], "integer");
        assert_eq!(props["use_radians"]["type"], "boolean");
        assert_eq!(props["values"]["type"], "array");
        assert_eq!(props["options"]["type"], "object");
    }

    #[test]
    fn test_anthropic_tools_empty() {
        let result = tools_to_anthropic_format(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_anthropic_tools_no_function_wrapper() {
        // Anthropic format does NOT use a "function" wrapper like OpenAI
        let tools = vec![simple_tool()];
        let result = tools_to_anthropic_format(&tools);

        assert!(result[0].get("type").is_none(), "Anthropic tools should not have a 'type' wrapper");
        assert!(result[0].get("function").is_none(), "Anthropic tools should not have 'function' wrapper");
        assert!(result[0].get("name").is_some(), "Name should be at top level");
        assert!(result[0].get("input_schema").is_some(), "input_schema should be at top level");
    }

    // =======================================================================
    // Parsing tool calls from responses
    // =======================================================================

    // -- OpenAI -------------------------------------------------------------

    #[test]
    fn test_parse_openai_single_tool_call() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_abc123",
                        "type": "function",
                        "function": {
                            "name": "search_web",
                            "arguments": "{\"query\":\"rust programming\"}"
                        }
                    }]
                }
            }]
        });

        let available = vec![simple_tool()];
        let calls = parse_openai_tool_calls(&response, Some(&available));

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "call_abc123");
        assert_eq!(calls[0].tool_name, "search_web");
        assert_eq!(calls[0].skill_name, "test-skill");
        assert_eq!(calls[0].arguments["query"], "rust programming");
        assert!(calls[0].thought_signature.is_none());
    }

    #[test]
    fn test_parse_openai_multiple_tool_calls() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [
                        {
                            "id": "call_1",
                            "function": {
                                "name": "search_web",
                                "arguments": "{\"query\":\"a\"}"
                            }
                        },
                        {
                            "id": "call_2",
                            "function": {
                                "name": "get_time",
                                "arguments": "{}"
                            }
                        }
                    ]
                }
            }]
        });

        let calls = parse_openai_tool_calls(&response, None);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].id, "call_1");
        assert_eq!(calls[1].id, "call_2");
    }

    #[test]
    fn test_parse_openai_dot_separated_name() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_1",
                        "function": {
                            "name": "web-search.search_web",
                            "arguments": "{\"query\":\"test\"}"
                        }
                    }]
                }
            }]
        });

        let calls = parse_openai_tool_calls(&response, None);
        assert_eq!(calls[0].skill_name, "web-search");
        assert_eq!(calls[0].tool_name, "search_web");
    }

    #[test]
    fn test_parse_openai_activate_skill() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_1",
                        "function": {
                            "name": "activate_skill",
                            "arguments": "{\"skill_name\":\"code-execution\"}"
                        }
                    }]
                }
            }]
        });

        let calls = parse_openai_tool_calls(&response, None);
        assert_eq!(calls[0].skill_name, "system");
        assert_eq!(calls[0].tool_name, "activate_skill");
    }

    #[test]
    fn test_parse_openai_no_tool_calls() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": "Hello world"
                }
            }]
        });

        let calls = parse_openai_tool_calls(&response, None);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_openai_invalid_json_arguments() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "call_1",
                        "function": {
                            "name": "search_web",
                            "arguments": "not valid json"
                        }
                    }]
                }
            }]
        });

        let calls = parse_openai_tool_calls(&response, None);
        assert_eq!(calls.len(), 1);
        // Invalid JSON falls back to empty object
        assert_eq!(calls[0].arguments, json!({}));
    }

    #[test]
    fn test_parse_openai_empty_response() {
        let calls = parse_openai_tool_calls(&json!({}), None);
        assert!(calls.is_empty());
    }

    // -- Gemini -------------------------------------------------------------

    #[test]
    fn test_parse_gemini_single_tool_call() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "search_web",
                            "args": {"query": "rust programming"}
                        }
                    }]
                }
            }]
        });

        let available = vec![simple_tool()];
        let calls = parse_gemini_tool_calls(&response, Some(&available));

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "search_web");
        assert_eq!(calls[0].skill_name, "test-skill");
        assert_eq!(calls[0].arguments["query"], "rust programming");
        // Gemini generates UUIDs for call IDs
        assert!(!calls[0].id.is_empty());
    }

    #[test]
    fn test_parse_gemini_multiple_tool_calls() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {
                            "functionCall": {
                                "name": "search_web",
                                "args": {"query": "a"}
                            }
                        },
                        {
                            "functionCall": {
                                "name": "get_time",
                                "args": {}
                            }
                        }
                    ]
                }
            }]
        });

        let calls = parse_gemini_tool_calls(&response, None);
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn test_parse_gemini_with_thought_signature() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "search_web",
                            "args": {"query": "test"}
                        },
                        "thoughtSignature": "abc123signature"
                    }]
                }
            }]
        });

        let calls = parse_gemini_tool_calls(&response, None);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].thought_signature, Some("abc123signature".to_string()));
    }

    #[test]
    fn test_parse_gemini_no_tool_calls() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "Hello world"
                    }]
                }
            }]
        });

        let calls = parse_gemini_tool_calls(&response, None);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_gemini_empty_response() {
        let calls = parse_gemini_tool_calls(&json!({}), None);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_gemini_dot_separated_name() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "web-search.search_web",
                            "args": {}
                        }
                    }]
                }
            }]
        });

        let calls = parse_gemini_tool_calls(&response, None);
        assert_eq!(calls[0].skill_name, "web-search");
        assert_eq!(calls[0].tool_name, "search_web");
    }

    // -- Anthropic ----------------------------------------------------------

    #[test]
    fn test_parse_anthropic_single_tool_call() {
        let response = json!({
            "id": "msg_01XFDUDYJgAACzvnptvVoYEL",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "tool_use",
                    "id": "toolu_01A09q90qw90lq917835lq9",
                    "name": "search_web",
                    "input": {"query": "rust programming"}
                }
            ],
            "stop_reason": "tool_use"
        });

        let available = vec![simple_tool()];
        let calls = parse_anthropic_tool_calls(&response, Some(&available));

        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "toolu_01A09q90qw90lq917835lq9");
        assert_eq!(calls[0].tool_name, "search_web");
        assert_eq!(calls[0].skill_name, "test-skill");
        assert_eq!(calls[0].arguments["query"], "rust programming");
        assert!(calls[0].thought_signature.is_none());
    }

    #[test]
    fn test_parse_anthropic_multiple_tool_calls() {
        let response = json!({
            "content": [
                {
                    "type": "text",
                    "text": "I'll check both for you."
                },
                {
                    "type": "tool_use",
                    "id": "toolu_01",
                    "name": "search_web",
                    "input": {"query": "weather"}
                },
                {
                    "type": "tool_use",
                    "id": "toolu_02",
                    "name": "get_time",
                    "input": {}
                }
            ]
        });

        let calls = parse_anthropic_tool_calls(&response, None);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].id, "toolu_01");
        assert_eq!(calls[0].tool_name, "search_web");
        assert_eq!(calls[1].id, "toolu_02");
        assert_eq!(calls[1].tool_name, "get_time");
    }

    #[test]
    fn test_parse_anthropic_text_blocks_ignored() {
        let response = json!({
            "content": [
                {"type": "text", "text": "Let me search that."},
                {
                    "type": "tool_use",
                    "id": "toolu_01",
                    "name": "search_web",
                    "input": {"query": "test"}
                }
            ]
        });

        let calls = parse_anthropic_tool_calls(&response, None);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool_name, "search_web");
    }

    #[test]
    fn test_parse_anthropic_no_tool_calls() {
        let response = json!({
            "content": [
                {"type": "text", "text": "Hello world"}
            ]
        });

        let calls = parse_anthropic_tool_calls(&response, None);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_anthropic_empty_response() {
        let calls = parse_anthropic_tool_calls(&json!({}), None);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_anthropic_dot_separated_name() {
        let response = json!({
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "web-search.search_web",
                "input": {"query": "test"}
            }]
        });

        let calls = parse_anthropic_tool_calls(&response, None);
        assert_eq!(calls[0].skill_name, "web-search");
        assert_eq!(calls[0].tool_name, "search_web");
    }

    #[test]
    fn test_parse_anthropic_activate_skill() {
        let response = json!({
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "activate_skill",
                "input": {"skill_name": "code-execution"}
            }]
        });

        let calls = parse_anthropic_tool_calls(&response, None);
        assert_eq!(calls[0].skill_name, "system");
        assert_eq!(calls[0].tool_name, "activate_skill");
    }

    #[test]
    fn test_parse_anthropic_missing_input() {
        let response = json!({
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "get_time"
            }]
        });

        let calls = parse_anthropic_tool_calls(&response, None);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].arguments, json!({}));
    }

    // =======================================================================
    // has_tool_calls
    // =======================================================================

    #[test]
    fn test_has_tool_calls_openai_true() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{"id": "1", "function": {"name": "test", "arguments": "{}"}}]
                }
            }]
        });
        assert!(has_tool_calls_openai(&response));
    }

    #[test]
    fn test_has_tool_calls_openai_false_no_calls() {
        let response = json!({
            "choices": [{
                "message": {
                    "content": "Hello"
                }
            }]
        });
        assert!(!has_tool_calls_openai(&response));
    }

    #[test]
    fn test_has_tool_calls_openai_false_empty_array() {
        let response = json!({
            "choices": [{
                "message": {
                    "tool_calls": []
                }
            }]
        });
        assert!(!has_tool_calls_openai(&response));
    }

    #[test]
    fn test_has_tool_calls_openai_false_empty() {
        assert!(!has_tool_calls_openai(&json!({})));
    }

    #[test]
    fn test_has_tool_calls_gemini_true() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {"name": "test", "args": {}}
                    }]
                }
            }]
        });
        assert!(has_tool_calls_gemini(&response));
    }

    #[test]
    fn test_has_tool_calls_gemini_false_text_only() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{"text": "Hello"}]
                }
            }]
        });
        assert!(!has_tool_calls_gemini(&response));
    }

    #[test]
    fn test_has_tool_calls_gemini_false_empty() {
        assert!(!has_tool_calls_gemini(&json!({})));
    }

    #[test]
    fn test_has_tool_calls_anthropic_true() {
        let response = json!({
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "test",
                "input": {}
            }]
        });
        assert!(has_tool_calls_anthropic(&response));
    }

    #[test]
    fn test_has_tool_calls_anthropic_true_mixed() {
        let response = json!({
            "content": [
                {"type": "text", "text": "Let me help."},
                {"type": "tool_use", "id": "toolu_01", "name": "test", "input": {}}
            ]
        });
        assert!(has_tool_calls_anthropic(&response));
    }

    #[test]
    fn test_has_tool_calls_anthropic_false_text_only() {
        let response = json!({
            "content": [{"type": "text", "text": "Hello"}]
        });
        assert!(!has_tool_calls_anthropic(&response));
    }

    #[test]
    fn test_has_tool_calls_anthropic_false_empty_content() {
        let response = json!({"content": []});
        assert!(!has_tool_calls_anthropic(&response));
    }

    #[test]
    fn test_has_tool_calls_anthropic_false_no_content() {
        assert!(!has_tool_calls_anthropic(&json!({})));
    }

    // =======================================================================
    // extract_text
    // =======================================================================

    #[test]
    fn test_extract_text_openai_simple() {
        let response = json!({
            "choices": [{
                "message": {"content": "Hello world"}
            }]
        });
        assert_eq!(extract_text_openai(&response), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_text_openai_null_content() {
        let response = json!({
            "choices": [{
                "message": {"content": null}
            }]
        });
        assert_eq!(extract_text_openai(&response), None);
    }

    #[test]
    fn test_extract_text_openai_empty() {
        assert_eq!(extract_text_openai(&json!({})), None);
    }

    #[test]
    fn test_extract_text_gemini_simple() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{"text": "Hello world"}]
                }
            }]
        });
        assert_eq!(extract_text_gemini(&response), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_text_gemini_multiple_parts() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {"text": "Hello "},
                        {"text": "world"}
                    ]
                }
            }]
        });
        assert_eq!(extract_text_gemini(&response), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_text_gemini_no_text_parts() {
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {"name": "test", "args": {}}
                    }]
                }
            }]
        });
        assert_eq!(extract_text_gemini(&response), None);
    }

    #[test]
    fn test_extract_text_gemini_empty() {
        assert_eq!(extract_text_gemini(&json!({})), None);
    }

    #[test]
    fn test_extract_text_anthropic_simple() {
        let response = json!({
            "content": [{"type": "text", "text": "Hello world"}]
        });
        assert_eq!(extract_text_anthropic(&response), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_text_anthropic_multiple_text_blocks() {
        let response = json!({
            "content": [
                {"type": "text", "text": "Hello "},
                {"type": "text", "text": "world"}
            ]
        });
        assert_eq!(extract_text_anthropic(&response), Some("Hello world".to_string()));
    }

    #[test]
    fn test_extract_text_anthropic_mixed_with_tool_use() {
        let response = json!({
            "content": [
                {"type": "text", "text": "Let me search."},
                {"type": "tool_use", "id": "toolu_01", "name": "search_web", "input": {}}
            ]
        });
        assert_eq!(extract_text_anthropic(&response), Some("Let me search.".to_string()));
    }

    #[test]
    fn test_extract_text_anthropic_only_tool_use() {
        let response = json!({
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "search_web",
                "input": {}
            }]
        });
        assert_eq!(extract_text_anthropic(&response), None);
    }

    #[test]
    fn test_extract_text_anthropic_empty() {
        assert_eq!(extract_text_anthropic(&json!({})), None);
    }

    #[test]
    fn test_extract_text_anthropic_empty_content() {
        let response = json!({"content": []});
        assert_eq!(extract_text_anthropic(&response), None);
    }

    // =======================================================================
    // resolve_tool_call
    // =======================================================================

    #[test]
    fn test_resolve_dot_separated() {
        let (skill, tool) = resolve_tool_call("web-search.search_web", None);
        assert_eq!(skill, "web-search");
        assert_eq!(tool, "search_web");
    }

    #[test]
    fn test_resolve_activate_skill() {
        let (skill, tool) = resolve_tool_call("activate_skill", None);
        assert_eq!(skill, "system");
        assert_eq!(tool, "activate_skill");
    }

    #[test]
    fn test_resolve_from_available_tools() {
        let tools = vec![simple_tool()];
        let (skill, tool) = resolve_tool_call("search_web", Some(&tools));
        assert_eq!(skill, "test-skill");
        assert_eq!(tool, "search_web");
    }

    #[test]
    fn test_resolve_unknown_no_tools() {
        let (skill, tool) = resolve_tool_call("unknown_tool", None);
        assert_eq!(skill, "unknown");
        assert_eq!(tool, "unknown_tool");
    }

    #[test]
    fn test_resolve_unknown_not_in_tools() {
        let tools = vec![simple_tool()];
        let (skill, tool) = resolve_tool_call("other_tool", Some(&tools));
        assert_eq!(skill, "unknown");
        assert_eq!(tool, "other_tool");
    }

    #[test]
    fn test_resolve_multiple_dots() {
        // splitn(2, '.') should split at first dot only
        let (skill, tool) = resolve_tool_call("a.b.c", None);
        assert_eq!(skill, "a");
        assert_eq!(tool, "b.c");
    }

    // =======================================================================
    // Cross-provider consistency
    // =======================================================================

    #[test]
    fn test_all_providers_produce_same_tool_count() {
        let tools = vec![simple_tool(), no_params_tool(), multi_param_tool()];

        let openai = tools_to_openai_format(&tools);
        let gemini = tools_to_gemini_format(&tools);
        let anthropic = tools_to_anthropic_format(&tools);

        assert_eq!(openai.len(), 3);
        assert_eq!(gemini["functionDeclarations"].as_array().unwrap().len(), 3);
        assert_eq!(anthropic.len(), 3);
    }

    #[test]
    fn test_all_providers_preserve_tool_names() {
        let tools = vec![simple_tool()];

        let openai = tools_to_openai_format(&tools);
        let gemini = tools_to_gemini_format(&tools);
        let anthropic = tools_to_anthropic_format(&tools);

        assert_eq!(openai[0]["function"]["name"], "search_web");
        assert_eq!(gemini["functionDeclarations"][0]["name"], "search_web");
        assert_eq!(anthropic[0]["name"], "search_web");
    }

    #[test]
    fn test_all_providers_preserve_required_params() {
        let tools = vec![simple_tool()];

        let openai = tools_to_openai_format(&tools);
        let gemini = tools_to_gemini_format(&tools);
        let anthropic = tools_to_anthropic_format(&tools);

        assert_eq!(openai[0]["function"]["parameters"]["required"], json!(["query"]));
        assert_eq!(gemini["functionDeclarations"][0]["parameters"]["required"], json!(["query"]));
        assert_eq!(anthropic[0]["input_schema"]["required"], json!(["query"]));
    }

    #[test]
    fn test_all_providers_parse_tool_calls_consistently() {
        let tools = vec![simple_tool()];

        let openai_resp = json!({
            "choices": [{"message": {"tool_calls": [{
                "id": "call_1",
                "function": {"name": "search_web", "arguments": "{\"query\":\"test\"}"}
            }]}}]
        });

        let gemini_resp = json!({
            "candidates": [{"content": {"parts": [{
                "functionCall": {"name": "search_web", "args": {"query": "test"}}
            }]}}]
        });

        let anthropic_resp = json!({
            "content": [{"type": "tool_use", "id": "toolu_01", "name": "search_web", "input": {"query": "test"}}]
        });

        let openai_calls = parse_openai_tool_calls(&openai_resp, Some(&tools));
        let gemini_calls = parse_gemini_tool_calls(&gemini_resp, Some(&tools));
        let anthropic_calls = parse_anthropic_tool_calls(&anthropic_resp, Some(&tools));

        // All should produce exactly one call
        assert_eq!(openai_calls.len(), 1);
        assert_eq!(gemini_calls.len(), 1);
        assert_eq!(anthropic_calls.len(), 1);

        // All should resolve to the same skill and tool
        assert_eq!(openai_calls[0].skill_name, "test-skill");
        assert_eq!(gemini_calls[0].skill_name, "test-skill");
        assert_eq!(anthropic_calls[0].skill_name, "test-skill");

        assert_eq!(openai_calls[0].tool_name, "search_web");
        assert_eq!(gemini_calls[0].tool_name, "search_web");
        assert_eq!(anthropic_calls[0].tool_name, "search_web");

        // All should have the same arguments
        assert_eq!(openai_calls[0].arguments["query"], "test");
        assert_eq!(gemini_calls[0].arguments["query"], "test");
        assert_eq!(anthropic_calls[0].arguments["query"], "test");
    }

    // =======================================================================
    // Token usage extraction
    // =======================================================================

    #[test]
    fn test_extract_usage_openai() {
        let response = json!({
            "usage": {
                "prompt_tokens": 42,
                "completion_tokens": 18,
                "total_tokens": 60
            }
        });
        let usage = extract_usage_openai(&response).unwrap();
        assert_eq!(usage, TokenUsage { prompt_tokens: 42, completion_tokens: 18 });
    }

    #[test]
    fn test_extract_usage_openai_missing() {
        assert!(extract_usage_openai(&json!({})).is_none());
    }

    #[test]
    fn test_extract_usage_openai_partial() {
        let response = json!({ "usage": { "prompt_tokens": 10 } });
        let usage = extract_usage_openai(&response).unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 0);
    }

    #[test]
    fn test_extract_usage_gemini() {
        let response = json!({
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 50,
                "totalTokenCount": 150
            }
        });
        let usage = extract_usage_gemini(&response).unwrap();
        assert_eq!(usage, TokenUsage { prompt_tokens: 100, completion_tokens: 50 });
    }

    #[test]
    fn test_extract_usage_gemini_missing() {
        assert!(extract_usage_gemini(&json!({})).is_none());
    }

    #[test]
    fn test_extract_usage_gemini_partial() {
        let response = json!({ "usageMetadata": { "promptTokenCount": 30 } });
        let usage = extract_usage_gemini(&response).unwrap();
        assert_eq!(usage.prompt_tokens, 30);
        assert_eq!(usage.completion_tokens, 0);
    }

    #[test]
    fn test_extract_usage_anthropic() {
        let response = json!({
            "usage": {
                "input_tokens": 200,
                "output_tokens": 80
            }
        });
        let usage = extract_usage_anthropic(&response).unwrap();
        assert_eq!(usage, TokenUsage { prompt_tokens: 200, completion_tokens: 80 });
    }

    #[test]
    fn test_extract_usage_anthropic_missing() {
        assert!(extract_usage_anthropic(&json!({})).is_none());
    }

    #[test]
    fn test_extract_usage_anthropic_partial() {
        let response = json!({ "usage": { "input_tokens": 15 } });
        let usage = extract_usage_anthropic(&response).unwrap();
        assert_eq!(usage.prompt_tokens, 15);
        assert_eq!(usage.completion_tokens, 0);
    }

    // =======================================================================
    // Finish reason extraction
    // =======================================================================

    #[test]
    fn test_finish_reason_openai_stop() {
        let response = json!({ "choices": [{ "finish_reason": "stop" }] });
        assert_eq!(extract_finish_reason_openai(&response), Some(FinishReason::Stop));
    }

    #[test]
    fn test_finish_reason_openai_tool_calls() {
        let response = json!({ "choices": [{ "finish_reason": "tool_calls" }] });
        assert_eq!(extract_finish_reason_openai(&response), Some(FinishReason::ToolUse));
    }

    #[test]
    fn test_finish_reason_openai_length() {
        let response = json!({ "choices": [{ "finish_reason": "length" }] });
        assert_eq!(extract_finish_reason_openai(&response), Some(FinishReason::MaxTokens));
    }

    #[test]
    fn test_finish_reason_openai_other() {
        let response = json!({ "choices": [{ "finish_reason": "content_filter" }] });
        assert_eq!(
            extract_finish_reason_openai(&response),
            Some(FinishReason::Other("content_filter".to_string()))
        );
    }

    #[test]
    fn test_finish_reason_openai_missing() {
        assert_eq!(extract_finish_reason_openai(&json!({})), None);
    }

    #[test]
    fn test_finish_reason_gemini_stop_text() {
        let response = json!({
            "candidates": [{
                "content": { "parts": [{ "text": "hello" }] },
                "finishReason": "STOP"
            }]
        });
        assert_eq!(extract_finish_reason_gemini(&response), Some(FinishReason::Stop));
    }

    #[test]
    fn test_finish_reason_gemini_stop_with_tool_calls() {
        // Gemini uses STOP even for tool calls — should detect ToolUse
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{ "functionCall": { "name": "test", "args": {} } }]
                },
                "finishReason": "STOP"
            }]
        });
        assert_eq!(extract_finish_reason_gemini(&response), Some(FinishReason::ToolUse));
    }

    #[test]
    fn test_finish_reason_gemini_max_tokens() {
        let response = json!({
            "candidates": [{
                "content": { "parts": [{ "text": "partial..." }] },
                "finishReason": "MAX_TOKENS"
            }]
        });
        assert_eq!(extract_finish_reason_gemini(&response), Some(FinishReason::MaxTokens));
    }

    #[test]
    fn test_finish_reason_gemini_other() {
        let response = json!({
            "candidates": [{
                "content": { "parts": [] },
                "finishReason": "SAFETY"
            }]
        });
        assert_eq!(
            extract_finish_reason_gemini(&response),
            Some(FinishReason::Other("SAFETY".to_string()))
        );
    }

    #[test]
    fn test_finish_reason_gemini_missing() {
        assert_eq!(extract_finish_reason_gemini(&json!({})), None);
    }

    #[test]
    fn test_finish_reason_anthropic_end_turn() {
        let response = json!({ "stop_reason": "end_turn" });
        assert_eq!(extract_finish_reason_anthropic(&response), Some(FinishReason::Stop));
    }

    #[test]
    fn test_finish_reason_anthropic_tool_use() {
        let response = json!({ "stop_reason": "tool_use" });
        assert_eq!(extract_finish_reason_anthropic(&response), Some(FinishReason::ToolUse));
    }

    #[test]
    fn test_finish_reason_anthropic_max_tokens() {
        let response = json!({ "stop_reason": "max_tokens" });
        assert_eq!(extract_finish_reason_anthropic(&response), Some(FinishReason::MaxTokens));
    }

    #[test]
    fn test_finish_reason_anthropic_other() {
        let response = json!({ "stop_reason": "stop_sequence" });
        assert_eq!(
            extract_finish_reason_anthropic(&response),
            Some(FinishReason::Other("stop_sequence".to_string()))
        );
    }

    #[test]
    fn test_finish_reason_anthropic_missing() {
        assert_eq!(extract_finish_reason_anthropic(&json!({})), None);
    }
}
