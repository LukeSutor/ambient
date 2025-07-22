// filepath: c:\Users\Luke\Desktop\coding\local-computer-use\app\src-tauri\src\db.rs
use once_cell::sync::Lazy;
use rusqlite::types::{Value as RusqliteValue, ValueRef};
use rusqlite::{
  ffi::sqlite3_auto_extension, params_from_iter, Connection, Result as RusqliteResult,
};
use rusqlite_migration::{Migrations, M};
use serde_json::Value as JsonValue;
use sqlite_vec::sqlite3_vec_init;
use std::fs;
use std::sync::Mutex;
use tauri::Manager;
use zerocopy::IntoBytes;

pub struct DbState(pub Mutex<Option<Connection>>);

pub static GLOBAL_DB_STATE: Lazy<DbState> = Lazy::new(|| DbState(Mutex::new(None)));

lazy_static::lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
            M::up("
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
                    frequency TEXT DEFAULT 'one_time', -- one_time, daily, weekly, bi_weekly, monthly, quarterly, yearly, custom_N
                    last_completed_at TEXT, -- When the task was last completed
                    first_scheduled_at TEXT NOT NULL DEFAULT (datetime('now')), -- When the task was first scheduled
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
                    reasoning TEXT, -- LLM's reasoning for the decision
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
                    summary TEXT NOT NULL, -- Compressed summary of current user activity
                    active_url TEXT, -- Current active browser URL if applicable
                    active_applications TEXT, -- JSON array of active application names
                    created_at TEXT NOT NULL DEFAULT (datetime('now'))
                );

                -- Indexes for activity summaries (optimized for recent queries)
                CREATE INDEX IF NOT EXISTS idx_activity_summaries_created_at ON activity_summaries(created_at DESC);
            ")
        ]);
}

/// Initializes the SQLite database connection, registers extensions, and runs migrations.
pub fn initialize_database(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
  // 1. Resolve the database path
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
  fs::create_dir_all(&app_data_path)
    .map_err(|e| format!("Failed to create app data directory: {}", e))?;
  let db_path = app_data_path.join("sqlite.db");
  println!("[db] Database path: {:?}", db_path);

  // 2. Register the sqlite_vec extension
  unsafe {
    match sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ()))) {
      rusqlite::ffi::SQLITE_OK => {
        println!("[db] Successfully registered sqlite_vec extension.");
      }
      err_code => {
        return Err(format!(
          "Failed to register sqlite_vec extension. SQLite error code: {}",
          err_code
        ));
      }
    }
  }

  // 3. Open the database connection
  let mut conn = Connection::open(&db_path).map_err(|e| {
    println!("[db] Failed to open database connection: {}", e);
    format!("Failed to open database connection: {}", e)
  })?;

  // 4. Apply migrations
  println!("[db] Applying database migrations...");
  MIGRATIONS.to_latest(&mut conn).map_err(|e| {
    // Provide more context on migration errors
    let err_msg = match e {
      // Updated pattern matching for the struct variant
      rusqlite_migration::Error::RusqliteError { query: _, err } => {
        format!("SQLite error during migration: {}", err)
      }
      rusqlite_migration::Error::MigrationDefinition(def_err) => {
        format!("Migration definition error: {}", def_err)
      }
      // Add other rusqlite_migration::Error variants if needed
      _ => format!("Unknown migration error: {}", e),
    };
    println!("[db] Migration failed: {}", err_msg);
    err_msg
  })?;
  println!("[db] Migrations applied successfully.");

  // 5. Return the connection
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
    } // Handle potential NaN/Infinity
    ValueRef::Text(t_bytes) => JsonValue::String(String::from_utf8_lossy(t_bytes).to_string()),
    ValueRef::Blob(b) => JsonValue::String(format!("Blob({} bytes)", b.len())), // Represent blob as string placeholder
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
    // Blobs from JSON are tricky; require specific format like base64 string if needed
    // JsonValue::Array(_) => Err("Arrays not directly supported as parameters".to_string()),
    // JsonValue::Object(_) => Err("Objects not directly supported as parameters".to_string()),
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
  params: Option<Vec<JsonValue>>, // Accept parameters as JSON array
) -> Result<JsonValue, String> {
  println!("[db] Executing SQL: {}", sql);
  if let Some(p) = &params {
    println!("[db] With params: {:?}", p);
  }

  let maybe_conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;

  if let Some(conn) = maybe_conn_guard.as_ref() {
    // Convert JSON params to rusqlite params
    let rusqlite_params: Vec<RusqliteValue> = match params {
      Some(json_params) => json_params
        .iter()
        .map(json_to_rusqlite)
        .collect::<Result<Vec<_>, _>>()?,
      None => Vec::new(),
    };

    // Check if it's a SELECT query (simple check)
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

      // Correctly map the Vec<Map> to JsonValue::Array
      results
        .map(|vec_of_maps| {
          // Convert each Map into a JsonValue::Object
          let json_values: Vec<JsonValue> =
            vec_of_maps.into_iter().map(JsonValue::Object).collect();
          // Wrap the Vec<JsonValue> in JsonValue::Array
          JsonValue::Array(json_values)
        })
        .map_err(|e| format!("Row processing failed: {}", e))
    } else {
      // For INSERT, UPDATE, DELETE, etc.
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
/// Returns events in descending order of timestamp (most recent first).
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
/// - `state`: The database state.
/// - `timestamp`: Unix epoch seconds (UTC).
/// - `application`: The application name (e.g., "Code.exe").
/// - `description`: Description of the event.
/// - `description_embedding`: Embedding vector as Vec<f32>.
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

  // Store the embedding as a BLOB using the sqlite-vec extension (f32 array)
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
        (&description_embedding).as_bytes(), // Use reference for AsBytes
      ],
    )
    .map_err(|e| format!("Failed to insert event: {}", e))?;

  Ok(())
}

/// Inserts a new workflow record into the workflows table.
/// - `state`: The database state.
/// - `name`: Name of the workflow.
/// - `description`: Optional description.
/// - `url`: url of the website string.
/// - `steps_json`: Steps as a JSON string.
/// - `recording_start`: Unix epoch seconds (UTC) when recording started.
/// - `recording_end`: Unix epoch seconds (UTC) when recording ended.
/// - `last_updated`: Unix epoch seconds (UTC) when last updated.
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
/// Returns workflows in descending order of last_updated (most recent first).
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

/// Closes the current database connection, deletes the database file,
/// and initializes a fresh database.
#[tauri::command]
pub fn reset_database(
  state: tauri::State<'_, DbState>,
  app_handle: tauri::AppHandle,
) -> Result<(), String> {
  println!("[db] Attempting to reset database...");

  // 1. Resolve the database path (same logic as initialize_database)
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
  let db_path = app_data_path.join("sqlite.db");
  println!("[db] Target database path for reset: {:?}", db_path);

  // 2. Lock the state and take the connection (this closes it when `_conn_guard` drops)
  let mut conn_guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  let old_conn = conn_guard.take(); // Takes the Option<Connection>, leaving None

  // Explicitly drop the old connection if it exists
  if let Some(conn) = old_conn {
    drop(conn); // Ensure connection is closed before deleting file
    println!("[db] Closed existing database connection.");
  } else {
    println!("[db] No existing database connection found in state.");
  }

  // 3. Delete the database file
  if db_path.exists() {
    println!("[db] Deleting database file: {:?}", db_path);
    fs::remove_file(&db_path)
      .map_err(|e| format!("Failed to delete database file {:?}: {}", db_path, e))?;
    println!("[db] Database file deleted successfully.");
  } else {
    println!("[db] Database file not found, skipping deletion.");
  }

  // 4. Re-initialize the database
  println!("[db] Re-initializing database...");
  match initialize_database(&app_handle) {
    Ok(new_conn) => {
      // 5. Store the new connection in the state
      *conn_guard = Some(new_conn);
      println!("[db] Database reset and re-initialized successfully.");
      Ok(())
    }
    Err(e) => {
      // If initialization fails, the state remains None
      println!("[db] Failed to re-initialize database: {}", e);
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
  let mut stmt = conn.prepare("SELECT id, name, description, url, steps_json, recording_start, recording_end, last_updated FROM workflows WHERE id = ?1")
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
    active_applications: Option<String>, // JSON string of application names
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
        .execute(
            sql,
            rusqlite::params![summary, active_url, active_applications],
        )
        .map_err(|e| format!("Failed to insert activity summary: {}", e))?;

    // Return the ID of the inserted row
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

    let result = stmt
        .query_row([], |row| {
            let mut map = serde_json::Map::new();
            map.insert("id".to_string(), serde_json::json!(row.get::<_, i64>(0)?));
            map.insert("summary".to_string(), serde_json::json!(row.get::<_, String>(1)?));
            map.insert("active_url".to_string(), serde_json::json!(row.get::<_, Option<String>>(2)?));
            map.insert("active_applications".to_string(), serde_json::json!(row.get::<_, Option<String>>(3)?));
            map.insert("created_at".to_string(), serde_json::json!(row.get::<_, String>(4)?));
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
            map.insert("summary".to_string(), serde_json::json!(row.get::<_, String>(1)?));
            map.insert("active_url".to_string(), serde_json::json!(row.get::<_, Option<String>>(2)?));
            map.insert("active_applications".to_string(), serde_json::json!(row.get::<_, Option<String>>(3)?));
            map.insert("created_at".to_string(), serde_json::json!(row.get::<_, String>(4)?));
            Ok(serde_json::Value::Object(map))
        })
        .map_err(|e| format!("Query map failed: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row processing failed: {}", e))?;

    Ok(serde_json::Value::Array(rows))
}
