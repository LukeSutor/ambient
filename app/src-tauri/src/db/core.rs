use crate::constants::{KEYRING_DB_KEY_PREFIX, KEYRING_SERVICE};
use keyring::Entry;
use once_cell::sync::Lazy;
use rand::RngCore;
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

        -- Models registry
        -- `model` is the API model identifier (e.g. "qwen3vl-2b", "gpt-4o").
        -- It is NOT unique — the integer `id` is the canonical row key.
        CREATE TABLE IF NOT EXISTS models (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          model TEXT NOT NULL,
          display_name TEXT NOT NULL,
          short_description TEXT NOT NULL DEFAULT '',
          description TEXT NOT NULL DEFAULT '',
          provider TEXT NOT NULL DEFAULT 'unknown',
          is_cloud INTEGER NOT NULL DEFAULT 0,
          is_enabled INTEGER NOT NULL DEFAULT 1,
          is_internal INTEGER NOT NULL DEFAULT 1,
          api_url TEXT,
          api_key TEXT,
          request_format TEXT NOT NULL DEFAULT 'openai'
        );

        INSERT OR IGNORE INTO models (model, display_name, short_description, description, provider, is_cloud, is_enabled, is_internal, request_format) VALUES
          ('qwen3vl-2b', 'Local', 'Runs on your device.', 'Ultimate privacy. Runs entirely on your device with no internet required. Your data never leaves your machine.', 'local', 0, 1, 1, 'openai'),
          ('gemini-3-flash', 'Gemini 3 Flash', 'Fast cloud model.', 'Google''s fast model with advanced reasoning, tool use, and multimodal capabilities.', 'google', 1, 1, 1, 'gemini'),
          ('gemini-3-pro', 'Gemini 3 Pro', 'Most advanced model.', 'Google''s most advanced model with state-of-the-art reasoning and generation capabilities.', 'google', 1, 1, 1, 'gemini');

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

        -- ============================================================================
        -- AUTOMATION TABLES
        -- ============================================================================

        -- Automation tasks (scheduled and semantic/event-based)
        CREATE TABLE IF NOT EXISTS automation_tasks (
          id TEXT PRIMARY KEY,
          name TEXT NOT NULL,
          description TEXT NOT NULL DEFAULT '',
          task_type TEXT NOT NULL CHECK(task_type IN ('scheduled', 'semantic')),
          is_enabled INTEGER NOT NULL DEFAULT 1,
          is_system INTEGER NOT NULL DEFAULT 0,
          prompt_template TEXT NOT NULL DEFAULT '',
          model_id INTEGER,
          disabled_skills TEXT NOT NULL DEFAULT '[]',
          notify_on_complete INTEGER NOT NULL DEFAULT 1,
          notify_on_error INTEGER NOT NULL DEFAULT 1,
          max_iterations INTEGER NOT NULL DEFAULT 10,
          timeout_seconds INTEGER NOT NULL DEFAULT 120,
          schedule_type TEXT CHECK(schedule_type IN ('interval', 'daily', 'weekdays', 'specific_days')),
          schedule_value TEXT,
          schedule_timezone TEXT NOT NULL DEFAULT 'local',
          trigger_type TEXT CHECK(trigger_type IN ('screen_content', 'url_visit')),
          trigger_config TEXT,
          last_run_at TEXT,
          next_run_at TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL,
          FOREIGN KEY (model_id) REFERENCES models(id)
        );

        CREATE INDEX IF NOT EXISTS idx_automation_tasks_type ON automation_tasks(task_type);
        CREATE INDEX IF NOT EXISTS idx_automation_tasks_enabled ON automation_tasks(is_enabled);
        CREATE INDEX IF NOT EXISTS idx_automation_tasks_system ON automation_tasks(is_system);

        -- Automation execution history
        CREATE TABLE IF NOT EXISTS automation_runs (
          id TEXT PRIMARY KEY,
          task_id TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed', 'cancelled')),
          result_text TEXT,
          error_message TEXT,
          started_at TEXT NOT NULL,
          completed_at TEXT,
          credits_used REAL NOT NULL DEFAULT 0.0,
          FOREIGN KEY (task_id) REFERENCES automation_tasks(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_automation_runs_task ON automation_runs(task_id);
        CREATE INDEX IF NOT EXISTS idx_automation_runs_started ON automation_runs(started_at DESC);

        -- Semantic trigger patterns for event-based automations
        CREATE TABLE IF NOT EXISTS automation_triggers (
          id TEXT PRIMARY KEY,
          task_id TEXT NOT NULL,
          trigger_type TEXT NOT NULL CHECK(trigger_type IN ('screen_content', 'url_visit')),
          trigger_config TEXT NOT NULL DEFAULT '{}',
          is_enabled INTEGER NOT NULL DEFAULT 1,
          created_at TEXT NOT NULL,
          FOREIGN KEY (task_id) REFERENCES automation_tasks(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_automation_triggers_task ON automation_triggers(task_id);
      "#,
    )
  ])
});

/// Per-user database path: {app_data}/databases/database_{user_id}.sqlite
///
/// Each user gets their own encrypted database file within their profile
/// directory, identified by their Supabase user ID (UUID).
fn get_user_db_path(app_handle: &tauri::AppHandle, user_id: &str) -> Result<PathBuf, String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
  let profile_dir = app_data_path
    .join(crate::constants::PROFILES_DIR)
    .join(user_id);
  if let Err(e) = std::fs::create_dir_all(&profile_dir) {
    return Err(format!("Failed to create user profile directory: {}", e));
  }
  Ok(profile_dir.join(crate::constants::USER_DB_FILENAME))
}

/// Generate or retrieve the per-user database encryption key from the OS keyring.
///
/// Each user gets a unique 32-byte AES-256 key stored in the OS credential store
/// (Windows Credential Manager / macOS Keychain / Linux Secret Service).
/// The key is used as a raw SQLCipher key via `PRAGMA key`.
///
/// If no key exists for this user, a cryptographically random key is generated
/// and stored. If the stored key is invalid (wrong length), it is replaced.
fn get_or_create_user_db_key(user_id: &str) -> Result<Vec<u8>, String> {
  let keyring_name = format!("{}{}", KEYRING_DB_KEY_PREFIX, user_id);
  let entry = Entry::new(KEYRING_SERVICE, &keyring_name)
    .map_err(|e| format!("Keyring entry error for DB key: {}", e))?;

  // Try to read existing key
  match entry.get_password() {
    Ok(key_hex) => {
      // Decode hex to bytes
      let key_bytes = hex_decode(&key_hex)?;
      if key_bytes.len() == 32 {
        return Ok(key_bytes);
      }
      log::warn!("[db] Invalid DB key length in keyring ({}), generating new key", key_bytes.len());
    }
    Err(keyring::Error::NoEntry) => {
      log::info!("[db] No existing DB key for user, generating new key");
    }
    Err(e) => {
      return Err(format!("Keyring read error for DB key: {}", e));
    }
  }

  // Generate a new 32-byte random key
  let mut key = [0u8; 32];
  rand::rngs::OsRng.fill_bytes(&mut key);
  let key_hex = hex_encode(&key);

  entry
    .set_password(&key_hex)
    .map_err(|e| format!("Failed to store DB key in keyring: {}", e))?;

  log::info!("[db] Generated and stored new DB encryption key for user");
  Ok(key.to_vec())
}

/// Set the SQLCipher encryption key on a connection.
///
/// Must be called immediately after opening a connection, before any other
/// operations. Uses raw hex key format to avoid PBKDF2 key derivation overhead.
/// Verifies the key works by querying `sqlite_master`.
fn set_encryption_key(conn: &Connection, key_bytes: &[u8]) -> Result<(), String> {
  let hex_key = hex_encode(key_bytes);
  // SQLCipher raw key format: PRAGMA key = "x'<hex>'";
  conn
    .execute_batch(&format!("PRAGMA key = \"x'{}'\"", hex_key))
    .map_err(|e| format!("Failed to set database encryption key: {}", e))?;

  // Verify the key by reading the schema — if wrong, SQLCipher returns
  // "file is not a database" on the first real query.
  conn
    .execute_batch("SELECT count(*) FROM sqlite_master;")
    .map_err(|e| format!("Database key verification failed (wrong key?): {}", e))?;

  Ok(())
}

/// Encode bytes to hex string.
fn hex_encode(bytes: &[u8]) -> String {
  bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decode hex string to bytes.
fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
  if hex.len() % 2 != 0 {
    return Err("Invalid hex string length".to_string());
  }
  (0..hex.len())
    .step_by(2)
    .map(|i| {
      u8::from_str_radix(&hex[i..i + 2], 16)
        .map_err(|e| format!("Invalid hex character: {}", e))
    })
    .collect()
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
  let stem = db_path
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("database");
  let backup_name = format!("{}_backup_{}.sqlite", stem, timestamp);
  let backup_path = db_path
    .parent()
    .ok_or("Cannot determine database parent directory")?
    .join(&backup_name);

  fs::copy(db_path, &backup_path)
    .map_err(|e| format!("Failed to back up database: {}", e))?;

  log::info!("[db] Database backed up to {:?}", backup_path);
  Ok(backup_path)
}

/// Open (or create) an encrypted per-user SQLite database.
///
/// Each user's database is identified by their Supabase user ID and encrypted
/// with a per-user key stored in the OS keyring. The flow:
///
/// 1. Resolve path: `{app_data}/databases/database_{user_id}.sqlite`
/// 2. Register sqlite-vec extension (idempotent)
/// 3. Open SQLite connection
/// 4. Set SQLCipher encryption key via `PRAGMA key` (raw hex)
/// 5. Apply schema migrations (with backup + recovery on failure)
///
/// If migrations fail, the existing database is backed up and a fresh
/// encrypted database is created. A `database_recovered` event is emitted.
pub fn initialize_user_database(
  app_handle: &tauri::AppHandle,
  user_id: &str,
) -> Result<Connection, String> {
  let db_path = get_user_db_path(app_handle, user_id)?;
  log::info!("[db] Opening database for user at {:?}", db_path);

  register_sqlite_vec_extension()?;

  let key_bytes = get_or_create_user_db_key(user_id)?;

  let mut conn = Connection::open(&db_path)
    .map_err(|e| format!("Failed to open database connection: {}", e))?;

  set_encryption_key(&conn, &key_bytes)?;

  match apply_migrations(&mut conn) {
    Ok(()) => {
      log::info!("[db] User database initialized successfully");
      Ok(conn)
    }
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

      set_encryption_key(&fresh_conn, &key_bytes)?;

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

/// Open the database for the currently authenticated user.
///
/// Reads the stored auth state to determine the user ID, then opens (or creates)
/// their encrypted database. Any previously open connection is closed first.
///
/// Called by the frontend after successful login, and by `lib.rs` on app startup
/// when a stored session exists.
#[tauri::command]
pub fn open_user_database(
  state: tauri::State<DbState>,
  app_handle: tauri::AppHandle,
) -> Result<(), String> {
  // Read user ID from keyring
  let user_id = crate::auth::storage::get_current_user_id()
    .map_err(|e| format!("Failed to get current user ID: {}", e))?
    .ok_or("No active session. Please log in first.")?;

  // Close existing connection if any
  {
    let mut guard = state
      .0
      .lock()
      .map_err(|_| "Failed to acquire DB lock".to_string())?;
    if let Some(conn) = guard.take() {
      if let Err((_, e)) = conn.close() {
        log::warn!("[db] Error closing previous connection: {}", e);
      }
    }
  }

  // Initialize user's encrypted database
  let conn = initialize_user_database(&app_handle, &user_id)?;

  let mut guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;
  *guard = Some(conn);

  log::info!("[db] Opened encrypted database for user {}", &user_id);
  Ok(())
}

/// Close the current database connection.
///
/// Called before logout to cleanly release the database. After this,
/// all DB operations will return "Database not available" until a new
/// user logs in.
#[tauri::command]
pub fn close_user_database(state: tauri::State<DbState>) -> Result<(), String> {
  let mut guard = state
    .0
    .lock()
    .map_err(|_| "Failed to acquire DB lock".to_string())?;

  if let Some(conn) = guard.take() {
    if let Err((_, e)) = conn.close() {
      log::warn!("[db] Error closing database connection: {}", e);
    }
    log::info!("[db] User database closed");
  }
  Ok(())
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

/// Closes the current database connection, deletes the database file, and initializes a fresh one.
///
/// Reads the user ID from the stored auth state to determine which database to reset.
#[tauri::command]
pub fn reset_database(
  state: tauri::State<'_, DbState>,
  app_handle: tauri::AppHandle,
) -> Result<(), String> {
  log::info!("[db] Attempting to reset database...");

  // Get user ID from keyring
  let user_id = crate::auth::storage::get_current_user_id()
    .map_err(|e| format!("Failed to get current user ID: {}", e))?
    .ok_or("No active session. Cannot reset database without a logged-in user.")?;
  let db_path = get_user_db_path(&app_handle, &user_id)?;
  log::debug!("[db] Target database path for reset: {:?}", db_path);

  // Close existing connection
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

  // Delete the database file
  log::info!("[db] Deleting database file: {:?}", db_path);
  match fs::remove_file(&db_path) {
    Ok(_) => log::info!("[db] Database file deleted successfully."),
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
      log::debug!("[db] Database file not found, skipping deletion.")
    }
    Err(e) => return Err(format!("Failed to delete database file: {}", e)),
  }

  // Re-initialize with the same user's encryption key
  log::info!("[db] Re-initializing database...");
  match initialize_user_database(&app_handle, &user_id) {
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
