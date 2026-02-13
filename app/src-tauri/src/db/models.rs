use crate::db::core::DbState;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// A model entry from the database
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct ModelEntry {
    pub id: i64,
    pub model: String,
    pub display_name: String,
    pub description: String,
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
}

/// Get all models from the database
#[tauri::command]
pub fn get_models(state: tauri::State<DbState>) -> Result<Vec<ModelEntry>, String> {
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let mut stmt = conn
        .prepare(
            "SELECT id, model, display_name, description, is_cloud, is_premium, daily_limit, color, badge_label, badge_variant, badge_class, icon, icon_color, icon_bg FROM models ORDER BY id",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let models = stmt
        .query_map([], |row| {
            Ok(ModelEntry {
                id: row.get(0)?,
                model: row.get(1)?,
                display_name: row.get(2)?,
                description: row.get(3)?,
                is_cloud: row.get::<_, i32>(4)? != 0,
                is_premium: row.get::<_, i32>(5)? != 0,
                daily_limit: row.get(6)?,
                color: row.get(7)?,
                badge_label: row.get(8)?,
                badge_variant: row.get(9)?,
                badge_class: row.get(10)?,
                icon: row.get(11)?,
                icon_color: row.get(12)?,
                icon_bg: row.get(13)?,
            })
        })
        .map_err(|e| format!("Failed to query models: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect models: {}", e))?;

    Ok(models)
}
