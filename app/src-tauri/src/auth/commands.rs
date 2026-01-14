use crate::auth::types::{
    AuthState, UserInfo, SupabaseUser, AuthError,
};
use crate::auth::storage::{
    retrieve_auth_state, clear_auth_state,
};
use crate::auth::auth_flow::{get_env_vars, refresh_token};
use tauri::{AppHandle, Emitter};

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
                match refresh_token().await {
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

#[tauri::command]
pub async fn is_authenticated() -> Result<bool, String> {
    // Check new auth state - convert error to string before any await
    let state_result = retrieve_auth_state()
        .map_err(|e| e.to_string());
    
    match state_result {
        Ok(Some(state)) => {
            let is_expired = state.is_access_token_expired();
            if is_expired {
                // Try to refresh
                match refresh_token().await {
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
                match refresh_token().await {
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

#[tauri::command]
pub async fn get_user(access_token: &str) -> Result<SupabaseUser, String> {
    let (base_url, api_key) = get_env_vars()?;
    let endpoint = format!("{}/auth/v1/user", base_url);
    
    let client = reqwest::Client::new();
    let response = client
        .get(&endpoint)
        .header("apikey", &api_key)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<AuthError>(&response_text) {
            return Err(err.get_message());
        }
        return Err(format!("Failed to get user: {}", response_text));
    }
    
    let user: SupabaseUser = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse user: {}. Body: {}", e, response_text))?;
    
    Ok(user)
}

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
                match refresh_token().await {
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

#[tauri::command]
pub async fn emit_auth_changed(app_handle: AppHandle) -> Result<(), String> {
    app_handle
        .emit("auth_changed", ())
        .map_err(|e| format!("Failed to emit auth_changed event: {}", e))
}