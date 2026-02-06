use crate::db::conversations::update_conversation_name;
use crate::events::{emitter::emit, types::{RENAME_CONVERSATION, RenameConversationEvent, GenerateConversationNameEvent}};
use crate::models::llm::{client::generate, prompts::get_prompt, schemas::get_schema, types::{LlmRequest, LlmResponse}};
use tauri::AppHandle;

pub async fn handle_generate_conversation_name(
  app_handle: &AppHandle,
  event: GenerateConversationNameEvent,
) -> Result<(), String> {
  log::info!(
    "[generate_conversation_name] Generating name for conversation ID: {}",
    event.conv_id
  );
  
  // Load system prompt
  let system_prompt = match get_prompt("generate_conversation_name") {
    Some(p) => p.to_string(),
    None => {
      log::error!("[generate_conversation_name] Missing system prompt: generate_conversation_name");
      return Err("Missing system prompt: generate_conversation_name".into());
    }
  };

  // Load schema
  let schema = match get_schema("generate_conversation_name") {
    Some(s) => Some(s.to_string()),
    None => {
      log::error!("[generate_conversation_name] Missing schema: generate_conversation_name");
      return Err("Missing schema: generate_conversation_name".into());
    }
  };

  // Generate name
  let request = LlmRequest::new(event.message.clone())
    .with_system_prompt(Some(system_prompt))
    .with_json_schema(schema)
    .with_use_thinking(Some(false))
    .with_stream(Some(false));

  let generated_name = match generate(app_handle.clone(), request, Some(true)).await {
    Ok(LlmResponse::Text(generated)) => {
      log::info!("[generate_conversation_name] Generated conversation name");
      generated
    }
    Ok(_) => {
      log::error!("[generate_conversation_name] Received tool calls instead of text name");
      return Err("Received tool calls instead of text name".into());
    }
    Err(e) => {
      log::error!("[generate_conversation_name] Failed to generate conversation name: {}", e);
      return Err("Failed to generate conversation name".into());
    }
  };

  // Extract the name text
  let extracted_name = match serde_json::from_str::<serde_json::Value>(&generated_name) {
    Ok(json) => match json.get("name") {
      Some(name_value) => match name_value.as_str() {
        Some(name_text) => name_text.to_string(),
        None => {
          log::error!("[generate_conversation_name] Name field is not a string");
          return Err("Name field is not a string".into());
        }
      },
      None => {
        log::error!("[generate_conversation_name] No 'name' field found in JSON response");
        return Err("No 'name' field found in JSON response".into());
      }
    },
    Err(e) => {
      log::error!("[generate_conversation_name] Failed to parse JSON response: {}", e);
      return Err("Failed to parse JSON response".into());
    }
  };

  // Skip if extracted name is empty
  if extracted_name.trim().is_empty() {
    log::info!("[generate_conversation_name] Extracted name is empty, skipping save");
    return Ok(());
  }

  // Save to db
  match update_conversation_name(app_handle.clone(), event.conv_id.clone(), extracted_name.clone()).await {
    Ok(_) => {}
    Err(e) => {
      log::error!(
        "[generate_conversation_name] Failed to rename conversation {}: {}",
        event.conv_id,
        e
      );
    }
  };

  // Emit name change event
  let name_event = RenameConversationEvent {
    conv_id: event.conv_id,
    new_name: extracted_name,
    timestamp: chrono::Utc::now().to_rfc3339(),
  };
  let _ = emit(RENAME_CONVERSATION, name_event);
  Ok(())
}
