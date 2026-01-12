use crate::db::core::DbState;
use crate::db::memory::insert_memory_entry;
use crate::events::{emitter::emit, types::*};
use crate::memory::types::MemoryEntry;
use crate::models::embedding::embedding::generate_embedding;
use crate::models::llm::{client::generate, prompts::get_prompt, schemas::get_schema};
use chrono;
use tauri::{AppHandle, Manager};

/// Handle screen summarization event
pub async fn handle_extract_interactive_memory(
  app_handle: &AppHandle,
  event: ExtractInteractiveMemoryEvent,
) -> Result<(), String> {
  // Return an error if message id is empty
  if event.message_id.is_empty() {
    log::error!("[memory] Missing message ID for interactive memory extraction");
    return Err("Missing message ID for interactive memory extraction".into());
  }

  // Load system prompt
  let system_prompt = match get_prompt("extract_interactive_memory") {
    Some(p) => p.to_string(),
    None => {
      log::error!("[memory] Missing system prompt: extract_interactive_memory");
      return Err("Missing system prompt: extract_interactive_memory".into());
    }
  };

  // Load schema
  let schema = match get_schema("extract_interactive_memory") {
    Some(s) => Some(s.to_string()),
    None => {
      log::error!("[memory] Missing schema: extract_interactive_memory");
      return Err("Missing schema: extract_interactive_memory".into());
    }
  };

  // Generate memory extraction
  let extracted_memory = match generate(
    app_handle.clone(),
    event.message.clone(),
    Some(system_prompt),
    schema,
    None,
    Some(false),
    Some(false),
    Some(true),
  )
  .await
  {
    Ok(generated) => {
      log::info!("[memory] Extracted interactive memory: {}", generated);
      generated
    }
    Err(e) => {
      log::error!("[memory] Failed to extract interactive memory: {}", e);
      return Err("Failed to extract interactive memory".into());
    }
  };

  // Extract the memory text
  let extracted_memory = match serde_json::from_str::<serde_json::Value>(&extracted_memory) {
    Ok(json) => match json.get("memory") {
      Some(memory_value) => match memory_value.as_str() {
        Some(memory_text) => memory_text.to_string(),
        None => {
          log::error!("[memory] Memory field is not a string");
          return Err("Memory field is not a string".into());
        }
      },
      None => {
        log::error!("[memory] No 'memory' field found in JSON response");
        return Err("No 'memory' field found in JSON response".into());
      }
    },
    Err(e) => {
      log::error!("[memory] Failed to parse JSON response: {}", e);
      return Err("Failed to parse JSON response".into());
    }
  };

  // Skip if extracted memory is empty
  if extracted_memory.trim().is_empty() {
    log::info!("[memory] Extracted memory is empty, skipping save");
    return Ok(());
  }

  // Generate memory embedding
  let embedding = match generate_embedding(app_handle.clone(), extracted_memory.clone()).await {
    Ok(emb) => emb,
    Err(e) => {
      log::error!("[memory] Failed to generate embedding: {}", e);
      return Err("Failed to generate embedding".into());
    }
  };

  let memory = MemoryEntry {
    id: uuid::Uuid::new_v4().to_string(),
    message_id: event.message_id.clone(),
    memory_type: "interactive".to_string(),
    text: extracted_memory.clone(),
    embedding,
    timestamp: chrono::Utc::now().to_rfc3339(),
    similarity: None,
  };

  // Save to database
  let db_state = app_handle.state::<DbState>();
  if let Err(e) = insert_memory_entry(db_state, memory.clone()) {
    log::error!("[memory] Failed to save memory entry: {}", e);
    return Err("Failed to save memory entry".into());
  }

  // Emit event that memory was extracted and saved
  let memory_extracted_event = MemoryExtractedEvent {
    memory,
    timestamp: chrono::Utc::now().to_rfc3339(),
  };
  let _ = emit(MEMORY_EXTRACTED, memory_extracted_event);
  Ok(())
}