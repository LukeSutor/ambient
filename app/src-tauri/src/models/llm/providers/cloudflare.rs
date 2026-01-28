use crate::models::llm::types::{LlmRequest, LlmProvider, LlmResponse};
use crate::events::{emitter::emit, types::*};
use crate::auth::commands::get_access_token_command;
use crate::db::token_usage::add_token_usage;
use crate::constants::CLOUDFLARE_COMPLETIONS_WORKER_URL;
use crate::models::llm::providers::translation::{tools_to_gemini_format, has_tool_calls_gemini, parse_gemini_tool_calls, extract_text_gemini};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use base64::{Engine as _, engine::general_purpose};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tokio_stream::StreamExt;
use std::fs;


const MAX_RECENT_ATTACHMENTS: usize = 3;

/// Map roles to Gemini format
fn role_to_gemini(role: &str) -> &str {
  match role {
    "system" => "model",
    "assistant" => "model",
    "user" => "user",
    _ => "user",
  }
}

/// Builds messages in the Gemini API format
async fn build_content(
  app_handle: &AppHandle,
  user_prompt: String,
  conv_id: &Option<String>,
  current_message_id: &Option<String>,
) -> Result<Vec<Value>, String> {
  let mut content = Vec::new();

  if let Some(conversation_id) = conv_id {
    if let Ok(conv_messages) =
      crate::db::conversations::get_messages(app_handle.clone(), conversation_id.clone()).await
    {
      // Collect IDs of the most recent images/pdfs across all messages
      let mut valid_attachments = Vec::new();
      for msg in conv_messages.iter().rev() {
        for attachment in msg.attachments.iter().rev() {
          if valid_attachments.len() < MAX_RECENT_ATTACHMENTS {
            valid_attachments.push(attachment.id.clone());
          }
        }
      }

      for msg in conv_messages {
        let is_current = current_message_id.as_ref().map_or(false, |id| id == &msg.id);
        let msg_content = if is_current {
          &user_prompt
        } else {
          &msg.content
        };

        let mut content_parts = Vec::new();

        for attachment in msg.attachments {
          if !valid_attachments.contains(&attachment.id) {
            continue;
          }

          if attachment.file_type.starts_with("image/") || attachment.file_type == "application/pdf" {
            // Attach image as base64 data URL
            if let Some(rel_path) = attachment.file_path {
              let full_path = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Could not resolve app data directory: {}", e))?
                .join(rel_path);

              if full_path.exists() {
                if let Ok(bytes) = fs::read(&full_path) {
                  let base64_data = general_purpose::STANDARD.encode(bytes);
                  content_parts.push(json!({
                    "inlineData": {
                      "mimeType": attachment.file_type,
                      "data": base64_data,
                    },
                  }));
                }
              }
            }
          } else if attachment.file_type == "ambient/ocr" {
            // Attach OCR text
            if let Some(extracted_text) = attachment.extracted_text {
              content_parts.push(json!({
                "text": format!("Extracted text from user's screen:\n{}", extracted_text)
              }));
            }
          }
        }

        // Add text content last
        content_parts.push(json!({"text": msg_content}));

        content.push(json!({
          "role": role_to_gemini(msg.role.as_str()),
          "parts": content_parts
        }));
      }
    }
  }
  Ok(content)
}

pub struct CloudflareProvider;

#[async_trait::async_trait]
impl LlmProvider for CloudflareProvider {
  async fn generate(
    &self,
    app_handle: AppHandle,
    mut request: LlmRequest,
  ) -> Result<LlmResponse, String> {
    // Load user settings to get model selection
    let settings = crate::settings::service::load_user_settings(app_handle.clone())
      .await
      .map_err(|e| format!("Failed to load user settings: {}", e))?;
    let model = &settings.model_selection.as_str();

    // Handle internal tools translation
    if let Some(internal_tools) = &request.internal_tools {
      request.tools = Some(tools_to_gemini_format(internal_tools));
    }

    let should_stream = request.stream.unwrap_or(false);
    
    // Build content
    let content = if let Some(msgs) = request.messages.clone() {
      let mut formatted_content = Vec::new();
      for msg in msgs {
        formatted_content.push(json!({
          "role": role_to_gemini(msg.role.as_str()),
          "parts": [{"text": msg.content}]
        }));
      }
      formatted_content
    } else {
      let mut content = build_content(
        &app_handle,
        request.prompt.clone(),
        &request.conv_id,
        &request.current_message_id
      ).await?;

      // Add user prompt if no current message id is provided
      if request.current_message_id.is_none() {
        content.push(json!({
          "role": "user",
          "parts": [{"text": request.prompt.clone()}]
        }));
      }
      content
    };

    // Get user access token
    let access_token = get_access_token_command()
      .await?
      .ok_or_else(|| "No access token found. Please sign in.".to_string())?;

    // Build request body
    let mut body = json!({
        "modelType": model,
        "content": content,
        "stream": should_stream,
        "token": access_token,
        "systemPrompt": request.system_prompt.unwrap_or_else(|| "You are a helpful assistant.".to_string()),
    });

    if let Some(schema_str) = request.json_schema {
      if let Ok(schema_value) = serde_json::from_str::<Value>(&schema_str) {
        body["jsonSchema"] = schema_value;
      } else {
        // Fallback to json_object
        body["jsonSchema"] = json!({ "type": "json_object" });
      }
    }

    // Add tools if provided
    if let Some(tools) = request.tools {
      body["tools"] = tools;
    }

    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let mut prompt_tokens = 0u64;
    let mut completion_tokens = 0u64;

    if should_stream {
      let resp = client
        .post(CLOUDFLARE_COMPLETIONS_WORKER_URL)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send streaming request: {}", e))?;

      if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        log::error!("Cloudflare streaming error status: {}. Body: {}", status, text);
        return Err(format!("Cloudflare error {}: {}", status, text));
      }

      let mut full = String::new();
      let mut buffer = String::new();
      let mut stream = resp.bytes_stream();
      while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk.map_err(|e| format!("Error reading stream: {}", e)) else {
          log::warn!("Stream chunk error encountered");
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
            log::debug!("Ignoring non-data line: {}", line);
            continue;
          }
          let data = &line[6..];
          if data == "[DONE]" {
            continue;
          }
          if let Ok(obj) = serde_json::from_str::<Value>(data) {
            // Update token counts if present in usageMetadata
            if let Some(usage) = obj.get("usageMetadata") {
              if let Some(p) = usage.get("promptTokenCount").and_then(|v| v.as_u64()) {
                prompt_tokens = p;
              }
              if let Some(c) = usage.get("candidatesTokenCount").and_then(|v| v.as_u64()) {
                completion_tokens = c;
              }
            }

            // Extract content piece from Gemini structure
            if let Some(piece) = obj
              .get("candidates")
              .and_then(|c| c.as_array())
              .and_then(|a| a.get(0))
              .and_then(|c| c.get("content"))
              .and_then(|c| c.get("parts"))
              .and_then(|p| p.as_array())
              .and_then(|a| a.get(0))
              .and_then(|p| p.get("text"))
              .and_then(|t| t.as_str())
            {
              full.push_str(piece);
              let _ = emit(
                CHAT_STREAM,
                ChatStreamEvent {
                  delta: piece.to_string(),
                  is_finished: false,
                  full_response: full.clone(),
                  conv_id: request.conv_id.clone(),
                },
              );
            }
          } else {
            log::warn!("Failed to parse line as JSON: {}", data);
          }
        }
      }

      // Final event
      let _ = emit(
        CHAT_STREAM,
        ChatStreamEvent {
          delta: "".to_string(),
          is_finished: true,
          full_response: full.clone(),
          conv_id: request.conv_id.clone(),
        },
      );

      // Save token usage
      add_token_usage(
          app_handle.clone(),
          model,
          prompt_tokens,
          completion_tokens,
      ).await?;

      Ok(LlmResponse::Text(full))
    } else {
      let resp = client
        .post(CLOUDFLARE_COMPLETIONS_WORKER_URL)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

      let status = resp.status();
      if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        log::error!("Cloudflare error status: {}. Body: {}", status, text);
        return Err(format!("Cloudflare error {}: {}", status, text));
      }

      let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

      // Extract token counts
      if let Some(usage) = json.get("usageMetadata") {
        prompt_tokens = usage
          .get("promptTokenCount")
          .and_then(|v| v.as_u64())
          .unwrap_or(0);
        completion_tokens = usage
          .get("candidatesTokenCount")
          .and_then(|v| v.as_u64())
          .unwrap_or(0);
      }

      // Check for tool calls or extract text
      let response = if has_tool_calls_gemini(&json) {
        LlmResponse::ToolCalls(parse_gemini_tool_calls(&json))
      } else {
        let content = extract_text_gemini(&json)
          .unwrap_or_else(|| {
            log::warn!("Failed to extract content from Gemini structure, falling back to as_str()");
            json.as_str().unwrap_or("").to_string()
          });
        LlmResponse::Text(content)
      };

      // Save token usage
      add_token_usage(
          app_handle.clone(),
          model,
          prompt_tokens,
          completion_tokens,
      ).await?;

      Ok(response)
    }
  }
}

