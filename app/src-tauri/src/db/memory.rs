use crate::db::core::DbState;
use crate::memory::types::MemoryEntry;
use tauri::State;
use zerocopy::IntoBytes;
use rusqlite::OptionalExtension;
use crate::models::embedding::embedding::generate_embedding;
use tauri::Manager;
use rusqlite::params;

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

	// Upsert embedding into virtual table using the mapping rowid
	// Some virtual tables (like vec) don't support OR REPLACE reliably. Do UPDATE first, then INSERT if needed.
	let updated = conn
		.execute(
			"UPDATE memory_entries_vec SET embedding = ?1 WHERE rowid = ?2",
			rusqlite::params![memory_entry.embedding.as_bytes(), rowid],
		)
		.map_err(|e| format!("Failed to update embedding in memory_entries_vec: {}", e))?;

	if updated == 0 {
		conn
			.execute(
				"INSERT INTO memory_entries_vec(rowid, embedding) VALUES (?1, ?2)",
				rusqlite::params![rowid, memory_entry.embedding.as_bytes()],
			)
			.map_err(|e| format!("Failed to insert embedding into memory_entries_vec: {}", e))?;
	}
    
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

/// Retrieves memory entries joined with the original message content, with pagination.
/// Useful for UI display where the user can expand a memory to see context.
#[tauri::command]
pub fn get_memory_entries_with_message(
	state: State<DbState>,
	offset: u32,
	limit: u32,
) -> Result<serde_json::Value, String> {
	let mut conn_guard = state
		.0
		.lock()
		.map_err(|_| "Failed to acquire DB lock".to_string())?;
	let conn = conn_guard
		.as_mut()
		.ok_or("Database connection not available.".to_string())?;

	let sql = r#"
		SELECT 
		  me.id,
		  me.message_id,
		  me.memory_type,
		  me.text,
		  me.timestamp,
		  cm.content AS message_content
		FROM memory_entries me
		LEFT JOIN conversation_messages cm ON cm.id = me.message_id
		ORDER BY me.timestamp DESC
		LIMIT ?1 OFFSET ?2
	"#;

	let mut stmt = conn
		.prepare(sql)
		.map_err(|e| format!("Prepare failed: {}", e))?;

	let rows = stmt
		.query_map(rusqlite::params![limit, offset], |row| {
			let mut map = serde_json::Map::new();
			map.insert("id".to_string(), serde_json::json!(row.get::<_, String>(0)?));
			map.insert("message_id".to_string(), serde_json::json!(row.get::<_, String>(1)?));
			map.insert("memory_type".to_string(), serde_json::json!(row.get::<_, String>(2)?));
			map.insert("text".to_string(), serde_json::json!(row.get::<_, String>(3)?));
			map.insert("timestamp".to_string(), serde_json::json!(row.get::<_, String>(4)?));
			// message_content may be NULL if message was deleted; represent as null in JSON
			let message_content: Option<String> = row.get(5).ok();
			map.insert("message_content".to_string(), serde_json::json!(message_content));
			Ok(serde_json::Value::Object(map))
		})
		.map_err(|e| format!("Query map failed: {}", e))?
		.collect::<Result<Vec<_>, _>>()
		.map_err(|e| format!("Row processing failed: {}", e))?;

	Ok(serde_json::Value::Array(rows))
}

/// Deletes a single memory entry and cleans up any vector index mappings.
#[tauri::command]
pub fn delete_memory_entry(state: State<DbState>, id: String) -> Result<(), String> {
	let mut conn_guard = state
		.0
		.lock()
		.map_err(|_| "Failed to acquire DB lock".to_string())?;
	let conn = conn_guard
		.as_mut()
		.ok_or("Database connection not available.".to_string())?;

	let tx = conn
		.transaction()
		.map_err(|e| format!("Failed to start transaction: {}", e))?;

	// Find rowid in mapping table
	let rowid: Option<i64> = tx
		.query_row(
			"SELECT rowid FROM memory_entry_vec_map WHERE memory_id = ?1",
			params![&id],
			|row| row.get(0),
		)
		.optional()
		.map_err(|e| format!("Failed to fetch mapping rowid: {}", e))?;

	if let Some(rid) = rowid {
		tx.execute(
			"DELETE FROM memory_entries_vec WHERE rowid = ?1",
			params![rid],
		)
		.map_err(|e| format!("Failed to delete from memory_entries_vec: {}", e))?;
	}

	tx.execute(
		"DELETE FROM memory_entry_vec_map WHERE memory_id = ?1",
		params![&id],
	)
	.map_err(|e| format!("Failed to delete from memory_entry_vec_map: {}", e))?;

	tx.execute("DELETE FROM memory_entries WHERE id = ?1", params![&id])
		.map_err(|e| format!("Failed to delete from memory_entries: {}", e))?;

	tx.commit()
		.map_err(|e| format!("Failed to commit delete transaction: {}", e))?;

	Ok(())
}

/// Deletes all memory entries and fully clears related vector index tables.
#[tauri::command]
pub fn delete_all_memories(state: State<DbState>) -> Result<(), String> {
	let mut conn_guard = state
		.0
		.lock()
		.map_err(|_| "Failed to acquire DB lock".to_string())?;
	let conn = conn_guard
		.as_mut()
		.ok_or("Database connection not available.".to_string())?;

	let tx = conn
		.transaction()
		.map_err(|e| format!("Failed to start transaction: {}", e))?;

	tx.execute("DELETE FROM memory_entries_vec", [])
		.map_err(|e| format!("Failed to clear memory_entries_vec: {}", e))?;
	tx.execute("DELETE FROM memory_entry_vec_map", [])
		.map_err(|e| format!("Failed to clear memory_entry_vec_map: {}", e))?;
	tx.execute("DELETE FROM memory_entries", [])
		.map_err(|e| format!("Failed to clear memory_entries: {}", e))?;

	tx.commit()
		.map_err(|e| format!("Failed to commit delete-all transaction: {}", e))?;

	Ok(())
}

pub async fn find_similar_memories(
	app_handle: &tauri::AppHandle,
	prompt: &str,
	k: u32,
	p: f32,
) -> Result<Vec<MemoryEntry>, String> {
	// 1) Generate query embedding for the prompt
	let query_embedding: Vec<f32> = generate_embedding(app_handle.clone(), prompt.to_string())
		.await
		.map_err(|e| format!("Failed to generate embedding: {}", e))?;

	// 2) Get DB connection
	let db_state = app_handle.state::<DbState>();
	let conn_guard = db_state
		.0
		.lock()
		.map_err(|_| "Failed to acquire DB lock".to_string())?;
	let conn = conn_guard
		.as_ref()
		.ok_or_else(|| "Database connection not available".to_string())?;

	// Optional safety check to ensure vector table exists
	let vec_table_exists: Option<String> = conn
		.query_row(
			"SELECT name FROM sqlite_master WHERE type='table' AND name='memory_entries_vec'",
			[],
			|row| row.get(0),
		)
		.optional()
		.map_err(|e| format!("Failed to check for memory_entries_vec: {}", e))?;
	if vec_table_exists.is_none() {
		// No vector index, return empty (graceful degradation)
		return Ok(Vec::new());
	}

	// 3) Query using cosine distance function directly
	//    Join through the mapping table to fetch full memory entries (including embedding BLOB).
	let sql = r#"
		SELECT
		  me.id,
		  me.message_id,
		  me.memory_type,
		  me.text,
		  me.embedding,
		  me.timestamp,
		  vec_distance_cosine(v.embedding, ?1) AS cosine_distance
		FROM memory_entries_vec AS v
		JOIN memory_entry_vec_map AS m ON v.rowid = m.rowid
		JOIN memory_entries AS me ON me.id = m.memory_id
		ORDER BY cosine_distance
		LIMIT ?2
	"#;

	// Ensure k is at least 1
	let k_neighbors = if k == 0 { 1 } else { k };

	let mut stmt = conn
		.prepare(sql)
		.map_err(|e| format!("Prepare failed: {}", e))?;

	// Pass the query vector as raw bytes to sqlite-vec; it expects little-endian f32 bytes
	let mut rows = stmt
		.query(rusqlite::params![query_embedding.as_bytes(), k_neighbors])
		.map_err(|e| format!("Query execution failed: {}", e))?;

	log::info!("[memory] Searching for top {} similar memories with threshold {:.2}", k_neighbors, p);

	let mut results: Vec<MemoryEntry> = Vec::new();
	while let Some(row) = rows.next().map_err(|e| format!("Row fetch failed: {}", e))? {
		let id: String = row.get(0).map_err(|e| e.to_string())?;
		let message_id: String = row.get(1).map_err(|e| e.to_string())?;
		let memory_type: String = row.get(2).map_err(|e| e.to_string())?;
		let text: String = row.get(3).map_err(|e| e.to_string())?;
		let embedding_blob: Vec<u8> = row.get(4).map_err(|e| e.to_string())?;
		let timestamp: String = row.get(5).map_err(|e| e.to_string())?;
		let cosine_distance: f64 = row.get(6).map_err(|e| e.to_string())?;

		// Convert cosine distance to cosine similarity
		// Cosine distance is typically 1 - cosine_similarity, so similarity = 1 - distance
		let similarity = 1.0 - cosine_distance as f32;

		// Print memory and similarity for debugging
		log::debug!(
			"[memory] Found memory with cosine similarity {:.4}: {}",
			similarity,
			text
		);
		
		// Only include memories that meet the similarity threshold
		if similarity >= p {
			// Convert BLOB back into Vec<f32> (little-endian)
			let embedding = bytes_to_f32_vec(&embedding_blob)?;

			results.push(MemoryEntry {
				id,
				message_id,
				memory_type,
				text,
				embedding,
				timestamp,
				similarity: Some(similarity as f64),
			});
		}
	}

	Ok(results)
}

/// Convert a BLOB of little-endian f32 bytes into Vec<f32>.
fn bytes_to_f32_vec(blob: &[u8]) -> Result<Vec<f32>, String> {
	if blob.len() % 4 != 0 {
		return Err(format!(
			"Invalid embedding BLOB length: {} (not divisible by 4)",
			blob.len()
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