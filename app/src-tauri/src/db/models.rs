use crate::db::core::DbState;
use crate::events::{emit, MODELS_CHANGED, ModelsChangedEvent};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use ts_rs::TS;

/// Maximum length for user-provided display names.
const MAX_DISPLAY_NAME_LEN: usize = 40;

/// A model entry from the database.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct ModelEntry {
    #[ts(type = "number")]
    pub id: i64,
    /// Model identifier sent in API requests (e.g. "qwen3vl-2b", "gpt-4o").
    /// NOT unique — use `id` for lookups.
    pub model: String,
    /// User-friendly name shown in the UI (e.g. "Local", "Gemini 3 Flash").
    pub display_name: String,
    /// Short one-liner for compact contexts like the HUD dropdown.
    pub short_description: String,
    /// Longer description for the full settings page model selector.
    pub description: String,
    /// The model provider for display (e.g. "local", "google", "openai", "deepseek").
    /// Resolves to a provider image in the UI: `providers/{provider}.png`.
    pub provider: String,
    pub is_cloud: bool,
    /// Whether this model is enabled/visible in the UI.
    pub is_enabled: bool,
    /// Whether this is an internal (built-in) model vs. user-added BYOK model.
    pub is_internal: bool,
    /// API endpoint URL for BYOK models (e.g. "https://api.openai.com/v1/chat/completions").
    pub api_url: Option<String>,
    /// API key for BYOK models. Stored encrypted in SQLCipher DB. Optional for localhost models.
    pub api_key: Option<String>,
    /// Request format: "openai", "gemini", or "anthropic". Determines which provider to route to.
    pub request_format: String,
}

/// Helper to build a ModelEntry from a row with the standard column order.
fn model_from_row(row: &rusqlite::Row) -> rusqlite::Result<ModelEntry> {
    Ok(ModelEntry {
        id: row.get(0)?,
        model: row.get(1)?,
        display_name: row.get(2)?,
        short_description: row.get(3)?,
        description: row.get(4)?,
        provider: row.get(5)?,
        is_cloud: row.get::<_, i32>(6)? != 0,
        is_enabled: row.get::<_, i32>(7)? != 0,
        is_internal: row.get::<_, i32>(8)? != 0,
        api_url: row.get(9)?,
        api_key: row.get(10)?,
        request_format: row.get(11)?,
    })
}

const SELECT_COLS: &str = "id, model, display_name, short_description, description, provider, \
     is_cloud, is_enabled, is_internal, api_url, api_key, request_format";

/// Get all models from the database.
#[tauri::command]
pub fn get_models(state: tauri::State<DbState>) -> Result<Vec<ModelEntry>, String> {
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let query = format!("SELECT {} FROM models ORDER BY is_internal DESC, id", SELECT_COLS);
    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let models = stmt
        .query_map([], model_from_row)
        .map_err(|e| format!("Failed to query models: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect models: {}", e))?;

    Ok(models)
}

/// Look up a single model by its integer primary key.
/// Used by the LLM client to determine provider routing.
pub fn get_model_by_id(app_handle: &tauri::AppHandle, id: i64) -> Result<ModelEntry, String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let query = format!("SELECT {} FROM models WHERE id = ?1", SELECT_COLS);
    conn.query_row(&query, params![id], model_from_row)
        .map_err(|e| format!("Model id {} not found: {}", id, e))
}

/// Toggle a model's enabled state.
///
/// Returns the model id (as string) that should now be selected (may differ from
/// current selection if the disabled model was the active one). Returns `None` if
/// the selection doesn't need to change.
#[tauri::command]
pub async fn toggle_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    model_id: i64,
    enabled: bool,
) -> Result<Option<String>, String> {
    let fallback_id = {
        let db_guard = state.0.lock().unwrap();
        let conn = db_guard
            .as_ref()
            .ok_or("Database connection not available")?;

        // When disabling, ensure at least one model stays enabled.
        if !enabled {
            let enabled_count: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM models WHERE is_enabled = 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| format!("Failed to count enabled models: {}", e))?;

            // If this is the only enabled model, block the toggle.
            let is_currently_enabled: bool = conn
                .query_row(
                    "SELECT is_enabled FROM models WHERE id = ?1",
                    rusqlite::params![model_id],
                    |row| row.get::<_, bool>(0),
                )
                .map_err(|e| format!("Model id {} not found: {}", model_id, e))?;

            if is_currently_enabled && enabled_count <= 1 {
                return Err("Cannot disable the last enabled model".to_string());
            }
        }

        let rows_affected = conn
            .execute(
                "UPDATE models SET is_enabled = ?1 WHERE id = ?2",
                rusqlite::params![enabled as i32, model_id],
            )
            .map_err(|e| format!("Failed to toggle model: {}", e))?;

        if rows_affected == 0 {
            return Err(format!("Model id {} not found", model_id));
        }

        log::info!("[models] Model id {} is_enabled set to {}", model_id, enabled);

        // If disabling, find a fallback (prefer local models, then lowest id)
        if !enabled {
            conn.query_row(
                "SELECT id FROM models WHERE is_enabled = 1 AND id != ?1 ORDER BY is_cloud ASC, id ASC LIMIT 1",
                rusqlite::params![model_id],
                |row| row.get::<_, i64>(0),
            )
            .ok()
        } else {
            None
        }
    };

    // If disabling the currently selected model, switch to the fallback
    let mut switched_to: Option<String> = None;
    if !enabled {
        if let Some(fallback) = fallback_id {
            let settings = crate::settings::service::load_user_settings(app_handle.clone())
                .await
                .unwrap_or_default();

            let model_id_str = model_id.to_string();
            if settings.model_selection.0 == model_id_str {
                let fallback_str = fallback.to_string();
                let mut updated_settings = settings;
                updated_settings.model_selection = crate::settings::types::ModelSelection(fallback_str.clone());

                if let Err(e) = crate::settings::service::save_user_settings(
                    app_handle.clone(),
                    updated_settings,
                ).await {
                    log::warn!("[models] Failed to auto-switch model selection: {}", e);
                } else {
                    log::info!("[models] Auto-switched model selection from id {} to id {}", model_id, fallback);
                    switched_to = Some(fallback_str);

                    // Notify frontend so settings provider updates
                    let _ = app_handle.emit("settings_changed", ());
                }
            }
        }
    }

    // Notify frontend listeners so model lists update in real time
    let _ = emit(
        MODELS_CHANGED,
        ModelsChangedEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(switched_to)
}

/// Parameters for adding a custom (BYOK) model.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct AddCustomModelParams {
    /// The model identifier for API requests (e.g. "gpt-4o").
    pub model: String,
    /// Full API endpoint URL.
    pub api_url: String,
    /// API authentication key. Empty string treated as None (for localhost models).
    pub api_key: String,
    /// Request format: "openai", "gemini", or "anthropic".
    pub request_format: String,
    /// Provider for display icon (e.g. "openai", "google", "deepseek", "unknown").
    pub provider: String,
    /// Optional display name (max 40 chars). Falls back to model.
    pub display_name: String,
}

/// Parameters for updating a custom (BYOK) model.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct UpdateCustomModelParams {
    /// The database id of the model to update.
    pub id: i64,
    /// Updated model identifier for API requests.
    pub model: String,
    /// Updated API endpoint URL.
    pub api_url: String,
    /// Updated API key. Empty string treated as None.
    pub api_key: String,
    /// Updated request format.
    pub request_format: String,
    /// Updated provider for display icon.
    pub provider: String,
    /// Updated display name (max 40 chars).
    pub display_name: String,
}

/// Add a custom BYOK model.
///
/// Returns the new model's database id.
#[tauri::command]
pub fn add_custom_model(
    _app_handle: tauri::AppHandle,
    state: tauri::State<DbState>,
    params: AddCustomModelParams,
) -> Result<i64, String> {
    // Validate request_format
    if !["openai", "gemini", "anthropic"].contains(&params.request_format.as_str()) {
        return Err(format!("Invalid request format: {}", params.request_format));
    }

    if params.model.trim().is_empty() {
        return Err("Model identifier is required".to_string());
    }

    if params.api_url.trim().is_empty() {
        return Err("API URL is required".to_string());
    }

    // Build display name
    let display_name = if params.display_name.trim().is_empty() {
        params.model.clone()
    } else {
        params.display_name.clone()
    };
    let display_name = truncate_display_name(&display_name);

    // API key: empty string → None
    let api_key = if params.api_key.trim().is_empty() {
        None
    } else {
        Some(params.api_key.trim().to_string())
    };

    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    conn.execute(
        "INSERT INTO models (model, display_name, short_description, description, provider, \
         is_cloud, is_enabled, is_internal, api_url, api_key, \
         request_format) \
         VALUES (?1, ?2, ?3, ?4, ?5, 1, 1, 0, ?6, ?7, ?8)",
        rusqlite::params![
            params.model.trim(),
            display_name,
            "Custom model added by you.",
            "Custom model added by you.",
            params.provider.trim(),
            params.api_url.trim(),
            api_key,
            params.request_format.trim(),
        ],
    )
    .map_err(|e| format!("Failed to add custom model: {}", e))?;

    let new_id = conn.last_insert_rowid();

    log::info!("[models] Added custom model '{}' id={} (provider: {}, format: {})",
        params.model.trim(), new_id, params.provider, params.request_format);

    let _ = emit(
        MODELS_CHANGED,
        ModelsChangedEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(new_id)
}

/// Update an existing custom BYOK model. Internal models cannot be edited.
#[tauri::command]
pub fn update_custom_model(
    _app_handle: tauri::AppHandle,
    state: tauri::State<DbState>,
    params: UpdateCustomModelParams,
) -> Result<(), String> {
    if !["openai", "gemini", "anthropic"].contains(&params.request_format.as_str()) {
        return Err(format!("Invalid request format: {}", params.request_format));
    }

    if params.model.trim().is_empty() {
        return Err("Model identifier is required".to_string());
    }

    if params.api_url.trim().is_empty() {
        return Err("API URL is required".to_string());
    }

    let display_name = if params.display_name.trim().is_empty() {
        params.model.clone()
    } else {
        params.display_name.clone()
    };
    let display_name = truncate_display_name(&display_name);

    let api_key = if params.api_key.trim().is_empty() {
        None
    } else {
        Some(params.api_key.trim().to_string())
    };

    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    // Verify it's a custom model
    let is_internal: bool = conn
        .query_row(
            "SELECT is_internal FROM models WHERE id = ?1",
            params![params.id],
            |row| row.get::<_, i32>(0).map(|v| v != 0),
        )
        .map_err(|e| format!("Model id {} not found: {}", params.id, e))?;

    if is_internal {
        return Err("Internal models cannot be edited".to_string());
    }

    conn.execute(
        "UPDATE models SET model = ?1, display_name = ?2, provider = ?3, api_url = ?4, \
         api_key = ?5, request_format = ?6 WHERE id = ?7",
        rusqlite::params![
            params.model.trim(),
            display_name,
            params.provider.trim(),
            params.api_url.trim(),
            api_key,
            params.request_format.trim(),
            params.id,
        ],
    )
    .map_err(|e| format!("Failed to update model: {}", e))?;

    log::info!("[models] Updated custom model id {}", params.id);

    let _ = emit(
        MODELS_CHANGED,
        ModelsChangedEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(())
}

/// Delete a custom BYOK model. Internal models cannot be deleted.
#[tauri::command]
pub async fn delete_custom_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    model_id: i64,
) -> Result<(), String> {
    // Phase 1: DB operations (sync, no await while holding lock)
    {
        let db_guard = state.0.lock().unwrap();
        let conn = db_guard
            .as_ref()
            .ok_or("Database connection not available")?;

        // Verify it's a custom model
        let is_internal: bool = conn
            .query_row(
                "SELECT is_internal FROM models WHERE id = ?1",
                params![model_id],
                |row| row.get::<_, i32>(0).map(|v| v != 0),
            )
            .map_err(|e| format!("Model id {} not found: {}", model_id, e))?;

        if is_internal {
            return Err("Internal models cannot be deleted".to_string());
        }

        conn.execute("DELETE FROM models WHERE id = ?1 AND is_internal = 0", params![model_id])
            .map_err(|e| format!("Failed to delete model: {}", e))?;

        log::info!("[models] Deleted custom model id {}", model_id);
    }
    // MutexGuard dropped here — safe to await below

    // Phase 2: Check if deleted model was selected and fix up settings
    let model_id_str = model_id.to_string();
    let settings = crate::settings::service::load_user_settings(app_handle.clone())
        .await
        .unwrap_or_default();

    if settings.model_selection.0 == model_id_str {
        let fallback = {
            let db_guard = state.0.lock().unwrap();
            let conn = db_guard
                .as_ref()
                .ok_or("Database connection not available")?;

            conn.query_row(
                "SELECT id FROM models WHERE is_enabled = 1 ORDER BY is_internal DESC, is_cloud ASC, id ASC LIMIT 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .ok()
        };

        if let Some(fallback_id) = fallback {
            let mut updated = settings;
            updated.model_selection = crate::settings::types::ModelSelection(fallback_id.to_string());
            if let Err(e) = crate::settings::service::save_user_settings(app_handle.clone(), updated).await {
                log::warn!("[models] Failed to auto-switch after delete: {}", e);
            } else {
                let _ = app_handle.emit("settings_changed", ());
            }
        }
    }

    let _ = emit(
        MODELS_CHANGED,
        ModelsChangedEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(())
}

/// Truncate a display name to MAX_DISPLAY_NAME_LEN characters.
fn truncate_display_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.len() > MAX_DISPLAY_NAME_LEN {
        trimmed.chars().take(MAX_DISPLAY_NAME_LEN).collect()
    } else {
        trimmed.to_string()
    }
}
