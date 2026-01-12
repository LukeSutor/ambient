use crate::auth::storage::store_cognito_auth; // You might want to rename this function to `store_auth`
use crate::auth::types::{CognitoUserInfo, SignInResult, SignUpResult}; // You'll need to adapt these types or map to them
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
extern crate dotenv;

// Helper to get environment variables
fn get_env_vars() -> Result<(String, String), String> {
    if let Err(e) = dotenv::dotenv() {
        log::warn!("Warning: Could not load .env file: {}", e);
    }
    
    let url = std::env::var("SUPABASE_URL")
        .map_err(|_| "Missing SUPABASE_URL".to_string())?;
    let key = std::env::var("SUPABASE_ANON_KEY")
        .map_err(|_| "Missing SUPABASE_ANON_KEY".to_string())?;
        
    Ok((url, key))
}

pub async fn sign_up(
  email: String,
  password: String,
  given_name: Option<String>,
  family_name: Option<String>,
) -> Result<SignUpResult, String> {
  let (base_url, api_key) = get_env_vars()?;
  let endpoint = format!("{}/auth/v1/signup", base_url);

  // Supabase accepts arbitrary metadata in the signup body
  let mut data = HashMap::new();
  if let Some(gn) = given_name { data.insert("given_name".to_string(), gn); }
  if let Some(fn_name) = family_name { data.insert("family_name".to_string(), fn_name); }

  let body = serde_json::json!({
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
    let error_text = response.text().await.unwrap_or_default();
    return Err(format!("SignUp failed: {}", error_text));
  }

  // Note: If email confirmation is ON, Supabase returns the User but session might be null.
  // If OFF, it logs them in immediately.
  let json_response: serde_json::Value = response.json().await
      .map_err(|e| format!("Failed to parse response: {}", e))?;

  // Map to your existing SignUpResult type
  let user_id = json_response["id"].as_str().unwrap_or("").to_string();
  
  // You might want to adjust SignUpResult definition later, 
  // but here we map what we can.
  Ok(SignUpResult {
    user_sub: user_id, 
    user_confirmed: false, // In Supabase, usually requires email click unless auto-confirm is on
    verification_required: true,
    destination: Some(email.clone()),
    delivery_medium: Some("EMAIL".to_string()),
    session: None, // Logic depends on if auto-sign-in is enabled
  })
}

pub async fn sign_in(email: String, password: String) -> Result<SignInResult, String> {
  let (base_url, api_key) = get_env_vars()?;
  let endpoint = format!("{}/auth/v1/token?grant_type=password", base_url);

  let body = serde_json::json!({
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
    let error_text = response.text().await.unwrap_or_default();
    return Err(format!("SignIn failed: {}", error_text));
  }

  let auth_res: SupabaseAuthResponse = response.json().await
    .map_err(|e| format!("Failed to parse response: {}", e))?;

  // Extract metadata
  let meta = auth_res.user.user_metadata.unwrap_or_default();
  let given_name = meta.get("given_name").and_then(|v| v.as_str()).map(String::from);
  let family_name = meta.get("family_name").and_then(|v| v.as_str()).map(String::from);

  // Map to your existing types
  let user_info = CognitoUserInfo {
      username: auth_res.user.email.clone().unwrap_or_default(),
      email: auth_res.user.email,
      given_name,
      family_name,
      sub: auth_res.user.id,
  };

  let sign_in_result = SignInResult {
      access_token: auth_res.access_token,
      // Supabase access tokens are JWTs, so they double as ID tokens essentially, 
      // though typically you use access_token for API and refresh_token for keeping session
      id_token: Some("".to_string()), 
      refresh_token: Some(auth_res.refresh_token),
      expires_in: auth_res.expires_in,
      user_info,
  };

  // Reuse your existing storage logic
  if let Err(e) = store_cognito_auth(&sign_in_result) {
      log::warn!("Warning: Failed to store authentication: {}", e);
  }

  Ok(sign_in_result)
}

// Remove confirm_sign_up and resend_confirmation_code if you disable email confirmation
// Or implement them using Supabase's /verify endpoint if needed.
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