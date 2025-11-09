use super::LlmProvider;
use crate::events::{emitter::emit, types::*};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio_stream::StreamExt;

const OPENROUTER_BASE: &str = "https://openrouter.ai/api/v1";

fn get_openrouter_api_key() -> Result<String, String> {
  // Attempt to load environment from .env once
  let _ = dotenv::dotenv();
  // Prefer environment variable; dotenv can populate it at startup
  if let Ok(key) = std::env::var("OPENROUTER_API_KEY") {
    if !key.is_empty() {
      return Ok(key);
    }
  }
  Err("Missing OPENROUTER_API_KEY".to_string())
}

fn model_from_selection(selection: crate::settings::types::ModelSelection) -> &'static str {
  match selection {
    crate::settings::types::ModelSelection::GptOss => "openai/gpt-oss-20b",
    crate::settings::types::ModelSelection::Gpt5 => "openai/gpt-5-chat",
    _ => "openai/gpt-oss-20b",
  }
}

async fn build_messages(
  app_handle: &AppHandle,
  prompt: String,
  system_prompt: Option<String>,
  conv_id: &Option<String>,
) -> Vec<Value> {
  let mut messages = Vec::new();
  let system_prompt = system_prompt.unwrap_or_else(|| "You are a helpful assistant".to_string());
  messages.push(json!({"role":"system","content":system_prompt}));

  if let Some(conversation_id) = conv_id {
    if let Ok(conv_messages) =
      crate::db::conversations::get_messages(app_handle.clone(), conversation_id.clone()).await
    {
      for msg in conv_messages {
        messages.push(json!({"role": msg.role.as_str(), "content": msg.content}));
      }
    }
  }

  messages.push(json!({"role":"user","content":prompt}));
  messages
}

pub struct OpenRouterProvider;

#[async_trait::async_trait]
impl LlmProvider for OpenRouterProvider {
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
    let api_key = get_openrouter_api_key()?;

    // Load user settings to get model selection
    let settings = crate::settings::service::load_user_settings(app_handle.clone())
      .await
      .map_err(|e| format!("Failed to load user settings: {}", e))?;
    let model = model_from_selection(settings.model_selection);

    let should_stream = stream.unwrap_or(false);
    let messages = build_messages(&app_handle, prompt.clone(), system_prompt, &conv_id).await;

    // Log all messages for debugging
    log::debug!(
      "OpenRouter messages: {}",
      serde_json::to_string_pretty(&messages).unwrap_or_default()
    );

    // Build request body
    let mut body = json!({
        "model": model,
        "messages": messages,
        "stream": should_stream,
    });

    if let Some(schema_str) = json_schema {
      if let Ok(schema_value) = serde_json::from_str::<Value>(&schema_str) {
        // Prefer structured outputs per OpenRouter docs
        body["response_format"] = json!({
            "type": "json_schema",
            "json_schema": {
                "name": "structured_output",
                "strict": true,
                "schema": schema_value,
            }
        });
      } else {
        // Fallback to json_object
        body["response_format"] = json!({ "type": "json_object" });
      }
    }

    // Optional: pass through any custom params; ignore use_thinking (local-only)
    let client = reqwest::Client::new();
    let url = format!("{}/chat/completions", OPENROUTER_BASE);

    let mut headers = HeaderMap::new();
    headers.insert(
      AUTHORIZATION,
      HeaderValue::from_str(&format!("Bearer {}", api_key))
        .map_err(|_| "Invalid API key header".to_string())?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    if should_stream {
      let resp = client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send streaming request: {}", e))?;

      if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenRouter error {}: {}", status, text));
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
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
      if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenRouter error {}: {}", status, text));
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
