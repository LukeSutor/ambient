use crate::db::core::DbState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::Manager;
use ts_rs::TS;

/// A model entry from the database.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct ModelEntry {
    pub id: i64,
    /// Unique model key (e.g. "qwen3vl-2b", "gemini-3-flash").
    pub model: String,
    /// User-friendly name shown in the UI (e.g. "Local", "Gemini 3 Flash").
    pub display_name: String,
    /// Short one-liner for compact contexts like the HUD dropdown.
    pub short_description: String,
    /// Longer description for the full settings page model selector.
    pub description: String,
    /// The model provider (e.g. "local", "google", "openai").
    pub provider: String,
    pub is_cloud: bool,
    pub is_premium: bool,
    pub daily_limit: Option<i32>,
    pub color: String,
    pub badge_label: String,
    pub badge_variant: String,
    pub badge_class: String,
    pub icon: String,
    pub icon_color: String,
    pub icon_bg: String,
    /// Whether this model is enabled/visible in the UI.
    pub is_enabled: bool,
}

/// Get all models from the database.
#[tauri::command]
pub fn get_models(state: tauri::State<DbState>) -> Result<Vec<ModelEntry>, String> {
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let mut stmt = conn
        .prepare(
            "SELECT id, model, display_name, short_description, description, provider, \
             is_cloud, is_premium, daily_limit, color, badge_label, badge_variant, \
             badge_class, icon, icon_color, icon_bg, is_enabled FROM models ORDER BY id",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let models = stmt
        .query_map([], |row| {
            Ok(ModelEntry {
                id: row.get(0)?,
                model: row.get(1)?,
                display_name: row.get(2)?,
                short_description: row.get(3)?,
                description: row.get(4)?,
                provider: row.get(5)?,
                is_cloud: row.get::<_, i32>(6)? != 0,
                is_premium: row.get::<_, i32>(7)? != 0,
                daily_limit: row.get(8)?,
                color: row.get(9)?,
                badge_label: row.get(10)?,
                badge_variant: row.get(11)?,
                badge_class: row.get(12)?,
                icon: row.get(13)?,
                icon_color: row.get(14)?,
                icon_bg: row.get(15)?,
                is_enabled: row.get::<_, i32>(16)? != 0,
            })
        })
        .map_err(|e| format!("Failed to query models: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect models: {}", e))?;

    Ok(models)
}

/// Look up a single model by its unique key.
/// Used by the LLM client to determine provider routing.
pub fn get_model_by_key(app_handle: &tauri::AppHandle, model_key: &str) -> Result<ModelEntry, String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    conn.query_row(
        "SELECT id, model, display_name, short_description, description, provider, \
         is_cloud, is_premium, daily_limit, color, badge_label, badge_variant, \
         badge_class, icon, icon_color, icon_bg, is_enabled FROM models WHERE model = ?1",
        params![model_key],
        |row| {
            Ok(ModelEntry {
                id: row.get(0)?,
                model: row.get(1)?,
                display_name: row.get(2)?,
                short_description: row.get(3)?,
                description: row.get(4)?,
                provider: row.get(5)?,
                is_cloud: row.get::<_, i32>(6)? != 0,
                is_premium: row.get::<_, i32>(7)? != 0,
                daily_limit: row.get(8)?,
                color: row.get(9)?,
                badge_label: row.get(10)?,
                badge_variant: row.get(11)?,
                badge_class: row.get(12)?,
                icon: row.get(13)?,
                icon_color: row.get(14)?,
                icon_bg: row.get(15)?,
                is_enabled: row.get::<_, i32>(16)? != 0,
            })
        },
    )
    .map_err(|e| format!("Model '{}' not found: {}", model_key, e))
}

/// Toggle a model's enabled state.
#[tauri::command]
pub fn toggle_model(
    state: tauri::State<DbState>,
    model_key: String,
    enabled: bool,
) -> Result<(), String> {
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let rows_affected = conn
        .execute(
            "UPDATE models SET is_enabled = ?1 WHERE model = ?2",
            rusqlite::params![enabled as i32, model_key],
        )
        .map_err(|e| format!("Failed to toggle model: {}", e))?;

    if rows_affected == 0 {
        return Err(format!("Model '{}' not found", model_key));
    }

    log::info!("[models] Model '{}' is_enabled set to {}", model_key, enabled);
    Ok(())
}
