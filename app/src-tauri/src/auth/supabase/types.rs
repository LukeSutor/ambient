use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SupabaseAuthResponse {
    pub access_token: Option<String>,
    pub token_type: Option<String>,
    pub expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub user: Option<SupabaseUser>,
    pub session: Option<SupabaseSession>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SupabaseSession {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: String,
    pub user: SupabaseUser,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SupabaseUser {
    pub id: String,
    pub email: Option<String>,
    pub user_metadata: Option<HashMap<String, serde_json::Value>>,
    pub confirmed_at: Option<String>,
    pub app_metadata: Option<HashMap<String, serde_json::Value>>,
    pub created_at: String,
    pub updated_at: String,
    pub is_anonymous: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SupabaseErrorResponse {
    pub code: Option<String>,
    pub error_code: Option<String>,
    pub msg: Option<String>,
}

impl SupabaseErrorResponse {
    pub fn message(&self) -> String {
        self.msg
            .clone()
            .or(self.code.clone())
            .or(self.error_code.clone())
            .unwrap_or_else(|| "Unknown error".to_string())
    }
}