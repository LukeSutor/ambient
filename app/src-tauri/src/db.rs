// filepath: c:\Users\Luke\Desktop\coding\local-computer-use\app\src-tauri\src\db.rs
use rusqlite::{ffi::sqlite3_auto_extension, Connection, Result as RusqliteResult, params_from_iter};
use rusqlite::types::{ValueRef, Value as RusqliteValue}; // Added
use sqlite_vec::sqlite3_vec_init;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use rusqlite_migration::{Migrations, M};
use serde::Serialize; // Keep Serialize if needed elsewhere, otherwise remove
use serde_json::Value as JsonValue; // Added
use std::sync::Mutex; // Ensure DbState uses Mutex if not already defined

// Define DbState if it's not defined elsewhere (assuming it wraps the connection)
pub struct DbState(pub Mutex<Option<Connection>>);


lazy_static::lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
            M::up("
                -- Migration 001: Create core activity tracking tables
                CREATE TABLE IF NOT EXISTS events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    timestamp INTEGER NOT NULL,              -- Unix epoch seconds UTC
                    active_app_name TEXT NOT NULL,           -- e.g., 'Code.exe', 'chrome.exe'
                    active_window_title TEXT,                -- Window title, nullable
                    active_url TEXT,                         -- URL if browser, nullable
                    activity_type TEXT NOT NULL,             -- VLM/Logic classification (e.g., 'Coding', 'Browsing')
                    activity_details TEXT,                   -- More specific details, nullable
                    confidence REAL,                         -- Confidence score (0.0-1.0), nullable
                    embedding BLOB NOT NULL,                 -- Text embedding of activity (for sqlite-vec)
                    duration_hint_secs INTEGER               -- Optional: Estimated duration since last *different* event
                );

                -- Indexes for common queries
                CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events (timestamp);
                CREATE INDEX IF NOT EXISTS idx_events_activity_type ON events (activity_type);
                CREATE INDEX IF NOT EXISTS idx_events_app_name ON events (active_app_name);

                -- Table for discovered patterns
                CREATE TABLE IF NOT EXISTS patterns (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    pattern_type TEXT NOT NULL,              -- e.g., 'sequence', 'periodicity', 'cluster'
                    trigger_conditions TEXT NOT NULL,        -- JSON describing triggers (time, preceding events, app, etc.)
                    predicted_next_event TEXT,               -- JSON describing likely next event(s), nullable
                    recommended_action TEXT,                 -- JSON describing suggested action, nullable
                    confidence_score REAL NOT NULL,          -- Reliability metric (support, confidence, etc.)
                    frequency_metric REAL,                   -- How often it occurs (e.g., per week), nullable
                    last_detected_timestamp INTEGER,         -- When this pattern was last updated/seen
                    created_timestamp INTEGER NOT NULL,      -- When the pattern was first discovered
                    is_active INTEGER NOT NULL DEFAULT 1     -- Boolean (0/1) if the pattern is enabled for recommendations
                );

                CREATE INDEX IF NOT EXISTS idx_patterns_type ON patterns (pattern_type);
                CREATE INDEX IF NOT EXISTS idx_patterns_active_timestamp ON patterns (is_active, last_detected_timestamp);

                -- Table for application settings
                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY NOT NULL,
                    value TEXT NOT NULL                      -- Store simple values directly, complex ones as JSON
                );

                -- Default settings examples (insert these via code after migration if needed)
                -- INSERT OR IGNORE INTO settings (key, value) VALUES ('capture_interval_seconds', '60');
                -- INSERT OR IGNORE INTO settings (key, value) VALUES ('ignored_apps', '[]'); -- JSON array of app names
                -- INSERT OR IGNORE INTO settings (key, value) VALUES ('enable_vlm', 'true');
            ")
            // Add more migrations here as needed
            // M::up("
            //     -- Migration 002: Add a new column
            //     ALTER TABLE documents ADD COLUMN source TEXT;
            // "),
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
            rusqlite_migration::Error::RusqliteError { query: _, err } => format!("SQLite error during migration: {}", err),
            rusqlite_migration::Error::MigrationDefinition(def_err) => format!("Migration definition error: {}", def_err),
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
        ValueRef::Real(f) => JsonValue::Number(serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0))), // Handle potential NaN/Infinity
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
        _ => Err(format!("Unsupported JSON type for parameter: {:?}", json_value)),
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

    let maybe_conn_guard = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;

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
            let mut stmt = conn.prepare(&sql).map_err(|e| format!("Prepare failed: {}", e))?;
            let column_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();

            let results: Result<Vec<serde_json::Map<String, JsonValue>>, _> = stmt
                .query_map(params_from_iter(rusqlite_params.iter()), |row| {
                    let mut map = serde_json::Map::new();
                    for (i, col_name) in column_names.iter().enumerate() {
                        let value_ref = row.get_ref_unwrap(i);
                        let json_value = rusqlite_to_json(value_ref)
                            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(i, value_ref.data_type(), Box::new(e)))?;
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
                    let json_values: Vec<JsonValue> = vec_of_maps
                        .into_iter()
                        .map(JsonValue::Object)
                        .collect();
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
    let mut conn_guard = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
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