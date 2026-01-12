use crate::db::core::DbState;
use tauri::State;

/// Inserts a new workflow record into the workflows table.
#[tauri::command]
pub fn insert_workflow(
  state: State<DbState>,
  name: String,
  description: Option<String>,
  url: String,
  steps_json: String,
  recording_start: i64,
  recording_end: i64,
  last_updated: i64,
) -> Result<(), String> {
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;
  let sql = r#"INSERT INTO workflows (name, description, url, steps_json, recording_start, recording_end, last_updated) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#;
  conn
    .execute(
      sql,
      rusqlite::params![
        name,
        description,
        url,
        steps_json,
        recording_start,
        recording_end,
        last_updated
      ],
    )
    .map_err(|e| format!("Failed to insert workflow: {}", e))?;
  Ok(())
}

/// Retrieves workflows from the database with pagination.
#[tauri::command]
pub fn get_workflows(
  state: State<DbState>,
  offset: u32,
  limit: u32,
) -> Result<serde_json::Value, String> {
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;
  let sql = r#"SELECT id, name, description, url, steps_json, recording_start, recording_end, last_updated FROM workflows ORDER BY last_updated DESC, id DESC LIMIT ?1 OFFSET ?2"#;
  let mut stmt = conn
    .prepare(sql)
    .map_err(|e| format!("Prepare failed: {}", e))?;
  let rows = stmt
    .query_map(rusqlite::params![limit, offset], |row| {
      let mut map = serde_json::Map::new();
      map.insert("id".to_string(), serde_json::json!(row.get::<_, i64>(0)?));
      map.insert(
        "name".to_string(),
        serde_json::json!(row.get::<_, String>(1)?),
      );
      map.insert(
        "description".to_string(),
        serde_json::json!(row.get::<_, Option<String>>(2)?),
      );
      map.insert(
        "url".to_string(),
        serde_json::json!(row.get::<_, Option<String>>(3)?),
      );
      map.insert(
        "steps_json".to_string(),
        serde_json::json!(row.get::<_, String>(4)?),
      );
      map.insert(
        "recording_start".to_string(),
        serde_json::json!(row.get::<_, i64>(5)?),
      );
      map.insert(
        "recording_end".to_string(),
        serde_json::json!(row.get::<_, i64>(6)?),
      );
      map.insert(
        "last_updated".to_string(),
        serde_json::json!(row.get::<_, i64>(7)?),
      );
      Ok(serde_json::Value::Object(map))
    })
    .map_err(|e| format!("Query map failed: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Row processing failed: {}", e))?;
  Ok(serde_json::Value::Array(rows))
}

/// Deletes a workflow from the database by its ID.
#[tauri::command]
pub fn delete_workflow(state: State<DbState>, id: i64) -> Result<(), String> {
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;
  let sql = "DELETE FROM workflows WHERE id = ?1";
  let affected = conn
    .execute(sql, rusqlite::params![id])
    .map_err(|e| format!("Failed to delete workflow: {}", e))?;
  if affected == 0 {
    Err(format!("No workflow found with id {}", id))
  } else {
    Ok(())
  }
}

/// Retrieves a workflow from the database by its ID.
pub fn get_workflow_by_id(state: State<DbState>, id: i64) -> Result<serde_json::Value, String> {
  use rusqlite::params;
  use serde_json::json;
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;
  let mut stmt = conn.prepare("SELECT id, name, description, url, steps_json, recording_start, recording_end, last_updated FROM workflows WHERE id = ?1").map_err(|e| format!("Prepare failed: {}", e))?;
  let wf = stmt
    .query_row(params![id], |row| {
      Ok(json!({
        "id": row.get::<_, i64>(0)?,
        "name": row.get::<_, Option<String>>(1)?,
        "description": row.get::<_, Option<String>>(2)?,
        "url": row.get::<_, String>(3)?,
        "steps_json": row.get::<_, String>(4)?,
        "recording_start": row.get::<_, i64>(5)?,
        "recording_end": row.get::<_, Option<i64>>(6)?,
        "last_updated": row.get::<_, i64>(7)?,
      }))
    })
    .map_err(|e| format!("Workflow not found: {}", e))?;
  Ok(wf)
}
