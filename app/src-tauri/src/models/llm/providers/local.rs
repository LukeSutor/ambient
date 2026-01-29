use crate::models::llm::types::{LlmRequest, LlmProvider, LlmResponse};
use crate::db::token_usage::add_token_usage;
use crate::models::llm::providers::translation::{tools_to_openai_format, has_tool_calls_openai, parse_openai_tool_calls, resolve_tool_call, format_messages_for_openai};
use crate::models::llm::server::{perform_health_check, get_current_server_config};
use crate::events::{emitter::emit, types::*};
use serde_json::{json, Value};
use base64::{Engine as _, engine::general_purpose};
use tauri::{AppHandle, Manager};
use std::fs;

pub struct LocalProvider;

const MAX_RECENT_ATTACHMENTS: usize = 3;

/// Build messages according to OpenAI conversations format
async fn build_messages(
  app_handle: &AppHandle,
  system_prompt: String,
  user_prompt: String,
  conv_id: &Option<String>,
  current_message_id: &Option<String>,
) -> Result<Vec<Value>, String> {
  let mut messages = Vec::new();

  messages.push(json!({
    "role": "system",
    "content": system_prompt
  }));

  if let Some(conversation_id) = conv_id {
    if let Ok(conv_messages) =
      crate::db::conversations::get_messages(app_handle.clone(), conversation_id.clone()).await
    {
      // Collect IDs of most recent images/pdfs across all messages
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
        let content = if is_current {
          &user_prompt
        } else {
          &msg.content
        };

        let mut content_blocks = Vec::new();

        for attachment in msg.attachments {
          if !valid_attachments.contains(&attachment.id) {
            continue;
          }

          if attachment.file_type.starts_with("image/") {
            // Attach image as base64 data URL
            if let Some(rel_path) = attachment.file_path {
              let full_path = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Could not resolve app data directory: {}", e))?
                .join(rel_path);

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
          } else if attachment.file_type == "application/pdf" {
            // Extract text from PDF and attach to prompt
            if let Some(rel_path) = attachment.file_path {
              let full_path = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("Could not resolve app data directory: {}", e))?
                .join(rel_path);

              if full_path.exists() {
                if let Ok(bytes) = fs::read(&full_path) {
                  let Ok(pdf_text) = pdf_extract::extract_text_from_mem(&bytes) else {
                    continue;
                  };
                  content_blocks.push(json!({
                    "type": "text",
                    "text": format!("Extracted text from {}:\n{}", attachment.file_name, pdf_text)
                  }));
                }
              }
            }
          } else if attachment.file_type == "ambient/ocr" {
            // Attach OCR text
            if let Some(extracted_text) = attachment.extracted_text {
              content_blocks.push(json!({
                "type": "text",
                "text": format!("Extracted text from user's screen:\n{}", extracted_text)
              }));
            }
          }
        }

        // Add text content last
        content_blocks.push(json!({"type": "text", "text": content}));

        messages.push(json!({
          "role": msg.role.as_str(),
          "content": content_blocks
        }));
      }
    }
  }

  Ok(messages)
}

#[async_trait::async_trait]
impl LlmProvider for LocalProvider {
  async fn generate(
    &self,
    app_handle: AppHandle,
    mut request: LlmRequest,
  ) -> Result<LlmResponse, String> {
    log::info!("[llama_server] Starting chat completion generation");
    let config = get_current_server_config(&app_handle).map_err(|e| e.to_string())?;

    // Check if server is healthy first
    if let Err(e) = perform_health_check(&config).await {
      return Err(format!("Server health check failed: {}", e));
    }

    // Handle internal tools translation
    if let Some(internal_tools) = &request.internal_tools {
      request.tools = Some(json!(tools_to_openai_format(internal_tools)));
    }

    let should_stream = request.stream.unwrap_or(false);
    let enable_thinking = request.use_thinking.unwrap_or(false);

    // Build messages
    let messages = if let Some(msgs) = request.messages.clone() {
      let mut formatted_msgs = Vec::new();
      
      // Add system prompt if provided
      if let Some(system_prompt) = &request.system_prompt {
        formatted_msgs.push(json!({
          "role": "system",
          "content": system_prompt
        }));
      }

      // Format messages according to OpenAI spec
      formatted_msgs.extend(format_messages_for_openai(&msgs));
      formatted_msgs
    } else {
      let system_prompt = request.system_prompt.clone().unwrap_or("You are a helpful assistant".to_string());
      let mut msgs = build_messages(
        &app_handle,
        system_prompt,
        request.prompt.clone(),
        &request.conv_id,
        &request.current_message_id
      ).await?;

      // Add user prompt if no current message id is provided and no history used
      if request.current_message_id.is_none() {
        msgs.push(json!({
          "role": "user",
          "content": request.prompt
        }));
      }
      msgs
    };

    // Build request body
    let mut request_body = json!({
        "model": "local",
        "messages": messages,
        "stream": should_stream,
        "temperature": 0.7,
        "top_p": 0.8,
        "top_k": 20,
        "seed": 3407,
        "repeat_penalty": 1.0,
        "presence_penalty": 1.5,
        "max_tokens": 32768
    });

    // Add JSON schema if provided
    if let Some(schema) = request.json_schema {
      if let Ok(schema_value) = serde_json::from_str::<Value>(&schema) {
        request_body["response_format"] = json!({
            "type": "json_object",
            "schema": schema_value
        });
      } else {
        return Err("Invalid JSON schema provided".to_string());
      }
    }

    // Add thinking parameter
    request_body["chat_template_kwargs"] = json!({
        "enable_thinking": enable_thinking
    });

    // Add tools if provided
    if let Some(tools) = request.tools {
      request_body["tools"] = tools;
    }

    let client = reqwest::Client::new();
    let completion_url = format!("{}/v1/chat/completions", config.base_url());

    let mut prompt_tokens = 0u64;
    let mut completion_tokens = 0u64;

    // Pretty print the raw json request body
    log::debug!("[llama_server] Completion request body: {}", serde_json::to_string_pretty(&request_body).unwrap_or_else(|_| "Failed to pretty print".to_string()));

    if should_stream {
      // Handle streaming response
      let response = client
        .post(&completion_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", config.api_key))
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
        return Err(format!("Server returned error {}: {}", status, error_text));
      }

      // Process streaming response
      let mut full_response = String::new();
      let mut tool_calls_map: std::collections::HashMap<usize, (String, String, String)> = std::collections::HashMap::new();
      let mut stream = response.bytes_stream();

      use tokio_stream::StreamExt;

      while let Some(chunk_result) = stream.next().await {
        match chunk_result {
          Ok(chunk) => {
            let chunk_str = String::from_utf8_lossy(&chunk);

            // Parse SSE format
            for line in chunk_str.lines() {
              if line.starts_with("data: ") {
                let data = &line[6..]; // Remove "data: " prefix

                if data == "[DONE]" {
                  break;
                }

                if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                  // Capture timings if present
                  if let Some(timings) = json_data.get("timings") {
                    prompt_tokens = timings["prompt_n"].as_u64().unwrap_or(prompt_tokens);
                    completion_tokens = timings["predicted_n"].as_u64().unwrap_or(completion_tokens);
                  }

                  if let Some(choices) = json_data["choices"].as_array() {
                    if let Some(choice) = choices.get(0) {
                      // Process delta
                      if let Some(delta) = choice.get("delta") {
                        // Text content
                        if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                          full_response.push_str(content);

                          // Emit stream event to frontend
                          let stream_data = ChatStreamEvent {
                            delta: content.to_string(),
                            is_finished: false,
                            full_response: full_response.clone(),
                            conv_id: request.conv_id.clone(),
                          };

                          if let Err(e) = emit(CHAT_STREAM, stream_data) {
                            log::error!("[llama_server] Failed to emit stream event: {}", e);
                          }
                        }

                        // Tool calls
                        if let Some(tool_calls) = delta.get("tool_calls").and_then(|t| t.as_array()) {
                          for tc in tool_calls {
                            let index = tc["index"].as_u64().unwrap_or(0) as usize;
                            let entry = tool_calls_map.entry(index).or_insert_with(|| (String::new(), String::new(), String::new()));
                            
                            if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                              entry.0 = id.to_string();
                            }
                            
                            if let Some(func) = tc.get("function") {
                              if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                entry.1.push_str(name);
                              }
                              if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
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
          }
          Err(e) => {
            return Err(format!("Error reading stream: {}", e));
          }
        }
      }

      // Final stream completion event
      let final_stream_data = ChatStreamEvent {
        delta: "".to_string(),
        is_finished: true,
        full_response: full_response.clone(),
        conv_id: request.conv_id.clone(),
      };

      let _ = emit(CHAT_STREAM, final_stream_data);

      // Save token usage
      add_token_usage(
          app_handle.clone(),
          "local",
          prompt_tokens,
          completion_tokens,
      ).await?;

      // Decide whether to return text or tool calls
      if !tool_calls_map.is_empty() {
        let mut tool_calls = Vec::new();
        let mut sorted_indices: Vec<_> = tool_calls_map.keys().collect();
        sorted_indices.sort();

        for idx in sorted_indices {
          let (id, name, args) = &tool_calls_map[idx];
          let (skill, tool) = resolve_tool_call(name, request.internal_tools.as_deref());

          tool_calls.push(crate::skills::types::ToolCall {
            id: if id.is_empty() { uuid::Uuid::new_v4().to_string() } else { id.clone() },
            skill_name: skill,
            tool_name: tool,
            arguments: serde_json::from_str(args).unwrap_or(json!({})),
          });
        }
        Ok(LlmResponse::ToolCalls(tool_calls))
      } else {
        Ok(LlmResponse::Text(full_response))
      }
    } else {
      // Handle non-streaming response
      let response = client
        .post(&completion_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", config.api_key))
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
        return Err(format!("Server returned error {}: {}", status, error_text));
      }

      let result: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

      // Extract token usage
      if let Some(timings) = result.get("timings") {
        prompt_tokens = timings["prompt_n"].as_u64().unwrap_or(0);
        completion_tokens = timings["predicted_n"].as_u64().unwrap_or(0);
      }

      // Save token usage
      add_token_usage(
          app_handle.clone(),
          "local",
          prompt_tokens,
          completion_tokens,
      ).await?;

      // Extract generated content
      let generated_text = result["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

      // Emit stream event to frontend even for non-streaming for consistency
      if !generated_text.is_empty() {
        let stream_data = ChatStreamEvent {
          delta: generated_text.clone(),
          is_finished: false,
          full_response: generated_text.clone(),
          conv_id: request.conv_id.clone(),
        };
        let _ = emit(CHAT_STREAM, stream_data);
      }

      // Final stream completion event
      let final_stream_data = ChatStreamEvent {
        delta: "".to_string(),
        is_finished: true,
        full_response: generated_text.clone(),
        conv_id: request.conv_id.clone(),
      };
      let _ = emit(CHAT_STREAM, final_stream_data);

      // Extract tool calls if present
      if has_tool_calls_openai(&result) {
        let tool_calls = parse_openai_tool_calls(&result, request.internal_tools.as_deref());
        Ok(LlmResponse::ToolCalls(tool_calls))
      } else {
        Ok(LlmResponse::Text(generated_text))
      }
    }
  }
}

// Remove the old ToolEnabledProvider implementation
