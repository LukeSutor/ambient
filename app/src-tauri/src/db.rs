use once_cell::sync::Lazy;
use rusqlite::types::{Value as RusqliteValue, ValueRef};
use rusqlite::{
  ffi::sqlite3_auto_extension,
  params_from_iter,
  Connection,
  Result as RusqliteResult,
};
use rusqlite_migration::{Migrations, M};
use serde_json::Value as JsonValue;
use sqlite_vec::sqlite3_vec_init;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;
use zerocopy::IntoBytes;

pub struct DbState(pub Mutex<Option<Connection>>);

pub static GLOBAL_DB_STATE: Lazy<DbState> = Lazy::new(|| DbState(Mutex::new(None)));

static MIGRATIONS: Lazy<Migrations<'static>> = Lazy::new(|| {
  Migrations::new(vec![
    M::up(r#"
        -- Conversation tables
        CREATE TABLE IF NOT EXISTS conversations (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          message_count INTEGER NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS conversation_messages (
          id TEXT PRIMARY KEY,
          conversation_id TEXT NOT NULL,
          role TEXT NOT NULL,
          content TEXT NOT NULL,
          timestamp TEXT NOT NULL,
          FOREIGN KEY (conversation_id) REFERENCES conversations (id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON conversation_messages(conversation_id);
        CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON conversation_messages(timestamp);

        -- Task tracking
        CREATE TABLE IF NOT EXISTS tasks (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          description TEXT,
          category TEXT,
          priority INTEGER DEFAULT 1,
          frequency TEXT DEFAULT 'one_time',
          last_completed_at TEXT,
          first_scheduled_at TEXT NOT NULL DEFAULT (datetime('now')),
          created_at TEXT NOT NULL DEFAULT (datetime('now')),
          updated_at TEXT NOT NULL DEFAULT (datetime('now')),
          status TEXT DEFAULT 'pending' CHECK(status IN ('pending', 'completed'))
        );

        CREATE TABLE IF NOT EXISTS task_steps (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          task_id INTEGER NOT NULL,
          step_number INTEGER NOT NULL,
          title TEXT NOT NULL,
          description TEXT,
          status TEXT DEFAULT 'pending' CHECK(status IN ('pending', 'completed')),
          completed_at TEXT,
          FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
          UNIQUE(task_id, step_number)
        );

        CREATE TABLE IF NOT EXISTS task_progress (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          task_id INTEGER NOT NULL,
          step_id INTEGER,
          reasoning TEXT,
          timestamp TEXT NOT NULL DEFAULT (datetime('now')),
          FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
          FOREIGN KEY (step_id) REFERENCES task_steps(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_tasks_category ON tasks(category);
        CREATE INDEX IF NOT EXISTS idx_tasks_frequency ON tasks(frequency);
        CREATE INDEX IF NOT EXISTS idx_tasks_first_scheduled_at ON tasks(first_scheduled_at);
        CREATE INDEX IF NOT EXISTS idx_tasks_last_completed_at ON tasks(last_completed_at);
        CREATE INDEX IF NOT EXISTS idx_task_steps_task_id ON task_steps(task_id);
        CREATE INDEX IF NOT EXISTS idx_task_steps_status ON task_steps(status);
        CREATE INDEX IF NOT EXISTS idx_task_progress_task_id ON task_progress(task_id);
        CREATE INDEX IF NOT EXISTS idx_task_progress_timestamp ON task_progress(timestamp);

        -- User activity summaries
        CREATE TABLE IF NOT EXISTS activity_summaries (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          summary TEXT NOT NULL,
          active_url TEXT,
          active_applications TEXT,
          created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_activity_summaries_created_at ON activity_summaries(created_at DESC);

        -- Events (with embedding)
        CREATE TABLE IF NOT EXISTS events (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          timestamp INTEGER NOT NULL,
          application TEXT NOT NULL,
          description TEXT,
          description_embedding BLOB
        );
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_events_application ON events(application);

        -- Workflows
        CREATE TABLE IF NOT EXISTS workflows (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          name TEXT NOT NULL,
          description TEXT,
          url TEXT,
          steps_json TEXT NOT NULL,
          recording_start INTEGER NOT NULL,
          recording_end INTEGER,
          last_updated INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_workflows_last_updated ON workflows(last_updated DESC);
      "#)
  ])
});

fn get_db_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
  if let Err(e) = fs::create_dir_all(&app_data_path) {
    return Err(format!("Failed to create app data directory: {}", e));
  }
  Ok(app_data_path.join("database.sqlite"))
}

/// Initializes the SQLite database connection, registers extensions, and runs migrations.
pub fn initialize_database(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
  // Resolve path and log
  let db_path = get_db_path(app_handle)?;
  log::info!("[db] Database path: {:?}", db_path);

  // Register sqlite_vec extension globally for future connections
  unsafe {
    let rc = sqlite3_auto_extension(Some(std::mem::transmute(
      sqlite3_vec_init as *const (),
    )));
    if rc != 0 {
      return Err(format!(
        "Failed to register sqlite_vec extension. SQLite error code: {}",
        rc
      ));
    }
  }
  log::info!("[db] Registered sqlite_vec extension");

  // Open connection
  let mut conn = Connection::open(&db_path)
    .map_err(|e| format!("Failed to open database connection: {}", e))?;

  // Apply migrations
  log::info!("[db] Applying database migrations...");
  MIGRATIONS
    .to_latest(&mut conn)
    .map_err(|e| match e {
      rusqlite_migration::Error::RusqliteError { query: _, err } => {
        format!("SQLite error during migration: {}", err)
      }
      rusqlite_migration::Error::MigrationDefinition(def_err) => {
        format!("Migration definition error: {}", def_err)
      }
      other => format!("Unknown migration error: {}", other),
    })?;
  log::info!("[db] Migrations applied successfully.");

  Ok(conn)
}

// --- Database Commands ---

// Helper to convert rusqlite ValueRef to serde_json Value
fn rusqlite_to_json(value_ref: ValueRef) -> RusqliteResult<JsonValue> {
  Ok(match value_ref {
    ValueRef::Null => JsonValue::Null,
    ValueRef::Integer(i) => JsonValue::Number(i.into()),
    ValueRef::Real(f) => {
      JsonValue::Number(serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)))
    }
    ValueRef::Text(t_bytes) => JsonValue::String(String::from_utf8_lossy(t_bytes).to_string()),
    ValueRef::Blob(b) => JsonValue::String(format!("Blob({} bytes)", b.len())),
  })
}

// Helper to convert serde_json Value to rusqlite Value
fn json_to_rusqlite(json_value: &JsonValue) -> Result<RusqliteValue, String> {
  match json_value {
    JsonValue::Null => Ok(RusqliteValue::Null),
    JsonValue::Bool(b) => Ok(RusqliteValue::Integer(*b as i64)),
    JsonValue::Number(n) => {
      if let Some(i) = n.as_i64() {
        Ok(RusqliteValue::Integer(i))
      } else if let Some(f) = n.as_f64() {
        Ok(RusqliteValue::Real(f))
      } else {
        Err("Unsupported number type".to_string())
      }
    }
    JsonValue::String(s) => Ok(RusqliteValue::Text(s.clone())),
    _ => Err(format!(
      "Unsupported JSON type for parameter: {:?}",
      json_value
    )),
  }
}

/// Executes an arbitrary SQL command. For dev/debug purposes.
/// Returns query results as JSON for SELECT, or rows affected for others.
#[tauri::command]
pub fn execute_sql(
  state: tauri::State<DbState>,
  sql: String,
  params: Option<Vec<JsonValue>>,
) -> Result<serde_json::Value, String> {
  log::debug!("[db] Executing SQL: {}", sql);
  if let Some(p) = &params {
    log::debug!("[db] With params: {:?}", p);
  }

  let maybe_conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;

  if let Some(conn) = maybe_conn_guard.as_ref() {
    let rusqlite_params: Vec<RusqliteValue> = match params {
      Some(json_params) => json_params
        .iter()
        .map(json_to_rusqlite)
        .collect::<Result<Vec<_>, _>>()?,
      None => Vec::new(),
    };

    let is_select = sql.trim_start().to_lowercase().starts_with("select");
    if is_select {
      let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Prepare failed: {}", e))?;
      let column_names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();

      let results: Result<Vec<serde_json::Map<String, JsonValue>>, _> = stmt
        .query_map(params_from_iter(rusqlite_params.iter()), |row| {
          let mut map = serde_json::Map::new();
          for (i, col_name) in column_names.iter().enumerate() {
            let value_ref = row.get_ref_unwrap(i);
            let json_value = rusqlite_to_json(value_ref).map_err(|e| {
              rusqlite::Error::FromSqlConversionFailure(i, value_ref.data_type(), Box::new(e))
            })?;
            map.insert(col_name.clone(), json_value);
          }
          Ok(map)
        })
        .map_err(|e| format!("Query map failed: {}", e))?
        .collect();

      results
        .map(|vec_of_maps| {
          let json_values: Vec<JsonValue> =
            vec_of_maps.into_iter().map(JsonValue::Object).collect();
          JsonValue::Array(json_values)
        })
        .map_err(|e| format!("Row processing failed: {}", e))
    } else {
      let rows_affected = conn
        .execute(&sql, params_from_iter(rusqlite_params.iter()))
        .map_err(|e| format!("Execute failed: {}", e))?;
      Ok(serde_json::json!({ "rows_affected": rows_affected }))
    }
  } else {
    Err("Database connection not available.".to_string())
  }
}

/// Retrieves events from the database with pagination.
#[tauri::command]
pub fn get_events(
  state: tauri::State<DbState>,
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

  let sql = r#"
        SELECT id, timestamp, application, description
        FROM events
        ORDER BY timestamp DESC, id DESC
        LIMIT ?1 OFFSET ?2
    "#;

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
  state: tauri::State<DbState>,
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

  let sql = r#"
        INSERT INTO events (timestamp, application, description, description_embedding)
        VALUES (?1, ?2, ?3, ?4)
    "#;

  conn
    .execute(
      sql,
      rusqlite::params![
        timestamp,
        application,
        description,
        (&description_embedding).as_bytes(),
      ],
    )
    .map_err(|e| format!("Failed to insert event: {}", e))?;

  Ok(())
}

/// Inserts a new workflow record into the workflows table.
#[tauri::command]
pub fn insert_workflow(
  state: tauri::State<DbState>,
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

  let sql = r#"
        INSERT INTO workflows (name, description, url, steps_json, recording_start, recording_end, last_updated)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
    "#;

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
  state: tauri::State<DbState>,
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

  let sql = r#"
        SELECT id, name, description, url, steps_json, recording_start, recording_end, last_updated
        FROM workflows
        ORDER BY last_updated DESC, id DESC
        LIMIT ?1 OFFSET ?2
    "#;

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
pub fn delete_workflow(state: tauri::State<DbState>, id: i64) -> Result<(), String> {
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

/// Closes the current database connection, deletes the database file, and initializes a fresh database.
#[tauri::command]
pub fn reset_database(
  state: tauri::State<'_, DbState>,
  app_handle: tauri::AppHandle,
) -> Result<(), String> {
  log::info!("[db] Attempting to reset database...");

  let db_path = get_db_path(&app_handle)?;
  log::debug!("[db] Target database path for reset: {:?}", db_path);

  // Take and drop existing connection
  let mut conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let old_conn = conn_guard.take();
  drop(conn_guard);
  if old_conn.is_some() {
    log::info!("[db] Closed existing database connection.");
  } else {
    log::debug!("[db] No existing database connection found in state.");
  }

  // Delete file
  log::info!("[db] Deleting database file: {:?}", db_path);
  match fs::remove_file(&db_path) {
    Ok(_) => log::info!("[db] Database file deleted successfully."),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
      log::debug!("[db] Database file not found, skipping deletion.")
    }
    Err(e) => return Err(format!("Failed to delete database file: {}", e)),
  }

  // Re-init and store connection
  log::info!("[db] Re-initializing database...");
  match initialize_database(&app_handle) {
    Ok(new_conn) => {
      let mut guard = state
        .0
        .lock()
        .map_err(|_| "Failed to acquire DB lock".to_string())?;
      *guard = Some(new_conn);
      log::info!("[db] Database reset and re-initialized successfully.");
      Ok(())
    }
    Err(e) => {
      log::error!("[db] Failed to re-initialize database: {}", e);
      Err(format!("Failed to re-initialize database: {}", e))
    }
  }
}

/// Retrieves a workflow from the database by its ID.
pub fn get_workflow_by_id(
  state: tauri::State<DbState>,
  id: i64,
) -> Result<serde_json::Value, String> {
  use rusqlite::params;
  use serde_json::json;
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;
  let mut stmt = conn
    .prepare("SELECT id, name, description, url, steps_json, recording_start, recording_end, last_updated FROM workflows WHERE id = ?1")
    .map_err(|e| format!("Prepare failed: {}", e))?;
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

/// Inserts a new activity summary into the database.
#[tauri::command]
pub fn insert_activity_summary(
  state: tauri::State<DbState>,
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

  let sql = r#"
        INSERT INTO activity_summaries (summary, active_url, active_applications)
        VALUES (?1, ?2, ?3)
    "#;

  conn
    .execute(sql, rusqlite::params![summary, active_url, active_applications])
    .map_err(|e| format!("Failed to insert activity summary: {}", e))?;

  Ok(conn.last_insert_rowid())
}

/// Gets the most recent activity summary from the database.
pub fn get_latest_activity_summary(
  state: &tauri::State<DbState>,
) -> Result<Option<serde_json::Value>, String> {
  let conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let conn = conn_guard
    .as_ref()
    .ok_or("Database connection not available.".to_string())?;

  let sql = r#"
        SELECT id, summary, active_url, active_applications, created_at
        FROM activity_summaries
        ORDER BY created_at DESC
        LIMIT 1
    "#;

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
  state: tauri::State<DbState>,
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

  let sql = r#"
        SELECT id, summary, active_url, active_applications, created_at
        FROM activity_summaries
        ORDER BY created_at DESC
        LIMIT ?1 OFFSET ?2
    "#;

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
