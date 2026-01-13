use crate::auth::types::{AuthToken, SignInResult, KEYRING_SERVICE, KEYRING_USER};
use keyring::Entry;
use std::fs;
use std::path::PathBuf;

/// Store OAuth token in keyring
pub fn store_token(token: &AuthToken) -> Result<(), Box<dyn std::error::Error>> {
  let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
  let token_json = serde_json::to_string(token)?;
  entry.set_password(&token_json)?;
  Ok(())
}

/// Retrieve OAuth token from keyring
pub fn retrieve_token() -> Result<Option<AuthToken>, Box<dyn std::error::Error>> {
  let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
  match entry.get_password() {
    Ok(token_json) => {
      let token: AuthToken = serde_json::from_str(&token_json)?;
      Ok(Some(token))
    }
    Err(keyring::Error::NoEntry) => Ok(None),
    Err(e) => Err(Box::new(e)),
  }
}

/// Clear OAuth token from keyring
pub fn clear_stored_token() -> Result<(), Box<dyn std::error::Error>> {
  let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
  entry.delete_password()?;
  Ok(())
}

/// Store Authentication result securely
pub fn store_auth(sign_in_result: &SignInResult) -> Result<(), Box<dyn std::error::Error>> {
  // Get the app data directory
  let app_data_dir = get_app_data_dir()?;
  let auth_file_path = app_data_dir.join("auth.json");

  // Serialize the auth result
  let auth_json = serde_json::to_string(sign_in_result)?;

  // Write to file with secure permissions
  fs::write(&auth_file_path, auth_json)?;

  // Also store a flag in keyring to indicate we have auth stored
  let entry = Entry::new(KEYRING_SERVICE, "has_auth")?;
  entry.set_password("true")?;

  Ok(())
}

/// Retrieve stored Authentication
pub fn retrieve_auth() -> Result<Option<SignInResult>, Box<dyn std::error::Error>> {
  // First check if we have auth stored
  let entry = Entry::new(KEYRING_SERVICE, "has_auth")?;
  match entry.get_password() {
    Ok(_) => {
      // We have auth, try to read from file
      let app_data_dir = get_app_data_dir()?;
      let auth_file_path = app_data_dir.join("auth.json");

      if auth_file_path.exists() {
        let auth_json = fs::read_to_string(&auth_file_path)?;
        let sign_in_result: SignInResult = serde_json::from_str(&auth_json)?;
        Ok(Some(sign_in_result))
      } else {
        // File doesn't exist, clear the flag and return None
        let _ = entry.delete_password();
        Ok(None)
      }
    }
    Err(keyring::Error::NoEntry) => Ok(None),
    Err(e) => Err(Box::new(e)),
  }
}

/// Clear stored Authentication
pub fn clear_auth() -> Result<(), Box<dyn std::error::Error>> {
  // Clear the keyring flag
  let entry = Entry::new(KEYRING_SERVICE, "has_auth")?;
  let _ = entry.delete_password();

  // Remove the auth file
  let app_data_dir = get_app_data_dir()?;
  let auth_file_path = app_data_dir.join("auth.json");

  if auth_file_path.exists() {
    fs::remove_file(&auth_file_path)?;
  }

  Ok(())
}

/// Helper function to get app data directory
fn get_app_data_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
  let app_data_dir = if cfg!(target_os = "windows") {
    std::env::var("APPDATA")
      .map(|path| std::path::PathBuf::from(path))
      .unwrap_or_else(|_| {
        std::env::var("USERPROFILE")
          .map(|path| {
            std::path::PathBuf::from(path)
              .join("AppData")
              .join("Roaming")
          })
          .unwrap_or_else(|_| std::path::PathBuf::from("."))
      })
      .join("local-computer-use")
  } else if cfg!(target_os = "macos") {
    std::env::var("HOME")
      .map(|path| {
        std::path::PathBuf::from(path)
          .join("Library")
          .join("Application Support")
          .join("local-computer-use")
      })
      .unwrap_or_else(|_| std::path::PathBuf::from(".").join("local-computer-use"))
  } else {
    // Linux
    std::env::var("HOME")
      .map(|path| {
        std::path::PathBuf::from(path)
          .join(".local")
          .join("share")
          .join("local-computer-use")
      })
      .unwrap_or_else(|_| std::path::PathBuf::from(".").join("local-computer-use"))
  };

  // Create directory if it doesn't exist
  if !app_data_dir.exists() {
    fs::create_dir_all(&app_data_dir)?;
  }

  Ok(app_data_dir)
}
