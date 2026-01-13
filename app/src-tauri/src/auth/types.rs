use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthToken {
  pub access_token: String,
  pub refresh_token: Option<String>,
  pub id_token: Option<String>,
  pub expires_in: Option<std::time::Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignUpResult {
  pub user_sub: String,
  pub user_confirmed: bool,
  pub verification_required: bool,
  pub destination: Option<String>,
  pub delivery_medium: Option<String>,
  pub session: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserInfo {
  pub username: String,
  pub email: Option<String>,
  pub given_name: Option<String>,
  pub family_name: Option<String>,
  pub sub: String, // User's unique identifier
}

// Alias for backward compatibility
pub type CognitoUserInfo = UserInfo;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignInResult {
  pub access_token: String,
  pub id_token: Option<String>,
  pub refresh_token: Option<String>,
  pub expires_in: i64,
  pub user_info: UserInfo,
}

// Constants
pub const KEYRING_SERVICE: &str = "local-computer-use";
pub const KEYRING_USER: &str = "oauth_tokens";
