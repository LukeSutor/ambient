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
  Migrations::new(vec![
    M::up(
      r#"
        -- Conversation tables
        CREATE TABLE IF NOT EXISTS conversations (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          conv_type TEXT NOT NULL DEFAULT 'chat',
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          message_count INTEGER NOT NULL DEFAULT 0,
          prompt_cached_at TEXT
        );

        CREATE TABLE IF NOT EXISTS conversation_messages (
          id TEXT PRIMARY KEY,
          conversation_id TEXT NOT NULL,
          role TEXT NOT NULL,
          content TEXT NOT NULL,
          timestamp TEXT NOT NULL,
          message_type TEXT NOT NULL DEFAULT 'text',
          metadata TEXT,
          FOREIGN KEY (conversation_id) REFERENCES conversations (id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_conversation_id ON conversation_messages(conversation_id);
        CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON conversation_messages(timestamp);
        CREATE INDEX IF NOT EXISTS idx_messages_type ON conversation_messages(message_type);

        -- Active skills per conversation
        CREATE TABLE IF NOT EXISTS conversation_skills (
          id TEXT PRIMARY KEY,
          conversation_id TEXT NOT NULL,
          skill_name TEXT NOT NULL,
          activated_at TEXT NOT NULL,
          FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
          UNIQUE(conversation_id, skill_name)
        );

        CREATE INDEX IF NOT EXISTS idx_conv_skills ON conversation_skills(conversation_id);

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

        -- Memory FTS (full-text search)
        CREATE VIRTUAL TABLE IF NOT EXISTS memory_entries_fts USING fts5(
          text,
          content='memory_entries'
        );

        CREATE TRIGGER IF NOT EXISTS memory_entries_ai AFTER INSERT ON memory_entries BEGIN
          INSERT INTO memory_entries_fts(rowid, text) VALUES (new.rowid, new.text);
        END;
        CREATE TRIGGER IF NOT EXISTS memory_entries_ad AFTER DELETE ON memory_entries BEGIN
          INSERT INTO memory_entries_fts(memory_entries_fts, rowid, text) VALUES('delete', old.rowid, old.text);
        END;
        CREATE TRIGGER IF NOT EXISTS memory_entries_au AFTER UPDATE ON memory_entries BEGIN
          INSERT INTO memory_entries_fts(memory_entries_fts, rowid, text) VALUES('delete', old.rowid, old.text);
          INSERT INTO memory_entries_fts(rowid, text) VALUES (new.rowid, new.text);
        END;

        -- Models registry (drives model selector, chart colors, rate limits)
        CREATE TABLE IF NOT EXISTS models (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          model TEXT NOT NULL UNIQUE,
          display_name TEXT NOT NULL,
          short_description TEXT NOT NULL DEFAULT '',
          description TEXT NOT NULL DEFAULT '',
          provider TEXT NOT NULL DEFAULT 'local',
          is_cloud INTEGER NOT NULL DEFAULT 0,
          is_premium INTEGER NOT NULL DEFAULT 0,
          is_enabled INTEGER NOT NULL DEFAULT 1,
          daily_limit INTEGER,
          color TEXT NOT NULL DEFAULT '#888888',
          badge_label TEXT NOT NULL DEFAULT '',
          badge_variant TEXT NOT NULL DEFAULT 'outline',
          badge_class TEXT NOT NULL DEFAULT '',
          icon TEXT NOT NULL DEFAULT 'shield',
          icon_color TEXT NOT NULL DEFAULT 'text-gray-600',
          icon_bg TEXT NOT NULL DEFAULT 'bg-gray-100'
        );

        INSERT OR IGNORE INTO models (model, display_name, short_description, description, provider, is_cloud, is_premium, is_enabled, daily_limit, color, badge_label, badge_variant, icon, icon_color, icon_bg) VALUES
          ('qwen3vl-2b', 'Local', 'Runs on your device.', 'Ultimate privacy. Runs entirely on your device with no internet required. Your data never leaves your machine.', 'local', 0, 0, 1, NULL, '#10b981', 'Private', 'outline', 'shield', 'text-green-600', 'bg-green-100'),
          ('gemini-3-flash', 'Gemini 3 Flash', 'Fast cloud model.', 'Google''s fast model with advanced reasoning, tool use, and multimodal capabilities.', 'google', 1, 0, 1, 3, '#60a5fa', 'Enhanced', 'outline', 'zap', 'text-blue-600', 'bg-blue-100'),
          ('gemini-3-pro', 'Gemini 3 Pro', 'Most advanced model.', 'Google''s most advanced model with state-of-the-art reasoning and generation capabilities.', 'google', 1, 1, 1, 0, '#2563eb', 'Premium', 'default', 'crown', 'text-white', 'bg-gradient-to-r from-purple-500 to-pink-500');

        -- Token usage tracking
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
    )
  ])
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

/// Register the sqlite_vec extension globally (idempotent).
fn register_sqlite_vec_extension() -> Result<(), String> {
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
  Ok(())
}

/// Try to apply migrations to an open connection.
/// Returns `Ok(())` on success, or an error string on failure.
fn apply_migrations(conn: &mut Connection) -> Result<(), String> {
  log::info!("[db] Applying database migrations...");
  MIGRATIONS.to_latest(conn).map_err(|e| match e {
    rusqlite_migration::Error::RusqliteError { query: _, err } => {
      format!("SQLite error during migration: {}", err)
    }
    rusqlite_migration::Error::MigrationDefinition(def_err) => {
      format!("Migration definition error: {}", def_err)
    }
    other => format!("Unknown migration error: {}", other),
  })?;
  log::info!("[db] Migrations applied successfully.");
  Ok(())
}

/// Back up the existing database file by copying it to a timestamped path.
/// Returns the backup path on success.
fn backup_database(db_path: &PathBuf) -> Result<PathBuf, String> {
  let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
  let backup_name = format!("database_backup_{}.sqlite", timestamp);
  let backup_path = db_path
    .parent()
    .ok_or("Cannot determine database parent directory")?
    .join(&backup_name);

  fs::copy(db_path, &backup_path)
    .map_err(|e| format!("Failed to back up database: {}", e))?;

  log::info!("[db] Database backed up to {:?}", backup_path);
  Ok(backup_path)
}

/// Initializes the SQLite database connection, registers extensions, and runs migrations.
///
/// If migrations fail (e.g. schema version mismatch after an update), the existing
/// database is **backed up** to a timestamped file and a fresh database is created.
/// A `database_recovered` event is emitted so the frontend can inform the user.
pub fn initialize_database(app_handle: &tauri::AppHandle) -> Result<Connection, String> {
  let db_path = get_db_path(app_handle)?;

  register_sqlite_vec_extension()?;

  let mut conn =
    Connection::open(&db_path).map_err(|e| format!("Failed to open database connection: {}", e))?;

  match apply_migrations(&mut conn) {
    Ok(()) => Ok(conn),
    Err(migration_err) => {
      log::warn!(
        "[db] Migration failed: {}. Attempting recovery with backup...",
        migration_err
      );

      // Close the broken connection
      if let Err((_, e)) = conn.close() {
        log::warn!("[db] Error closing connection during recovery: {}", e);
      }

      // Back up the existing database so the user never loses data
      let backup_path = match backup_database(&db_path) {
        Ok(path) => path,
        Err(backup_err) => {
          return Err(format!(
            "Migration failed ({}) and backup also failed ({}). \
             Please manually copy {:?} before restarting.",
            migration_err, backup_err, db_path
          ));
        }
      };

      // Delete the old database and create a fresh one
      if let Err(e) = fs::remove_file(&db_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
          return Err(format!(
            "Migration failed and could not remove old database: {}",
            e
          ));
        }
      }

      let mut fresh_conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open fresh database: {}", e))?;

      apply_migrations(&mut fresh_conn).map_err(|e| {
        format!(
          "Failed to apply migrations to fresh database: {}. Backup at {:?}",
          e, backup_path
        )
      })?;

      // Emit recovery event so the frontend can notify the user
      let backup_path_str = backup_path.to_string_lossy().to_string();
      log::info!(
        "[db] Database recovered successfully. Backup at: {}",
        backup_path_str
      );

      // Emit event on a best-effort basis (emitter may not be ready yet during setup)
      if let Ok(emitter) = std::panic::catch_unwind(|| {
        crate::events::emitter::emit(
          crate::events::types::DATABASE_RECOVERED,
          crate::events::types::DatabaseRecoveredEvent {
            backup_path: backup_path_str.clone(),
            reason: migration_err.clone(),
            timestamp: chrono::Utc::now().to_rfc3339(),
          },
        )
      }) {
        if let Err(e) = emitter {
          log::debug!("[db] Could not emit recovery event (emitter not ready): {}", e);
        }
      }

      Ok(fresh_conn)
    }
  }
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

/// Convert a BLOB of little-endian f32 bytes into Vec<f32>.
pub fn bytes_to_f32_vec(blob: &[u8]) -> Result<Vec<f32>, String> {
  if blob.len() % 4 != 0 {
    return Err(format!(
      "Invalid embedding BLOB length: {} (not divisible by 4)",
      blob.len()
    ));
  }
  // Ensure correct dimension of 768
  if blob.len() / 4 != 768 {
    return Err(format!(
      "Invalid embedding dimension: {} (expected 768)",
      blob.len() / 4
    ));
  }
  let mut out = Vec::with_capacity(blob.len() / 4);
  for chunk in blob.chunks_exact(4) {
    let arr = <[u8; 4]>::try_from(chunk)
      .map_err(|_| "Failed to convert bytes to f32 (chunk size)".to_string())?;
    out.push(f32::from_le_bytes(arr));
  }
  Ok(out)
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
