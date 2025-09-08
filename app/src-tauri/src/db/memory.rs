use crate::db::core::DbState;
use crate::memory::types::MemoryEntry;
use tauri::State;
use zerocopy::IntoBytes; // for embedding.as_bytes() with sqlite-vec
use rusqlite::OptionalExtension;

/// Inserts a new memory entry into the memory_entries table.
#[tauri::command]
pub fn insert_memory_entry(
    state: State<DbState>,
    memory_entry: MemoryEntry,
) -> Result<(), String> {
    let conn_guard = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn_guard.as_ref().ok_or("Database connection not available.".to_string())?;
    
    let sql = r#"INSERT INTO memory_entries (id, message_id, memory_type, text, embedding, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#;
    
    // Convert embedding Vec<f32> into a Vec<u8> (little-endian) for BLOB storage
    let embedding_bytes: Vec<u8> = memory_entry
        .embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect();

    conn.execute(
        sql,
        rusqlite::params![
            memory_entry.id,
            memory_entry.message_id,
            memory_entry.memory_type,
            memory_entry.text,
            embedding_bytes,
            memory_entry.timestamp
        ]
    )
    .map_err(|e| format!("Failed to insert memory entry: {}", e))?;

    // Also insert into sqlite-vec virtual table for similarity search (tables created via migrations)

    // Insert mapping row to obtain rowid
    conn.execute(
        "INSERT OR IGNORE INTO memory_entry_vec_map(memory_id) VALUES (?1)",
        rusqlite::params![memory_entry.id],
    )
    .map_err(|e| format!("Failed to insert mapping row: {}", e))?;

    // Get mapping rowid (select to be safe in case of IGNORE)
    let rowid: i64 = conn
        .query_row(
            "SELECT rowid FROM memory_entry_vec_map WHERE memory_id = ?1",
            rusqlite::params![memory_entry.id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to fetch mapping rowid: {}", e))?;

    // Insert embedding into virtual table using the mapping rowid
    // Use zerocopy for &[u8] representation
    conn.execute(
        "INSERT OR REPLACE INTO memory_entries_vec(rowid, embedding) VALUES (?1, ?2)",
        rusqlite::params![rowid, memory_entry.embedding.as_bytes()],
    )
    .map_err(|e| format!("Failed to insert embedding into memory_entries_vec: {}", e))?;
    
    Ok(())
}

/// Retrieves memory entries from the database with pagination.
#[tauri::command]
pub fn get_memory_entries(
    state: State<DbState>,
    offset: u32,
    limit: u32,
) -> Result<serde_json::Value, String> {
    let conn_guard = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn_guard.as_ref().ok_or("Database connection not available.".to_string())?;
    
    let sql = r#"SELECT id, message_id, memory_type, text, timestamp FROM memory_entries ORDER BY timestamp DESC LIMIT ?1 OFFSET ?2"#;
    let mut stmt = conn.prepare(sql).map_err(|e| format!("Prepare failed: {}", e))?;
    
    let rows = stmt.query_map(rusqlite::params![limit, offset], |row| {
        let mut map = serde_json::Map::new();
        map.insert("id".to_string(), serde_json::json!(row.get::<_, String>(0)?));
        map.insert("message_id".to_string(), serde_json::json!(row.get::<_, String>(1)?));
        map.insert("memory_type".to_string(), serde_json::json!(row.get::<_, String>(2)?));
        map.insert("text".to_string(), serde_json::json!(row.get::<_, String>(3)?));
        map.insert("timestamp".to_string(), serde_json::json!(row.get::<_, String>(4)?));
        Ok(serde_json::Value::Object(map))
    })
    .map_err(|e| format!("Query map failed: {}", e))?
    .collect::<Result<Vec<_>, _>>()
    .map_err(|e| format!("Row processing failed: {}", e))?;
    
    Ok(serde_json::Value::Array(rows))
}

/// Retrieves the top-k most similar memory entries to a given embedding using sqlite-vec distance.
#[tauri::command]
pub fn get_top_k_similar_memory(
    state: State<DbState>,
    query_embedding: Vec<f32>,
    k: u32,
) -> Result<serde_json::Value, String> {
    let conn_guard = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn_guard.as_ref().ok_or("Database connection not available.".to_string())?;

    // Sanity check that virtual table exists (should via migrations). If missing, return empty.
    let vec_table_exists: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='memory_entries_vec'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to check for memory_entries_vec: {}", e))?;
    if vec_table_exists.is_none() { return Ok(serde_json::json!([])); }

    // Prepare similarity query
    let sql = r#"
        SELECT
          m.memory_id,
          me.message_id,
          me.memory_type,
          me.text,
          me.timestamp,
          distance
        FROM memory_entries_vec
        JOIN memory_entry_vec_map m USING(rowid)
        JOIN memory_entries me ON me.id = m.memory_id
        WHERE embedding MATCH ?1
        ORDER BY distance
        LIMIT ?2
    "#;

    let mut stmt = conn.prepare(sql).map_err(|e| format!("Prepare failed: {}", e))?;
    let rows = stmt
        .query_map(
            rusqlite::params![query_embedding.as_bytes(), k],
            |row| {
                let mut map = serde_json::Map::new();
                map.insert("id".to_string(), serde_json::json!(row.get::<_, String>(0)?));
                map.insert("message_id".to_string(), serde_json::json!(row.get::<_, String>(1)?));
                map.insert("memory_type".to_string(), serde_json::json!(row.get::<_, String>(2)?));
                map.insert("text".to_string(), serde_json::json!(row.get::<_, String>(3)?));
                map.insert("timestamp".to_string(), serde_json::json!(row.get::<_, String>(4)?));
                map.insert("distance".to_string(), serde_json::json!(row.get::<_, f64>(5)?));
                Ok(serde_json::Value::Object(map))
            },
        )
        .map_err(|e| format!("Query map failed: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Row processing failed: {}", e))?;

    Ok(serde_json::Value::Array(rows))
}