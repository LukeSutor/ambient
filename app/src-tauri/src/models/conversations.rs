//! Simple Conversation Management System
//!
//! This module provides basic conversation management with persistent storage.
//!
//! ## Features
//! - Create new conversations
//! - Load existing conversations  
//! - Reset conversation messages
//! - Delete conversations
//! - Add messages to conversations
//! - Auto-generated conversation names
//!
//! ## Usage Examples (TypeScript)
//!
//! ```typescript
//! import { invoke } from '@tauri-apps/api/core';
//!
//! // Create a new conversation
//! const conversation = await invoke('create_conversation', {
//!   name: "My Chat" // Optional
//! });
//!
//! // Add a message
//! await invoke('add_message', {
//!   conversationId: conversation.id,
//!   role: "user",
//!   content: "Hello!"
//! });
//!
//! // Get messages
//! const messages = await invoke('get_messages', {
//!   conversationId: conversation.id
//! });
//!
//! // List conversations
//! const conversations = await invoke('list_conversations');
//!
//! // Reset conversation (clear all messages)
//! await invoke('reset_conversation', {
//!   conversationId: conversation.id
//! });
//!
//! // Delete conversation
//! await invoke('delete_conversation', {
//!   conversationId: conversation.id
//! });
//! ```

use crate::db::DbState;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

/// Message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// Conversation structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: i32,
}

/// Initialize the conversations database tables
pub fn initialize_conversations_db(conn: &Connection) -> SqliteResult<()> {
    // Create conversations table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            message_count INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )?;

    // Create messages table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversation_messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations (id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create indexes for performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON conversation_messages(conversation_id)",
        [],
    )?;
    
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON conversation_messages(timestamp)",
        [],
    )?;

    println!("[conversations] Database tables initialized successfully");
    Ok(())
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
) -> Result<Conversation, String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let conversation_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let conversation_name = name.unwrap_or_else(|| format!("New Chat {}", now.format("%m/%d %H:%M")));

    let conversation = Conversation {
        id: conversation_id.clone(),
        name: conversation_name.clone(),
        created_at: now,
        updated_at: now,
        message_count: 0,
    };

    conn.execute(
        "INSERT INTO conversations (id, name, created_at, updated_at, message_count)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            conversation_id,
            conversation_name,
            now.to_rfc3339(),
            now.to_rfc3339(),
            0
        ],
    ).map_err(|e| format!("Failed to create conversation: {}", e))?;

    println!("[conversations] Created conversation: {} ({})", conversation_name, conversation_id);
    Ok(conversation)
}

/// Add a message to a conversation
#[tauri::command]
pub async fn add_message(
    app_handle: AppHandle,
    conversation_id: String,
    role: String,
    content: String,
) -> Result<Message, String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let message_id = Uuid::new_v4().to_string();
    let now = Utc::now();

    let message = Message {
        id: message_id.clone(),
        conversation_id: conversation_id.clone(),
        role: role.clone(),
        content: content.clone(),
        timestamp: now,
    };

    // Insert the message
    conn.execute(
        "INSERT INTO conversation_messages (id, conversation_id, role, content, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![message_id, conversation_id, role, content, now.to_rfc3339()],
    ).map_err(|e| format!("Failed to add message: {}", e))?;

    // Update conversation
    conn.execute(
        "UPDATE conversations SET message_count = message_count + 1, updated_at = ?1 WHERE id = ?2",
        params![now.to_rfc3339(), conversation_id],
    ).map_err(|e| format!("Failed to update conversation: {}", e))?;

    // Auto-update conversation name if it's the first user message
    if role == "user" {
        let message_count: i32 = conn.query_row(
            "SELECT message_count FROM conversations WHERE id = ?1",
            params![conversation_id],
            |row| row.get(0),
        ).unwrap_or(0);

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
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, role, content, timestamp 
         FROM conversation_messages 
         WHERE conversation_id = ?1 
         ORDER BY timestamp ASC"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let messages = stmt.query_map(params![conversation_id], |row| {
        let timestamp_str: String = row.get(4)?;
        let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(4, "timestamp".to_string(), rusqlite::types::Type::Text))?
            .with_timezone(&Utc);

        Ok(Message {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            timestamp,
        })
    }).map_err(|e| format!("Failed to query messages: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Failed to collect messages: {}", e))?;

    Ok(messages)
}

/// Get a conversation by ID
#[tauri::command]
pub async fn get_conversation(
    app_handle: AppHandle,
    conversation_id: String,
) -> Result<Conversation, String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let conversation = conn.query_row(
        "SELECT id, name, created_at, updated_at, message_count FROM conversations WHERE id = ?1",
        params![conversation_id],
        |row| {
            let created_at_str: String = row.get(2)?;
            let updated_at_str: String = row.get(3)?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(2, "created_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|_| rusqlite::Error::InvalidColumnType(3, "updated_at".to_string(), rusqlite::types::Type::Text))?
                .with_timezone(&Utc);

            Ok(Conversation {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at,
                updated_at,
                message_count: row.get(4)?,
            })
        }
    ).map_err(|e| format!("Failed to get conversation: {}", e))?;

    Ok(conversation)
}

/// List all conversations
#[tauri::command]
pub async fn list_conversations(app_handle: AppHandle) -> Result<Vec<Conversation>, String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let mut stmt = conn.prepare(
        "SELECT id, name, created_at, updated_at, message_count 
         FROM conversations 
         ORDER BY updated_at DESC"
    ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let conversations = stmt.query_map([], |row| {
        let created_at_str: String = row.get(2)?;
        let updated_at_str: String = row.get(3)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(2, "created_at".to_string(), rusqlite::types::Type::Text))?
            .with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|_| rusqlite::Error::InvalidColumnType(3, "updated_at".to_string(), rusqlite::types::Type::Text))?
            .with_timezone(&Utc);

        Ok(Conversation {
            id: row.get(0)?,
            name: row.get(1)?,
            created_at,
            updated_at,
            message_count: row.get(4)?,
        })
    }).map_err(|e| format!("Failed to query conversations: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Failed to collect conversations: {}", e))?;

    Ok(conversations)
}

/// Reset a conversation (delete all messages)
#[tauri::command]
pub async fn reset_conversation(
    app_handle: AppHandle,
    conversation_id: String,
) -> Result<(), String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    // Delete all messages
    conn.execute(
        "DELETE FROM conversation_messages WHERE conversation_id = ?1",
        params![conversation_id],
    ).map_err(|e| format!("Failed to delete messages: {}", e))?;

    // Reset message count
    let now = Utc::now();
    conn.execute(
        "UPDATE conversations SET message_count = 0, updated_at = ?1 WHERE id = ?2",
        params![now.to_rfc3339(), conversation_id],
    ).map_err(|e| format!("Failed to reset conversation: {}", e))?;

    println!("[conversations] Reset conversation: {}", conversation_id);
    Ok(())
}

/// Delete a conversation completely
#[tauri::command]
pub async fn delete_conversation(
    app_handle: AppHandle,
    conversation_id: String,
) -> Result<(), String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    // Delete messages first (foreign key constraint)
    conn.execute(
        "DELETE FROM conversation_messages WHERE conversation_id = ?1",
        params![conversation_id],
    ).map_err(|e| format!("Failed to delete messages: {}", e))?;

    // Delete conversation
    conn.execute(
        "DELETE FROM conversations WHERE id = ?1",
        params![conversation_id],
    ).map_err(|e| format!("Failed to delete conversation: {}", e))?;

    println!("[conversations] Deleted conversation: {}", conversation_id);
    Ok(())
}

/// Update conversation name (optional utility)
#[tauri::command]
pub async fn update_conversation_name(
    app_handle: AppHandle,
    conversation_id: String,
    name: String,
) -> Result<(), String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let now = Utc::now();
    conn.execute(
        "UPDATE conversations SET name = ?1, updated_at = ?2 WHERE id = ?3",
        params![name, now.to_rfc3339(), conversation_id],
    ).map_err(|e| format!("Failed to update conversation name: {}", e))?;

    println!("[conversations] Updated conversation name: {}", conversation_id);
    Ok(())
}
