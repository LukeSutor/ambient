//! Anthropic Messages API provider.
//!
//! Connects directly to Anthropic's `/v1/messages` endpoint.
//! Uses `format_messages_for_anthropic` / `tools_to_anthropic_format` translation.
//!
//! Key differences from OpenAI/Gemini:
//! - System prompt passed as top-level `system` field (not in messages)
//! - `max_tokens` is required
//! - Tool calls are `tool_use` content blocks in the response
//! - Streaming uses typed SSE events (`message_start`, `content_block_start`,
//!   `content_block_delta`, `content_block_stop`, `message_delta`, `message_stop`)
//!
//! Auth: Reads `ANTHROPIC_API_KEY` environment variable at runtime.

use crate::db::token_usage::add_token_usage;
use crate::events::{emitter::emit, types::*};
use crate::models::llm::client::ResolvedModel;
use crate::models::llm::providers::translation::{
    extract_text_anthropic, format_messages_for_anthropic, has_tool_calls_anthropic,
    parse_anthropic_tool_calls, resolve_tool_call, tools_to_anthropic_format,
};
use crate::models::llm::types::{LlmProvider, LlmRequest, LlmResponse};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tauri::AppHandle;
use tokio_stream::StreamExt;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u64 = 8192;

pub struct AnthropicProvider;

impl AnthropicProvider {
    fn get_api_key() -> Result<String, String> {
        std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY environment variable not set".to_string())
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicProvider {
    async fn generate(
        &self,
        app_handle: AppHandle,
        request: LlmRequest,
        resolved_model: &ResolvedModel,
    ) -> Result<LlmResponse, String> {
        log::info!("[anthropic] Starting message generation");
        let api_key = Self::get_api_key()?;

        let should_stream = request.stream.unwrap_or(false);
        let model_key = &resolved_model.model_key;

        // Build messages — system messages are excluded by format_messages_for_anthropic
        let messages = if let Some(msgs) = request.messages.clone() {
            format_messages_for_anthropic(&app_handle, &msgs)
        } else {
            vec![json!({
                "role": "user",
                "content": [{ "type": "text", "text": request.prompt }]
            })]
        };

        // Build request body
        let mut body = json!({
            "model": model_key,
            "messages": messages,
            "max_tokens": DEFAULT_MAX_TOKENS,
            "stream": should_stream,
        });

        // System prompt — top-level field for Anthropic
        if let Some(system_prompt) = &request.system_prompt {
            body["system"] = json!(system_prompt);
        }

        // Tools
        if let Some(internal_tools) = &request.internal_tools {
            body["tools"] = json!(tools_to_anthropic_format(internal_tools));
        }

        let client = reqwest::Client::new();
        let mut prompt_tokens = 0u64;
        let mut completion_tokens = 0u64;

        if should_stream {
            let resp = client
                .post(ANTHROPIC_API_URL)
                .header("Content-Type", "application/json")
                .header("x-api-key", &api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Failed to send streaming request: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                log::error!("[anthropic] Streaming error {}: {}", status, text);
                if status.as_u16() == 429 {
                    return Err("rate_limit_exceeded".to_string());
                }
                return Err(format!("Anthropic error {}: {}", status, text));
            }

            let mut full_text = String::new();
            // Track content blocks by index: (type, id, name, accumulated_json)
            let mut content_blocks: std::collections::HashMap<
                usize,
                (String, String, String, String),
            > = std::collections::HashMap::new();
            let mut buffer = String::new();
            let mut stream = resp.bytes_stream();
            let mut was_cancelled = false;

            while let Some(chunk) = stream.next().await {
                // Check cancellation
                if let Some(ref cancel_signal) = request.cancel_signal {
                    if cancel_signal.load(Ordering::SeqCst) {
                        log::info!("[anthropic] Generation cancelled by user");
                        was_cancelled = true;
                        break;
                    }
                }

                let Ok(chunk) = chunk.map_err(|e| format!("Stream error: {}", e)) else {
                    log::warn!("[anthropic] Stream chunk error");
                    break;
                };

                let chunk_text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&chunk_text);

                // Parse SSE events — Anthropic uses `event:` + `data:` pairs
                while let Some(newline_idx) = buffer.find('\n') {
                    let line = buffer[..newline_idx].trim().to_string();
                    buffer.drain(..=newline_idx);

                    if line.is_empty() {
                        continue;
                    }

                    // We only care about data lines — the event type is embedded
                    // in the JSON payload as `"type"`.
                    if !line.starts_with("data: ") {
                        continue;
                    }
                    let data = &line[6..];

                    if let Ok(obj) = serde_json::from_str::<Value>(data) {
                        let event_type = obj
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("");

                        match event_type {
                            "message_start" => {
                                // Extract input token count from initial message
                                if let Some(usage) = obj
                                    .get("message")
                                    .and_then(|m| m.get("usage"))
                                {
                                    prompt_tokens = usage
                                        .get("input_tokens")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0);
                                }
                            }

                            "content_block_start" => {
                                let index =
                                    obj.get("index").and_then(|i| i.as_u64()).unwrap_or(0)
                                        as usize;

                                if let Some(block) = obj.get("content_block") {
                                    let block_type = block
                                        .get("type")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("text")
                                        .to_string();

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

                                    content_blocks.insert(
                                        index,
                                        (block_type, id, name, String::new()),
                                    );
                                }
                            }

                            "content_block_delta" => {
                                let index =
                                    obj.get("index").and_then(|i| i.as_u64()).unwrap_or(0)
                                        as usize;

                                if let Some(delta) = obj.get("delta") {
                                    let delta_type = delta
                                        .get("type")
                                        .and_then(|t| t.as_str())
                                        .unwrap_or("");

                                    match delta_type {
                                        "text_delta" => {
                                            if let Some(text) =
                                                delta.get("text").and_then(|t| t.as_str())
                                            {
                                                full_text.push_str(text);

                                                let _ = emit(
                                                    CHAT_STREAM,
                                                    ChatStreamEvent {
                                                        delta: text.to_string(),
                                                        is_finished: false,
                                                        full_response: full_text.clone(),
                                                        conv_id: request.conv_id.clone(),
                                                        message_id: request
                                                            .assistant_message_id
                                                            .clone(),
                                                    },
                                                );
                                            }
                                        }

                                        "input_json_delta" => {
                                            // Accumulate JSON fragments for tool_use blocks
                                            if let Some(partial) = delta
                                                .get("partial_json")
                                                .and_then(|p| p.as_str())
                                            {
                                                if let Some(entry) =
                                                    content_blocks.get_mut(&index)
                                                {
                                                    entry.3.push_str(partial);
                                                }
                                            }
                                        }

                                        _ => {}
                                    }
                                }
                            }

                            "message_delta" => {
                                // Extract output token count
                                if let Some(usage) = obj.get("usage") {
                                    completion_tokens = usage
                                        .get("output_tokens")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(completion_tokens);
                                }
                            }

                            _ => {} // content_block_stop, message_stop, ping
                        }
                    }
                }
            }

            // Handle cancellation
            if was_cancelled {
                let final_text = if full_text.is_empty() {
                    "*Request cancelled by you*".to_string()
                } else {
                    format!("{}\n\n*Request cancelled by you*", full_text)
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
                model_key,
                prompt_tokens,
                completion_tokens,
            )
            .await?;

            // Collect tool calls from accumulated content blocks
            let mut tool_calls = Vec::new();
            let mut sorted_indices: Vec<_> = content_blocks.keys().collect();
            sorted_indices.sort();

            for idx in sorted_indices {
                let (block_type, id, name, accumulated_json) = &content_blocks[idx];
                if block_type == "tool_use" {
                    let arguments: Value = serde_json::from_str(accumulated_json)
                        .unwrap_or(json!({}));

                    let (skill_name, tool_name) =
                        resolve_tool_call(name, request.internal_tools.as_deref());

                    tool_calls.push(crate::skills::types::ToolCall {
                        id: id.clone(),
                        skill_name,
                        tool_name,
                        arguments,
                        thought_signature: None,
                    });
                }
            }

            if !tool_calls.is_empty() {
                let text = if full_text.is_empty() {
                    None
                } else {
                    Some(full_text)
                };
                Ok(LlmResponse::tool_calls(tool_calls, text))
            } else {
                let _ = emit(
                    CHAT_STREAM,
                    ChatStreamEvent {
                        delta: "".to_string(),
                        is_finished: true,
                        full_response: full_text.clone(),
                        conv_id: request.conv_id.clone(),
                        message_id: request.assistant_message_id.clone(),
                    },
                );
                Ok(LlmResponse::text(full_text))
            }
        } else {
            // ── Non-streaming ──────────────────────────────────────────────
            let resp = client
                .post(ANTHROPIC_API_URL)
                .header("Content-Type", "application/json")
                .header("x-api-key", &api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Failed to send request: {}", e))?;

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                log::error!("[anthropic] Error {}: {}", status, text);
                if status.as_u16() == 429 {
                    return Err("rate_limit_exceeded".to_string());
                }
                return Err(format!("Anthropic error {}: {}", status, text));
            }

            let json_resp: Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Token counts
            if let Some(usage) = json_resp.get("usage") {
                prompt_tokens = usage
                    .get("input_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                completion_tokens = usage
                    .get("output_tokens")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }

            // Save token usage
            add_token_usage(
                app_handle.clone(),
                model_key,
                prompt_tokens,
                completion_tokens,
            )
            .await?;

            let content = extract_text_anthropic(&json_resp);

            if has_tool_calls_anthropic(&json_resp) {
                let tool_calls =
                    parse_anthropic_tool_calls(&json_resp, request.internal_tools.as_deref());
                Ok(LlmResponse::tool_calls(tool_calls, content))
            } else {
                let text = content.unwrap_or_default();
                Ok(LlmResponse::text(text))
            }
        }
    }
}
