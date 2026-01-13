// New auth modules
pub mod auth_types;
pub mod auth_storage;

// Legacy modules (kept for compatibility during transition)
pub mod supabase;
pub mod commands;
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
};

// Re-export JWT utilities
pub use jwt::*;

// Re-export deep link functionality
pub use deep_link::*;

// Legacy re-exports (for backward compatibility with existing code)
// These can be gradually removed as the codebase is updated
pub use types::{SignUpResult, SignInResult, UserInfo as LegacyUserInfo};
