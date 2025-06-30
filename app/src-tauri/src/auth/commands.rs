use crate::auth::cognito;
use crate::auth::jwt::is_token_expired;
use crate::auth::oauth2::google; // Add this import
use crate::auth::storage::{
  clear_cognito_auth, clear_stored_token, retrieve_cognito_auth, retrieve_token,
};
use crate::auth::types::{CognitoUserInfo, SignInResult, SignUpResult};

#[tauri::command]
pub async fn logout() -> Result<String, String> {
  // Clear OAuth tokens
  if let Err(e) = clear_stored_token() {
    eprintln!("Warning: Failed to clear OAuth token: {}", e);
  }

  // Clear Cognito authentication
  clear_cognito_auth().map_err(|e| format!("Failed to clear authentication: {}", e))?;

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

  // Check Cognito authentication
  match retrieve_cognito_auth() {
    Ok(Some(auth)) => {
      // Check if the access token is expired
      match is_token_expired(&auth.access_token) {
        Ok(true) => {
          // Token is expired, clear auth and return false
          let _ = clear_cognito_auth();
          Ok(false)
        }
        Ok(false) => {
          // Token is still valid
          Ok(true)
        }
        Err(_) => {
          // Error checking expiration, assume expired and clear
          let _ = clear_cognito_auth();
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
  username: String,
  password: String,
  email: String,
  given_name: Option<String>,
  family_name: Option<String>,
) -> Result<SignUpResult, String> {
  cognito::sign_up(username, password, email, given_name, family_name).await
}

#[tauri::command]
pub async fn cognito_confirm_sign_up(
  username: String,
  confirmation_code: String,
  session: Option<String>,
) -> Result<String, String> {
  cognito::confirm_sign_up(username, confirmation_code, session).await
}

#[tauri::command]
pub async fn cognito_resend_confirmation_code(username: String) -> Result<SignUpResult, String> {
  cognito::resend_confirmation_code(username).await
}

#[tauri::command]
pub async fn cognito_sign_in(username: String, password: String) -> Result<SignInResult, String> {
  cognito::sign_in(username, password).await
}

#[tauri::command]
pub async fn get_current_user() -> Result<Option<CognitoUserInfo>, String> {
  match retrieve_cognito_auth() {
    Ok(Some(auth)) => {
      // Check if token is still valid
      match is_token_expired(&auth.access_token) {
        Ok(true) => {
          // Token is expired, clear auth and return None
          let _ = clear_cognito_auth();
          Ok(None)
        }
        Ok(false) => {
          // Token is still valid, return user info
          Ok(Some(auth.user_info))
        }
        Err(_) => {
          // Error checking expiration, assume expired and clear
          let _ = clear_cognito_auth();
          Ok(None)
        }
      }
    }
    Ok(None) => Ok(None),
    Err(e) => Err(format!("Failed to retrieve user: {}", e)),
  }
}

#[tauri::command]
pub async fn get_access_token() -> Result<Option<String>, String> {
  match retrieve_cognito_auth() {
    Ok(Some(auth)) => {
      // Check if token is still valid
      match is_token_expired(&auth.access_token) {
        Ok(true) => {
          // Token is expired, clear auth and return None
          let _ = clear_cognito_auth();
          Ok(None)
        }
        Ok(false) => {
          // Token is still valid, return access token
          Ok(Some(auth.access_token))
        }
        Err(_) => {
          // Error checking expiration, assume expired and clear
          let _ = clear_cognito_auth();
          Ok(None)
        }
      }
    }
    Ok(None) => Ok(None),
    Err(e) => Err(format!("Failed to retrieve access token: {}", e)),
  }
}

// Google OAuth2 commands
#[tauri::command]
pub async fn google_initiate_auth() -> Result<String, String> {
  google::initiate_google_auth().await
}

#[tauri::command]
pub async fn google_handle_callback(code: String, state: Option<String>) -> Result<SignInResult, String> {
  google::handle_google_callback(code, state).await
}

#[tauri::command]
pub async fn google_sign_out() -> Result<String, String> {
  google::google_sign_out().await
}
