use crate::auth::types::{StoredAuthState, Session};
use crate::constants::{AUTH_KEY, STORE_PATH, KEYRING_ENCRYPTION_KEY, KEYRING_AUTH_KEY, KEYRING_SERVICE};
use keyring::Entry;
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce
};
use rand::RngCore;
use base64::{prelude::BASE64_STANDARD, Engine};

#[derive(Serialize, Deserialize)]
struct TokenData {
    access_token: String,
    refresh_token: String,
}

fn get_app_handle() -> Option<AppHandle> {
    crate::events::get_emitter().get_app_handle()
}

/// Get or create an encryption key in the keyring
fn get_or_create_encryption_key() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_ENCRYPTION_KEY)?;
    
    match entry.get_password() {
        Ok(key_base64) => {
            let key = BASE64_STANDARD.decode(key_base64)?;
            if key.len() == 32 {
                return Ok(key);
            }
            log::warn!("[auth_storage] Invalid key length in keyring, generating new key");
        }
        Err(keyring::Error::NoEntry) => {
            log::info!("[auth_storage] No encryption key found, generating new one");
        }
        Err(e) => return Err(Box::new(e)),
    }

    // Generate a new 32-byte key
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    let key_base64 = BASE64_STANDARD.encode(key);
    entry.set_password(&key_base64)?;
    Ok(key.to_vec())
}

/// Store the complete auth state (session with tokens)
pub fn store_auth_state(state: &StoredAuthState) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = get_app_handle().ok_or("AppHandle not initialized")?;
    
    // Get encryption key from keyring
    let key_bytes = get_or_create_encryption_key()?;
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    
    // Encrypt sensitive tokens
    let token_data = TokenData {
        access_token: state.session.access_token.clone(),
        refresh_token: state.session.refresh_token.clone(),
    };
    let token_json = serde_json::to_string(&token_data)?;
    
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    let ciphertext = cipher.encrypt(nonce, token_json.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;
    
    // Combine nonce + ciphertext and base64 encode
    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    let encrypted_tokens_base64 = BASE64_STANDARD.encode(combined);
    
    // Prepare non-sensitive state for JSON storage (clear tokens, add encrypted blob)
    let mut store_data = serde_json::to_value(state)?;
    if let Some(obj) = store_data.as_object_mut() {
        // Clear tokens from the session object inside the JSON
        if let Some(session) = obj.get_mut("session").and_then(|s| s.as_object_mut()) {
            session.insert("access_token".to_string(), serde_json::Value::String(String::new()));
            session.insert("refresh_token".to_string(), serde_json::Value::String(String::new()));
        }
        // Add the encrypted tokens
        obj.insert("encrypted_tokens".to_string(), serde_json::Value::String(encrypted_tokens_base64));
    }
    
    // Store in tauri store
    let store = app_handle.store(STORE_PATH)?;
    store.set(AUTH_KEY, store_data);
    let _ = store.save();
    
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
    let app_handle = match get_app_handle() {
        Some(h) => h,
        None => return Ok(None),
    };

    // Try to get metadata from tauri store
    let store = app_handle.store(STORE_PATH)?;
    let auth_val = store.get(AUTH_KEY);
    
    if let Some(val) = auth_val {
        // Try to deserialize the stored state
        let mut state: StoredAuthState = match serde_json::from_value(val.clone()) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("[auth_storage] Failed to deserialize stored auth state: {}. Clearing corrupted data.", e);
                // Clear corrupted state
                let _ = clear_auth_state();
                return Ok(None);
            }
        };
        
        // Check for encrypted tokens in the JSON
        if let Some(encrypted_tokens_base64) = val.get("encrypted_tokens").and_then(|v| v.as_str()) {
            match decrypt_tokens(encrypted_tokens_base64) {
                Ok(token_data) => {
                    state.session.access_token = token_data.access_token;
                    state.session.refresh_token = token_data.refresh_token;
                    return Ok(Some(state));
                }
                Err(e) => {
                    log::warn!("[auth_storage] Failed to decrypt tokens: {}. Clearing auth state.", e);
                    // Decryption failed - encryption key might have been lost
                    // Clear the corrupted/unusable state and force re-authentication
                    let _ = clear_auth_state();
                    return Ok(None);
                }
            }
        }
    }
    
    Ok(None)
}

/// Helper function to decrypt tokens
fn decrypt_tokens(encrypted_tokens_base64: &str) -> Result<TokenData, Box<dyn std::error::Error>> {
    let combined = BASE64_STANDARD.decode(encrypted_tokens_base64)?;
    if combined.len() < 12 {
        return Err("Invalid encrypted token data: too short".into());
    }
    
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    let key_bytes = get_or_create_encryption_key()?;
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    
    let decrypted_bytes = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {}", e))?;
    
    let token_data: TokenData = serde_json::from_slice(&decrypted_bytes)?;
    
    // Validate decrypted tokens are not empty (additional safety check)
    if token_data.access_token.is_empty() || token_data.refresh_token.is_empty() {
        return Err("Decrypted tokens are empty".into());
    }
    
    Ok(token_data)
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

/// Clear all stored auth data
pub fn clear_auth_state() -> Result<(), Box<dyn std::error::Error>> {
    // Clear the keyring tokens
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_AUTH_KEY)?;
    let _ = entry.delete_credential();
    
    // Clear from tauri store
    if let Some(app_handle) = get_app_handle() {
        if let Ok(store) = app_handle.store(STORE_PATH) {
            store.delete(AUTH_KEY);
            let _ = store.save();
        }
    }
    
    log::info!("[auth_storage] Auth state cleared successfully");
    Ok(())
}