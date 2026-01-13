use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct SupabaseAuthResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub user: Option<SupabaseUser>,
    pub session: Option<SupabaseSession>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SupabaseSession {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub user: SupabaseUser,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SupabaseUser {
    pub id: String,
    pub email: Option<String>, 
    pub user_metadata: Option<HashMap<String, serde_json::Value>>,
    pub confirmed_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SupabaseErrorResponse {
    pub msg: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

impl SupabaseErrorResponse {
    pub fn message(&self) -> String {
        self.error_description
            .clone()
            .or(self.msg.clone())
            .or(self.error.clone())
            .unwrap_or_else(|| "Unknown error".to_string())
    }
}