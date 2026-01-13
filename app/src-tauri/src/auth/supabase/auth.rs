use crate::auth::storage::store_auth;
use crate::auth::supabase::types::*;
use crate::auth::types::{SignInResult, SignUpResult, UserInfo};
use serde_json::json;
use std::collections::HashMap;

extern crate dotenv;

// Helper to get environment variables
fn get_env_vars() -> Result<(String, String), String> {
    if let Err(e) = dotenv::dotenv() {
        log::warn!("Warning: Could not load .env file: {}", e);
    }
    
    let url = std::env::var("SUPABASE_URL")
        .map_err(|_| "Missing SUPABASE_URL environment variable".to_string())?;
    let key = std::env::var("SUPABASE_ANON_KEY")
        .map_err(|_| "Missing SUPABASE_ANON_KEY environment variable".to_string())?;
        
    Ok((url, key))
}

pub async fn sign_up(
  email: String,
  password: String,
  given_name: Option<String>,
  family_name: Option<String>,
) -> Result<SignUpResult, String> {
  log::info!("[auth] Attempting sign_up for email: {}", email);
  let (base_url, api_key) = get_env_vars()?;
  let endpoint = format!("{}/auth/v1/signup", base_url);

  let mut data = HashMap::new();
  if let Some(gn) = given_name { data.insert("given_name".to_string(), gn); }
  if let Some(fn_name) = family_name { data.insert("family_name".to_string(), fn_name); }

  let body = json!({
    "email": email,
    "password": password,
    "data": data
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

  if !response.status().is_success() {
    let status = response.status();
    let error_text = response.text().await.unwrap_or_default();
    log::error!("[auth] SignUp failed. Status: {}. Response: {}", status, error_text);
    if let Ok(err_obj) = serde_json::from_str::<SupabaseErrorResponse>(&error_text) {
        return Err(err_obj.message());
    }
    return Err(format!("SignUp failed: {}", error_text));
  }

  /* Success case - read body as text first for debugging */
  let response_text = response.text().await
     .map_err(|e| format!("Failed to read response text: {}", e))?;
  log::info!("[auth] Supabase SignUp raw response: {}", response_text);

  // Directly parse as SupabaseUser since we expect the raw user object for signup with confirmation enabled
  let user: SupabaseUser = serde_json::from_str(&response_text)
     .map_err(|e| format!("Failed to parse user from response: {}. Body: {}", e, response_text))?;

  let session_str: Option<String> = None;
  let mut user_confirmed = false;
  
  // Check confirmation status
  if let Some(confirmed_at) = &user.confirmed_at {
      if !confirmed_at.is_empty() {
          user_confirmed = true;
      }
  }

  log::info!("[auth] SignUp successful. User ID: {}, Confirmed: {}", user.id, user_confirmed);

  Ok(SignUpResult {
    user_sub: user.id, 
    user_confirmed, 
    verification_required: !user_confirmed,
    destination: Some(email.clone()),
    delivery_medium: Some("EMAIL".to_string()),
    session: session_str,
  })
}

pub async fn sign_in(email: String, password: String) -> Result<SignInResult, String> {
  log::info!("[auth] Attempting sign_in for email: {}", email);
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

  if !response.status().is_success() {
    let status = response.status();
    let error_text = response.text().await.unwrap_or_default();
    log::error!("[auth] SignIn failed. Status: {}. Response: {}", status, error_text);
    if let Ok(err_obj) = serde_json::from_str::<SupabaseErrorResponse>(&error_text) {
        return Err(err_obj.message());
    }
    return Err(format!("SignIn failed: {}", error_text));
  }

  /* Success case - read body as text first for debugging */
  let response_text = response.text().await
     .map_err(|e| format!("Failed to read response text: {}", e))?;
  // Be careful logging full sign-in response as it contains access_token
  // We'll log a truncated version or specific fields if needed, but for dev it helps to see structure
  log::info!("[auth] Supabase SignIn raw response: {:}...", response_text);

  let auth_res: SupabaseAuthResponse = serde_json::from_str(&response_text)
    .map_err(|e| format!("Failed to parse response: {}", e))?;

  let user = auth_res.user.clone().ok_or_else(|| {
      log::error!("[auth] No user info found in response: {:?}", auth_res);
      "No user info found in response".to_string()
  })?;
  
  let meta = user.user_metadata.unwrap_or_default();
  let given_name = meta.get("given_name").and_then(|v| v.as_str()).map(String::from);
  let family_name = meta.get("family_name").and_then(|v| v.as_str()).map(String::from);

  let user_info = UserInfo {
      username: user.email.clone().unwrap_or(user.id.clone()),
      email: user.email,
      given_name,
      family_name,
      sub: user.id,
  };

  let sign_in_result = SignInResult {
      access_token: auth_res.access_token.unwrap_or_default(),
      id_token: None, 
      refresh_token: auth_res.refresh_token,
      expires_in: auth_res.expires_in.unwrap_or(3600),
      user_info,
  };

  if let Err(e) = store_auth(&sign_in_result) {
      log::warn!("Warning: Failed to store authentication: {}", e);
  }

  log::info!("[auth] SignIn successful for user user_sub={}", sign_in_result.user_info.sub);
  Ok(sign_in_result)
}

pub async fn verify_otp(email: String, token: String, type_: String) -> Result<String, String> {
    log::info!("[auth] Attempting verify_otp for email: {}, type: {}", email, type_);
    let (base_url, api_key) = get_env_vars()?;
    let endpoint = format!("{}/auth/v1/verify", base_url);

    let body = json!({
        "type": type_,
        "token": token,
        "email": email
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

    if !response.status().is_success() {
       let status = response.status();
       let error_text = response.text().await.unwrap_or_default();
       log::error!("[auth] Verification failed. Status: {}. Response: {}", status, error_text);
       if let Ok(err_obj) = serde_json::from_str::<SupabaseErrorResponse>(&error_text) {
           return Err(err_obj.message());
       }
       return Err(format!("Verification failed: {}", error_text));
    }
    
    let response_text = response.text().await.unwrap_or_default();
    log::info!("[auth] Verify OTP success raw response: {}", response_text);

    Ok("Verification successful".to_string())
}

pub async fn resend_confirmation(email: String) -> Result<(), String> {
   // Trigger signup to resend email
   let _ = sign_up(email, "placeholder".to_string(), None, None).await; 
   Ok(())
}

pub async fn sign_out(access_token: String) -> Result<(), String> {
    let (base_url, api_key) = get_env_vars()?;
    let endpoint = format!("{}/auth/v1/logout", base_url);

    let client = reqwest::Client::new();
    let _ = client
        .post(&endpoint)
        .header("apikey", api_key)
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await;
        
    Ok(())
}