use crate::auth::types::{StoredAuthState, Session, KEYRING_SERVICE, KEYRING_AUTH_KEY};
use keyring::Entry;
use std::fs;
use std::path::PathBuf;

/// Store the complete auth state (session with tokens)
pub fn store_auth_state(state: &StoredAuthState) -> Result<(), Box<dyn std::error::Error>> {
    let app_data_dir = get_app_data_dir()?;
    let auth_file_path = app_data_dir.join("auth_state.json");
    
    // Serialize the auth state
    let auth_json = serde_json::to_string(state)?;
    
    // Write to file
    fs::write(&auth_file_path, auth_json)?;
    
    // Store a flag in keyring to indicate we have auth stored
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_AUTH_KEY)?;
    entry.set_password("active")?;
    
    log::info!("[auth_storage] Auth state stored successfully");
    Ok(())
}

/// Store a session (creates a new StoredAuthState)
pub fn store_session(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    let state = StoredAuthState::new(session.clone());
    store_auth_state(&state)
}

/// Retrieve the stored auth state
pub fn retrieve_auth_state() -> Result<Option<StoredAuthState>, Box<dyn std::error::Error>> {
    // First check if we have auth stored
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_AUTH_KEY)?;
    match entry.get_password() {
        Ok(status) if status == "active" => {
            // We have auth, try to read from file
            let app_data_dir = get_app_data_dir()?;
            let auth_file_path = app_data_dir.join("auth_state.json");
            
            if auth_file_path.exists() {
                let auth_json = fs::read_to_string(&auth_file_path)?;
                let state: StoredAuthState = serde_json::from_str(&auth_json)?;
                Ok(Some(state))
            } else {
                // File doesn't exist, clear the flag and return None
                let _ = entry.delete_password();
                Ok(None)
            }
        }
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

/// Get the current session if valid
pub fn get_current_session() -> Result<Option<Session>, Box<dyn std::error::Error>> {
    match retrieve_auth_state()? {
        Some(state) => Ok(Some(state.session)),
        None => Ok(None),
    }
}

/// Get the access token if valid
pub fn get_access_token() -> Result<Option<String>, Box<dyn std::error::Error>> {
    match retrieve_auth_state()? {
        Some(state) => {
            if state.is_access_token_expired() {
                log::info!("[auth_storage] Access token is expired");
                Ok(None)
            } else {
                Ok(Some(state.session.access_token))
            }
        }
        None => Ok(None),
    }
}

/// Get the refresh token
pub fn get_refresh_token() -> Result<Option<String>, Box<dyn std::error::Error>> {
    match retrieve_auth_state()? {
        Some(state) => Ok(Some(state.session.refresh_token)),
        None => Ok(None),
    }
}

/// Check if the session needs to be refreshed
pub fn needs_token_refresh() -> Result<bool, Box<dyn std::error::Error>> {
    match retrieve_auth_state()? {
        Some(state) => Ok(state.needs_refresh()),
        None => Ok(false),
    }
}

/// Clear all stored auth data
pub fn clear_auth_state() -> Result<(), Box<dyn std::error::Error>> {
    // Clear the keyring flag
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_AUTH_KEY)?;
    let _ = entry.delete_password();
    
    // Remove the auth file
    let app_data_dir = get_app_data_dir()?;
    let auth_file_path = app_data_dir.join("auth_state.json");
    
    if auth_file_path.exists() {
        fs::remove_file(&auth_file_path)?;
    }
    
    // Also clear the old auth.json if it exists (migration cleanup)
    let old_auth_file = app_data_dir.join("auth.json");
    if old_auth_file.exists() {
        let _ = fs::remove_file(&old_auth_file);
    }
    
    log::info!("[auth_storage] Auth state cleared successfully");
    Ok(())
}

/// Update the session after a token refresh
pub fn update_session(session: &Session) -> Result<(), Box<dyn std::error::Error>> {
    store_session(session)
}

/// Helper function to get app data directory
fn get_app_data_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let app_data_dir = if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(|path| std::path::PathBuf::from(path))
            .unwrap_or_else(|_| {
                std::env::var("USERPROFILE")
                    .map(|path| {
                        std::path::PathBuf::from(path)
                            .join("AppData")
                            .join("Roaming")
                    })
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
            })
            .join("local-computer-use")
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .map(|path| {
                std::path::PathBuf::from(path)
                    .join("Library")
                    .join("Application Support")
                    .join("local-computer-use")
            })
            .unwrap_or_else(|_| std::path::PathBuf::from(".").join("local-computer-use"))
    } else {
        // Linux
        std::env::var("HOME")
            .map(|path| {
                std::path::PathBuf::from(path)
                    .join(".local")
                    .join("share")
                    .join("local-computer-use")
            })
            .unwrap_or_else(|_| std::path::PathBuf::from(".").join("local-computer-use"))
    };
    
    // Create directory if it doesn't exist
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir)?;
    }
    
    Ok(app_data_dir)
}