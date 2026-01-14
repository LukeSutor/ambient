use crate::auth::types::{
    AuthResponse, AuthError, Session, SignUpResponse, SupabaseUser, 
    VerifyOtpResponse, RefreshTokenResponse, ResendConfirmationResponse,
};
use crate::auth::storage::{store_session, get_refresh_token, clear_auth_state, get_access_token};
use serde_json::json;

extern crate dotenv;

/// Get Supabase environment variables
pub fn get_env_vars() -> Result<(String, String), String> {
    if let Err(e) = dotenv::dotenv() {
        log::warn!("[supabase_auth] Warning: Could not load .env file: {}", e);
    }
    
    let url = std::env::var("SUPABASE_URL")
        .map_err(|_| "Missing SUPABASE_URL environment variable".to_string())?;
    let key = std::env::var("SUPABASE_ANON_KEY")
        .map_err(|_| "Missing SUPABASE_ANON_KEY environment variable".to_string())?;
    
    Ok((url, key))
}

#[tauri::command]
pub async fn sign_up(
    email: String,
    password: String,
    given_name: Option<String>,
    family_name: Option<String>,
) -> Result<SignUpResponse, String> {
    log::info!("[supabase_auth] Attempting sign_up for email: {}", email);
    let (base_url, api_key) = get_env_vars()?;
    let endpoint = format!("{}/auth/v1/signup", base_url);
    
    // Build user metadata
    let mut user_meta = serde_json::Map::new();
    if let Some(gn) = &given_name {
        user_meta.insert("given_name".to_string(), json!(gn));
    }
    if let Some(fn_name) = &family_name {
        user_meta.insert("family_name".to_string(), json!(fn_name));
    }
    
    let body = json!({
        "email": email,
        "password": password,
        "data": user_meta
    });
    
    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .header("apikey", &api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
        
    if !status.is_success() {
        return Err(response_text);
    }
    
    // Try to parse as session response (when autoconfirm is enabled)
    if let Ok(session) = serde_json::from_str::<Session>(&response_text) {
        // Store the session
        if let Err(e) = store_session(&session) {
            log::warn!("[supabase_auth] Failed to store session: {}", e);
        }
        
        return Ok(SignUpResponse {
            user: Some(session.user.clone()),
            session: Some(session),
            verification_required: false,
            destination: None,
            delivery_medium: None,
        });
    }
    
    // Parse as user object only (when email confirmation is required)
    let user: SupabaseUser = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse signup response: {}. Body: {}", e, response_text))?;
    
    let user_confirmed = user.confirmed_at.as_ref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    
    log::info!("[supabase_auth] SignUp successful. User ID: {}, Confirmed: {}", user.id, user_confirmed);
    
    Ok(SignUpResponse {
        user: Some(user),
        session: None,
        verification_required: !user_confirmed,
        destination: Some(email),
        delivery_medium: Some("EMAIL".to_string()),
    })
}

#[tauri::command]
pub async fn sign_in_with_password(email: String, password: String) -> Result<AuthResponse, String> {
    log::info!("[supabase_auth] Attempting sign_in for email: {}", email);
    let (base_url, api_key) = get_env_vars()?;
    let endpoint = format!("{}/auth/v1/token?grant_type=password", base_url);
    
    let body = json!({
        "email": email,
        "password": password
    });
    
    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .header("apikey", &api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
        
    if !status.is_success() {
        match serde_json::from_str::<AuthError>(&response_text) {
            Ok(err) => {
                let msg = err.get_message();
                
                // Handle unconfirmed email case
                if msg.contains("Email not confirmed") || err.error_code == Some("email_not_confirmed".to_string()) {
                    log::info!("[supabase_auth] User email not confirmed for {}. Automatically resending confirmation.", email);
                    
                    // Automatically resend confirmation email
                    let _ = resend_confirmation(email.clone()).await;
                    
                    return Ok(AuthResponse {
                        session: None,
                        user: None,
                        weak_password: None,
                        verification_required: true,
                        destination: Some(email),
                        delivery_medium: Some("EMAIL".to_string()),
                    });
                }
            },
            Err(e) => {
                log::warn!("[supabase_auth] Failed to parse auth error: {}. Body: {}", e, response_text);
            }
        }
        return Err(response_text);
    }
    
    // Parse the session response
    let session: Session = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse session: {}. Body: {}", e, response_text))?;
    
    // Store the session
    if let Err(e) = store_session(&session) {
        log::warn!("[supabase_auth] Failed to store session: {}", e);
    }
    
    log::info!("[supabase_auth] SignIn successful for user: {}", session.user.id);
    
    Ok(AuthResponse {
        session: Some(session.clone()),
        user: Some(session.user),
        weak_password: None,
        verification_required: false,
        destination: None,
        delivery_medium: None,
    })
}

#[tauri::command]
pub async fn refresh_token() -> Result<RefreshTokenResponse, String> {
    log::info!("[supabase_auth] Attempting to refresh session");
    
    let refresh_token = get_refresh_token()
        .map_err(|e| format!("Failed to get refresh token: {}", e))?
        .ok_or("No refresh token available")?;
    
    refresh_session_with_token(&refresh_token).await
}

pub async fn refresh_session_with_token(refresh_token: &str) -> Result<RefreshTokenResponse, String> {
    let (base_url, api_key) = get_env_vars()?;
    let endpoint = format!("{}/auth/v1/token?grant_type=refresh_token", base_url);
    
    let body = json!({
        "refresh_token": refresh_token
    });
    
    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .header("apikey", &api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send refresh request: {}", e))?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
        
    if !status.is_success() {
        // Clear auth state on refresh failure
        let _ = clear_auth_state();
        
        return Err(response_text);
    }
    
    // Parse the new session
    let session: Session = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse refreshed session: {}. Body: {}", e, response_text))?;
    
    // Store the new session
    if let Err(e) = store_session(&session) {
        log::warn!("[supabase_auth] Failed to store refreshed session: {}", e);
    }
    
    log::info!("[supabase_auth] Session refreshed successfully");
    
    Ok(RefreshTokenResponse {
        session: session.clone(),
        user: session.user,
    })
}

#[tauri::command]
pub async fn verify_otp(email: String, token: String, otp_type: Option<String>) -> Result<VerifyOtpResponse, String> {
    let otp_type = otp_type.unwrap_or_else(|| "signup".to_string());

    log::info!("[supabase_auth] Attempting verify_otp for email: {}", email);
    let (base_url, api_key) = get_env_vars()?;
    let endpoint = format!("{}/auth/v1/verify", base_url);
    
    let body = json!({
        "email": email,
        "token": token,
        "type": otp_type
    });
    
    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .header("apikey", &api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
        
    if !status.is_success() {
        return Err(response_text);
    }
    
    // Try to parse as session response (verification may return a session)
    if let Ok(session) = serde_json::from_str::<Session>(&response_text) {
        // Store the session
        if let Err(e) = store_session(&session) {
            log::warn!("[supabase_auth] Failed to store session: {}", e);
        }
        
        return Ok(VerifyOtpResponse {
            session: Some(session.clone()),
            user: Some(session.user),
        });
    }
    
    // Try to parse as user only
    if let Ok(user) = serde_json::from_str::<SupabaseUser>(&response_text) {
        return Ok(VerifyOtpResponse {
            session: None,
            user: Some(user),
        });
    }
    
    log::info!("[supabase_auth] OTP verification successful");
    
    Ok(VerifyOtpResponse {
        session: None,
        user: None,
    })
}

#[tauri::command]
pub async fn resend_confirmation(email: String) -> Result<ResendConfirmationResponse, String> {
    log::info!("[supabase_auth] Attempting resend_confirmation for email: {}", email);
    let (base_url, api_key) = get_env_vars()?;
    let endpoint = format!("{}/auth/v1/resend", base_url);
    
    let body = json!({
        "email": email,
        "type": "signup"
    });
    
    let client = reqwest::Client::new();
    let response = client
        .post(&endpoint)
        .header("apikey", &api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    if !status.is_success() {
        return Err(response_text);
    }
    
    log::info!("[supabase_auth] Resend confirmation successful for email: {}", email);
    
    Ok(ResendConfirmationResponse {
        message_id: None,
    })
}

#[tauri::command]
pub async fn logout() -> Result<String, String> {
    // Get access token for server-side logout
    let access_token = get_access_token()
        .ok()
        .flatten();
    
    // Sign out from Supabase
    sign_out(access_token).await?;
    
    Ok("Logged out successfully".to_string())
}

pub async fn sign_out(access_token: Option<String>) -> Result<(), String> {
    log::info!("[supabase_auth] Signing out user");
    
    // Try to invalidate the session on the server
    if let Some(token) = access_token {
        let (base_url, api_key) = get_env_vars()?;
        let endpoint = format!("{}/auth/v1/logout", base_url);
        
        let client = reqwest::Client::new();
        let _ = client
            .post(&endpoint)
            .header("apikey", &api_key)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;
    }
    
    // Always clear local auth state
    clear_auth_state()
        .map_err(|e| format!("Failed to clear auth state: {}", e))?;
    
    log::info!("[supabase_auth] Sign out successful");
    Ok(())
}