//! Auth token storage with split architecture.
//!
//! Tokens are stored in two locations based on their sensitivity and lifetime:
//!
//! - **Refresh tokens** (long-lived, high-value) → OS keyring via `keyring` crate.
//!   The keyring provides OS-level encryption (Windows Credential Manager /
//!   macOS Keychain / Linux Secret Service). Refresh tokens never touch disk
//!   in any unencrypted form.
//!
//! - **Session tokens** (short-lived access tokens) → AES-256-GCM encrypted
//!   in `store.json` via `tauri-plugin-store`. Fast to read without keyring
//!   round-trips on every request.
//!
//! - **Session metadata** (user info, expiry times) → plaintext JSON in
//!   `store.json`. Non-sensitive data that doesn't need encryption.
//!
//! The AES-256-GCM encryption key itself is stored in the OS keyring,
//! so even if `store.json` is exfiltrated the session tokens cannot be read.

use crate::auth::types::{Session, StoredAuthState};
use crate::constants::{
    AUTH_KEY, KEYRING_CURRENT_USER_ID, KEYRING_ENCRYPTION_KEY, KEYRING_GOOGLE_REFRESH,
    KEYRING_SERVICE, KEYRING_SUPABASE_REFRESH, PROFILES_DIR, USER_STORE_FILENAME,
};
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{prelude::BASE64_STANDARD, Engine};
use keyring::Entry;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_store::StoreExt;

// ============================================================================
// Internal Types
// ============================================================================

/// Access tokens encrypted in store.json.
///
/// Only short-lived session tokens live here. Refresh tokens are
/// stored exclusively in the OS keyring.
#[derive(Serialize, Deserialize)]
struct EncryptedSessionTokens {
    supabase_access_token: String,
    google_access_token: Option<String>,
}

/// Non-sensitive session metadata stored as plaintext JSON in store.json.
#[derive(Serialize, Deserialize)]
struct SessionMetadata {
    user: crate::auth::types::SupabaseUser,
    token_type: String,
    expires_in: i64,
    expires_at: Option<i64>,
    stored_at: i64,
}

// ============================================================================
// AppHandle Access
// ============================================================================

fn get_app_handle() -> Option<AppHandle> {
    crate::events::get_emitter().get_app_handle()
}

// ============================================================================
// Per-User Store Path
// ============================================================================

/// Construct the per-user store path: `profiles/{user_id}/store.json`.
///
/// This path is relative to `app_data_dir` and resolved by `tauri_plugin_store`.
fn user_store_path(user_id: &str) -> String {
    format!("{}/{}/{}", PROFILES_DIR, user_id, USER_STORE_FILENAME)
}

/// Ensure the user's profile directory exists so tauri_plugin_store can save.
fn ensure_profile_dir(app_handle: &AppHandle, user_id: &str) -> Result<(), String> {
    let app_data = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data directory: {}", e))?;
    let profile_dir = app_data.join(PROFILES_DIR).join(user_id);
    std::fs::create_dir_all(&profile_dir)
        .map_err(|e| format!("Failed to create profile directory: {}", e))
}

/// Get the current user ID from the OS keyring.
///
/// Returns `Ok(None)` if no user is currently logged in.
/// Used by other modules to determine the active user without
/// expanding the full auth state.
pub fn get_current_user_id() -> Result<Option<String>, String> {
    match keyring_get(KEYRING_CURRENT_USER_ID)? {
        Some(id) if !id.is_empty() => Ok(Some(id)),
        _ => Ok(None),
    }
}

// ============================================================================
// Keyring Operations
// ============================================================================

/// Read a value from the OS keyring. Returns `Ok(None)` if no entry exists.
fn keyring_get(name: &str) -> Result<Option<String>, String> {
    let entry = Entry::new(KEYRING_SERVICE, name)
        .map_err(|e| format!("Keyring entry error for '{}': {}", name, e))?;

    match entry.get_password() {
        Ok(val) => Ok(Some(val)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Keyring read error for '{}': {}", name, e)),
    }
}

/// Write a value to the OS keyring.
fn keyring_set(name: &str, value: &str) -> Result<(), String> {
    let entry = Entry::new(KEYRING_SERVICE, name)
        .map_err(|e| format!("Keyring entry error for '{}': {}", name, e))?;

    entry
        .set_password(value)
        .map_err(|e| format!("Keyring write error for '{}': {}", name, e))
}

/// Delete a value from the OS keyring. Silently ignores missing entries.
fn keyring_delete(name: &str) {
    if let Ok(entry) = Entry::new(KEYRING_SERVICE, name) {
        let _ = entry.delete_credential();
    }
}

// ============================================================================
// AES-256-GCM Encryption
// ============================================================================

/// Get or create the AES-256-GCM encryption key in the OS keyring.
///
/// The key is 32 bytes, base64-encoded for keyring storage.
/// If no key exists or the stored key is invalid, a new one is generated.
fn get_or_create_encryption_key() -> Result<Vec<u8>, String> {
    if let Some(key_b64) = keyring_get(KEYRING_ENCRYPTION_KEY)? {
        let key = BASE64_STANDARD
            .decode(&key_b64)
            .map_err(|e| format!("Invalid encryption key encoding: {}", e))?;
        if key.len() == 32 {
            return Ok(key);
        }
        log::warn!("[auth_storage] Invalid key length in keyring, generating new key");
    }

    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    let key_b64 = BASE64_STANDARD.encode(key);
    keyring_set(KEYRING_ENCRYPTION_KEY, &key_b64)?;
    Ok(key.to_vec())
}

/// Encrypt data with AES-256-GCM. Returns base64-encoded `nonce || ciphertext`.
fn encrypt_bytes(plaintext: &[u8]) -> Result<String, String> {
    let key_bytes = get_or_create_encryption_key()?;
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("AES encryption failed: {}", e))?;

    let mut combined = nonce_bytes.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(BASE64_STANDARD.encode(combined))
}

/// Decrypt AES-256-GCM data from base64-encoded `nonce || ciphertext`.
fn decrypt_bytes(encrypted_b64: &str) -> Result<Vec<u8>, String> {
    let combined = BASE64_STANDARD
        .decode(encrypted_b64)
        .map_err(|e| format!("Invalid base64 in encrypted data: {}", e))?;

    if combined.len() < 12 {
        return Err("Encrypted data too short (missing nonce)".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    let key_bytes = get_or_create_encryption_key()?;
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("AES decryption failed: {}", e))
}

/// Encrypt session tokens to a base64 string.
fn encrypt_session_tokens(tokens: &EncryptedSessionTokens) -> Result<String, String> {
    let json = serde_json::to_vec(tokens)
        .map_err(|e| format!("Failed to serialize session tokens: {}", e))?;
    encrypt_bytes(&json)
}

/// Decrypt session tokens from a base64 string.
fn decrypt_session_tokens(encrypted_b64: &str) -> Result<EncryptedSessionTokens, String> {
    let plaintext = decrypt_bytes(encrypted_b64)?;
    let tokens: EncryptedSessionTokens = serde_json::from_slice(&plaintext)
        .map_err(|e| format!("Failed to deserialize session tokens: {}", e))?;

    if tokens.supabase_access_token.is_empty() {
        return Err("Decrypted supabase access token is empty".to_string());
    }

    Ok(tokens)
}

// ============================================================================
// Public API
// ============================================================================

/// Store a complete session, splitting tokens across keyring and store.json.
///
/// - Refresh tokens → OS keyring (persistent, most secure)
/// - Access tokens → AES-encrypted in store.json (fast access)
/// - Metadata (user, expiry) → plaintext in store.json
///
/// If the session is missing a Google access token (common during
/// Supabase-only token refreshes), the existing value is preserved.
/// Google refresh tokens are never overwritten with empty values.
pub fn store_session(session: &Session) -> Result<(), String> {
    let user_id = &session.user.id;
    if user_id.is_empty() {
        return Err("Cannot store session: user ID is empty".to_string());
    }

    // Set current user ID in keyring (bootstrap pointer for startup)
    keyring_set(KEYRING_CURRENT_USER_ID, user_id)?;

    // Store Supabase refresh token in keyring
    keyring_set(KEYRING_SUPABASE_REFRESH, &session.refresh_token)?;

    // Store Google refresh token in keyring if present.
    // If absent, leave existing keyring entry untouched — Google only
    // sends the refresh token once (on first consent).
    if let Some(ref token) = session.provider_refresh_token {
        if !token.is_empty() {
            keyring_set(KEYRING_GOOGLE_REFRESH, token)?;
        }
    }

    // Determine Google access token to encrypt.
    // If the new session doesn't include one (e.g. Supabase-only refresh),
    // preserve the existing value.
    let google_access_token = session
        .provider_token
        .clone()
        .filter(|t| !t.is_empty())
        .or_else(|| {
            // Best-effort: try to read existing google access token from store
            read_encrypted_tokens().ok().and_then(|t| t.google_access_token)
        });

    // Encrypt access tokens
    let tokens = EncryptedSessionTokens {
        supabase_access_token: session.access_token.clone(),
        google_access_token,
    };
    let encrypted = encrypt_session_tokens(&tokens)?;

    // Build metadata (plaintext)
    let metadata = SessionMetadata {
        user: session.user.clone(),
        token_type: session.token_type.clone(),
        expires_in: session.expires_in,
        expires_at: session.expires_at,
        stored_at: chrono::Utc::now().timestamp(),
    };

    // Write to per-user store
    let metadata_json = serde_json::to_value(&metadata)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

    let store_data = serde_json::json!({
        "session_data": metadata_json,
        "encrypted_tokens": encrypted,
    });

    let app_handle = get_app_handle().ok_or("AppHandle not initialized")?;

    // Ensure profile directory exists before store write
    ensure_profile_dir(&app_handle, user_id)?;

    let store_path = user_store_path(user_id);
    let store = app_handle
        .store(&store_path)
        .map_err(|e| format!("Failed to open user store: {}", e))?;

    store.set(AUTH_KEY, store_data);
    store
        .save()
        .map_err(|e| format!("Failed to save user store: {}", e))?;

    log::info!("[auth_storage] Session stored for user {}", user_id);
    Ok(())
}

/// Read only the encrypted tokens from store.json (without touching keyring).
///
/// Used internally to preserve existing Google access tokens during
/// Supabase-only session refreshes.
fn read_encrypted_tokens() -> Result<EncryptedSessionTokens, String> {
    let user_id = keyring_get(KEYRING_CURRENT_USER_ID)?
        .ok_or("No current user ID in keyring")?;

    let app_handle = get_app_handle().ok_or("AppHandle not initialized")?;
    let store_path = user_store_path(&user_id);
    let store = app_handle
        .store(&store_path)
        .map_err(|e| format!("Failed to open user store: {}", e))?;

    let auth_val = store
        .get(AUTH_KEY)
        .ok_or("No auth data in user store")?;

    let encrypted_b64 = auth_val["encrypted_tokens"]
        .as_str()
        .ok_or("Missing encrypted_tokens field")?;

    decrypt_session_tokens(encrypted_b64)
}

/// Retrieve the full auth state by reading from both store.json and keyring.
///
/// Reconstructs a complete `StoredAuthState` (with `Session`) by:
/// 1. Reading metadata + encrypted tokens from store.json
/// 2. Decrypting access tokens
/// 3. Reading refresh tokens from OS keyring
///
/// Returns `Ok(None)` if no session is stored or if decryption fails.
pub fn retrieve_auth_state() -> Result<Option<StoredAuthState>, String> {
    // Read current user ID from keyring (bootstrap pointer)
    let user_id = match keyring_get(KEYRING_CURRENT_USER_ID)? {
        Some(id) if !id.is_empty() => id,
        _ => return Ok(None), // No active session
    };

    let app_handle = match get_app_handle() {
        Some(h) => h,
        None => return Ok(None),
    };

    let store_path = user_store_path(&user_id);
    let store = app_handle
        .store(&store_path)
        .map_err(|e| format!("Failed to open user store: {}", e))?;

    let auth_val = match store.get(AUTH_KEY) {
        Some(v) => v,
        None => return Ok(None),
    };

    // Parse metadata
    let metadata: SessionMetadata = match serde_json::from_value(auth_val["session_data"].clone()) {
        Ok(m) => m,
        Err(e) => {
            log::warn!(
                "[auth_storage] Corrupted session metadata: {}. Clearing state.",
                e
            );
            let _ = clear_auth_state();
            return Ok(None);
        }
    };

    // Decrypt access tokens
    let encrypted_b64 = match auth_val["encrypted_tokens"].as_str() {
        Some(s) => s,
        None => {
            log::warn!("[auth_storage] Missing encrypted_tokens. Clearing state.");
            let _ = clear_auth_state();
            return Ok(None);
        }
    };

    let tokens = match decrypt_session_tokens(encrypted_b64) {
        Ok(t) => t,
        Err(e) => {
            log::warn!(
                "[auth_storage] Decryption failed: {}. Clearing state.",
                e
            );
            let _ = clear_auth_state();
            return Ok(None);
        }
    };

    // Read refresh tokens from keyring
    let supabase_refresh = keyring_get(KEYRING_SUPABASE_REFRESH)?
        .unwrap_or_default();
    let google_refresh = keyring_get(KEYRING_GOOGLE_REFRESH)?;

    // If supabase refresh token is missing, session is unusable
    if supabase_refresh.is_empty() {
        log::warn!("[auth_storage] No Supabase refresh token in keyring. Clearing state.");
        let _ = clear_auth_state();
        return Ok(None);
    }

    // Reconstruct full Session
    let session = Session {
        access_token: tokens.supabase_access_token,
        token_type: metadata.token_type,
        expires_in: metadata.expires_in,
        expires_at: metadata.expires_at,
        refresh_token: supabase_refresh,
        user: metadata.user,
        provider_token: tokens.google_access_token,
        provider_refresh_token: google_refresh,
    };

    Ok(Some(StoredAuthState {
        session,
        stored_at: metadata.stored_at,
    }))
}

/// Get the Supabase access token if not expired.
pub fn get_access_token() -> Result<Option<String>, String> {
    match retrieve_auth_state()? {
        Some(state) if !state.is_access_token_expired() => {
            Ok(Some(state.session.access_token))
        }
        Some(_) => {
            log::info!("[auth_storage] Access token is expired");
            Ok(None)
        }
        None => Ok(None),
    }
}

/// Get the Supabase refresh token directly from the OS keyring.
pub fn get_refresh_token() -> Result<Option<String>, String> {
    keyring_get(KEYRING_SUPABASE_REFRESH)
}

/// Get the Google provider access token (from encrypted store.json).
pub fn get_provider_token() -> Result<Option<String>, String> {
    match retrieve_auth_state()? {
        Some(state) => Ok(state.session.provider_token),
        None => Ok(None),
    }
}

/// Get the Google refresh token directly from the OS keyring.
pub fn get_google_refresh_token() -> Result<Option<String>, String> {
    keyring_get(KEYRING_GOOGLE_REFRESH)
}

/// Clear all stored auth data from both store.json and keyring.
///
/// Called on logout or when stored data becomes unusable.
/// The encryption key is preserved so other encrypted data
/// (if any) can still be read.
pub fn clear_auth_state() -> Result<(), String> {
    // Read current user ID before clearing (needed to find the right store)
    let user_id = keyring_get(KEYRING_CURRENT_USER_ID)?;

    // Clear refresh tokens and current user pointer from keyring
    keyring_delete(KEYRING_SUPABASE_REFRESH);
    keyring_delete(KEYRING_GOOGLE_REFRESH);
    keyring_delete(KEYRING_CURRENT_USER_ID);

    // Clear session data from per-user store
    if let Some(user_id) = user_id.filter(|id| !id.is_empty()) {
        if let Some(app_handle) = get_app_handle() {
            let store_path = user_store_path(&user_id);
            if let Ok(store) = app_handle.store(&store_path) {
                store.delete(AUTH_KEY);
                let _ = store.save();
            }
        }
    }

    log::info!("[auth_storage] Auth state cleared (keyring + user store)");
    Ok(())
}