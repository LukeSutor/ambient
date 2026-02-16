//! Google Gemini API provider.
//!
//! Connects directly to Google's Gemini REST API at
//! `generativelanguage.googleapis.com`.
//! Uses `format_messages_for_gemini` / `tools_to_gemini_format` translation.
//!
//! Handles Gemini 3 thought signatures for function calling:
//! - `thoughtSignature` is preserved on ToolCall structs by `parse_gemini_tool_calls`
//! - `format_messages_for_gemini` re-attaches signatures when sending history back
//!
//! Auth: Reads `GOOGLE_API_KEY` environment variable at runtime.

use crate::db::token_usage::add_token_usage;
use crate::events::{emitter::emit, types::*};
use crate::models::llm::client::ResolvedModel;
use crate::models::llm::providers::translation::{
    extract_text_gemini, format_messages_for_gemini, has_tool_calls_gemini,
    parse_gemini_tool_calls, tools_to_gemini_format,
};
use crate::models::llm::types::{LlmProvider, LlmRequest, LlmResponse};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use tauri::AppHandle;
use tokio_stream::StreamExt;

const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

pub struct GoogleProvider;

impl GoogleProvider {
    /// Resolve API key: prefer model-level key (BYOK), then env var.
    fn resolve_api_key(resolved_model: &ResolvedModel) -> Result<String, String> {
        if let Some(key) = &resolved_model.api_key {
            if !key.is_empty() {
                return Ok(key.clone());
            }
        }
        std::env::var("GOOGLE_API_KEY")
            .map_err(|_| "No API key configured for this model and GOOGLE_API_KEY environment variable not set".to_string())
    }

    /// Resolve API base URL: prefer model-level URL (BYOK), then default.
    fn resolve_api_base(resolved_model: &ResolvedModel) -> String {
        resolved_model.api_url.as_deref().unwrap_or(GEMINI_API_BASE).to_string()
    }
}

#[async_trait::async_trait]
impl LlmProvider for GoogleProvider {
    async fn generate(
        &self,
        app_handle: AppHandle,
        request: LlmRequest,
        resolved_model: &ResolvedModel,
    ) -> Result<LlmResponse, String> {
        log::info!("[google] Starting Gemini generation");
        let api_key = Self::resolve_api_key(resolved_model)?;
        let api_base = Self::resolve_api_base(resolved_model);

        let should_stream = request.stream.unwrap_or(false);
        let enable_thinking = request.use_thinking.unwrap_or(false);
        let model_key = &resolved_model.model;

        // Build content messages
        let mut content = Vec::new();
        if let Some(msgs) = request.messages.clone() {
            content.extend(format_messages_for_gemini(&app_handle, &msgs));
        } else {
            content.push(json!({
                "role": "user",
                "parts": [{"text": request.prompt.clone()}]
            }));
        }

        // Build request body
        let mut body = json!({
            "contents": content,
        });

        // System instruction — Gemini uses a separate field, not a role
        if let Some(system_prompt) = &request.system_prompt {
            body["systemInstruction"] = json!({
                "parts": [{ "text": system_prompt }]
            });
        }

        // Tools
        if let Some(internal_tools) = &request.internal_tools {
            body["tools"] = json!([tools_to_gemini_format(internal_tools)]);
        }

        // Generation config
        let mut generation_config = json!({});

        // Structured output
        if let Some(schema_str) = &request.json_schema {
            if let Ok(schema_value) = serde_json::from_str::<Value>(schema_str) {
                generation_config["responseMimeType"] = json!("application/json");
                generation_config["responseSchema"] = schema_value;
            }
        }

        if generation_config.as_object().map_or(false, |o| !o.is_empty()) {
            body["generationConfig"] = generation_config;
        }

        // Thinking configuration
        if enable_thinking {
            body["generationConfig"]["thinkingConfig"] = json!({
                "thinkingLevel": "HIGH"
            });
        }

        let client = reqwest::Client::new();
        let mut prompt_tokens = 0u64;
        let mut completion_tokens = 0u64;

        if should_stream {
            let endpoint = format!(
                "{}/models/{}:streamGenerateContent?key={}&alt=sse",
                api_base, model_key, api_key
            );

            let resp = client
                .post(&endpoint)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Failed to send streaming request: {}", e))?;

            if !resp.status().is_success() {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                log::error!("[google] Streaming error {}: {}", status, text);
                if status.as_u16() == 429 {
                    return Err("rate_limit_exceeded".to_string());
                }
                return Err(format!("Gemini error {}: {}", status, text));
            }

            let mut full = String::new();
            let mut tool_calls = Vec::new();
            let mut buffer = String::new();
            let mut stream = resp.bytes_stream();
            let mut was_cancelled = false;

            while let Some(chunk) = stream.next().await {
                // Check cancellation
                if let Some(ref cancel_signal) = request.cancel_signal {
                    if cancel_signal.load(Ordering::SeqCst) {
                        log::info!("[google] Generation cancelled by user");
                        was_cancelled = true;
                        break;
                    }
                }

                let Ok(chunk) = chunk.map_err(|e| format!("Stream error: {}", e)) else {
                    log::warn!("[google] Stream chunk error");
                    break;
                };
                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                while let Some(newline_idx) = buffer.find('\n') {
                    let line = buffer[..newline_idx].trim().to_string();
                    buffer.drain(..=newline_idx);

                    if line.is_empty() || line.starts_with(": ") {
                        continue;
                    }
                    if !line.starts_with("data: ") {
                        continue;
                    }
                    let data = &line[6..];
                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(obj) = serde_json::from_str::<Value>(data) {
                        // Token counts
                        if let Some(usage) = obj.get("usageMetadata") {
                            if let Some(p) =
                                usage.get("promptTokenCount").and_then(|v| v.as_u64())
                            {
                                prompt_tokens = p;
                            }
                            if let Some(c) =
                                usage.get("candidatesTokenCount").and_then(|v| v.as_u64())
                            {
                                completion_tokens = c;
                            }
                        }

                        // Tool calls
                        if has_tool_calls_gemini(&obj) {
                            let chunk_calls =
                                parse_gemini_tool_calls(&obj, request.internal_tools.as_deref());
                            tool_calls.extend(chunk_calls);
                        }

                        // Text content
                        if let Some(piece) = extract_text_gemini(&obj) {
                            full.push_str(&piece);
                            let _ = emit(
                                CHAT_STREAM,
                                ChatStreamEvent {
                                    delta: piece,
                                    is_finished: false,
                                    full_response: full.clone(),
                                    conv_id: request.conv_id.clone(),
                                    message_id: request.assistant_message_id.clone(),
                                },
                            );
                        }
                    } else {
                        log::warn!("[google] Failed to parse SSE line: {}", data);
                    }
                }
            }

            // Handle cancellation
            if was_cancelled {
                let final_text = if full.is_empty() {
                    "*Request cancelled by you*".to_string()
                } else {
                    format!("{}\n\n*Request cancelled by you*", full)
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
                resolved_model.id,
                prompt_tokens,
                completion_tokens,
            )
            .await?;

            if !tool_calls.is_empty() {
                let text = if full.is_empty() { None } else { Some(full) };
                Ok(LlmResponse::tool_calls(tool_calls, text))
            } else {
                let _ = emit(
                    CHAT_STREAM,
                    ChatStreamEvent {
                        delta: "".to_string(),
                        is_finished: true,
                        full_response: full.clone(),
                        conv_id: request.conv_id.clone(),
                        message_id: request.assistant_message_id.clone(),
                    },
                );
                Ok(LlmResponse::text(full))
            }
        } else {
            // ── Non-streaming ──────────────────────────────────────────────
            let endpoint = format!(
                "{}/models/{}:generateContent?key={}",
                api_base, model_key, api_key
            );

            let resp = client
                .post(&endpoint)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| format!("Failed to send request: {}", e))?;

            let status = resp.status();
            if !status.is_success() {
                let text = resp.text().await.unwrap_or_default();
                log::error!("[google] Error {}: {}", status, text);
                if status.as_u16() == 429 {
                    return Err("rate_limit_exceeded".to_string());
                }
                return Err(format!("Gemini error {}: {}", status, text));
            }

            let json_resp: Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {}", e))?;

            // Token counts
            if let Some(usage) = json_resp.get("usageMetadata") {
                prompt_tokens = usage
                    .get("promptTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                completion_tokens = usage
                    .get("candidatesTokenCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }

            let content = extract_text_gemini(&json_resp);

            let response = if has_tool_calls_gemini(&json_resp) {
                let tool_calls =
                    parse_gemini_tool_calls(&json_resp, request.internal_tools.as_deref());
                LlmResponse::tool_calls(tool_calls, content)
            } else {
                let text = content.unwrap_or_else(|| {
                    log::warn!("[google] Failed to extract content from Gemini response");
                    json_resp.as_str().unwrap_or("").to_string()
                });
                LlmResponse::text(text)
            };

            // Save token usage
            add_token_usage(
                app_handle.clone(),
                resolved_model.id,
                prompt_tokens,
                completion_tokens,
            )
            .await?;

            Ok(response)
        }
    }
}
