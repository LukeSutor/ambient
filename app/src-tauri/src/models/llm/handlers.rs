use crate::db::conversations::{
  create_attachments, add_attachments, add_message, add_message_with_id, update_conversation_name,
};
use crate::db::memory::find_similar_memories;
use crate::events::{emitter::emit, types::*};
use crate::models::llm::{client::generate, prompts::get_prompt, schemas::get_schema, types::LlmRequest};
use tauri::AppHandle;

#[tauri::command]
pub async fn handle_hud_chat(app_handle: AppHandle, event: HudChatEvent) -> Result<String, String> {
  // Save the user message to the database
  let _user_message = match add_message_with_id(
    &app_handle,
    event.conv_id.clone(),
    "user".to_string(),
    event.text.clone(),
    Some(event.message_id.clone()),
  )
  .await
  {
    Ok(message) => Some(message),
    Err(e) => {
      log::error!("[hud_chat] Failed to save user message: {}", e);
      None
    }
  };

  // Emit extract memory event
  let extract_event = ExtractInteractiveMemoryEvent {
    message: event.text.clone(),
    message_id: event.message_id.clone(),
    timestamp: chrono::Utc::now().to_rfc3339(),
  };
  let _ = emit(EXTRACT_INTERACTIVE_MEMORY, extract_event);

  // Create prompt
  let system_prompt_template = match get_prompt("hud_chat") {
    Some(template) => template,
    None => {
      log::error!("[hud_chat] Failed to get prompt template for 'hud_chat'");
      return Err("Failed to get prompt template for 'hud_chat'".into());
    }
  };

  // Create attachments and save them to the database
  let attachments = create_attachments(
    &app_handle.clone(),
    event.message_id.clone(),
    event.attachments.clone(),
  )
  .await;
  
  match attachments {
    Ok(att_records) => {
      log::info!("emitting attachments created event");
      // Emit attachments created event
      let now = chrono::Utc::now();
      let attachments_event = AttachmentsCreatedEvent {
        message_id: event.message_id.clone(),
        attachments: att_records.clone(),
        timestamp: now.to_rfc3339(),
      };
      let _ = emit(ATTACHMENTS_CREATED, attachments_event);

      // Link attachments to the message
      if let Err(e) = add_attachments(
        &app_handle.clone(),
        event.message_id.clone(),
        att_records,
      )
      .await
      {
        log::error!("[hud_chat] Failed to link attachments to message: {}", e);
      }
    }
    Err(e) => {
      log::error!("[hud_chat] Failed to create attachments: {}", e);
    }
  }

  // Get the current date time YYYY-MM-DD format
  let current_date_time = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

  let system_prompt = system_prompt_template.replace("{currentDateTime}", &current_date_time);

  // Get 3 most relevant memories
  let relevant_memories =
    match find_similar_memories(&app_handle.clone(), &event.text, 3, 0.8).await {
      Ok(memories) => memories,
      Err(e) => {
        log::warn!("[hud_chat] Failed to find similar memories: {}", e);
        Vec::new()
      }
    };

  // Create memory context string
  let mut memory_context = String::new();
  if !relevant_memories.is_empty() {
    memory_context.push_str("Relevant memories from past interactions:\n");
    for memory in relevant_memories {
      memory_context.push_str(&format!("- {}\n", memory.text));
    }
    memory_context.push_str("\n");
  }

  // Create user prompt with memory context
  let user_prompt = format!(
    "{}Task: {}",
    if memory_context.is_empty() {
      ""
    } else {
      memory_context.as_str()
    },
    event.text
  );

  log::info!("[hud_chat] Generated user prompt:\n{}", user_prompt);

  // Generate response
  let request = LlmRequest::new(user_prompt)
    .with_system_prompt(Some(system_prompt))
    .with_conv_id(Some(event.conv_id.clone()))
    .with_use_thinking(Some(false))
    .with_stream(Some(true))
    .with_current_message_id(Some(event.message_id.clone()));

  let response = match generate(app_handle.clone(), request, None).await {
    Ok(response) => {
      response
    }
    Err(e) => {
      log::error!("[hud_chat] Failed to generate response: {}", e);
      return Err("Failed to generate response".into());
    }
  };

  // Save the assistant response to the database
  if let Err(e) = add_message(
    &app_handle,
    event.conv_id.clone(),
    "assistant".to_string(),
    response.clone(),
  )
  .await
  {
    log::error!("[hud_chat] Failed to save assistant message: {}", e);
  }

  // Return response
  Ok(response)
}

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
    Ok(generated) => {
      log::info!("[generate_conversation_name] Generated conversation name");
      generated
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
