// New auth modules
pub mod auth_types;
pub mod auth_storage;
pub mod supabase_auth;
pub mod auth_commands;

// Legacy modules (kept for compatibility during transition)
pub mod supabase;
pub mod deep_link;
pub mod jwt;
pub mod storage;
pub mod types;

// Re-export new auth types
pub use auth_types::*;

// Re-export new auth storage
pub use auth_storage::{
    store_session, retrieve_auth_state, get_current_session,
    get_access_token as get_stored_access_token, get_refresh_token,
    needs_token_refresh, clear_auth_state, update_session,
    // Legacy compatibility
    store_token, retrieve_token, clear_stored_token,
};

// Re-export new auth service
pub use supabase_auth::{
    sign_up as supabase_sign_up,
    sign_in_with_password,
    refresh_session,
    refresh_session_with_token,
    verify_otp as supabase_verify_otp,
    resend_confirmation as supabase_resend_confirmation,
    sign_out as supabase_sign_out,
    get_user as supabase_get_user,
};

// Re-export new auth commands
pub use auth_commands::*;

// Re-export JWT utilities
pub use jwt::*;

// Re-export deep link functionality
pub use deep_link::*;

// Legacy re-exports (for backward compatibility with existing code)
// These can be gradually removed as the codebase is updated
pub use types::{SignUpResult, SignInResult, UserInfo as LegacyUserInfo, AuthToken as LegacyAuthToken};
