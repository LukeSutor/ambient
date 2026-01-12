#[derive(Clone)]
pub struct SupabaseUser {
  pub id: String,
  pub email: Option<String>,
  pub phone: Option<String>,
  pub confirmed_at: Option<String>,
  pub last_sign_in_at: Option<String>,
  pub app_metadata: serde_json::Value,
  pub user_metadata: serde_json::Value,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Clone)]
pub struct SupabaseAuthResponse {
  pub access_token: String,
  pub refresh_token: String,
  pub expires_in: i64,
  pub token_type: String,
  pub user: SupabaseUser,
}