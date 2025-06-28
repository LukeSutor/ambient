use anyhow::Result;
use mistralrs::{
    TextMessageRole, GgufModelBuilder, RequestBuilder
};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String, // Changed from TextMessageRole to String for easier serialization
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<ConversationMessage>,
    pub system_prompt: String,
}

pub struct Qwen3State {
    pub model: Option<mistralrs::Model>,
    pub conversations: HashMap<String, Conversation>,
    pub current_conversation_id: Option<String>,
}

impl Qwen3State {
    pub fn new() -> Self {
        Self {
            model: None,
            conversations: HashMap::new(),
            current_conversation_id: None,
        }
    }
}

pub static GLOBAL_QWEN3_STATE: Lazy<Mutex<Qwen3State>> = Lazy::new(|| {
    Mutex::new(Qwen3State::new())
});

/// Initialize the Qwen3 model on application startup
pub async fn initialize_qwen3_model() -> Result<(), String> {
    println!("[Qwen3] Initializing model...");    let model = GgufModelBuilder::new(
        "C:/Users/Luke/AppData/Roaming/com.tauri.dev/models/vlm/",
        vec!["Qwen3-1.7B-Q8_0.gguf"],
    )
        .with_logging()
        .build()
        .await
        .map_err(|e| format!("Failed to build Qwen3 model: {}", e))?;

    println!("[Qwen3] Model built successfully, storing in global state...");

    let mut state = GLOBAL_QWEN3_STATE.lock()
        .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
    
    state.model = Some(model);
    
    println!("[Qwen3] Model initialization complete.");
    Ok(())
}

/// Create a new conversation or get existing one
fn get_or_create_conversation(
    state: &mut Qwen3State, 
    conversation_id: Option<String>,
    system_prompt: Option<String>
) -> String {
    let conv_id = conversation_id.unwrap_or_else(|| {
        format!("conv_{}", chrono::Utc::now().timestamp_millis())
    });
    
    if !state.conversations.contains_key(&conv_id) {
        let conversation = Conversation {
            id: conv_id.clone(),
            messages: Vec::new(),
            system_prompt: system_prompt.unwrap_or_else(|| "You are a helpful assistant.".to_string()),
        };
        state.conversations.insert(conv_id.clone(), conversation);
    }
    
    state.current_conversation_id = Some(conv_id.clone());
    conv_id
}

/// Generates a response from the Qwen3 model with conversation management
#[tauri::command]
pub async fn generate_qwen3(
    prompt: String,
    thinking: Option<bool>,
    reset_conversation: Option<bool>,
    conversation_id: Option<String>,
    system_prompt: Option<String>,
) -> Result<String, String> {
    println!("[Qwen3] Generating response for prompt: {}", prompt);
    
    let thinking = thinking.unwrap_or(false);
    let reset_conversation = reset_conversation.unwrap_or(false);
    
    // First, get the model reference and release the lock
    let model = {
        let state = GLOBAL_QWEN3_STATE.lock()
            .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
        
        state.model.as_ref()
            .ok_or("Qwen3 model not initialized. Please restart the application.".to_string())?
            .clone() // Clone the model reference
    };
    
    // Now handle conversation management with a separate lock
    let (conv_id, conversation_messages, system_prompt_text) = {
        let mut state = GLOBAL_QWEN3_STATE.lock()
            .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
        
        // Handle conversation management
        let conv_id = if reset_conversation {
            // Remove existing conversation if reset requested
            if let Some(id) = &conversation_id {
                state.conversations.remove(id);
            } else if let Some(current_id) = &state.current_conversation_id {
                state.conversations.remove(current_id);
            }
            get_or_create_conversation(&mut state, conversation_id, system_prompt)
        } else {
            get_or_create_conversation(&mut state, conversation_id, system_prompt)
        };
        
        let conversation = state.conversations.get_mut(&conv_id)
            .ok_or("Failed to get conversation".to_string())?;
        
        // Add user message to conversation
        conversation.messages.push(ConversationMessage {
            role: "User".to_string(),
            content: prompt.clone(),
        });
        
        // Clone the data we need for the request
        let messages = conversation.messages.clone();
        let sys_prompt = conversation.system_prompt.clone();
        
        (conv_id, messages, sys_prompt)
    };
    
    // Build request with conversation history
    let mut request_builder = RequestBuilder::new();
    
    // Add system message
    request_builder = request_builder.add_message(
        TextMessageRole::System,
        &system_prompt_text,
    );
    
    // Add conversation history
    for msg in &conversation_messages {
        let role = match msg.role.as_str() {
            "User" => TextMessageRole::User,
            "Assistant" => TextMessageRole::Assistant,
            _ => TextMessageRole::User, // Default fallback
        };
        request_builder = request_builder.add_message(role, &msg.content);
    }
    
    // Enable thinking if requested
    let request_builder = request_builder.enable_thinking(thinking);
    
    println!("[Qwen3] Sending request with {} messages in conversation", conversation_messages.len());
    
    let response = model
        .send_chat_request(request_builder)
        .await
        .map_err(|e| format!("Failed to send chat request: {}", e))?;

    let content = response.choices[0]
        .message
        .content
        .as_ref()
        .cloned()
        .unwrap_or_else(|| "".to_string());

    // Add assistant response to conversation with a new lock
    {
        let mut state = GLOBAL_QWEN3_STATE.lock()
            .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
        
        if let Some(conversation) = state.conversations.get_mut(&conv_id) {
            conversation.messages.push(ConversationMessage {
                role: "Assistant".to_string(),
                content: content.clone(),
            });
        }
    }

    println!("[Qwen3] Response generated successfully.");

    Ok(content)
}

/// Get current conversation history
#[tauri::command]
pub fn get_conversation_history(conversation_id: Option<String>) -> Result<Vec<ConversationMessage>, String> {
    let state = GLOBAL_QWEN3_STATE.lock()
        .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
    
    let conv_id = conversation_id
        .or_else(|| state.current_conversation_id.clone())
        .ok_or("No conversation ID provided and no current conversation".to_string())?;
    
    let conversation = state.conversations.get(&conv_id)
        .ok_or(format!("Conversation with ID '{}' not found", conv_id))?;
    
    Ok(conversation.messages.clone())
}

/// Reset a specific conversation or the current one
#[tauri::command]
pub fn reset_conversation(conversation_id: Option<String>) -> Result<(), String> {
    let mut state = GLOBAL_QWEN3_STATE.lock()
        .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
    
    let conv_id = conversation_id
        .or_else(|| state.current_conversation_id.clone())
        .ok_or("No conversation ID provided and no current conversation".to_string())?;
    
    state.conversations.remove(&conv_id);
    
    if state.current_conversation_id.as_ref() == Some(&conv_id) {
        state.current_conversation_id = None;
    }
    
    println!("[Qwen3] Reset conversation: {}", conv_id);
    Ok(())
}

/// List all conversation IDs
#[tauri::command]
pub fn list_conversations() -> Result<Vec<String>, String> {
    let state = GLOBAL_QWEN3_STATE.lock()
        .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
    
    Ok(state.conversations.keys().cloned().collect())
}

/// Get current conversation ID
#[tauri::command]
pub fn get_current_conversation_id() -> Result<Option<String>, String> {
    let state = GLOBAL_QWEN3_STATE.lock()
        .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
    
    Ok(state.current_conversation_id.clone())
}

/// Check if the Qwen3 model is initialized
#[tauri::command]
pub fn is_qwen3_model_initialized() -> Result<bool, String> {
    let state = GLOBAL_QWEN3_STATE.lock()
        .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
    
    Ok(state.model.is_some())
}

/// Get model initialization status and conversation count
#[tauri::command]
pub fn get_qwen3_status() -> Result<serde_json::Value, String> {
    let state = GLOBAL_QWEN3_STATE.lock()
        .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;
    
    Ok(serde_json::json!({
        "model_initialized": state.model.is_some(),
        "conversation_count": state.conversations.len(),
        "current_conversation_id": state.current_conversation_id
    }))
}

/// Legacy generate function for backward compatibility
#[tauri::command]
pub async fn generate(prompt: String) -> Result<String, String> {
    generate_qwen3(prompt, Some(false), Some(false), None, None).await
}