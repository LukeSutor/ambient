use crate::db::core::DbState;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerUseSession {
  pub id: String,
  pub conversation_id: String,
  pub data: Vec<serde_json::Value>,
  pub created_at: String,
  pub updated_at: String,
}

fn remove_screenshots(data: &mut Vec<serde_json::Value>) -> Vec<serde_json::Value> {
  for content in data.iter_mut() {
    if let Some(parts) = content.get_mut("parts").and_then(|p| p.as_array_mut()) {
      for part in parts.iter_mut() {
        if let Some(fr) = part.get_mut("functionResponse") {
          if let Some(fr_obj) = fr.as_object_mut() {
            fr_obj.remove("parts");
          }
        }
      }
    }
  }
  data.to_vec()
}

pub async fn save_computer_use_session(
  app_handle: AppHandle,
  conversation_id: String,
  data: Vec<serde_json::Value>,
) -> Result<(), String> {
  let state = app_handle.state::<DbState>();
  let db_guard = state.0.lock().unwrap();
  let conn = db_guard
    .as_ref()
    .ok_or("Database connection not available")?;

  let id = Uuid::new_v4().to_string();
  let now = Utc::now();

  // Remove screenshots from data before saving
  let data = remove_screenshots(&mut data.clone());

  conn.execute(
    "INSERT INTO computer_use_sessions (id, conversation_id, data, created_at, updated_at) 
     VALUES (?1, ?2, ?3, ?4, ?5)
     ON CONFLICT(conversation_id) DO UPDATE SET 
       data = excluded.data, 
       updated_at = excluded.updated_at",
    params![
      id,
      conversation_id,
      serde_json::to_string(&data).unwrap(),
      now.to_rfc3339(),
      now.to_rfc3339()
    ],
  ).map_err(|e| format!("Failed to save computer use session: {}", e))?;

  Ok(())
}

pub async fn get_computer_use_session(
  app_handle: AppHandle,
  conversation_id: String,
) -> Result<ComputerUseSession, String> {
  let state = app_handle.state::<DbState>();
  let db_guard = state.0.lock().unwrap();
  let conn = db_guard
    .as_ref()
    .ok_or("Database connection not available")?;

  let mut stmt = conn.prepare(
    "SELECT id, conversation_id, data, created_at, updated_at FROM computer_use_sessions WHERE conversation_id = ?1 LIMIT 1",
  ).map_err(|e| format!("Failed to prepare statement: {}", e))?;

  let mut session_iter = stmt.query_map(params![conversation_id], |row| {
    let data_str: String = row.get(2)?;
    let data: Vec<serde_json::Value> = serde_json::from_str(&data_str).map_err(|e| {
      rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
    })?;

    Ok(ComputerUseSession {
      id: row.get(0)?,
      conversation_id: row.get(1)?,
      data,
      created_at: row.get(3)?,
      updated_at: row.get(4)?,
    })
  }).map_err(|e| format!("Failed to get computer use session: {}", e))?;

  session_iter
    .next()
    .ok_or_else(|| "No session found".to_string())?
    .map_err(|e| format!("Failed to retrieve session: {}", e))
}
