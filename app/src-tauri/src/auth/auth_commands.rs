use crate::auth::auth_types::{
    AuthResponse, AuthState, SignUpResponse, UserInfo, VerifyOtpResponse,
    RefreshTokenResponse, ResendConfirmationResponse,
};
use crate::auth::auth_storage::{
    retrieve_auth_state, clear_auth_state, get_access_token,
    // Legacy compatibility
    clear_stored_token, retrieve_token,
};
use crate::auth::supabase_auth;
use tauri::{AppHandle, Emitter};

// ============================================================================
// Sign Up
// ============================================================================

#[tauri::command]
pub async fn sign_up(
    email: String,
    password: String,
    given_name: Option<String>,
    family_name: Option<String>,
) -> Result<SignUpResponse, String> {
    supabase_auth::sign_up(email, password, given_name, family_name).await
}

// ============================================================================
// Sign In
// ============================================================================

#[tauri::command]
pub async fn sign_in(
    email: String,
    password: String,
) -> Result<AuthResponse, String> {
    supabase_auth::sign_in_with_password(email, password).await
}

// ============================================================================
// Verify OTP (Email Confirmation)
// ============================================================================

#[tauri::command]
pub async fn verify_otp(
    email: String,
    token: String,
    otp_type: Option<String>,
) -> Result<VerifyOtpResponse, String> {
    let otp_type = otp_type.unwrap_or_else(|| "signup".to_string());
    supabase_auth::verify_otp(email, token, otp_type).await
}

// ============================================================================
// Resend Confirmation
// ============================================================================

#[tauri::command]
pub async fn resend_confirmation(email: String) -> Result<ResendConfirmationResponse, String> {
    supabase_auth::resend_confirmation(email).await
}

// ============================================================================
// Refresh Token
// ============================================================================

#[tauri::command]
pub async fn refresh_token() -> Result<RefreshTokenResponse, String> {
    supabase_auth::refresh_session().await
}

// ============================================================================
// Sign Out
// ============================================================================

#[tauri::command]
pub async fn logout() -> Result<String, String> {
    // Get access token for server-side logout
    let access_token = get_access_token()
        .ok()
        .flatten();
    
    // Clear legacy OAuth tokens
    if let Err(e) = clear_stored_token() {
        log::warn!("[auth_commands] Warning: Failed to clear OAuth token: {}", e);
    }
    
    // Sign out from Supabase
    supabase_auth::sign_out(access_token).await?;
    
    Ok("Logged out successfully".to_string())
}

// ============================================================================
// Get Auth State
// ============================================================================

#[tauri::command]
pub async fn get_auth_state() -> Result<AuthState, String> {
    // Extract data synchronously first to avoid Send issues
    let state_result = retrieve_auth_state()
        .map_err(|e| e.to_string());
    
    match state_result {
        Ok(Some(state)) => {
            // Extract what we need before any potential await
            let is_expired = state.is_access_token_expired();
            
            if is_expired {
                // Try to refresh
                match supabase_auth::refresh_session().await {
                    Ok(refreshed) => {
                        Ok(AuthState {
                            is_authenticated: true,
                            user: Some(UserInfo::from(&refreshed.user)),
                            access_token: Some(refreshed.session.access_token),
                            expires_at: refreshed.session.expires_at,
                        })
                    }
                    Err(_) => {
                        // Clear auth state on refresh failure
                        let _ = clear_auth_state();
                        Ok(AuthState {
                            is_authenticated: false,
                            user: None,
                            access_token: None,
                            expires_at: None,
                        })
                    }
                }
            } else {
                Ok(AuthState {
                    is_authenticated: true,
                    user: Some(UserInfo::from(&state.session.user)),
                    access_token: Some(state.session.access_token),
                    expires_at: state.session.expires_at,
                })
            }
        }
        Ok(None) => Ok(AuthState {
            is_authenticated: false,
            user: None,
            access_token: None,
            expires_at: None,
        }),
        Err(e) => {
            log::error!("[auth_commands] Failed to get auth state: {}", e);
            Ok(AuthState {
                is_authenticated: false,
                user: None,
                access_token: None,
                expires_at: None,
            })
        }
    }
}

// ============================================================================
// Check Authentication Status
// ============================================================================

#[tauri::command]
pub async fn is_authenticated() -> Result<bool, String> {
    // Check legacy OAuth tokens first
    if let Ok(Some(_token)) = retrieve_token() {
        return Ok(true);
    }
    
    // Check new auth state - convert error to string before any await
    let state_result = retrieve_auth_state()
        .map_err(|e| e.to_string());
    
    match state_result {
        Ok(Some(state)) => {
            let is_expired = state.is_access_token_expired();
            if is_expired {
                // Try to refresh
                match supabase_auth::refresh_session().await {
                    Ok(_) => Ok(true),
                    Err(_) => {
                        let _ = clear_auth_state();
                        Ok(false)
                    }
                }
            } else {
                Ok(true)
            }
        }
        Ok(None) => Ok(false),
        Err(_) => Ok(false),
    }
}

// ============================================================================
// Get Current User
// ============================================================================

#[tauri::command]
pub async fn get_current_user() -> Result<Option<UserInfo>, String> {
    // Convert error to string before any await
    let state_result = retrieve_auth_state()
        .map_err(|e| e.to_string());
    
    match state_result {
        Ok(Some(state)) => {
            let is_expired = state.is_access_token_expired();
            let user_from_state = UserInfo::from(&state.session.user);
            
            if is_expired {
                // Try to refresh
                match supabase_auth::refresh_session().await {
                    Ok(refreshed) => Ok(Some(UserInfo::from(&refreshed.user))),
                    Err(_) => {
                        let _ = clear_auth_state();
                        Ok(None)
                    }
                }
            } else {
                Ok(Some(user_from_state))
            }
        }
        Ok(None) => Ok(None),
        Err(e) => {
            log::error!("[auth_commands] Failed to get current user: {}", e);
            Ok(None)
        }
    }
}

// ============================================================================
// Get Access Token
// ============================================================================

#[tauri::command]
pub async fn get_access_token_command() -> Result<Option<String>, String> {
    // Convert error to string before any await
    let state_result = retrieve_auth_state()
        .map_err(|e| e.to_string());
    
    match state_result {
        Ok(Some(state)) => {
            let is_expired = state.is_access_token_expired();
            let current_token = state.session.access_token.clone();
            
            if is_expired {
                // Try to refresh
                match supabase_auth::refresh_session().await {
                    Ok(refreshed) => Ok(Some(refreshed.session.access_token)),
                    Err(_) => {
                        let _ = clear_auth_state();
                        Ok(None)
                    }
                }
            } else {
                Ok(Some(current_token))
            }
        }
        Ok(None) => Ok(None),
        Err(e) => Err(format!("Failed to retrieve access token: {}", e)),
    }
}

// ============================================================================
// Emit Auth Changed Event
// ============================================================================

#[tauri::command]
pub async fn emit_auth_changed(app_handle: AppHandle) -> Result<(), String> {
    app_handle
        .emit("auth_changed", ())
        .map_err(|e| format!("Failed to emit auth_changed event: {}", e))
}