use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// User metadata from Supabase - matches the user_metadata and identity_data fields
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct UserMetadata {
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub phone_verified: Option<bool>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub sub: Option<String>,
}

impl Default for UserMetadata {
    fn default() -> Self {
        Self {
            email: None,
            email_verified: None,
            phone_verified: None,
            given_name: None,
            family_name: None,
            sub: None,
        }
    }
}

/// App metadata from Supabase - contains provider information
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct AppMetadata {
    pub provider: Option<String>,
    pub providers: Option<Vec<String>>,
}

impl Default for AppMetadata {
    fn default() -> Self {
        Self {
            provider: None,
            providers: None,
        }
    }
}

/// User identity from Supabase
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct UserIdentity {
    pub identity_id: String,
    pub id: String,
    pub user_id: String,
    pub identity_data: Option<UserMetadata>,
    pub provider: String,
    pub last_sign_in_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub email: Option<String>,
}

/// Complete Supabase User object
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct SupabaseUser {
    pub id: String,
    pub aud: Option<String>,
    pub role: Option<String>,
    pub email: Option<String>,
    pub email_confirmed_at: Option<String>,
    pub phone: Option<String>,
    pub phone_confirmed_at: Option<String>,
    pub confirmation_sent_at: Option<String>,
    pub confirmed_at: Option<String>,
    pub last_sign_in_at: Option<String>,
    pub app_metadata: Option<AppMetadata>,
    pub user_metadata: Option<UserMetadata>,
    pub identities: Option<Vec<UserIdentity>>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub is_anonymous: Option<bool>,
}

/// Complete session object from Supabase
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct Session {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub expires_at: Option<i64>,
    pub refresh_token: String,
    pub user: SupabaseUser,
}

/// Weak password indicator from Supabase
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct WeakPassword {
    pub message: Option<String>,
    pub reasons: Option<Vec<String>>,
}

/// Sign In Response - returned after successful password sign in
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct AuthResponse {
    pub session: Option<Session>,
    pub user: Option<SupabaseUser>,
    pub weak_password: Option<WeakPassword>,
    /// Whether verification is required (email confirmation) - used if sign in fails due to unconfirmed email
    pub verification_required: bool,
    /// Where the verification was sent (email address)
    pub destination: Option<String>,
    /// Delivery medium (EMAIL, SMS)
    pub delivery_medium: Option<String>,
}

/// Sign Up Response - returned after user registration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct SignUpResponse {
    /// The user object if signup was successful
    pub user: Option<SupabaseUser>,
    /// Session is null if email confirmation is required
    pub session: Option<Session>,
    /// Whether verification is required (email confirmation)
    pub verification_required: bool,
    /// Where the verification was sent (email address)
    pub destination: Option<String>,
    /// Delivery medium (EMAIL, SMS)
    pub delivery_medium: Option<String>,
}

/// OTP Verification Response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct VerifyOtpResponse {
    pub session: Option<Session>,
    pub user: Option<SupabaseUser>,
}

/// Token Refresh Response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct RefreshTokenResponse {
    pub session: Session,
    pub user: SupabaseUser,
}

/// Resend Confirmation Response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct ResendConfirmationResponse {
    pub message_id: Option<String>,
}

/// Supabase Auth Error Response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct AuthError {
    pub code: Option<String>,
    pub error_code: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
    #[serde(rename = "msg")]
    pub message: Option<String>,
}

impl AuthError {
    pub fn get_message(&self) -> String {
        self.message
            .clone()
            .or(self.error_description.clone())
            .or(self.error.clone())
            .or(self.error_code.clone())
            .or(self.code.clone())
            .unwrap_or_else(|| "Unknown error".to_string())
    }
}

/// The complete auth state that gets stored locally
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct StoredAuthState {
    /// Current session with tokens
    pub session: Session,
    /// Timestamp when the auth state was stored (Unix timestamp)
    pub stored_at: i64,
}

impl StoredAuthState {
    pub fn new(session: Session) -> Self {
        Self {
            session,
            stored_at: chrono::Utc::now().timestamp(),
        }
    }
    
    /// Check if the access token has expired
    pub fn is_access_token_expired(&self) -> bool {
        if let Some(expires_at) = self.session.expires_at {
            let now = chrono::Utc::now().timestamp();
            now >= expires_at
        } else {
            // Fall back to expires_in calculation
            let now = chrono::Utc::now().timestamp();
            let expires_at = self.stored_at + self.session.expires_in;
            now >= expires_at
        }
    }
    
    /// Check if token needs refresh (within 5 minutes of expiry)
    pub fn needs_refresh(&self) -> bool {
        const REFRESH_THRESHOLD_SECS: i64 = 300; // 5 minutes
        
        if let Some(expires_at) = self.session.expires_at {
            let now = chrono::Utc::now().timestamp();
            now >= (expires_at - REFRESH_THRESHOLD_SECS)
        } else {
            let now = chrono::Utc::now().timestamp();
            let expires_at = self.stored_at + self.session.expires_in;
            now >= (expires_at - REFRESH_THRESHOLD_SECS)
        }
    }
}

/// Simplified user info for frontend usage
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct UserInfo {
    pub id: String,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub email_verified: Option<bool>,
    pub provider: Option<String>,
    pub created_at: Option<String>,
}

impl From<&SupabaseUser> for UserInfo {
    fn from(user: &SupabaseUser) -> Self {
        let (given_name, family_name, email_verified) = user
            .user_metadata
            .as_ref()
            .map(|m| (m.given_name.clone(), m.family_name.clone(), m.email_verified))
            .unwrap_or((None, None, None));
        
        let provider = user
            .app_metadata
            .as_ref()
            .and_then(|m| m.provider.clone());
        
        Self {
            id: user.id.clone(),
            email: user.email.clone(),
            given_name,
            family_name,
            email_verified,
            provider,
            created_at: user.created_at.clone(),
        }
    }
}

/// Current auth state exposed to the frontend
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "auth.ts")]
pub struct AuthState {
    pub is_authenticated: bool,
    pub user: Option<UserInfo>,
    pub access_token: Option<String>,
    pub expires_at: Option<i64>,
}

// Keyrind constants
pub const KEYRING_SERVICE: &str = "local-computer-use";
pub const KEYRING_AUTH_KEY: &str = "supabase_auth";
