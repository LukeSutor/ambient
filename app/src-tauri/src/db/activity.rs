use crate::db::core::DbState;
use tauri::State;

/// Inserts a new activity summary into the database.
#[tauri::command]
pub fn insert_activity_summary(
  state: State<DbState>,
  summary: String,
  active_url: Option<String>,
  active_applications: Option<String>,
) -> Result<i64, String> {
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;
  let sql = r#"INSERT INTO activity_summaries (summary, active_url, active_applications) VALUES (?1, ?2, ?3)"#;
  conn
    .execute(
      sql,
      rusqlite::params![summary, active_url, active_applications],
    )
    .map_err(|e| format!("Failed to insert activity summary: {}", e))?;
  Ok(conn.last_insert_rowid())
}

/// Gets the most recent activity summary from the database.
pub fn get_latest_activity_summary(
  state: &State<DbState>,
) -> Result<Option<serde_json::Value>, String> {
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;
  let sql = r#"SELECT id, summary, active_url, active_applications, created_at FROM activity_summaries ORDER BY created_at DESC LIMIT 1"#;
  let mut stmt = conn
    .prepare(sql)
    .map_err(|e| format!("Prepare failed: {}", e))?;
  let result = stmt.query_row([], |row| {
    let mut map = serde_json::Map::new();
    map.insert("id".to_string(), serde_json::json!(row.get::<_, i64>(0)?));
    map.insert(
      "summary".to_string(),
      serde_json::json!(row.get::<_, String>(1)?),
    );
    map.insert(
      "active_url".to_string(),
      serde_json::json!(row.get::<_, Option<String>>(2)?),
    );
    map.insert(
      "active_applications".to_string(),
      serde_json::json!(row.get::<_, Option<String>>(3)?),
    );
    map.insert(
      "created_at".to_string(),
      serde_json::json!(row.get::<_, String>(4)?),
    );
    Ok(serde_json::Value::Object(map))
  });
  match result {
    Ok(summary) => Ok(Some(summary)),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
    Err(e) => Err(format!("Failed to get latest activity summary: {}", e)),
  }
}

/// Gets recent activity summaries with pagination.
#[tauri::command]
pub fn get_activity_summaries(
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
  let sql = r#"SELECT id, summary, active_url, active_applications, created_at FROM activity_summaries ORDER BY created_at DESC LIMIT ?1 OFFSET ?2"#;
  let mut stmt = conn
    .prepare(sql)
    .map_err(|e| format!("Prepare failed: {}", e))?;
  let rows = stmt
    .query_map(rusqlite::params![limit, offset], |row| {
      let mut map = serde_json::Map::new();
      map.insert("id".to_string(), serde_json::json!(row.get::<_, i64>(0)?));
      map.insert(
        "summary".to_string(),
        serde_json::json!(row.get::<_, String>(1)?),
      );
      map.insert(
        "active_url".to_string(),
        serde_json::json!(row.get::<_, Option<String>>(2)?),
      );
      map.insert(
        "active_applications".to_string(),
        serde_json::json!(row.get::<_, Option<String>>(3)?),
      );
      map.insert(
        "created_at".to_string(),
        serde_json::json!(row.get::<_, String>(4)?),
      );
      Ok(serde_json::Value::Object(map))
    })
    .map_err(|e| format!("Query map failed: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Row processing failed: {}", e))?;
  Ok(serde_json::Value::Array(rows))
}
