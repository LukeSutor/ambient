use crate::auth::types::{
    AuthResponse, AuthError, Session, SignUpResponse, SupabaseUser, 
    VerifyOtpResponse, RefreshTokenResponse, ResendConfirmationResponse,
    OAuthUrlResponse, AuthErrorResponse, AuthErrorCode,
};
use crate::auth::storage::{store_session, get_refresh_token, clear_auth_state, get_access_token, retrieve_auth_state};
use crate::auth::security::{
    HTTP_CLIENT,
    check_rate_limit, record_attempt, clear_rate_limit, RateLimitOp,
};
use serde_json::json;
use tokio::sync::Mutex;
use once_cell::sync::Lazy;
use crate::constants::{SUPABASE_URL, SUPABASE_ANON_KEY};

#[tauri::command]
pub async fn sign_up(
    email: String,
    password: String,
    full_name: Option<String>,
) -> Result<SignUpResponse, String> {
    log::info!("[supabase_auth] Attempting sign up");
    
    check_rate_limit(RateLimitOp::SignUp, &email)?;
    record_attempt(RateLimitOp::SignUp, &email);
    
    let endpoint = format!("{}/auth/v1/signup", SUPABASE_URL);
    
    // Build user metadata
    let mut user_meta = serde_json::Map::new();
    if let Some(name) = &full_name {
        user_meta.insert("full_name".to_string(), json!(name));
    }
    user_meta.insert("avatar_url".to_string(), json!(""));
    
    let body = json!({
        "email": email,
        "password": password,
        "data": user_meta
    });
    
    let response = HTTP_CLIENT
        .post(&endpoint)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let err = AuthErrorResponse::network_error(format!("Failed to send request: {}", e));
            err.to_string()
        })?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
        
    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<AuthError>(&response_text) {
            return Err(AuthErrorResponse::from_supabase_error(&err).to_string());
        }
        return Err(AuthErrorResponse::new(AuthErrorCode::ServerError, response_text).to_string());
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
    
    log::info!("[supabase_auth] SignUp successful. Confirmed: {}", user_confirmed);
    
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
    log::info!("[supabase_auth] Attempting sign in");
    
    check_rate_limit(RateLimitOp::SignIn, &email)?;
    record_attempt(RateLimitOp::SignIn, &email);
    
    let endpoint = format!("{}/auth/v1/token?grant_type=password", SUPABASE_URL);
    
    let body = json!({
        "email": email,
        "password": password
    });
    
    let response = HTTP_CLIENT
        .post(&endpoint)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            AuthErrorResponse::network_error(format!("Failed to send request: {}", e)).to_string()
        })?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
        
    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<AuthError>(&response_text) {
            let auth_err = AuthErrorResponse::from_supabase_error(&err);
            
            if auth_err.code == AuthErrorCode::EmailNotConfirmed {
                log::info!("[supabase_auth] User email not confirmed");
                return Ok(AuthResponse {
                    session: None,
                    user: None,
                    weak_password: None,
                    verification_required: true,
                    destination: Some(email),
                    delivery_medium: Some("EMAIL".to_string()),
                });
            }
            
            return Err(auth_err.to_string());
        }
        return Err(AuthErrorResponse::new(AuthErrorCode::ServerError, response_text).to_string());
    }
    
    // Parse the session response
    let session: Session = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse session: {}. Body: {}", e, response_text))?;
    
    // Store the session
    if let Err(e) = store_session(&session) {
        log::warn!("[supabase_auth] Failed to store session: {}", e);
        return Err(AuthErrorResponse::storage_error(format!("Failed to store session: {}", e)).to_string());
    }
    
    // Clear rate limit on successful login
    clear_rate_limit(RateLimitOp::SignIn, &email);
    
    log::info!("[supabase_auth] SignIn successful");
    
    Ok(AuthResponse {
        session: Some(session.clone()),
        user: Some(session.user),
        weak_password: None,
        verification_required: false,
        destination: None,
        delivery_medium: None,
    })
}

static REFRESH_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub async fn refresh_token() -> Result<RefreshTokenResponse, String> {
    log::info!("[supabase_auth] Attempting to refresh session");
    
    // Acquire lock to serialize refresh requests
    let _guard = REFRESH_MUTEX.lock().await;

    // Check if the token was already refreshed by another thread while we waited
    if let Ok(Some(state)) = retrieve_auth_state().map_err(|e| e.to_string()) {
        if !state.is_access_token_expired() {
            log::info!("[supabase_auth] Token already refreshed by concurrent request. Returning valid session.");
            return Ok(RefreshTokenResponse {
                session: state.session.clone(),
                user: state.session.user,
            });
        }
    }
    
    // Rate limiting for refresh to prevent abuse
    check_rate_limit(RateLimitOp::RefreshToken, "global")?;
    record_attempt(RateLimitOp::RefreshToken, "global");
    
    let refresh_token = get_refresh_token()
        .map_err(|e| AuthErrorResponse::storage_error(format!("Failed to get refresh token: {}", e)).to_string())?
        .ok_or_else(|| AuthErrorResponse::session_expired().to_string())?;
    
    refresh_session_with_token(&refresh_token).await
}

pub async fn refresh_session_with_token(refresh_token: &str) -> Result<RefreshTokenResponse, String> {
    let endpoint = format!("{}/auth/v1/token?grant_type=refresh_token", SUPABASE_URL);
    
    let body = json!({
        "refresh_token": refresh_token
    });
    
    let response = HTTP_CLIENT
        .post(&endpoint)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AuthErrorResponse::network_error(format!("Failed to send refresh request: {}", e)).to_string())?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
        
    if !status.is_success() {
        // Clear auth state on refresh failure
        let _ = clear_auth_state();
        
        if let Ok(err) = serde_json::from_str::<AuthError>(&response_text) {
            return Err(AuthErrorResponse::from_supabase_error(&err).to_string());
        }
        return Err(AuthErrorResponse::session_expired().to_string());
    }
    
    // Parse the new session
    let session: Session = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse refreshed session: {}. Body: {}", e, response_text))?;
    
    // Store the new session
    if let Err(e) = store_session(&session) {
        log::warn!("[supabase_auth] Failed to store refreshed session: {}", e);
        return Err(AuthErrorResponse::storage_error(format!("Failed to store session: {}", e)).to_string());
    }
    
    // Clear rate limit on success
    clear_rate_limit(RateLimitOp::RefreshToken, "global");
    
    log::info!("[supabase_auth] Session refreshed successfully");
    
    Ok(RefreshTokenResponse {
        session: session.clone(),
        user: session.user,
    })
}

#[tauri::command]
pub async fn verify_otp(email: String, token: String, otp_type: Option<String>) -> Result<VerifyOtpResponse, String> {
    let otp_type = otp_type.unwrap_or_else(|| "signup".to_string());

    log::info!("[supabase_auth] Attempting to verify otp");
    
    check_rate_limit(RateLimitOp::VerifyOtp, &email)?;
    record_attempt(RateLimitOp::VerifyOtp, &email);
    
    let endpoint = format!("{}/auth/v1/verify", SUPABASE_URL);
    
    let body = json!({
        "email": email,
        "token": token,
        "type": otp_type
    });
    
    let response = HTTP_CLIENT
        .post(&endpoint)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AuthErrorResponse::network_error(format!("Failed to send request: {}", e)).to_string())?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
        
    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<AuthError>(&response_text) {
            return Err(AuthErrorResponse::from_supabase_error(&err).to_string());
        }
        return Err(AuthErrorResponse::new(AuthErrorCode::ServerError, response_text).to_string());
    }
    
    // Clear rate limit on success
    clear_rate_limit(RateLimitOp::VerifyOtp, &email);
    
    // Try to parse as session response (verification may return a session)
    if let Ok(session) = serde_json::from_str::<Session>(&response_text) {
        // Store the session
        if let Err(e) = store_session(&session) {
            log::warn!("[supabase_auth] Failed to store session: {}", e);
            return Err(AuthErrorResponse::storage_error(format!("Failed to store session: {}", e)).to_string());
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
    log::info!("[supabase_auth] Attempting resend confirmation");
    
    check_rate_limit(RateLimitOp::ResendConfirmation, &email)?;
    record_attempt(RateLimitOp::ResendConfirmation, &email);
    
    let endpoint = format!("{}/auth/v1/resend", SUPABASE_URL);
    
    let body = json!({
        "email": email,
        "type": "signup"
    });
    
    let response = HTTP_CLIENT
        .post(&endpoint)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AuthErrorResponse::network_error(format!("Failed to send request: {}", e)).to_string())?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    if !status.is_success() {
        if let Ok(err) = serde_json::from_str::<AuthError>(&response_text) {
            return Err(AuthErrorResponse::from_supabase_error(&err).to_string());
        }
        return Err(AuthErrorResponse::new(AuthErrorCode::ServerError, response_text).to_string());
    }
    
    log::info!("[supabase_auth] Resend confirmation successful");
    
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
        let endpoint = format!("{}/auth/v1/logout", SUPABASE_URL);
        
        let _ = HTTP_CLIENT
            .post(&endpoint)
            .header("apikey", SUPABASE_ANON_KEY)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;
    }
    
    // Always clear local auth state
    clear_auth_state()
        .map_err(|e| AuthErrorResponse::storage_error(format!("Failed to clear auth state: {}", e)).to_string())?;
    
    log::info!("[supabase_auth] Sign out successful");
    Ok(())
}

// ============================================================================
// Google OAuth Functions
// ============================================================================

/// Generate the Google OAuth authorization URL
/// Returns the URL that should be opened in the system browser
/// Note: Supabase handles PKCE internally for social OAuth providers
#[tauri::command]
pub async fn sign_in_with_google() -> Result<OAuthUrlResponse, String> {
    log::info!("[supabase_auth] Initiating Google OAuth sign in");
    
    // Build the OAuth authorization URL
    let redirect_uri = "ambient://auth/callback";
    let provider = "google";
    
    let auth_url = format!(
        "{}/auth/v1/authorize?provider={}&redirect_to={}",
        SUPABASE_URL,
        provider,
        urlencoding::encode(redirect_uri)
    );
    
    log::info!("[supabase_auth] Generated Google OAuth URL");
    
    Ok(OAuthUrlResponse { url: auth_url })
}

/// Fetch user profile from the public.profiles table
pub async fn fetch_user_profile(user_id: &str, access_token: &str) -> Result<serde_json::Value, String> {
    let endpoint = format!("{}/rest/v1/profiles?id=eq.{}&select=*", SUPABASE_URL, user_id);
    
    let response = HTTP_CLIENT
        .get(&endpoint)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Range", "0-0") // Just get one
        .send()
        .await
        .map_err(|e| format!("Failed to fetch profile: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Profile fetch failed: {}", response.status()));
    }
    
    let profiles: Vec<serde_json::Value> = response.json().await
        .map_err(|e| format!("Failed to parse profile response: {}", e))?;
    
    Ok(profiles.into_iter().next().unwrap_or(serde_json::json!({})))
}

/// Handle the OAuth callback URL
pub async fn handle_oauth_callback(callback_url: &str) -> Result<AuthResponse, String> {
    log::info!("[supabase_auth] Handling OAuth callback URL");
    
    let parsed = url::Url::parse(callback_url)
        .map_err(|e| AuthErrorResponse::oauth_error(format!("Failed to parse callback URL: {}", e)).to_string())?;
    
    // Check for error in query params
    let query_pairs: std::collections::HashMap<String, String> = parsed
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    
    if let Some(error) = query_pairs.get("error") {
        let error_description = query_pairs.get("error_description")
            .cloned()
            .unwrap_or_default();
        return Err(AuthErrorResponse::oauth_error(format!("{}: {}", error, error_description)).to_string());
    }
    
    // Check for tokens in the URL fragment (Supabase social OAuth flow)
    // Fragment comes after # in the URL
    if let Some(fragment) = parsed.fragment() {
        let fragment_pairs: std::collections::HashMap<String, String> = 
            url::form_urlencoded::parse(fragment.as_bytes())
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
        
        if let (Some(access_token), Some(refresh_token)) = 
            (fragment_pairs.get("access_token"), fragment_pairs.get("refresh_token")) 
        {
            // Validate tokens are not empty
            if access_token.is_empty() || refresh_token.is_empty() {
                return Err(AuthErrorResponse::oauth_error("Received empty tokens in OAuth callback").to_string());
            }
            
            // We have tokens directly, fetch the user and create session
            // This validates the tokens server-side
            return handle_tokens_from_fragment(access_token, refresh_token, &fragment_pairs).await;
        }
    }
    
    // Check for authorization code in query params (used for custom OAuth server flows)
    // This is for when you build your own OAuth server with Supabase, not social login
    if query_pairs.contains_key("code") {
        log::info!("[supabase_auth] Received authorization code - this flow is not supported for social OAuth");
        // For code exchange, we'd need to have stored the code_verifier
        // This path is for advanced custom OAuth flows
        return Err(AuthErrorResponse::oauth_error(
            "Authorization code flow not implemented for social OAuth. Tokens expected in URL fragment."
        ).to_string());
    }
    
    Err(AuthErrorResponse::oauth_error("No authorization code or tokens found in callback URL").to_string())
}

/// Handle tokens received in URL fragment
/// Validates tokens by fetching user info before trusting them
async fn handle_tokens_from_fragment(
    access_token: &str,
    refresh_token: &str,
    fragment_pairs: &std::collections::HashMap<String, String>,
) -> Result<AuthResponse, String> {
    log::info!("[supabase_auth] Handling tokens from URL fragment");
    
    // Validate token format
    if access_token.len() < 10 || refresh_token.len() < 10 {
        return Err(AuthErrorResponse::oauth_error("Invalid token format received").to_string());
    }
    
    // Get user info using the access token - this validates the token server-side
    let endpoint = format!("{}/auth/v1/user", SUPABASE_URL);
    
    let response = HTTP_CLIENT
        .get(&endpoint)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| AuthErrorResponse::network_error(format!("Failed to get user info: {}", e)).to_string())?;
    
    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    
    if !status.is_success() {
        // Token validation failed - this is a security concern
        log::error!("[supabase_auth] Token validation failed - server rejected the access token");
        return Err(AuthErrorResponse::oauth_error("Token validation failed - access token rejected by server").to_string());
    }
    
    let user: SupabaseUser = serde_json::from_str(&response_text)
        .map_err(|e| {
            log::error!("[supabase_auth] Failed to parse user from token validation: {}", e);
            AuthErrorResponse::oauth_error(format!("Failed to parse user info: {}", e)).to_string()
        })?;
    
    // Validate user has required fields
    if user.id.is_empty() {
        return Err(AuthErrorResponse::oauth_error("Invalid user data received from OAuth").to_string());
    }
    
    // Parse expires_in from fragment
    let expires_in: i64 = fragment_pairs.get("expires_in")
        .and_then(|s| s.parse().ok())
        .unwrap_or(3600);
    
    let expires_at: Option<i64> = fragment_pairs.get("expires_at")
        .and_then(|s| s.parse().ok());
    
    // Create session object
    let session = Session {
        access_token: access_token.to_string(),
        token_type: fragment_pairs.get("token_type")
            .cloned()
            .unwrap_or_else(|| "bearer".to_string()),
        expires_in,
        expires_at,
        refresh_token: refresh_token.to_string(),
        user: user.clone(),
    };
    
    // Store the session
    if let Err(e) = store_session(&session) {
        log::warn!("[supabase_auth] Failed to store session: {}", e);
        return Err(AuthErrorResponse::storage_error(format!("Failed to store session: {}", e)).to_string());
    }
    
    log::info!("[supabase_auth] OAuth sign in successful");
    
    Ok(AuthResponse {
        session: Some(session),
        user: Some(user),
        weak_password: None,
        verification_required: false,
        destination: None,
        delivery_medium: None,
    })
}