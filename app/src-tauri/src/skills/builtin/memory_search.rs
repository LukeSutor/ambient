//! Memory search skill implementation.
//!
//! This skill provides semantic search through stored memories from
//! past conversations.
//!
//! # Tools
//!
//! - `search_memories`: Search memories using semantic similarity
//!
//! # Status
//!
//! **TODO**: This is a stub implementation. Actual memory search
//! should use the existing `find_similar_memories` function.

use super::ToolCall;
use serde_json::Value;
use tauri::AppHandle;

/// Execute a memory search tool.
pub async fn execute(
    app_handle: &AppHandle,
    call: &ToolCall,
) -> Result<Value, String> {
    match call.tool_name.as_str() {
        "search_memories" => search_memories(app_handle, call).await,
        _ => Err(format!("Unknown tool: {}", call.tool_name)),
    }
}

/// Search through stored memories.
async fn search_memories(
    app_handle: &AppHandle,
    call: &ToolCall,
) -> Result<Value, String> {
    let query = call
        .arguments
        .get("query")
        .and_then(|q| q.as_str())
        .ok_or_else(|| "Missing 'query' argument".to_string())?;

    let limit = call
        .arguments
        .get("limit")
        .and_then(|l| l.as_u64())
        .unwrap_or(5) as usize;

    let min_similarity = call
        .arguments
        .get("min_similarity")
        .and_then(|s| s.as_f64())
        .unwrap_or(0.7);

    log::info!(
        "[memory_search] Searching for: {} (limit: {}, min_similarity: {})",
        query,
        limit,
        min_similarity
    );

    // Use existing memory search function
    let memories = crate::db::memory::find_similar_memories(
        &app_handle,
        query,
        limit as u32,
        min_similarity as f32,
    )
    .await
    .map_err(|e| {
        log::error!("[memory_search] Failed to search memories: {}", e);
        e
    })?;

    // Convert memory entries to result format
    let results: Vec<Value> = memories
        .into_iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id,
                "text": m.text,
                "memory_type": m.memory_type,
                "timestamp": m.timestamp,
                "similarity": m.similarity.unwrap_or(0.0),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "results": results,
        "query": query
    }))
}
