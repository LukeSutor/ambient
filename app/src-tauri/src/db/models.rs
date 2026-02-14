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
    pub id: i64,
    /// Unique model key (e.g. "qwen3vl-2b", "gemini-3-flash", or user-provided).
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
    pub is_premium: bool,
    pub daily_limit: Option<i32>,
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
    /// The model identifier sent in API requests (e.g. "gpt-4o", "claude-3-5-sonnet-20241022").
    /// For internal models this is NULL since they use their own routing.
    pub model_id: Option<String>,
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
        is_premium: row.get::<_, i32>(7)? != 0,
        daily_limit: row.get(8)?,
        is_enabled: row.get::<_, i32>(9)? != 0,
        is_internal: row.get::<_, i32>(10)? != 0,
        api_url: row.get(11)?,
        api_key: row.get(12)?,
        request_format: row.get(13)?,
        model_id: row.get(14)?,
    })
}

const SELECT_COLS: &str = "id, model, display_name, short_description, description, provider, \
     is_cloud, is_premium, daily_limit, is_enabled, is_internal, api_url, api_key, request_format, model_id";

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

/// Look up a single model by its unique key.
/// Used by the LLM client to determine provider routing.
pub fn get_model_by_key(app_handle: &tauri::AppHandle, model_key: &str) -> Result<ModelEntry, String> {
    let state = app_handle.state::<DbState>();
    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    let query = format!("SELECT {} FROM models WHERE model = ?1", SELECT_COLS);
    conn.query_row(&query, params![model_key], model_from_row)
        .map_err(|e| format!("Model '{}' not found: {}", model_key, e))
}

/// Toggle a model's enabled state.
///
/// Returns the model key that should now be selected (may differ from current
/// selection if the disabled model was the active one). Returns `None` if
/// the selection doesn't need to change.
#[tauri::command]
pub async fn toggle_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, DbState>,
    model_key: String,
    enabled: bool,
    allowed_models: Vec<String>,
) -> Result<Option<String>, String> {
    let fallback_model = {
        let db_guard = state.0.lock().unwrap();
        let conn = db_guard
            .as_ref()
            .ok_or("Database connection not available")?;

        // If disabling, ensure at least one other *allowed* model stays enabled.
        // allowed_models contains model keys the user can actually access
        // (based on their tier). This prevents auto-selecting a model the
        // user doesn't have permission to use.
        if !enabled {
            if allowed_models.is_empty() {
                return Err("Cannot disable the last enabled model".to_string());
            }

            let placeholders: String = allowed_models
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 1))
                .collect::<Vec<_>>()
                .join(",");

            let query = format!(
                "SELECT COUNT(*) FROM models WHERE is_enabled = 1 AND model IN ({})",
                placeholders,
            );

            let mut stmt = conn.prepare(&query)
                .map_err(|e| format!("Failed to prepare count query: {}", e))?;

            let params: Vec<&dyn rusqlite::types::ToSql> = allowed_models
                .iter()
                .map(|s| s as &dyn rusqlite::types::ToSql)
                .collect();

            let allowed_enabled_count: i32 = stmt
                .query_row(params.as_slice(), |row| row.get(0))
                .map_err(|e| format!("Failed to count enabled allowed models: {}", e))?;

            // If disabling this model would leave zero allowed-and-enabled
            // models, block the operation
            let is_allowed = allowed_models.contains(&model_key);
            let would_remain = if is_allowed {
                allowed_enabled_count - 1
            } else {
                allowed_enabled_count
            };

            if would_remain < 1 {
                return Err("Cannot disable the last enabled model".to_string());
            }
        }

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

        // If disabling, find a fallback from allowed models only
        if !enabled && !allowed_models.is_empty() {
            let placeholders: String = allowed_models
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 2)) // +2 because ?1 is model_key
                .collect::<Vec<_>>()
                .join(",");

            let query = format!(
                "SELECT model FROM models WHERE is_enabled = 1 AND model != ?1 AND model IN ({}) ORDER BY is_cloud ASC, id ASC LIMIT 1",
                placeholders,
            );

            let mut stmt = conn.prepare(&query)
                .map_err(|e| format!("Failed to prepare fallback query: {}", e))?;

            let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::with_capacity(1 + allowed_models.len());
            params.push(Box::new(model_key.clone()));
            for m in &allowed_models {
                params.push(Box::new(m.clone()));
            }
            let params_ref: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

            let fallback: Option<String> = stmt
                .query_row(params_ref.as_slice(), |row| row.get(0))
                .ok();
            fallback
        } else if !enabled {
            // No allowed_models provided — fall back to any enabled model
            let fallback: Option<String> = conn
                .query_row(
                    "SELECT model FROM models WHERE is_enabled = 1 AND model != ?1 ORDER BY is_cloud ASC, id ASC LIMIT 1",
                    rusqlite::params![model_key],
                    |row| row.get(0),
                )
                .ok();
            fallback
        } else {
            None
        }
    };

    // If disabling the currently selected model, switch to the fallback
    let mut switched_to: Option<String> = None;
    if !enabled {
        if let Some(ref fallback) = fallback_model {
            let settings = crate::settings::service::load_user_settings(app_handle.clone())
                .await
                .unwrap_or_default();

            if settings.model_selection.0 == model_key {
                let mut updated_settings = settings;
                updated_settings.model_selection = crate::settings::types::ModelSelection(fallback.clone());

                if let Err(e) = crate::settings::service::save_user_settings(
                    app_handle.clone(),
                    updated_settings,
                ).await {
                    log::warn!("[models] Failed to auto-switch model selection: {}", e);
                } else {
                    log::info!("[models] Auto-switched model selection from '{}' to '{}'", model_key, fallback);
                    switched_to = Some(fallback.clone());

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
    pub model_id: String,
    /// Full API endpoint URL.
    pub api_url: String,
    /// API authentication key. Empty string treated as None (for localhost models).
    pub api_key: String,
    /// Request format: "openai", "gemini", or "anthropic".
    pub request_format: String,
    /// Provider for display icon (e.g. "openai", "google", "deepseek", "unknown").
    pub provider: String,
    /// Optional display name (max 40 chars). Falls back to model_id.
    pub display_name: String,
}

/// Parameters for updating a custom (BYOK) model.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "models.ts")]
pub struct UpdateCustomModelParams {
    /// The current model key (used to find the row).
    pub model_key: String,
    /// Updated model identifier for API requests.
    pub model_id: String,
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
/// The model key in the DB is set to `model_id`. The `display_name` defaults
/// to `model_id` if left empty and is truncated to 40 characters.
#[tauri::command]
pub fn add_custom_model(
    _app_handle: tauri::AppHandle,
    state: tauri::State<DbState>,
    params: AddCustomModelParams,
) -> Result<String, String> {
    // Validate request_format
    if !["openai", "gemini", "anthropic"].contains(&params.request_format.as_str()) {
        return Err(format!("Invalid request format: {}", params.request_format));
    }

    if params.model_id.trim().is_empty() {
        return Err("Model ID is required".to_string());
    }

    if params.api_url.trim().is_empty() {
        return Err("API URL is required".to_string());
    }

    // Build display name
    let display_name = if params.display_name.trim().is_empty() {
        params.model_id.clone()
    } else {
        params.display_name.clone()
    };
    let display_name = truncate_display_name(&display_name);

    // Model key = model_id (unique in DB)
    let model_key = params.model_id.trim().to_string();

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
         is_cloud, is_premium, is_enabled, daily_limit, is_internal, api_url, api_key, \
         request_format, model_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, 1, 0, 1, NULL, 0, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            model_key,
            display_name,
            "Custom model added by you.",
            "Custom model added by you.",
            params.provider.trim(),
            params.api_url.trim(),
            api_key,
            params.request_format.trim(),
            params.model_id.trim(),
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            format!("A model with key '{}' already exists", model_key)
        } else {
            format!("Failed to add custom model: {}", e)
        }
    })?;

    log::info!("[models] Added custom model '{}' (provider: {}, format: {})",
        model_key, params.provider, params.request_format);

    let _ = emit(
        MODELS_CHANGED,
        ModelsChangedEvent {
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    );

    Ok(model_key)
}

/// Update an existing custom BYOK model. Internal models cannot be edited.
#[tauri::command]
pub fn update_custom_model(
    app_handle: tauri::AppHandle,
    state: tauri::State<DbState>,
    params: UpdateCustomModelParams,
) -> Result<(), String> {
    if !["openai", "gemini", "anthropic"].contains(&params.request_format.as_str()) {
        return Err(format!("Invalid request format: {}", params.request_format));
    }

    if params.model_id.trim().is_empty() {
        return Err("Model ID is required".to_string());
    }

    if params.api_url.trim().is_empty() {
        return Err("API URL is required".to_string());
    }

    let display_name = if params.display_name.trim().is_empty() {
        params.model_id.clone()
    } else {
        params.display_name.clone()
    };
    let display_name = truncate_display_name(&display_name);

    let api_key = if params.api_key.trim().is_empty() {
        None
    } else {
        Some(params.api_key.trim().to_string())
    };

    let new_model_key = params.model_id.trim().to_string();

    let db_guard = state.0.lock().unwrap();
    let conn = db_guard
        .as_ref()
        .ok_or("Database connection not available")?;

    // Verify it's a custom model
    let is_internal: bool = conn
        .query_row(
            "SELECT is_internal FROM models WHERE model = ?1",
            params![params.model_key],
            |row| row.get::<_, i32>(0).map(|v| v != 0),
        )
        .map_err(|e| format!("Model '{}' not found: {}", params.model_key, e))?;

    if is_internal {
        return Err("Internal models cannot be edited".to_string());
    }

    // Update the model — also update the model key if the model_id changed
    conn.execute(
        "UPDATE models SET model = ?1, display_name = ?2, provider = ?3, api_url = ?4, \
         api_key = ?5, request_format = ?6, model_id = ?7 WHERE model = ?8",
        rusqlite::params![
            new_model_key,
            display_name,
            params.provider.trim(),
            params.api_url.trim(),
            api_key,
            params.request_format.trim(),
            params.model_id.trim(),
            params.model_key,
        ],
    )
    .map_err(|e| {
        if e.to_string().contains("UNIQUE constraint failed") {
            format!("A model with key '{}' already exists", new_model_key)
        } else {
            format!("Failed to update model: {}", e)
        }
    })?;

    // If the model key changed, update settings if this was the selected model
    let old_key = params.model_key.clone();
    if old_key != new_model_key {
        let settings_handle = app_handle.clone();
        let new_key = new_model_key.clone();
        tauri::async_runtime::spawn(async move {
            if let Ok(settings) = crate::settings::service::load_user_settings(settings_handle.clone()).await {
                if settings.model_selection.0 == old_key {
                    let mut updated = settings;
                    updated.model_selection = crate::settings::types::ModelSelection(new_key);
                    if let Err(e) = crate::settings::service::save_user_settings(settings_handle.clone(), updated).await {
                        log::warn!("[models] Failed to update model selection after rename: {}", e);
                    } else {
                        let _ = settings_handle.emit("settings_changed", ());
                    }
                }
            }
        });
    }

    log::info!("[models] Updated custom model '{}'", params.model_key);

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
    model_key: String,
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
                "SELECT is_internal FROM models WHERE model = ?1",
                params![model_key],
                |row| row.get::<_, i32>(0).map(|v| v != 0),
            )
            .map_err(|e| format!("Model '{}' not found: {}", model_key, e))?;

        if is_internal {
            return Err("Internal models cannot be deleted".to_string());
        }

        conn.execute("DELETE FROM models WHERE model = ?1 AND is_internal = 0", params![model_key])
            .map_err(|e| format!("Failed to delete model: {}", e))?;

        log::info!("[models] Deleted custom model '{}'", model_key);
    }
    // MutexGuard dropped here — safe to await below

    // Phase 2: Check if deleted model was selected and fix up settings
    let settings = crate::settings::service::load_user_settings(app_handle.clone())
        .await
        .unwrap_or_default();

    if settings.model_selection.0 == model_key {
        let fallback = {
            let db_guard = state.0.lock().unwrap();
            let conn = db_guard
                .as_ref()
                .ok_or("Database connection not available")?;

            conn.query_row(
                "SELECT model FROM models WHERE is_enabled = 1 ORDER BY is_internal DESC, is_cloud ASC, id ASC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
        };

        if let Some(fallback_key) = fallback {
            let mut updated = settings;
            updated.model_selection = crate::settings::types::ModelSelection(fallback_key);
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
