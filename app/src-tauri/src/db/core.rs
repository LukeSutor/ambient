use once_cell::sync::Lazy;
use rusqlite::types::{Value as RusqliteValue, ValueRef};
use rusqlite::{
  ffi::sqlite3_auto_extension, params_from_iter, Connection, Result as RusqliteResult,
};
use rusqlite_migration::{Migrations, M};
use serde_json::Value as JsonValue;
use sqlite_vec::sqlite3_vec_init;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::Manager;

pub struct DbState(pub Mutex<Option<Connection>>);

// Database schema migrations
static MIGRATIONS: Lazy<Migrations<'static>> = Lazy::new(|| {
  Migrations::new(vec![M::up(
    r#"
        -- Conversation tables
        CREATE TABLE IF NOT EXISTS conversations (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          conv_type TEXT NOT NULL DEFAULT 'chat',
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

        -- Memory tables
        CREATE TABLE IF NOT EXISTS memory_entries (
          id TEXT PRIMARY KEY,
          message_id TEXT NOT NULL,
          memory_type TEXT NOT NULL,
          text TEXT NOT NULL,
          embedding BLOB NOT NULL,
          timestamp TEXT NOT NULL,
          FOREIGN KEY (message_id) REFERENCES conversation_messages(id) ON DELETE CASCADE
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS memory_entries_vec USING vec0(embedding float[768]);
        CREATE TABLE IF NOT EXISTS memory_entry_vec_map (
          memory_id TEXT UNIQUE NOT NULL,
          FOREIGN KEY(memory_id) REFERENCES memory_entries(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_memory_entries_timestamp ON memory_entries(timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_memory_entries_memory_type ON memory_entries(memory_type);
        CREATE INDEX IF NOT EXISTS idx_memory_entries_message_id ON memory_entries(message_id);

        -- Computer use sessions
        CREATE TABLE IF NOT EXISTS computer_use_sessions (
          id TEXT PRIMARY KEY,
          conversation_id TEXT NOT NULL UNIQUE,
          data TEXT NOT NULL,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY (conversation_id) REFERENCES conversations (id) ON DELETE CASCADE
        );

        -- Token usage tracking
        CREATE TABLE IF NOT EXISTS models (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          model TEXT NOT NULL UNIQUE
        );

        -- Insert default models
        INSERT OR IGNORE INTO models (model) VALUES
          ('local'),
          ('fast'),
          ('pro'),
          ('computer-use');

        CREATE TABLE IF NOT EXISTS token_usage (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          model INTEGER,
          prompt_tokens INTEGER NOT NULL,
          completion_tokens INTEGER NOT NULL,
          timestamp TEXT NOT NULL,
          FOREIGN KEY (model) REFERENCES models(id)
        );

        CREATE INDEX IF NOT EXISTS idx_token_usage_timestamp ON token_usage(timestamp DESC);
        CREATE INDEX IF NOT EXISTS idx_token_usage_model ON token_usage(model);

        -- Message attachments
        CREATE TABLE IF NOT EXISTS attachments (
          id TEXT PRIMARY KEY,
          message_id TEXT NOT NULL,
          file_type TEXT NOT NULL,
          file_name TEXT NOT NULL,
          file_path TEXT,
          extracted_text TEXT,
          created_at TEXT NOT NULL,
          FOREIGN KEY (message_id) REFERENCES conversation_messages(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_attachments_message_id ON attachments(message_id);
      "#,
  )])
});

fn get_db_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
  if let Err(e) = std::fs::create_dir_all(&app_data_path) {
    return Err(format!("Failed to create app data directory: {}", e));
  }
  Ok(app_data_path.join("database.sqlite"))
}

/// Initializes the SQLite database connection, registers extensions, and runs migrations.
pub fn initialize_database(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
  let db_path = get_db_path(app_handle)?;

  unsafe {
    let rc = sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    if rc != 0 {
      return Err(format!(
        "Failed to register sqlite_vec extension. SQLite error code: {}",
        rc
      ));
    }
  }
  log::info!("[db] Registered sqlite_vec extension");

  let mut conn =
    Connection::open(&db_path).map_err(|e| format!("Failed to open database connection: {}", e))?;

  log::info!("[db] Applying database migrations...");
  MIGRATIONS.to_latest(&mut conn).map_err(|e| match e {
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

/// Closes the current database connection, deletes the database file, and initializes a fresh database.
#[tauri::command]
pub fn reset_database(
  state: tauri::State<'_, DbState>,
  app_handle: tauri::AppHandle,
) -> Result<(), String> {
  log::info!("[db] Attempting to reset database...");

  let db_path = get_db_path(&app_handle)?;
  log::debug!("[db] Target database path for reset: {:?}", db_path);

  let mut conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let old_conn = conn_guard.take();
  drop(conn_guard);
  if let Some(conn) = old_conn {
    if let Err((_, e)) = conn.close() {
      log::warn!("[db] Error closing database connection: {}", e);
    }
    log::info!("[db] Closed existing database connection.");
  }

  log::info!("[db] Deleting database file: {:?}", db_path);
  match fs::remove_file(&db_path) {
    Ok(_) => log::info!("[db] Database file deleted successfully."),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
      log::debug!("[db] Database file not found, skipping deletion.")
    }
    Err(e) => return Err(format!("Failed to delete database file: {}", e)),
  }

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
