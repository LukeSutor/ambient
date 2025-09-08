use crate::db::core::DbState;
use crate::memory::types::MemoryEntry;
use tauri::State;
use zerocopy::AsBytes;

/// Inserts a new memory entry into the memory_entries table.
#[tauri::command]
pub fn insert_memory_entry(
    state: State<DbState>,
    memory_entry: MemoryEntry,
) -> Result<(), String> {
    let conn_guard = state.0.lock().map_err(|_| "Failed to acquire DB lock".to_string())?;
    let conn = conn_guard.as_ref().ok_or("Database connection not available.".to_string())?;
    
    let sql = r#"INSERT INTO memory_entries (id, message_id, memory_type, text, embedding, timestamp) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#;
    
    conn.execute(
        sql,
        rusqlite::params![
            memory_entry.id,
            memory_entry.message_id,
            memory_entry.memory_type,
            memory_entry.text,
            memory_entry.embedding.as_bytes(),
            memory_entry.timestamp
        ]
    )
    .map_err(|e| format!("Failed to insert memory entry: {}", e))?;
    
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