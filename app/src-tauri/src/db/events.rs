use crate::db::core::DbState;
use tauri::State;
use zerocopy::IntoBytes;

/// Retrieves events from the database with pagination.
#[tauri::command]
pub fn get_events(
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
  let sql = r#"SELECT id, timestamp, application, description FROM events ORDER BY timestamp DESC, id DESC LIMIT ?1 OFFSET ?2"#;
  let mut stmt = conn
    .prepare(sql)
    .map_err(|e| format!("Prepare failed: {}", e))?;
  let rows = stmt
    .query_map(rusqlite::params![limit, offset], |row| {
      let mut map = serde_json::Map::new();
      map.insert("id".to_string(), serde_json::json!(row.get::<_, i64>(0)?));
      map.insert(
        "timestamp".to_string(),
        serde_json::json!(row.get::<_, i64>(1)?),
      );
      map.insert(
        "application".to_string(),
        serde_json::json!(row.get::<_, String>(2)?),
      );
      map.insert(
        "description".to_string(),
        serde_json::json!(row.get::<_, Option<String>>(3)?),
      );
      Ok(serde_json::Value::Object(map))
    })
    .map_err(|e| format!("Query map failed: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Row processing failed: {}", e))?;
  Ok(serde_json::Value::Array(rows))
}

/// Inserts a new event record into the events table.
#[tauri::command]
pub fn insert_event(
  state: State<DbState>,
  timestamp: i64,
  application: String,
  description: Option<String>,
  description_embedding: Vec<f32>,
) -> Result<(), String> {
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;
  let sql = r#"INSERT INTO events (timestamp, application, description, description_embedding) VALUES (?1, ?2, ?3, ?4)"#;
  conn
    .execute(
      sql,
      rusqlite::params![
        timestamp,
        application,
        description,
        (&description_embedding).as_bytes()
      ],
    )
    .map_err(|e| format!("Failed to insert event: {}", e))?;
  Ok(())
}
