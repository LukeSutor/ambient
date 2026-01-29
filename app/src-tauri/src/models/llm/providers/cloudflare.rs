use crate::models::llm::types::{LlmRequest, LlmProvider, LlmResponse};
use crate::events::{emitter::emit, types::*};
use crate::auth::commands::get_access_token_command;
use crate::db::token_usage::add_token_usage;
use crate::constants::CLOUDFLARE_COMPLETIONS_WORKER_URL;
use crate::models::llm::providers::translation::{tools_to_gemini_format, has_tool_calls_gemini, parse_gemini_tool_calls, extract_text_gemini, format_messages_for_gemini};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio_stream::StreamExt;


pub struct CloudflareProvider;

#[async_trait::async_trait]
impl LlmProvider for CloudflareProvider {
  async fn generate(
    &self,
    app_handle: AppHandle,
    request: LlmRequest,
  ) -> Result<LlmResponse, String> {
    // Load user settings to get model selection
    let settings = crate::settings::service::load_user_settings(app_handle.clone())
      .await
      .map_err(|e| format!("Failed to load user settings: {}", e))?;
    let model = &settings.model_selection.as_str();

    let should_stream = request.stream.unwrap_or(false);
    let mut content = Vec::new();

    // Build content
    if let Some(msgs) = request.messages.clone() {
      content.extend(format_messages_for_gemini(&app_handle, &msgs));
    } else {
      content.push(json!({
        "role": "user",
        "parts": [{"text": request.prompt.clone()}]
      }));
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

    // Handle internal tools translation
    if let Some(internal_tools) = &request.internal_tools {
      body["tools"] = tools_to_gemini_format(internal_tools);
    }

    // Pretty print the request body for debugging
    log::debug!("Cloudflare LLM Request Body: {}", serde_json::to_string_pretty(&body).unwrap());

    let client = reqwest::Client::new();

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
      reqwest::header::AUTHORIZATION,
      HeaderValue::from_str(&format!("Bearer {}", access_token))
        .map_err(|e| format!("Invalid access token format: {}", e))?,
    );

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
      let mut tool_calls = Vec::new();
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

            // Extract tool calls from this chunk
            if has_tool_calls_gemini(&obj) {
              let chunk_calls = parse_gemini_tool_calls(&obj, request.internal_tools.as_deref());
              tool_calls.extend(chunk_calls);
            }

            // Extract content piece from Gemini structure
            if let Some(piece) = extract_text_gemini(&obj) {
              full.push_str(&piece);
              let _ = emit(
                CHAT_STREAM,
                ChatStreamEvent {
                  delta: piece.clone(),
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

      if !tool_calls.is_empty() {
        Ok(LlmResponse::ToolCalls(tool_calls))
      } else {
        Ok(LlmResponse::Text(full))
      }
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
        LlmResponse::ToolCalls(parse_gemini_tool_calls(&json, request.internal_tools.as_deref()))
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

