use serde::{Deserialize, Serialize};

// OAuth2 provider types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OAuth2Provider {
  Google,
  Microsoft,
}

// OAuth2 token response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2TokenResponse {
  pub access_token: String,
  pub refresh_token: Option<String>,
  pub id_token: Option<String>,
  pub token_type: String,
  pub expires_in: Option<i64>,
  pub scope: Option<String>,
}

// OAuth2 user info structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2UserInfo {
  pub id: String,
  pub email: Option<String>,
  pub name: Option<String>,
  pub given_name: Option<String>,
  pub family_name: Option<String>,
  pub picture: Option<String>,
  pub provider: OAuth2Provider,
}
