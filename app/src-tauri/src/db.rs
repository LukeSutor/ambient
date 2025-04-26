// filepath: c:\Users\Luke\Desktop\coding\local-computer-use\app\src-tauri\src\db.rs
use rusqlite::{ffi::sqlite3_auto_extension, Connection, Result};
use sqlite_vec::sqlite3_vec_init;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;
// Import migration types
use rusqlite_migration::{Migrations, M};

// Define migrations (use M::up for SQL statements)
lazy_static::lazy_static! {
    static ref MIGRATIONS: Migrations<'static> =
        Migrations::new(vec![
            M::up("
                -- Migration 001: Create initial tables
                CREATE TABLE IF NOT EXISTS documents (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    content TEXT NOT NULL,
                    embedding BLOB -- For sqlite-vec
                    -- Add timestamp, etc. if needed
                );
                -- Add other initial tables here
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
    // ... (existing code for path resolution and directory creation) ...
    let app_data_path = app_handle
        .path()
        .app_data_dir()
        .ok_or_else(|| "Could not resolve app data directory.".to_string())?;
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
            rusqlite_migration::Error::Rusqlite(rusqlite_err) => format!("SQLite error during migration: {}", rusqlite_err),
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

#[derive(Serialize)] // So it can be sent to the frontend
pub struct Document {
    id: i64,
    content: String,
    // Embedding is not typically sent back in selects unless needed
    // embedding: Vec<u8>, // Use Vec<u8> if you need to send the raw blob
}

/// Inserts a new document with its content and embedding.
#[tauri::command]
pub fn insert_document(
    state: tauri::State<DbState>,
    content: String,
    embedding: Vec<f32>, // Receive f32 vector from frontend/caller
) -> Result<(), String> {
    let maybe_conn_guard = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;

    if let Some(conn) = maybe_conn_guard.as_ref() {
        let embedding_bytes = embedding.as_bytes(); // Convert Vec<f32> to &[u8]

        conn.execute(
            "INSERT INTO documents (content, embedding) VALUES (?1, ?2)",
            params![content, embedding_bytes],
        )
        .map_err(|e| {
            println!("[db] Failed to insert document: {}", e);
            format!("Failed to insert document: {}", e)
        })?;
        println!("[db] Inserted document successfully.");
        Ok(())
    } else {
        Err("Database connection not available.".to_string())
    }
}

/// Selects all documents (id and content only).
#[tauri::command]
pub fn select_documents(state: tauri::State<DbState>) -> Result<Vec<Document>, String> {
    let maybe_conn_guard = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;

    if let Some(conn) = maybe_conn_guard.as_ref() {
        let mut stmt = conn
            .prepare("SELECT id, content FROM documents")
            .map_err(|e| format!("Failed to prepare select statement: {}", e))?;

        let doc_iter = stmt
            .query_map([], |row| {
                Ok(Document {
                    id: row.get(0)?,
                    content: row.get(1)?,
                })
            })
            .map_err(|e| format!("Failed to execute select query: {}", e))?;

        let mut documents = Vec::new();
        for doc_result in doc_iter {
            match doc_result {
                Ok(doc) => documents.push(doc),
                Err(e) => return Err(format!("Failed to map row to Document: {}", e)),
            }
        }
        println!("[db] Selected {} documents.", documents.len());
        Ok(documents)
    } else {
        Err("Database connection not available.".to_string())
    }
}

// Example of how to access the connection in a command:
/*
#[tauri::command]
fn my_db_command(state: tauri::State<DbState>) -> Result<(), String> {
    let maybe_conn = state.0.lock().unwrap();
    if let Some(conn) = maybe_conn.as_ref() {
        // Use the connection 'conn'
        conn.execute("...", []).map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Database connection not available.".to_string())
    }
}
*/