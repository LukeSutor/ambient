use crate::db::core::DbState;
use crate::memory::types::MemoryEntry;
use crate::events::{emitter::emit, types::*};
use crate::models::llm::{prompts::get_prompt, schemas::get_schema, client::generate};
use crate::models::embedding::embedding::generate_embedding;
use tauri::{AppHandle, Manager};
use chrono;
use crate::db::memory::insert_memory_entry;

/// Handle screen summarization event
pub async fn handle_extract_interactive_memory(
    event: ExtractInteractiveMemoryEvent,
    app_handle: &AppHandle,
) {
    // Return an error if message id is empty
    if event.message_id.is_empty() {
        log::error!("[memory] Missing message ID for interactive memory extraction");
        return;
    }
    
    // Load system prompt
    let system_prompt = match get_prompt("extract_interactive_memory") {
        Some(p) => p.to_string(),
        None => {
            log::error!("[memory] Missing system prompt: extract_interactive_memory");
            return;
        }
    };

    // Load schema
    let schema = match get_schema("extract_interactive_memory") {
        Some(s) => Some(s.to_string()),
        None => {
            log::error!("[memory] Missing schema: extract_interactive_memory");
            return;
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
    .await {
        Ok(generated) => {
            log::info!("[memory] Extracted interactive memory: {}", generated);
            generated
        }
        Err(e) => {
            log::error!("[memory] Failed to extract interactive memory: {}", e);
            return;
        }
    };

    // Extract the memory text
    let extracted_memory = match serde_json::from_str::<serde_json::Value>(&extracted_memory) {
        Ok(json) => {
            match json.get("memory") {
                Some(memory_value) => {
                    match memory_value.as_str() {
                        Some(memory_text) => memory_text.to_string(),
                        None => {
                            log::error!("[memory] Memory field is not a string");
                            return;
                        }
                    }
                }
                None => {
                    log::error!("[memory] No 'memory' field found in JSON response");
                    return;
                }
            }
        }
        Err(e) => {
            log::error!("[memory] Failed to parse JSON response: {}", e);
            return;
        }
    };

    // Skip if extracted memory is empty
    if extracted_memory.trim().is_empty() {
        log::info!("[memory] Extracted memory is empty, skipping save");
        return;
    }

    // Generate memory embedding
    let embedding = match generate_embedding(app_handle.clone(), extracted_memory.clone()).await {
        Ok(emb) => emb,
        Err(e) => {
            log::error!("[memory] Failed to generate embedding: {}", e);
            return;
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
        return;
    }

    // Emit event that memory was extracted and saved
    let memory_extracted_event = MemoryExtractedEvent {
        memory,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    let _ = emit(MEMORY_EXTRACTED, memory_extracted_event);
}