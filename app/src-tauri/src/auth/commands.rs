use crate::auth::supabase;
use crate::auth::jwt::is_token_expired;
use crate::auth::storage::{
  clear_auth, clear_stored_token, retrieve_auth, retrieve_token,
};
use crate::auth::types::{UserInfo, SignInResult, SignUpResult};
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn logout() -> Result<String, String> {
  // Clear OAuth tokens
  if let Err(e) = clear_stored_token() {
    log::warn!("Warning: Failed to clear OAuth token: {}", e);
  }

  // Clear authentication
  clear_auth().map_err(|e| format!("Failed to clear authentication: {}", e))?;

  Ok("Logged out successfully".to_string())
}

#[tauri::command]
pub async fn get_stored_token() -> Result<Option<crate::auth::types::AuthToken>, String> {
  retrieve_token().map_err(|e| format!("Failed to retrieve token: {}", e))
}

#[tauri::command]
pub async fn is_authenticated() -> Result<bool, String> {
  // Check OAuth tokens first
  if let Ok(Some(_token)) = retrieve_token() {
    return Ok(true);
  }

  // Check Supabase authentication
  match retrieve_auth() {
    Ok(Some(auth)) => {
      // Check if the access token is expired
       match is_token_expired(&auth.access_token) {
        Ok(true) => {
          // Token is expired, clear auth and return false
          let _ = clear_auth();
          Ok(false)
        }
        Ok(false) => {
          // Token is still valid
          Ok(true)
        }
        Err(_) => {
          // Error checking expiration, assume expired and clear
          let _ = clear_auth();
          Ok(false)
        }
      }
    }
    Ok(None) => Ok(false),
    Err(_) => Ok(false),
  }
}

#[tauri::command]
pub async fn cognito_sign_up(
  email: String,
  password: String,
  given_name: Option<String>,
  family_name: Option<String>,
) -> Result<SignUpResult, String> {
  // Using Supabase implementation
  supabase::sign_up(email, password, given_name, family_name).await
}

#[tauri::command]
pub async fn cognito_confirm_sign_up(
  email: String,
  confirmation_code: String,
) -> Result<String, String> {
  // Supabase 'verify' uses type='signup' for email verification
  supabase::verify_otp(email, confirmation_code, "signup".to_string()).await
}

#[tauri::command]
pub async fn cognito_resend_confirmation_code(email: String) -> Result<SignUpResult, String> {
  supabase::resend_confirmation(email).await?;
  Ok(SignUpResult {
      user_sub: "".to_string(),
      user_confirmed: false,
      verification_required: true,
      destination: None,
      delivery_medium: None,
      session: None
  })
}

#[tauri::command]
pub async fn cognito_sign_in(email: String, password: String) -> Result<SignInResult, String> {
  supabase::sign_in(email, password).await
}

#[tauri::command]
pub async fn get_current_user() -> Result<Option<UserInfo>, String> {
  match retrieve_auth() {
    Ok(Some(auth)) => match is_token_expired(&auth.access_token) {
      Ok(true) => {
        let _ = clear_auth();
        Ok(None)
      }
      Ok(false) => Ok(Some(auth.user_info)),
      Err(err) => {
        log::error!("Failed to check token expiration: {}", err);
        let _ = clear_auth();
        Ok(None)
      }
    },
    Ok(None) => Ok(None),
    Err(err) => {
      log::error!("Failed to retrieve user: {}", err);
      Ok(None)
    }
  }
}

#[tauri::command]
pub async fn get_access_token() -> Result<Option<String>, String> {
  match retrieve_auth() {
    Ok(Some(auth)) => {
      // Check if token is still valid
      match is_token_expired(&auth.access_token) {
        Ok(true) => {
          // Token is expired, clear auth and return None
          let _ = clear_auth();
          Ok(None)
        }
        Ok(false) => {
          // Token is still valid, return access token
          Ok(Some(auth.access_token))
        }
        Err(_) => {
          // Error checking expiration, assume expired and clear
          let _ = clear_auth();
          Ok(None)
        }
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
