use super::LlmProvider;
use crate::events::{emitter::emit, types::*};
use crate::constants::CLOUDFLARE_COMPLETIONS_WORKER_URL;
use crate::auth::commands::get_access_token_command;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio_stream::StreamExt;

fn model_from_selection(selection: crate::settings::types::ModelSelection) -> &'static str {
  match selection {
    crate::settings::types::ModelSelection::GptOss => "openai/gpt-oss-20b",
    crate::settings::types::ModelSelection::Gpt5 => "openai/gpt-5-chat",
    _ => "openai/gpt-oss-20b",
  }
}

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
  prompt: String,
  conv_id: &Option<String>,
) -> Vec<Value> {
  let mut content = Vec::new();

  if let Some(conversation_id) = conv_id {
    if let Ok(conv_messages) =
      crate::db::conversations::get_messages(app_handle.clone(), conversation_id.clone()).await
    {
      for msg in conv_messages {
        content.push(json!({"role": role_to_gemini(msg.role.as_str()), "parts": {"text": msg.content}}));
      }
    }
  }

  content.push(json!({"role":"user","parts":{"text": prompt}}));
  content
}

pub struct CloudflareProvider;

#[async_trait::async_trait]
impl LlmProvider for CloudflareProvider {
  async fn generate(
    &self,
    app_handle: AppHandle,
    prompt: String,
    system_prompt: Option<String>,
    json_schema: Option<String>,
    conv_id: Option<String>,
    _use_thinking: Option<bool>,
    stream: Option<bool>,
  ) -> Result<String, String> {
    // Load user settings to get model selection
    let settings = crate::settings::service::load_user_settings(app_handle.clone())
      .await
      .map_err(|e| format!("Failed to load user settings: {}", e))?;
    let model = model_from_selection(settings.model_selection);

    let should_stream = stream.unwrap_or(false);
    let content = build_content(&app_handle, prompt.clone(), &conv_id).await;

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
        "systemPrompt": system_prompt.unwrap_or_else(|| "You are a helpful assistant.".to_string()),
    });

    if let Some(schema_str) = json_schema {
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

    if should_stream {
      let resp = client
        .post(&CLOUDFLARE_COMPLETIONS_WORKER_URL)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send streaming request: {}", e))?;

      if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Cloudflare error {}: {}", status, text));
      }

      let mut full = String::new();
      let mut stream = resp.bytes_stream();
      while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk.map_err(|e| format!("Error reading stream: {}", e)) else {
          break;
        };
        let text = String::from_utf8_lossy(&chunk);
        for line in text.lines() {
          let line = line.trim();
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
            if let Some(choice) = obj
              .get("choices")
              .and_then(|c| c.as_array())
              .and_then(|a| a.get(0))
            {
              if let Some(delta) = choice.get("delta").and_then(|d| d.as_object()) {
                if let Some(piece) = delta.get("content").and_then(|c| c.as_str()) {
                  full.push_str(piece);
                  let _ = emit(
                    CHAT_STREAM,
                    ChatStreamEvent {
                      delta: piece.to_string(),
                      is_finished: false,
                      full_response: full.clone(),
                      conv_id: conv_id.clone(),
                    },
                  );
                }
              }
            }
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
          conv_id: conv_id.clone(),
        },
      );

      Ok(full)
    } else {
      let resp = client
        .post(&CLOUDFLARE_COMPLETIONS_WORKER_URL)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
      if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("Cloudflare error {}: {}", status, text));
      }
      let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;
      let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

      Ok(content)
    }
  }
}

