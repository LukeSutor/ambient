use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_store::{Store, StoreExt};
use once_cell::sync::Lazy;
use crate::constants::{SETTINGS_STORE_PATH, SETTINGS_KEY};
use super::types::UserSettings;

// Cache for frequently accessed settings
static SETTINGS_CACHE: Lazy<Mutex<Option<UserSettings>>> = Lazy::new(|| Mutex::new(None));

/// Get the store instance for settings
async fn get_settings_store(app_handle: &AppHandle) -> Result<std::sync::Arc<Store<tauri::Wry>>, String> {
    app_handle
        .store(SETTINGS_STORE_PATH)
        .map_err(|e| format!("Failed to get settings store: {}", e))
}

/// Load settings from store with caching
async fn load_settings_internal(app_handle: &AppHandle) -> Result<UserSettings, String> {
    // Check cache first
    {
        let cache = SETTINGS_CACHE.lock().unwrap();
        if let Some(settings) = cache.as_ref() {
            return Ok(settings.clone());
        }
    }

    // Load from store
    let store = get_settings_store(app_handle).await?;
    
    let settings = match store.get(SETTINGS_KEY) {
        Some(value) => {
            serde_json::from_value(value.clone())
                .unwrap_or_else(|_| UserSettings::default())
        }
        None => UserSettings::default(),
    };

    // Update cache
    {
        let mut cache = SETTINGS_CACHE.lock().unwrap();
        *cache = Some(settings.clone());
    }

    Ok(settings)
}

/// Save settings to store and update cache
async fn save_settings_internal(app_handle: &AppHandle, settings: &UserSettings) -> Result<(), String> {
    let store = get_settings_store(app_handle).await?;
    
    let value = serde_json::to_value(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;
    
    store.set(SETTINGS_KEY, value);
    store.save()
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    // Update cache
    {
        let mut cache = SETTINGS_CACHE.lock().unwrap();
        *cache = Some(settings.clone());
    }

    Ok(())
}

#[tauri::command]
pub async fn load_user_settings(app_handle: AppHandle) -> Result<UserSettings, String> {
    load_settings_internal(&app_handle).await
}

#[tauri::command] 
pub async fn save_user_settings(app_handle: AppHandle, settings: UserSettings) -> Result<(), String> {
    save_settings_internal(&app_handle, &settings).await
}

#[tauri::command]
pub async fn refresh_settings_cache() -> Result<(), String> {
    let mut cache = SETTINGS_CACHE.lock().unwrap();
    *cache = None;
    Ok(())
}
