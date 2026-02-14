use super::types::UserSettings;
use crate::constants::{PROFILES_DIR, SETTINGS_KEY, USER_STORE_FILENAME};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_store::{Store, StoreExt};

/// Get the per-user store path for the currently authenticated user.
///
/// Returns `profiles/{user_id}/store.json` — a path relative to `app_data_dir`
/// that `tauri_plugin_store` will resolve automatically.
///
/// Returns `None` if no user is logged in.
fn get_user_store_path() -> Option<String> {
  let user_id = crate::auth::storage::get_current_user_id().ok()??;
  Some(format!("{}/{}/{}", PROFILES_DIR, user_id, USER_STORE_FILENAME))
}

/// Ensure the user's profile directory exists so tauri_plugin_store can save.
fn ensure_profile_dir(app_handle: &AppHandle, user_id: &str) -> Result<(), String> {
  let app_data_path = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
  let profile_dir = app_data_path.join(PROFILES_DIR).join(user_id);
  std::fs::create_dir_all(&profile_dir)
    .map_err(|e| format!("Failed to create profile directory: {}", e))
}

/// Get the store instance for the current user's settings.
///
/// If no user is logged in, returns an error. Each user's settings are
/// stored in their own profile directory.
fn get_user_settings_store(
  app_handle: &AppHandle,
) -> Result<std::sync::Arc<Store<tauri::Wry>>, String> {
  let store_path = get_user_store_path()
    .ok_or("No active session. Cannot access user settings.")?;
  app_handle
    .store(&store_path)
    .map_err(|e| format!("Failed to get user settings store: {}", e))
}

/// Load settings from the current user's store.
///
/// Returns defaults if the user is not logged in or if no settings have been saved yet.
async fn load_settings_internal(app_handle: &AppHandle) -> Result<UserSettings, String> {
  let store = match get_user_settings_store(app_handle) {
    Ok(s) => s,
    Err(_) => {
      // Not logged in — return defaults silently
      return Ok(UserSettings::default());
    }
  };

  let settings = match store.get(SETTINGS_KEY) {
    Some(value) => {
      serde_json::from_value(value.clone()).unwrap_or_else(|_| UserSettings::default())
    }
    None => UserSettings::default(),
  };
  Ok(settings)
}

/// Save settings to the current user's store.
async fn save_settings_internal(
  app_handle: &AppHandle,
  settings: &UserSettings,
) -> Result<(), String> {
  // Get user ID and ensure profile directory exists
  let user_id = crate::auth::storage::get_current_user_id()
    .map_err(|e| format!("Failed to retrieve user ID: {}", e))?
    .ok_or("No active session. Cannot save settings.")?;
  ensure_profile_dir(app_handle, &user_id)?;

  let store = get_user_settings_store(app_handle)?;

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
