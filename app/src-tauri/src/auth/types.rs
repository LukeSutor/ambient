use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserInfo {
  pub username: String,
  pub email: Option<String>,
  pub given_name: Option<String>,
  pub family_name: Option<String>,
  pub sub: String, // User's unique identifier
}

// Constants
pub const KEYRING_SERVICE: &str = "local-computer-use";
