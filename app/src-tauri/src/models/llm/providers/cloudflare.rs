use crate::models::llm::types::{LlmRequest, LlmProvider};
use crate::events::{emitter::emit, types::*};
use crate::constants::CLOUDFLARE_COMPLETIONS_WORKER_URL;
use crate::auth::commands::get_access_token_command;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio_stream::StreamExt;

// Map roles to Gemini format
fn role_to_gemini(role: &str) -> &str {
  match role {
    "system" => "model",
    "assistant" => "model",
    "user" => "user",
    _ => "user",
  }
}

// Builds messages in the Gemini API format
async fn build_content(
  app_handle: &AppHandle,
  user_prompt: String,
  conv_id: &Option<String>,
  current_message_id: &Option<String>,
) -> Vec<Value> {
  let mut content = Vec::new();

  if let Some(conversation_id) = conv_id {
    if let Ok(mut conv_messages) =
      crate::db::conversations::get_messages(app_handle.clone(), conversation_id.clone()).await
    {
      // Filter out the message we are currently augmenting to avoid doubling
      if let Some(mid) = current_message_id {
        conv_messages.retain(|m| &m.id != mid);
      }

      for msg in conv_messages {
        content.push(json!({"role": role_to_gemini(msg.role.as_str()), "parts": [{"text": msg.content}]}));
      }
    }
  }

  // Add user prompt
  content.push(json!({"role": "user", "parts": [{"text": user_prompt}]}));
  content
}

pub struct CloudflareProvider;

#[async_trait::async_trait]
impl LlmProvider for CloudflareProvider {
  async fn generate(
    &self,
    app_handle: AppHandle,
    request: LlmRequest,
  ) -> Result<String, String> {
    // Load user settings to get model selection
    let settings = crate::settings::service::load_user_settings(app_handle.clone())
      .await
      .map_err(|e| format!("Failed to load user settings: {}", e))?;
    let model = &settings.model_selection.as_str();

    let should_stream = request.stream.unwrap_or(false);
    let content = build_content(&app_handle, request.prompt.clone(), &request.conv_id, &request.current_message_id).await;

    // Log all content for debugging
    log::debug!(
      "Cloudflare content: {}",
      serde_json::to_string_pretty(&content).unwrap_or_default()
    );

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
        body["response_format"] = json!({ "type": "json_object" });
      }
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

      log::info!(
        "Cloudflare streaming usage - Prompt: {}, Completion: {}",
        prompt_tokens,
        completion_tokens
      );

      Ok(full)
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

      // Try extraction from full Gemini structure, or fallback to direct string if worker returned response.text
      let content = json
        .get("candidates")
        .and_then(|c| c.as_array())
        .and_then(|a| a.get(0))
        .and_then(|c| c.get("content"))
        .and_then(|c| c.get("parts"))
        .and_then(|p| p.as_array())
        .and_then(|a| a.get(0))
        .and_then(|p| p.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
          log::warn!("Failed to extract content from Gemini structure, falling back to as_str()");
          json.as_str().unwrap_or("").to_string()
        });

      log::info!(
        "Cloudflare usage - Prompt: {}, Completion: {}",
        prompt_tokens,
        completion_tokens
      );

      Ok(content)
    }
  }
}

