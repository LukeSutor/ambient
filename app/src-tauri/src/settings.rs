use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_store::{Store, StoreExt};
use once_cell::sync::Lazy;

// Cache for frequently accessed settings
static SETTINGS_CACHE: Lazy<Mutex<Option<UserSettings>>> = Lazy::new(|| Mutex::new(None));

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum HudSizeOption {
    Small,
    Normal, 
    Large,
}

impl Default for HudSizeOption {
    fn default() -> Self {
        Self::Normal
    }
}

impl HudSizeOption {
    pub fn to_dimensions(&self) -> HudDimensions {
        match self {
            Self::Small => HudDimensions {
                width: 400.0,
                collapsed_height: 50.0,
                expanded_height: 250.0,
            },
            Self::Normal => HudDimensions {
                width: 500.0,
                collapsed_height: 60.0,
                expanded_height: 350.0,
            },
            Self::Large => HudDimensions {
                width: 600.0,
                collapsed_height: 70.0,
                expanded_height: 450.0,
            },
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Normal => "normal",
            Self::Large => "large",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s {
            "small" => Self::Small,
            "large" => Self::Large,
            _ => Self::Normal, // Default fallback
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HudDimensions {
    pub width: f64,
    pub collapsed_height: f64,
    pub expanded_height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    pub hud_size: HudSizeOption,
    // Future extensible settings can be added here
    // pub theme: String,
    // pub auto_start: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            hud_size: HudSizeOption::default(),
        }
    }
}

const SETTINGS_STORE_PATH: &str = "user-settings.json";
const SETTINGS_KEY: &str = "settings";

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

/// Load user settings (cached for performance)
#[tauri::command]
pub async fn load_user_settings(app_handle: AppHandle) -> Result<UserSettings, String> {
    load_settings_internal(&app_handle).await
}

/// Save user settings
#[tauri::command] 
pub async fn save_user_settings(app_handle: AppHandle, settings: UserSettings) -> Result<(), String> {
    save_settings_internal(&app_handle, &settings).await
}

/// Clear settings cache
#[tauri::command]
pub async fn refresh_settings_cache() -> Result<(), String> {
    let mut cache = SETTINGS_CACHE.lock().unwrap();
    *cache = None;
    Ok(())
}
