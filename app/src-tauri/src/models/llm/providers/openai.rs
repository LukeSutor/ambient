//! OpenAI Chat Completions API provider.
//!
//! Connects directly to OpenAI's `/v1/chat/completions` endpoint.
//! Uses `format_messages_for_openai` / `tools_to_openai_format` translation.
//!
//! Auth: Reads `OPENAI_API_KEY` environment variable at runtime.

use crate::db::token_usage::add_token_usage;
use crate::events::{emitter::emit, types::*};
use crate::models::llm::client::ResolvedModel;
use crate::models::llm::providers::translation::{
    extract_text_openai, format_messages_for_openai, has_tool_calls_openai,
    parse_openai_tool_calls, resolve_tool_call, tools_to_openai_format,
};
use crate::models::llm::types::{LlmProvider, LlmRequest, LlmResponse};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tauri::AppHandle;
use tokio_stream::StreamExt;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

pub struct OpenAIProvider;

impl OpenAIProvider {
    /// Resolve API key: prefer model-level key (BYOK), then env var.
    fn resolve_api_key(resolved_model: &ResolvedModel) -> Result<String, String> {
        if let Some(key) = &resolved_model.api_key {
            if !key.is_empty() {
                return Ok(key.clone());
            }
        }
        std::env::var("OPENAI_API_KEY")
            .map_err(|_| "No API key configured for this model and OPENAI_API_KEY environment variable not set".to_string())
    }

    /// Resolve API URL: prefer model-level URL (BYOK), then default.
    fn resolve_api_url(resolved_model: &ResolvedModel) -> String {
        resolved_model.api_url.as_deref().unwrap_or(OPENAI_API_URL).to_string()
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAIProvider {
    async fn generate(
        &self,
        app_handle: AppHandle,
        request: LlmRequest,
        resolved_model: &ResolvedModel,
    ) -> Result<LlmResponse, String> {
        log::info!("[openai] Starting chat completion generation");
        let api_key = Self::resolve_api_key(resolved_model)?;
        let api_url = Self::resolve_api_url(resolved_model);

        let should_stream = request.stream.unwrap_or(false);

        // Build messages — system prompt is a regular "system" role message in OpenAI
        let system_prompt = request
            .system_prompt
            .clone()
            .unwrap_or_else(|| "You are a helpful assistant".to_string());
        let mut messages = vec![json!({
            "role": "system",
            "content": system_prompt
        })];

        if let Some(msgs) = request.messages.clone() {
            messages.extend(format_messages_for_openai(&app_handle, &msgs));
        } else {
            messages.push(json!({
                "role": "user",
                "content": request.prompt
            }));
        }

        // Build request body
        let mut request_body = json!({
            "model": resolved_model.effective_model_id(),
            "messages": messages,
            "stream": should_stream,
        });

        // Structured output via json_schema response_format
        if let Some(schema) = request.json_schema {
            if let Ok(schema_value) = serde_json::from_str::<Value>(&schema) {
                request_body["response_format"] = json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": "response",
                        "schema": schema_value,
                        "strict": true,
                    }
                });
            }
        }

        // Tools
        if let Some(internal_tools) = &request.internal_tools {
            request_body["tools"] = json!(tools_to_openai_format(internal_tools));
        }

        // Request usage info in streaming mode
        if should_stream {
            request_body["stream_options"] = json!({ "include_usage": true });
        }

        let client = reqwest::Client::new();
        let mut prompt_tokens = 0u64;
        let mut completion_tokens = 0u64;

        if should_stream {
            let response = client
                .post(&api_url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&request_body)
                .send()
                .await
                .map_err(|e| format!("Failed to send streaming request: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                log::error!("[openai] Error status: {}. Body: {}", status, error_text);
                if status.as_u16() == 429 {
                    return Err("rate_limit_exceeded".to_string());
                }
                return Err(format!("OpenAI error {}: {}", status, error_text));
            }

            let mut full_response = String::new();
            let mut tool_calls_map: std::collections::HashMap<usize, (String, String, String)> =
                std::collections::HashMap::new();
            let mut stream = response.bytes_stream();
            let mut was_cancelled = false;
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                // Check cancellation
                if let Some(ref cancel_signal) = request.cancel_signal {
                    if cancel_signal.load(Ordering::SeqCst) {
                        log::info!("[openai] Generation cancelled by user");
                        was_cancelled = true;
                        break;
                    }
                }

                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&chunk_str);

                        // Parse SSE lines from buffer
                        while let Some(newline_idx) = buffer.find('\n') {
                            let line = buffer[..newline_idx].trim().to_string();
                            buffer.drain(..=newline_idx);

                            if line.is_empty() || !line.starts_with("data: ") {
                                continue;
                            }

                            let data = &line[6..];
                            if data == "[DONE]" {
                                continue;
                            }

                            if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                                // Extract usage from the final chunk
                                if let Some(usage) = json_data.get("usage") {
                                    prompt_tokens =
                                        usage["prompt_tokens"].as_u64().unwrap_or(prompt_tokens);
                                    completion_tokens = usage["completion_tokens"]
                                        .as_u64()
                                        .unwrap_or(completion_tokens);
                                }

                                if let Some(choices) = json_data["choices"].as_array() {
                                    if let Some(choice) = choices.first() {
                                        if let Some(delta) = choice.get("delta") {
                                            // Text content
                                            if let Some(content) =
                                                delta.get("content").and_then(|c| c.as_str())
                                            {
                                                full_response.push_str(content);

                                                let stream_data = ChatStreamEvent {
                                                    delta: content.to_string(),
                                                    is_finished: false,
                                                    full_response: full_response.clone(),
                                                    conv_id: request.conv_id.clone(),
                                                    message_id: request
                                                        .assistant_message_id
                                                        .clone(),
                                                };
                                                let _ = emit(CHAT_STREAM, stream_data);
                                            }

                                            // Tool call deltas
                                            if let Some(tool_calls) = delta
                                                .get("tool_calls")
                                                .and_then(|t| t.as_array())
                                            {
                                                for tc in tool_calls {
                                                    let index =
                                                        tc["index"].as_u64().unwrap_or(0) as usize;
                                                    let entry = tool_calls_map
                                                        .entry(index)
                                                        .or_insert_with(|| {
                                                            (
                                                                String::new(),
                                                                String::new(),
                                                                String::new(),
                                                            )
                                                        });

                                                    if let Some(id) =
                                                        tc.get("id").and_then(|v| v.as_str())
                                                    {
                                                        entry.0 = id.to_string();
                                                    }
                                                    if let Some(func) = tc.get("function") {
                                                        if let Some(name) = func
                                                            .get("name")
                                                            .and_then(|v| v.as_str())
                                                        {
                                                            entry.1.push_str(name);
                                                        }
                                                        if let Some(args) = func
                                                            .get("arguments")
                                                            .and_then(|v| v.as_str())
                                                        {
                                                            entry.2.push_str(args);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Err(format!("Error reading stream: {}", e));
                    }
                }
            }

            // Handle cancellation
            if was_cancelled {
                let final_text = if full_response.is_empty() {
                    "*Request cancelled by you*".to_string()
                } else {
                    format!("{}\n\n*Request cancelled by you*", full_response)
                };
                let _ = emit(
                    CHAT_STREAM,
                    ChatStreamEvent {
                        delta: "".to_string(),
                        is_finished: true,
                        full_response: final_text.clone(),
                        conv_id: request.conv_id.clone(),
                        message_id: request.assistant_message_id.clone(),
                    },
                );
                return Ok(LlmResponse::Text(final_text));
            }

            // Save token usage
            add_token_usage(
                app_handle.clone(),
                &resolved_model.model_key,
                prompt_tokens,
                completion_tokens,
            )
            .await?;

            // Return tool calls or text
            if !tool_calls_map.is_empty() {
                let mut tool_calls = Vec::new();
                let mut sorted_indices: Vec<_> = tool_calls_map.keys().collect();
                sorted_indices.sort();

                for idx in sorted_indices {
                    let (id, name, args) = &tool_calls_map[idx];
                    let (skill, tool) = resolve_tool_call(name, request.internal_tools.as_deref());

                    tool_calls.push(crate::skills::types::ToolCall {
                        id: if id.is_empty() {
                            uuid::Uuid::new_v4().to_string()
                        } else {
                            id.clone()
                        },
                        skill_name: skill,
                        tool_name: tool,
                        arguments: serde_json::from_str(args).unwrap_or(json!({})),
                        thought_signature: None,
                    });
                }
                let text = if full_response.is_empty() {
                    None
                } else {
                    Some(full_response)
                };
                Ok(LlmResponse::tool_calls(tool_calls, text))
            } else {
                let _ = emit(
                    CHAT_STREAM,
                    ChatStreamEvent {
                        delta: "".to_string(),
                        is_finished: true,
                        full_response: full_response.clone(),
                        conv_id: request.conv_id.clone(),
                        message_id: request.assistant_message_id.clone(),
                    },
                );
                Ok(LlmResponse::text(full_response))
            }
        } else {
            // ── Non-streaming ──────────────────────────────────────────────
            let response = client
                .post(&api_url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&request_body)
                .send()
                .await
                .map_err(|e| format!("Failed to send request: {}", e))?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                log::error!("[openai] Error status: {}. Body: {}", status, error_text);
                if status.as_u16() == 429 {
                    return Err("rate_limit_exceeded".to_string());
                }
                return Err(format!("OpenAI error {}: {}", status, error_text));
            }

            let result: Value = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Extract token usage
            if let Some(usage) = result.get("usage") {
                prompt_tokens = usage["prompt_tokens"].as_u64().unwrap_or(0);
                completion_tokens = usage["completion_tokens"].as_u64().unwrap_or(0);
            }

            // Save token usage
            add_token_usage(
                app_handle.clone(),
                &resolved_model.model_key,
                prompt_tokens,
                completion_tokens,
            )
            .await?;

            let generated_text = extract_text_openai(&result).unwrap_or_default();

            if has_tool_calls_openai(&result) {
                let tool_calls =
                    parse_openai_tool_calls(&result, request.internal_tools.as_deref());
                let text = if generated_text.is_empty() {
                    None
                } else {
                    Some(generated_text)
                };
                Ok(LlmResponse::tool_calls(tool_calls, text))
            } else {
                Ok(LlmResponse::text(generated_text))
            }
        }
    }
}
