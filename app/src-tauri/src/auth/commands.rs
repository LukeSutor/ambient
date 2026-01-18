use crate::auth::types::{
    AuthState, UserInfo, SupabaseUser, AuthError, FullAuthState, AuthErrorResponse, AuthErrorCode,
};
use crate::auth::storage::{
    retrieve_auth_state, clear_auth_state,
};
use crate::auth::auth_flow::{get_env_vars, refresh_token, fetch_user_profile};
use crate::auth::security::HTTP_CLIENT;
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
            let needs_refresh = state.needs_refresh();
            
            let (user, token, expires_at) = if is_expired {
                // Try to refresh
                match refresh_token().await {
                    Ok(refreshed) => (refreshed.user, refreshed.session.access_token, refreshed.session.expires_at),
                    Err(_) => {
                        // Clear auth state on refresh failure
                        let _ = clear_auth_state();
                        return Ok(AuthState {
                            is_authenticated: false,
                            user: None,
                            needs_refresh: false,
                            expires_at: None,
                        });
                    }
                }
            } else {
                (state.session.user.clone(), state.session.access_token.clone(), state.session.expires_at)
            };

            let mut user_info = UserInfo::from(&user);

            // Pull latest info from profiles table
            if let Ok(profile) = fetch_user_profile(&user.id, &token).await {
                user_info = user_info.with_profile(&profile);
            }

            Ok(AuthState {
                is_authenticated: true,
                user: Some(user_info),
                needs_refresh,
                expires_at,
            })
        }
        Ok(None) => Ok(AuthState {
            is_authenticated: false,
            user: None,
            needs_refresh: false,
            expires_at: None,
        }),
        Err(e) => {
            log::error!("[auth_commands] Failed to get auth state: {}", e);
            Ok(AuthState {
                is_authenticated: false,
                user: None,
                needs_refresh: false,
                expires_at: None,
            })
        }
    }
}

/// Combined auth state fetch to reduce redundant API calls
#[tauri::command]
pub async fn get_full_auth_state(app_handle: AppHandle) -> Result<FullAuthState, String> {
    // Check online status
    let is_online = check_online().await;
    
    // Check setup complete (needs AppHandle)
    let is_setup_complete = crate::setup::check_setup_complete(app_handle).unwrap_or(false);
    
    // Get auth state
    let state_result = retrieve_auth_state()
        .map_err(|e| e.to_string());
    
    match state_result {
        Ok(Some(state)) => {
            let is_expired = state.is_access_token_expired();
            let needs_refresh = state.needs_refresh();
            
            let (user, token, expires_at) = if is_expired {
                match refresh_token().await {
                    Ok(refreshed) => (refreshed.user, refreshed.session.access_token, refreshed.session.expires_at),
                    Err(_) => {
                        let _ = clear_auth_state();
                        return Ok(FullAuthState {
                            is_online,
                            is_authenticated: false,
                            is_setup_complete,
                            user: None,
                            needs_refresh: false,
                            expires_at: None,
                        });
                    }
                }
            } else {
                (state.session.user.clone(), state.session.access_token.clone(), state.session.expires_at)
            };

            let mut user_info = UserInfo::from(&user);
            if let Ok(profile) = fetch_user_profile(&user.id, &token).await {
                user_info = user_info.with_profile(&profile);
            }

            Ok(FullAuthState {
                is_online,
                is_authenticated: true,
                is_setup_complete,
                user: Some(user_info),
                needs_refresh,
                expires_at,
            })
        }
        Ok(None) => Ok(FullAuthState {
            is_online,
            is_authenticated: false,
            is_setup_complete,
            user: None,
            needs_refresh: false,
            expires_at: None,
        }),
        Err(e) => {
            log::error!("[auth_commands] Failed to get full auth state: {}", e);
            Ok(FullAuthState {
                is_online,
                is_authenticated: false,
                is_setup_complete,
                user: None,
                needs_refresh: false,
                expires_at: None,
            })
        }
    }
}

/// Quick online check helper using TCP connection
async fn check_online() -> bool {
    let addr = "8.8.8.8:53";
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tokio::net::TcpStream::connect(addr)
    ).await {
        Ok(Ok(_)) => true,
        _ => false,
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
            
            let (user, token) = if is_expired {
                // Try to refresh
                match refresh_token().await {
                    Ok(refreshed) => (refreshed.user, refreshed.session.access_token),
                    Err(_) => {
                        let _ = clear_auth_state();
                        return Ok(None);
                    }
                }
            } else {
                (state.session.user.clone(), state.session.access_token.clone())
            };

            let mut user_info = UserInfo::from(&user);

            // Pull latest info from profiles table
            if let Ok(profile) = fetch_user_profile(&user.id, &token).await {
                user_info = user_info.with_profile(&profile);
            }
            
            Ok(Some(user_info))
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
    
    let response = HTTP_CLIENT
        .get(&endpoint)
        .header("apikey", &api_key)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| AuthErrorResponse::network_error(format!("Failed to send request: {}", e)).to_string())?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| AuthErrorResponse::new(AuthErrorCode::ServerError, format!("Failed to read response: {}", e)).to_string())?;
    
    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<AuthError>(&response_text) {
            return Err(AuthErrorResponse::from_supabase_error(&err).to_string());
        }
        return Err(AuthErrorResponse::new(AuthErrorCode::ServerError, format!("Failed to get user: {}", response_text)).to_string());
    }
    
    let user: SupabaseUser = serde_json::from_str(&response_text)
        .map_err(|e| AuthErrorResponse::new(AuthErrorCode::ServerError, format!("Failed to parse user: {}. Body: {}", e, response_text)).to_string())?;
    
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
        Err(e) => Err(AuthErrorResponse::storage_error(format!("Failed to retrieve access token: {}", e)).to_string()),
    }
}

#[tauri::command]
pub async fn emit_auth_changed(app_handle: AppHandle) -> Result<(), String> {
    app_handle
        .emit("auth_changed", ())
        .map_err(|e| format!("Failed to emit auth_changed event: {}", e))
}