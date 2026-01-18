use super::types::UserSettings;
use crate::constants::{SETTINGS_KEY, STORE_PATH};
use tauri::{AppHandle, Emitter};
use tauri_plugin_store::{Store, StoreExt};

/// Get the store instance for settings
async fn get_settings_store(
  app_handle: &AppHandle,
) -> Result<std::sync::Arc<Store<tauri::Wry>>, String> {
  app_handle
    .store(STORE_PATH)
    .map_err(|e| format!("Failed to get settings store: {}", e))
}

/// Load settings from store with caching
async fn load_settings_internal(app_handle: &AppHandle) -> Result<UserSettings, String> {
  // Load from store
  let store = get_settings_store(app_handle).await?;

  let settings = match store.get(SETTINGS_KEY) {
    Some(value) => {
      serde_json::from_value(value.clone()).unwrap_or_else(|_| UserSettings::default())
    }
    None => UserSettings::default(),
  };
  Ok(settings)
}

/// Save settings to store and update cache
async fn save_settings_internal(
  app_handle: &AppHandle,
  settings: &UserSettings,
) -> Result<(), String> {
  let store = get_settings_store(app_handle).await?;

  let value =
    serde_json::to_value(settings).map_err(|e| format!("Failed to serialize settings: {}", e))?;

  store.set(SETTINGS_KEY, value);
  store
    .save()
    .map_err(|e| format!("Failed to save settings: {}", e))?;

  Ok(())
}

#[tauri::command]
pub async fn load_user_settings(app_handle: AppHandle) -> Result<UserSettings, String> {
  load_settings_internal(&app_handle).await
}

#[tauri::command]
pub async fn save_user_settings(
  app_handle: AppHandle,
  settings: UserSettings,
) -> Result<(), String> {
  save_settings_internal(&app_handle, &settings).await
}

#[tauri::command]
pub async fn emit_settings_changed(app_handle: AppHandle) -> Result<(), String> {
  app_handle
    .emit("settings_changed", ())
    .map_err(|e| format!("Failed to emit settings_changed event: {}", e))
}
