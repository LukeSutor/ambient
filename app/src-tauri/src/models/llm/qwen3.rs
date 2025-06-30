use anyhow::Result;
use mistralrs::{
  ChatCompletionChunkResponse, ChunkChoice, Delta, GgufModelBuilder, RequestBuilder, Response,
  TextMessageRole,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
  pub role: String, // Changed from TextMessageRole to String for easier serialization
  pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamResponsePayload {
  pub content: String,
  pub is_finished: bool,
  pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
  pub id: String,
  pub messages: Vec<ConversationMessage>,
  pub system_prompt: String,
}

pub struct Qwen3State {
  pub model: Option<Arc<mistralrs::Model>>,
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

pub static GLOBAL_QWEN3_STATE: Lazy<Mutex<Qwen3State>> =
  Lazy::new(|| Mutex::new(Qwen3State::new()));

/// Initialize the Qwen3 model on application startup
pub async fn initialize_qwen3_model() -> Result<(), String> {
  println!("[Qwen3] Initializing model...");
  let model = GgufModelBuilder::new(
    "C:/Users/Luke/AppData/Roaming/com.tauri.dev/models/vlm/",
    vec!["Qwen3-1.7B-Q4_K_M.gguf"],
  )
  .with_logging()
  .build()
  .await
  .map_err(|e| format!("Failed to build Qwen3 model: {}", e))?;

  println!("[Qwen3] Model built successfully, storing in global state...");

  let mut state = GLOBAL_QWEN3_STATE
    .lock()
    .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

  state.model = Some(Arc::new(model));

  println!("[Qwen3] Model initialization complete.");
  Ok(())
}

/// Create a new conversation or get existing one
fn get_or_create_conversation(
  state: &mut Qwen3State,
  conversation_id: Option<String>,
  system_prompt: Option<String>,
) -> String {
  let conv_id =
    conversation_id.unwrap_or_else(|| format!("conv_{}", chrono::Utc::now().timestamp_millis()));

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

  // First, handle conversation management and prepare request data
  let (conv_id, conversation_messages, system_prompt_text) = {
    let mut state = GLOBAL_QWEN3_STATE
      .lock()
      .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

    // Handle conversation management
    let conv_id = if reset_conversation {
      // Remove existing conversation if reset requested
      if let Some(id) = &conversation_id {
        state.conversations.remove(id);
      } else if let Some(current_id) = state.current_conversation_id.clone() {
        state.conversations.remove(&current_id);
      }
      get_or_create_conversation(&mut state, conversation_id, system_prompt)
    } else {
      get_or_create_conversation(&mut state, conversation_id, system_prompt)
    };

    let conversation = state
      .conversations
      .get_mut(&conv_id)
      .ok_or("Failed to get conversation".to_string())?;

    // Add user message to conversation
    conversation.messages.push(ConversationMessage {
      role: "User".to_string(),
      content: prompt.clone(),
    });

    // Clone the data we need for the request
    let conversation_messages = conversation.messages.clone();
    let system_prompt_text = conversation.system_prompt.clone();

    (conv_id, conversation_messages, system_prompt_text)
  };

  // Now get the model reference and make the request
  let response = {
    // First, check if model is initialized and build the request
    let request_builder = {
      let state = GLOBAL_QWEN3_STATE
        .lock()
        .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

      // Check if model is initialized
      state
        .model
        .as_ref()
        .ok_or("Qwen3 model not initialized. Please restart the application.".to_string())?;

      // Build request with conversation history
      let mut request_builder = RequestBuilder::new();

      // Add system message
      request_builder = request_builder.add_message(TextMessageRole::System, &system_prompt_text);

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
      request_builder.enable_thinking(thinking)
    };

    println!(
      "[Qwen3] Sending request with {} messages in conversation",
      conversation_messages.len()
    );

    // Now make the request - get model reference and immediately clone the Arc
    let model = {
      let state = GLOBAL_QWEN3_STATE
        .lock()
        .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

      state
        .model
        .as_ref()
        .ok_or("Qwen3 model not initialized. Please restart the application.".to_string())?
        .clone()
    };

    model
      .send_chat_request(request_builder)
      .await
      .map_err(|e| format!("Failed to send chat request: {}", e))?
  };

  let content = response.choices[0]
    .message
    .content
    .as_ref()
    .cloned()
    .unwrap_or_else(|| "".to_string());

  // Add assistant response to conversation in a final scope
  {
    let mut state = GLOBAL_QWEN3_STATE
      .lock()
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

/// Stream responses from Qwen3 model with real-time updates via Tauri events
#[tauri::command]
pub async fn stream_qwen3(
  app_handle: tauri::AppHandle,
  prompt: String,
  thinking: Option<bool>,
  reset_conversation: Option<bool>,
  conversation_id: Option<String>,
  system_prompt: Option<String>,
) -> Result<String, String> {
  println!("[Qwen3] Starting streaming response for prompt: {}", prompt);

  let thinking = thinking.unwrap_or(false);
  let reset_conversation = reset_conversation.unwrap_or(false);

  // First, handle conversation management and prepare request data
  let (conv_id, conversation_messages, system_prompt_text) = {
    let mut state = GLOBAL_QWEN3_STATE
      .lock()
      .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

    // Handle conversation management
    let conv_id = if reset_conversation {
      // Remove existing conversation if reset requested
      if let Some(id) = &conversation_id {
        state.conversations.remove(id);
      } else if let Some(current_id) = state.current_conversation_id.clone() {
        state.conversations.remove(&current_id);
      }
      get_or_create_conversation(&mut state, conversation_id, system_prompt)
    } else {
      get_or_create_conversation(&mut state, conversation_id, system_prompt)
    };

    let conversation = state
      .conversations
      .get_mut(&conv_id)
      .ok_or("Failed to get conversation".to_string())?;

    // Add user message to conversation
    conversation.messages.push(ConversationMessage {
      role: "User".to_string(),
      content: prompt.clone(),
    });

    // Clone the data we need for the request
    let conversation_messages = conversation.messages.clone();
    let system_prompt_text = conversation.system_prompt.clone();

    (conv_id, conversation_messages, system_prompt_text)
  };

  // Build the request first
  let request_builder = {
    let state = GLOBAL_QWEN3_STATE
      .lock()
      .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

    // Check if model is initialized
    state
      .model
      .as_ref()
      .ok_or("Qwen3 model not initialized. Please restart the application.".to_string())?;

    // Build request with conversation history
    let mut request_builder = RequestBuilder::new();

    // Add system message
    request_builder = request_builder.add_message(TextMessageRole::System, &system_prompt_text);

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
    request_builder.enable_thinking(thinking)
  };

  println!(
    "[Qwen3] Sending streaming request with {} messages in conversation",
    conversation_messages.len()
  );

  // Get the model and create the stream
  let model = {
    let state = GLOBAL_QWEN3_STATE
      .lock()
      .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

    state
      .model
      .as_ref()
      .ok_or("Qwen3 model not initialized. Please restart the application.".to_string())?
      .clone()
  };

  let mut stream = model
    .stream_chat_request(request_builder)
    .await
    .map_err(|e| format!("Failed to create stream: {}", e))?;

  let mut full_content = String::new();

  // Process the stream and emit updates to the frontend
  while let Some(chunk) = stream.next().await {
    // Handle streaming response chunks properly
    if let Response::Chunk(ChatCompletionChunkResponse { choices, .. }) = chunk {
      if let Some(ChunkChoice {
        delta: Delta {
          content: Some(content),
          ..
        },
        ..
      }) = choices.first()
      {
        full_content.push_str(content);

        // Emit streaming update to frontend
        if let Err(e) = app_handle.emit(
          "qwen3-stream",
          StreamResponsePayload {
            content: content.clone(),
            is_finished: false,
            conversation_id: conv_id.clone(),
          },
        ) {
          eprintln!("[Qwen3] Failed to emit stream event: {}", e);
        }
      }
    } else {
      // Handle errors or other response types
      eprintln!("[Qwen3] Received non-chunk response in stream");
    }
  }

  // Add assistant response to conversation in a final scope
  {
    let mut state = GLOBAL_QWEN3_STATE
      .lock()
      .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

    if let Some(conversation) = state.conversations.get_mut(&conv_id) {
      conversation.messages.push(ConversationMessage {
        role: "Assistant".to_string(),
        content: full_content.clone(),
      });
    }
  }

  // Emit final completion event with the full content
  if let Err(e) = app_handle.emit(
    "qwen3-stream",
    StreamResponsePayload {
      content: full_content.clone(), // Send the complete content for completion event
      is_finished: true,
      conversation_id: conv_id.clone(),
    },
  ) {
    eprintln!("[Qwen3] Failed to emit completion event: {}", e);
  }

  println!("[Qwen3] Streaming response completed successfully.");

  Ok(full_content)
}

/// Get current conversation history
#[tauri::command]
pub fn get_conversation_history(
  conversation_id: Option<String>,
) -> Result<Vec<ConversationMessage>, String> {
  let state = GLOBAL_QWEN3_STATE
    .lock()
    .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

  let conv_id = conversation_id
    .or_else(|| state.current_conversation_id.clone())
    .ok_or("No conversation ID provided and no current conversation".to_string())?;

  let conversation = state
    .conversations
    .get(&conv_id)
    .ok_or(format!("Conversation with ID '{}' not found", conv_id))?;

  Ok(conversation.messages.clone())
}

/// Reset a specific conversation or the current one
#[tauri::command]
pub fn reset_conversation(conversation_id: Option<String>) -> Result<(), String> {
  let mut state = GLOBAL_QWEN3_STATE
    .lock()
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
  let state = GLOBAL_QWEN3_STATE
    .lock()
    .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

  Ok(state.conversations.keys().cloned().collect())
}

/// Get current conversation ID
#[tauri::command]
pub fn get_current_conversation_id() -> Result<Option<String>, String> {
  let state = GLOBAL_QWEN3_STATE
    .lock()
    .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

  Ok(state.current_conversation_id.clone())
}

/// Check if the Qwen3 model is initialized
#[tauri::command]
pub fn is_qwen3_model_initialized() -> Result<bool, String> {
  let state = GLOBAL_QWEN3_STATE
    .lock()
    .map_err(|_| "Failed to acquire Qwen3 state lock".to_string())?;

  Ok(state.model.is_some())
}

/// Get model initialization status and conversation count
#[tauri::command]
pub fn get_qwen3_status() -> Result<serde_json::Value, String> {
  let state = GLOBAL_QWEN3_STATE
    .lock()
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

/*
Frontend Usage Example:

// In your React component:
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

interface StreamResponsePayload {
  content: string;
  is_finished: boolean;
  conversation_id: string;
}

function useQwen3Stream() {
  const [streamContent, setStreamContent] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);

  useEffect(() => {
    const setupStreamListener = async () => {
      const unlisten = await listen<StreamResponsePayload>('qwen3-stream', (event) => {
        const { content, is_finished, conversation_id } = event.payload;

        if (is_finished) {
          setIsStreaming(false);
          console.log('Stream finished for conversation:', conversation_id);
        } else {
          setStreamContent(prev => prev + content);
        }
      });

      // Error listener
      const unlistenError = await listen<string>('qwen3-stream-error', (event) => {
        console.error('Stream error:', event.payload);
        setIsStreaming(false);
      });

      return () => {
        unlisten();
        unlistenError();
      };
    };

    setupStreamListener();
  }, []);

  const startStream = async (prompt: string) => {
    setStreamContent('');
    setIsStreaming(true);

    try {
      await invoke('stream_qwen3', {
        appHandle: undefined, // Tauri provides this automatically
        prompt,
        thinking: false,
        resetConversation: false,
        conversationId: null,
        systemPrompt: null,
      });
    } catch (error) {
      console.error('Failed to start stream:', error);
      setIsStreaming(false);
    }
  };

  return { streamContent, isStreaming, startStream };
}
*/
