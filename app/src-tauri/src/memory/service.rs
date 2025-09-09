use crate::db::core::DbState;
use crate::memory::types::MemoryEntry;
use crate::models::embedding::embedding::generate_embedding;
use rusqlite::OptionalExtension;
use tauri::Manager;
use zerocopy::IntoBytes;

pub async fn find_similar_memories(
	app_handle: &tauri::AppHandle,
	prompt: &str,
	k: u32,
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

	// 3) Query top-k similar rows using sqlite-vec's MATCH operator
	//    Join through the mapping table to fetch full memory entries (including embedding BLOB).
	let sql = r#"
		SELECT
		  me.id,
		  me.message_id,
		  me.memory_type,
		  me.text,
		  me.embedding,
		  me.timestamp,
		  v.distance
		FROM memory_entries_vec AS v
		JOIN memory_entry_vec_map AS m USING(rowid)
		JOIN memory_entries AS me ON me.id = m.memory_id
		WHERE v.embedding MATCH ?1
		ORDER BY v.distance
		LIMIT ?2
	"#;

	let mut stmt = conn
		.prepare(sql)
		.map_err(|e| format!("Prepare failed: {}", e))?;

	// Pass the query vector as raw bytes to sqlite-vec; it expects little-endian f32 bytes
	let mut rows = stmt
		.query(rusqlite::params![query_embedding.as_bytes(), k])
		.map_err(|e| format!("Query execution failed: {}", e))?;

	let mut results: Vec<MemoryEntry> = Vec::new();
	while let Some(row) = rows.next().map_err(|e| format!("Row fetch failed: {}", e))? {
		let id: String = row.get(0).map_err(|e| e.to_string())?;
		let message_id: String = row.get(1).map_err(|e| e.to_string())?;
		let memory_type: String = row.get(2).map_err(|e| e.to_string())?;
		let text: String = row.get(3).map_err(|e| e.to_string())?;
		let embedding_blob: Vec<u8> = row.get(4).map_err(|e| e.to_string())?;
		let timestamp: String = row.get(5).map_err(|e| e.to_string())?;
		// distance is at index 6, but we don't include it in MemoryEntry; still read to advance
		let _distance: f64 = row.get(6).map_err(|e| e.to_string())?;

		// Convert BLOB back into Vec<f32> (little-endian)
		let embedding = bytes_to_f32_vec(&embedding_blob)?;

		results.push(MemoryEntry {
			id,
			message_id,
			memory_type,
			text,
			embedding,
			timestamp,
		});
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

