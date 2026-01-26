use crate::db::core::DbState;
use crate::events::types::AttachmentData;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use ts_rs::TS;
use uuid::Uuid;
use rusqlite::Connection;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, TS)]
#[ts(rename_all = "lowercase")]
#[ts(export, export_to = "conversations.ts")]
pub enum Role {
  System,
  User,
  Assistant,
  FunctionCall,
}

impl Role {
  pub fn as_str(&self) -> &str {
    match self {
      Role::System => "system",
      Role::User => "user",
      Role::Assistant => "assistant",
      Role::FunctionCall => "functioncall",
    }
  }

  pub fn from_str(s: &str) -> Self {
    match s {
      "system" => Role::System,
      "user" => Role::User,
      "assistant" => Role::Assistant,
      "functioncall" => Role::FunctionCall,
      _ => Role::User,
    }
  }
}

/// Attachment structure
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "conversations.ts")]
pub struct Attachment {
  pub id: String,
  pub message_id: String,
  pub file_type: String,
  pub file_name: String,
  pub file_path: Option<String>,
  pub extracted_text: Option<String>,
  pub created_at: String,
}

/// Message structure
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "conversations.ts")]
pub struct Message {
  pub id: String,
  pub conversation_id: String,
  pub role: Role,
  pub content: String,
  pub timestamp: String,
  pub attachments: Vec<Attachment>,
}

/// Conversation structure
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "conversations.ts")]
pub struct Conversation {
  pub id: String,
  pub name: String,
  conv_type: String,
  pub created_at: String,
  pub updated_at: String,
  pub message_count: i32,
}

/// Generate a conversation name from the first message
fn generate_conversation_name(first_message: Option<&str>) -> String {
  if let Some(message) = first_message {
    let words: Vec<&str> = message.split_whitespace().take(5).collect();
    let preview = words.join(" ");
    if preview.len() > 40 {
      format!("{}...", &preview[..37])
    } else if preview.is_empty() {
      format!("New Chat {}", Utc::now().format("%m/%d %H:%M"))
    } else {
      preview
    }
  } else {
    format!("New Chat {}", Utc::now().format("%m/%d %H:%M"))
  }
}

/// Create a new conversation
#[tauri::command]
pub async fn create_conversation(
  app_handle: AppHandle,
  name: Option<String>,
  conv_type: Option<String>,
) -> Result<Conversation, String> {
  let state = app_handle.state::<DbState>();
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;

  let conversation_id = Uuid::new_v4().to_string();
  let now = Utc::now();
  let conversation_name = name.unwrap_or_else(|| format!("New Chat {}", now.format("%m/%d %H:%M")));
  let conversation_type = conv_type.unwrap_or_else(|| "chat".to_string());

  let conversation = Conversation {
    id: conversation_id.clone(),
    name: conversation_name.clone(),
    conv_type: conversation_type.clone(),
    created_at: now.to_rfc3339(),
    updated_at: now.to_rfc3339(),
    message_count: 0,
  };

  conn
    .execute(
      "INSERT INTO conversations (id, name, conv_type, created_at, updated_at, message_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
      params![
        conversation_id,
        conversation_name,
        conversation_type,
        now.to_rfc3339(),
        now.to_rfc3339(),
        0
      ],
    )
    .map_err(|e| format!("Failed to create conversation: {}", e))?;

  log::info!(
    "[conversations] Created conversation: {} ({})",
    conversation_name,
    conversation_id
  );
  Ok(conversation)
}

/// Add a message to a conversation
pub async fn add_message(
  app_handle: &AppHandle,
  conversation_id: String,
  role: String,
  content: String,
) -> Result<Message, String> {
  add_message_with_id(app_handle, conversation_id, role, content, None).await
}

pub async fn add_message_with_id(
  app_handle: &AppHandle,
  conversation_id: String,
  role: String,
  content: String,
  message_id: Option<String>,
) -> Result<Message, String> {
  let state = app_handle.state::<DbState>();
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;

  let message_id = message_id.unwrap_or_else(|| Uuid::new_v4().to_string());
  let now = Utc::now();

  let message = Message {
    id: message_id.clone(),
    conversation_id: conversation_id.clone(),
    role: Role::from_str(&role),
    content: content.clone(),
    timestamp: now.to_rfc3339(),
    attachments: vec![],
  };

  // Insert the message
  conn
    .execute(
      "INSERT INTO conversation_messages (id, conversation_id, role, content, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5)",
      params![
        message_id,
        conversation_id,
        Role::from_str(&role).as_str(),
        content,
        now.to_rfc3339()
      ],
    )
    .map_err(|e| format!("Failed to add message: {}", e))?;

  // Update conversation
  conn
    .execute(
      "UPDATE conversations SET message_count = message_count + 1, updated_at = ?1 WHERE id = ?2",
      params![now.to_rfc3339(), conversation_id],
    )
    .map_err(|e| format!("Failed to update conversation: {}", e))?;

  // Auto-update conversation name if it's the first user message
  if Role::from_str(&role) == Role::User {
    let message_count: i32 = conn
      .query_row(
        "SELECT message_count FROM conversations WHERE id = ?1",
        params![conversation_id],
        |row| row.get(0),
      )
      .unwrap_or(0);

    if message_count == 1 {
      let auto_name = generate_conversation_name(Some(&content));
      let _ = conn.execute(
        "UPDATE conversations SET name = ?1 WHERE id = ?2",
        params![auto_name, conversation_id],
      );
    }
  }

  Ok(message)
}

/// Get all messages for a conversation
#[tauri::command]
pub async fn get_messages(
  app_handle: AppHandle,
  conversation_id: String,
) -> Result<Vec<Message>, String> {
  let state = app_handle.state::<DbState>();
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;

  let mut stmt = conn
    .prepare(
      "SELECT m.id, m.conversation_id, m.role, m.content, m.timestamp, 
        a.id, a.message_id, a.file_type, a.file_name, a.file_path, a.extracted_text, a.created_at
        FROM conversation_messages m 
        LEFT JOIN attachments a ON m.id = a.message_id
        WHERE conversation_id = ?1 
        ORDER BY m.timestamp ASC",
    )
    .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let mut rows = stmt
      .query(params![conversation_id])
      .map_err(|e| format!("Failed to query messages: {}", e))?;

    let mut messages: Vec<Message> = Vec::new();

    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
      let msg_id: String = row.get(0).map_err(|e| e.to_string())?;

      if messages.is_empty() || messages.last().unwrap().id != msg_id {
        let role_str: String = row.get(2).map_err(|e| e.to_string())?;
        messages.push(Message {
          id: msg_id,
          conversation_id: row.get(1).map_err(|e| e.to_string())?,
          role: Role::from_str(&role_str),
          content: row.get(3).map_err(|e| e.to_string())?,
          timestamp: row.get(4).map_err(|e| e.to_string())?,
          attachments: Vec::new(),
        });
      }

      if let Some(attachment_id) = row.get::<_, Option<String>>(5).map_err(|e| e.to_string())? {
        if let Some(msg) = messages.last_mut() {
          msg.attachments.push(Attachment {
            id: attachment_id,
            message_id: row.get(6).map_err(|e| e.to_string())?,
            file_type: row.get(7).map_err(|e| e.to_string())?,
            file_name: row.get(8).map_err(|e| e.to_string())?,
            file_path: row.get(9).map_err(|e| e.to_string())?,
            extracted_text: row.get(10).map_err(|e| e.to_string())?,
            created_at: row.get(11).map_err(|e| e.to_string())?,
          });
        }
      }
    }

  Ok(messages)
}

/// Get a message by its id
#[tauri::command]
pub async fn get_message(app_handle: AppHandle, message_id: String) -> Result<Message, String> {
  let state = app_handle.state::<DbState>();
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;

  let mut stmt = conn
    .prepare(
      "SELECT 
        m.id, m.conversation_id, m.role, m.content, m.timestamp,
        a.id, a.message_id, a.file_type, a.file_name, a.file_path, a.extracted_text, a.created_at
        FROM conversation_messages m 
        LEFT JOIN attachments a ON m.id = a.message_id 
        WHERE m.id = ?1",
    )
    .map_err(|e| format!("Failed to prepare statement: {}", e))?;

  let mut rows = stmt
    .query(params![message_id])
    .map_err(|e| format!("Failed to query message: {}", e))?;

  let mut message_acc: Option<Message> = None;

  while let Some(row) = rows.next().map_err(|e| e.to_string())? {
    if message_acc.is_none() {
      let role_str: String = row.get(2).map_err(|e| e.to_string())?;
      message_acc = Some(Message {
        id: row.get(0).map_err(|e| e.to_string())?,
        conversation_id: row.get(1).map_err(|e| e.to_string())?,
        role: Role::from_str(&role_str),
        content: row.get(3).map_err(|e| e.to_string())?,
        timestamp: row.get(4).map_err(|e| e.to_string())?,
        attachments: Vec::new(),
      });
    }

    if let Some(ref mut msg) = message_acc {
      if let Some(attachment_id) = row.get::<_, Option<String>>(5).map_err(|e| e.to_string())? {
        msg.attachments.push(Attachment {
          id: attachment_id,
          message_id: row.get(6).map_err(|e| e.to_string())?,
          file_type: row.get(7).map_err(|e| e.to_string())?,
          file_name: row.get(8).map_err(|e| e.to_string())?,
          file_path: row.get(9).map_err(|e| e.to_string())?,
          extracted_text: row.get(10).map_err(|e| e.to_string())?,
          created_at: row.get(11).map_err(|e| e.to_string())?,
        });
      }
    }
  }

  let message = message_acc.ok_or_else(|| format!("Message not found: {}", message_id))?;

  Ok(message)
}

/// Get a conversation by ID
#[tauri::command]
pub async fn get_conversation(
  app_handle: AppHandle,
  conversation_id: String,
) -> Result<Conversation, String> {
  let state = app_handle.state::<DbState>();
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;

  let conversation = conn
    .query_row(
      "SELECT id, name, conv_type, created_at, updated_at, message_count FROM conversations WHERE id = ?1",
      params![conversation_id],
      |row| {
        let created_at: String = row.get(3)?;
        let updated_at: String = row.get(4)?;

        Ok(Conversation {
          id: row.get(0)?,
          name: row.get(1)?,
          conv_type: row.get(2)?,
          created_at,
          updated_at,
          message_count: row.get(5)?,
        })
      },
    )
    .map_err(|e| format!("Failed to get conversation: {}", e))?;

  Ok(conversation)
}

/// List all conversations
#[tauri::command]
pub async fn list_conversations(
  app_handle: AppHandle,
  limit: usize,
  offset: usize,
) -> Result<Vec<Conversation>, String> {
  log::info!(
    "[conversations] Listing conversations with limit {} and offset {}",
    limit,
    offset
  );
  let state = app_handle.state::<DbState>();
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;

  let mut stmt = conn
    .prepare(
      "SELECT id, name, conv_type, created_at, updated_at, message_count 
         FROM conversations 
         ORDER BY updated_at DESC
         LIMIT ?1 OFFSET ?2",
    )
    .map_err(|e| format!("Failed to prepare statement: {}", e))?;

  let conversations = stmt
    .query_map(params![limit, offset], |row| {
      let created_at: String = row.get(3)?;
      let updated_at: String = row.get(4)?;

      Ok(Conversation {
        id: row.get(0)?,
        name: row.get(1)?,
        conv_type: row.get(2)?,
        created_at,
        updated_at,
        message_count: row.get(5)?,
      })
    })
    .map_err(|e| format!("Failed to query conversations: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Failed to collect conversations: {}", e))?;

  Ok(conversations)
}

/// Delete a conversation completely
#[tauri::command]
pub async fn delete_conversation(
  app_handle: AppHandle,
  conversation_id: String,
) -> Result<(), String> {
  let state = app_handle.state::<DbState>();
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;

  // Ensure conversation exists
  let _conversation_exists: String = conn
    .query_row(
      "SELECT id FROM conversations WHERE id = ?1",
      params![conversation_id],
      |row| row.get(0),
    )
    .map_err(|_| format!("Conversation not found: {}", conversation_id))?;

  // Clean up attachments associated with messages in the conversation
  let mut stmt = conn
    .prepare(
      "SELECT a.file_path 
         FROM attachments a
         JOIN conversation_messages m ON a.message_id = m.id
         WHERE m.conversation_id = ?1",
    )
    .map_err(|e| format!("Failed to prepare statement: {}", e))?;

  let attachment_paths = stmt
    .query_map(params![conversation_id], |row| row.get::<_, Option<String>>(0))
    .map_err(|e| format!("Failed to query attachment paths: {}", e))?;
  // Create set of parent dirs to delete later
  let mut parent_dirs = std::collections::HashSet::new();
  for path_result in attachment_paths {
    if let Ok(Some(file_path)) = path_result.map(|p| p) {
      let full_path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {}", e))?
        .join(file_path);
      if full_path.exists() {
        std::fs::remove_file(&full_path)
          .map_err(|e| format!("Failed to delete attachment file: {}", e))?;
        if let Some(parent) = full_path.parent() {
          parent_dirs.insert(parent.to_path_buf());
        }
      }
    }
  }
  for dir in parent_dirs {
    if dir.exists() {
      std::fs::remove_dir_all(&dir)
        .map_err(|e| format!("Failed to delete attachment directory: {}", e))?;
    }
  }

  // Delete conversation
  conn
    .execute(
      "DELETE FROM conversations WHERE id = ?1",
      params![conversation_id],
    )
    .map_err(|e| format!("Failed to delete conversation: {}", e))?;

  log::info!("[conversations] Deleted conversation: {}", conversation_id);
  Ok(())
}

/// Update conversation name
#[tauri::command]
pub async fn update_conversation_name(
  app_handle: AppHandle,
  conversation_id: String,
  name: String,
) -> Result<(), String> {
  let state = app_handle.state::<DbState>();
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;

  let now = Utc::now();
  conn
    .execute(
      "UPDATE conversations SET name = ?1, updated_at = ?2 WHERE id = ?3",
      params![name, now.to_rfc3339(), conversation_id],
    )
    .map_err(|e| format!("Failed to update conversation name: {}", e))?;

  log::info!(
    "[conversations] Updated conversation name: {}",
    conversation_id
  );
  Ok(())
}

/// Create attachments and save to disk
pub async fn create_attachments(
  app_handle: &AppHandle,
  message_id: String,
  attachment_data: Vec<AttachmentData>,
) -> Result<Vec<Attachment>, String> {
  let mut attachments = Vec::new();
  let now = Utc::now();

  for data in attachment_data {
    let attachment_id = Uuid::new_v4().to_string();
    let file_path = match data.file_type.as_str() {
      "ambient/ocr" => None,
      _ => Some(format!(
      "attachments/{}/{}",
      message_id,
      data.name.replace("/", "_")
      )),
    };

    // Save to disk if not ocr
    if data.file_type != "ambient/ocr" {
      let full_path = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {}", e))?
        .join(file_path.as_ref().unwrap());
      if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent)
          .map_err(|e| format!("Failed to create attachment directory: {}", e))?;
      }
      
      let base64_data = if data.data.contains(",") {
        data.data.split(",").nth(1).unwrap_or(&data.data)
      } else {
        &data.data
      };

      let decoded_data = general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| format!("Failed to decode attachment data: {}", e))?;
      std::fs::write(&full_path, decoded_data)
        .map_err(|e| format!("Failed to write attachment file: {}", e))?;
    }

    let extracted_text = match data.file_type.as_str() {
      "ambient/ocr" => Some(data.data.clone()),
      _ => None,
    };

    let attachment = Attachment {
      id: attachment_id.clone(),
      message_id: message_id.clone(),
      file_type: data.file_type.clone(),
      file_name: data.name.clone(),
      file_path: file_path.clone(),
      extracted_text: extracted_text,
      created_at: now.to_rfc3339(),
    };
    attachments.push(attachment);
  }

  log::info!(
    "[conversations] Created {} attachments for message: {}",
    attachments.len(),
    message_id
  );

  Ok(attachments)
}

/// Add multiple attachments to a message
pub async fn add_attachments(
  app_handle: &AppHandle,
  message_id: String,
  attachments: Vec<Attachment>,
) -> Result<Vec<Attachment>, String> {
  let state = app_handle.state::<DbState>();
  let mut conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn: &mut Connection = conn_guard
    .as_mut()
    .ok_or("Database connection not available.".to_string())?;

  let now = Utc::now();
  let mut created_attachments = Vec::new();

  // Use a transaction for batch insertion
  let tx = conn
    .transaction()
    .map_err(|e| format!("Failed to start transaction: {}", e))?;

  for mut attachment in attachments {
    let attachment_id = Uuid::new_v4().to_string();
    attachment.id = attachment_id.clone();
    attachment.message_id = message_id.clone();
    attachment.created_at = now.to_rfc3339();

    tx.execute(
      "INSERT INTO attachments (id, message_id, file_type, file_name, file_path, extracted_text, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
      params![
        attachment.id,
        attachment.message_id,
        attachment.file_type,
        attachment.file_name,
        attachment.file_path,
        attachment.extracted_text,
        attachment.created_at
      ],
    )
    .map_err(|e| format!("Failed to add attachment {}: {}", attachment.file_name, e))?;

    created_attachments.push(attachment);
  }

  tx.commit()
    .map_err(|e| format!("Failed to commit transaction: {}", e))?;

  log::info!(
    "[conversations] Added {} attachments to message: {}",
    created_attachments.len(),
    message_id
  );
  Ok(created_attachments)
}