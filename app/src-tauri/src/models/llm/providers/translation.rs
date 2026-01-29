//! Tool format translation layer.
//!
//! Converts between the unified internal tool format and
//! provider-specific formats for OpenAI (local) and Gemini (cloud).
//!
//! This module provides bidirectional translation:
//! - **Internal → Provider**: Converts tool definitions to provider format
//! - **Provider → Internal**: Parses tool calls from provider responses

use crate::db::conversations::{Message, MessageType, MessageMetadata, Role};
use crate::skills::types::{ToolDefinition, ToolCall, ToolResult};
use serde_json::{json, Value};

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

            let name = if let Some(skill) = &tool.skill_name {
                format!("{}.{}", skill, tool.name)
            } else {
                tool.name.clone()
            };

            json!({
                "type": "function",
                "function": {
                    "name": name,
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

            let name = if let Some(skill) = &tool.skill_name {
                format!("{}.{}", skill, tool.name)
            } else {
                tool.name.clone()
            };

            json!({
                "name": name,
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

                            // Generate unique ID for this call (Gemini doesn't provide one)
                            let id = uuid::Uuid::new_v4().to_string();

                            let (skill_name, tool_name) = resolve_tool_call(&name, available_tools);

                            calls.push(ToolCall {
                                id,
                                skill_name,
                                tool_name,
                                arguments,
                            });
                        }
                    }
                }
            }
        }
    }

    calls
}

/// Formats tool results for OpenAI format.
///
/// Creates the proper structure for sending tool results back to OpenAI
/// format (role: "tool", tool_call_id, content).
pub fn format_openai_tool_results(results: &[ToolResult]) -> Vec<Value> {
    results
        .iter()
        .map(|result| {
            let content = if result.success {
                result.result
                    .as_ref()
                    .map(|r| serde_json::to_string(r).unwrap_or_default())
                    .unwrap_or_else(|| "Success".to_string())
            } else {
                format!("Error: {}", result.error.as_deref().unwrap_or("Unknown error"))
            };

            json!({
                "role": "tool",
                "tool_call_id": result.call_id,
                "content": content,
            })
        })
        .collect()
}

/// Formats tool results for Gemini format.
///
/// Creates the proper structure for sending tool results back to Gemini
/// format (role: "user", parts with functionResponse).
pub fn format_gemini_tool_results(results: &[ToolResult], tool_calls: &[ToolCall]) -> Value {
    let parts: Vec<Value> = results
        .iter()
        .zip(tool_calls.iter())
        .map(|(result, call)| {
            let response_value = if result.success {
                result.result.clone().unwrap_or(json!({"status": "success"}))
            } else {
                json!({"error": result.error.as_deref().unwrap_or("Unknown error")})
            };

            json!({
                "functionResponse": {
                    "name": call.tool_name.clone(),
                    "response": response_value
                }
            })
        })
        .collect();

    json!({
        "role": "user",
        "parts": parts
    })
}

/// Format conversation messages for OpenAI-compatible API according to the spec.
///
/// This properly formats:
/// - Assistant messages with tool calls (using `tool_calls` array)
/// - Tool result messages (using `tool_call_id`)
/// - Regular text messages
/// - Skips "Thinking" messages as they are internal state
pub fn format_messages_for_openai(msgs: &[Message]) -> Vec<Value> {
    let mut formatted = Vec::new();

    // Track pending tool calls to merge consecutive tool call messages
    let mut pending_tool_calls: Vec<Value> = Vec::new();

    for msg in msgs {
        match msg.message_type {
            // Skip "Thinking" messages - these are internal state and confuse the model
            MessageType::Thinking => {
                continue;
            }

            MessageType::ToolCall => {
                // Extract tool call from metadata and add to pending list
                if let Some(MessageMetadata::ToolCall { call_id, skill_name, tool_name, arguments }) = &msg.metadata {
                    let tool_name_full = format!("{}.{}", skill_name, tool_name);
                    pending_tool_calls.push(json!({
                        "id": call_id,
                        "type": "function",
                        "function": {
                            "name": tool_name_full,
                            "arguments": serde_json::to_string(arguments).unwrap_or_else(|_| "{}".to_string())
                        }
                    }));
                }
            }

            MessageType::ToolResult => {
                // First, flush any pending tool calls as a single assistant message
                if !pending_tool_calls.is_empty() {
                    formatted.push(json!({
                        "role": "assistant",
                        "content": null,
                        "tool_calls": pending_tool_calls.clone()
                    }));
                    pending_tool_calls.clear();
                }

                // Format tool result with tool_call_id
                if let Some(MessageMetadata::ToolResult { call_id, result, success, error }) = &msg.metadata {
                    let content = if *success {
                        result
                            .as_ref()
                            .map(|r| serde_json::to_string(r).unwrap_or_else(|_| "{}".to_string()))
                            .unwrap_or_else(|| "Success".to_string())
                    } else {
                        format!("Error: {}", error.as_deref().unwrap_or("Unknown error"))
                    };

                    formatted.push(json!({
                        "role": "tool",
                        "tool_call_id": call_id,
                        "content": content
                    }));
                }
            }

            MessageType::Text => {
                // Flush any pending tool calls first
                if !pending_tool_calls.is_empty() {
                    formatted.push(json!({
                        "role": "assistant",
                        "content": null,
                        "tool_calls": pending_tool_calls.clone()
                    }));
                    pending_tool_calls.clear();
                }

                // Regular text message
                let role = match msg.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                };

                formatted.push(json!({
                    "role": role,
                    "content": msg.content
                }));
            }
        }
    }

    // Flush any remaining pending tool calls
    if !pending_tool_calls.is_empty() {
        formatted.push(json!({
            "role": "assistant",
            "content": null,
            "tool_calls": pending_tool_calls
        }));
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
            for part in parts {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    return Some(text.to_string());
                }
            }
            None
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
        assert_eq!(result[0]["function"]["name"], "test.test_tool");
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
        assert_eq!(result["functionDeclarations"][0]["name"], "test.test_tool");
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
