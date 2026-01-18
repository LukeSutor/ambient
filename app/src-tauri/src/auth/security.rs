//! Security utilities for OAuth PKCE flow, CSRF protection, and rate limiting.

use once_cell::sync::Lazy;
use rand::Rng;
use sha2::{Sha256, Digest};
use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ============================================================================
// Shared HTTP Client
// ============================================================================

/// Shared HTTP client for all auth requests to avoid per-request overhead
pub static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .pool_max_idle_per_host(5)
        .build()
        .expect("Failed to create HTTP client")
});

// ============================================================================
// PKCE Flow
// ============================================================================

/// PKCE state stored during OAuth flow
#[derive(Debug, Clone)]
pub struct PkceState {
    pub code_verifier: String,
    pub state: String,
    pub created_at: Instant,
}

/// In-memory storage for PKCE state during OAuth flow
/// Uses a HashMap with state as key for quick lookup during callback
static PKCE_STORE: Lazy<Mutex<HashMap<String, PkceState>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

/// Characters allowed in PKCE code verifier (RFC 7636)
const PKCE_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";

/// Generate a cryptographically random code verifier for PKCE
/// Length is 64 characters (within RFC 7636 range of 43-128)
pub fn generate_code_verifier() -> String {
    let mut rng = rand::thread_rng();
    (0..64)
        .map(|_| {
            let idx = rng.gen_range(0..PKCE_CHARSET.len());
            PKCE_CHARSET[idx] as char
        })
        .collect()
}

/// Generate code challenge from verifier using S256 method (SHA-256 + base64url)
pub fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    BASE64_URL_SAFE_NO_PAD.encode(hash)
}

/// Generate a cryptographically random state parameter for CSRF protection
pub fn generate_state() -> String {
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    BASE64_URL_SAFE_NO_PAD.encode(&bytes)
}

/// Store PKCE state for later validation
/// Returns (code_challenge, state) to include in the OAuth URL
pub fn store_pkce_state() -> (String, String) {
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);
    let state = generate_state();
    
    let pkce_state = PkceState {
        code_verifier: code_verifier.clone(),
        state: state.clone(),
        created_at: Instant::now(),
    };
    
    // Clean up old entries (older than 10 minutes) before storing new one
    if let Ok(mut store) = PKCE_STORE.lock() {
        let cutoff = Instant::now() - Duration::from_secs(600);
        store.retain(|_, v| v.created_at > cutoff);
        store.insert(state.clone(), pkce_state);
    }
    
    (code_challenge, state)
}

/// Retrieve and consume PKCE state by state parameter
/// Returns the code_verifier if found and valid
pub fn retrieve_pkce_state(state: &str) -> Result<String, String> {
    let mut store = PKCE_STORE.lock()
        .map_err(|_| "Failed to acquire PKCE store lock")?;
    
    match store.remove(state) {
        Some(pkce_state) => {
            // Check if state is not expired (10 minute max lifetime)
            if pkce_state.created_at.elapsed() > Duration::from_secs(600) {
                return Err("OAuth state expired. Please try again.".to_string());
            }
            Ok(pkce_state.code_verifier)
        }
        None => Err("Invalid OAuth state. This may be a CSRF attack or the request has expired.".to_string())
    }
}

// ============================================================================
// Rate Limiting
// ============================================================================

/// Rate limiter state per operation type
#[derive(Debug, Clone)]
struct RateLimitState {
    attempts: Vec<Instant>,
    lockout_until: Option<Instant>,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self {
            attempts: Vec::new(),
            lockout_until: None,
        }
    }
}

/// Rate limit configuration
const MAX_ATTEMPTS_PER_WINDOW: usize = 5;
const WINDOW_DURATION_SECS: u64 = 60; // 1 minute window
const LOCKOUT_DURATION_SECS: u64 = 300; // 5 minute lockout after exceeding limit

/// In-memory rate limiting store keyed by operation + identifier (e.g., "sign_in:user@email.com")
static RATE_LIMIT_STORE: Lazy<Mutex<HashMap<String, RateLimitState>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});

/// Rate limit operations
#[derive(Debug, Clone, Copy)]
pub enum RateLimitOp {
    SignIn,
    SignUp,
    ResendConfirmation,
    VerifyOtp,
    RefreshToken,
}

impl RateLimitOp {
    fn as_str(&self) -> &'static str {
        match self {
            RateLimitOp::SignIn => "sign_in",
            RateLimitOp::SignUp => "sign_up",
            RateLimitOp::ResendConfirmation => "resend_confirmation",
            RateLimitOp::VerifyOtp => "verify_otp",
            RateLimitOp::RefreshToken => "refresh_token",
        }
    }
}

/// Check if an operation is rate limited
/// Returns Ok(()) if allowed, Err with message if rate limited
pub fn check_rate_limit(op: RateLimitOp, identifier: &str) -> Result<(), String> {
    let key = format!("{}:{}", op.as_str(), identifier);
    let now = Instant::now();
    
    let mut store = RATE_LIMIT_STORE.lock()
        .map_err(|_| "Internal error: failed to check rate limit")?;
    
    let state = store.entry(key).or_default();
    
    // Check if currently locked out
    if let Some(lockout_until) = state.lockout_until {
        if now < lockout_until {
            let remaining = (lockout_until - now).as_secs();
            return Err(format!(
                "Too many attempts. Please try again in {} seconds.",
                remaining
            ));
        } else {
            // Lockout expired, reset state
            state.lockout_until = None;
            state.attempts.clear();
        }
    }
    
    // Remove attempts outside the window
    let window_start = now - Duration::from_secs(WINDOW_DURATION_SECS);
    state.attempts.retain(|&t| t > window_start);
    
    // Check if limit exceeded
    if state.attempts.len() >= MAX_ATTEMPTS_PER_WINDOW {
        state.lockout_until = Some(now + Duration::from_secs(LOCKOUT_DURATION_SECS));
        return Err(format!(
            "Too many attempts. Please try again in {} seconds.",
            LOCKOUT_DURATION_SECS
        ));
    }
    
    Ok(())
}

/// Record an attempt for rate limiting
pub fn record_attempt(op: RateLimitOp, identifier: &str) {
    let key = format!("{}:{}", op.as_str(), identifier);
    
    if let Ok(mut store) = RATE_LIMIT_STORE.lock() {
        let state = store.entry(key).or_default();
        state.attempts.push(Instant::now());
    }
}

/// Clear rate limit state for an identifier (e.g., after successful auth)
pub fn clear_rate_limit(op: RateLimitOp, identifier: &str) {
    let key = format!("{}:{}", op.as_str(), identifier);
    
    if let Ok(mut store) = RATE_LIMIT_STORE.lock() {
        store.remove(&key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pkce_verifier_length() {
        let verifier = generate_code_verifier();
        assert_eq!(verifier.len(), 64);
    }
    
    #[test]
    fn test_pkce_verifier_charset() {
        let verifier = generate_code_verifier();
        for c in verifier.chars() {
            assert!(PKCE_CHARSET.contains(&(c as u8)));
        }
    }
    
    #[test]
    fn test_code_challenge_deterministic() {
        let verifier = "test_verifier_string";
        let challenge1 = generate_code_challenge(verifier);
        let challenge2 = generate_code_challenge(verifier);
        assert_eq!(challenge1, challenge2);
    }
    
    #[test]
    fn test_state_uniqueness() {
        let state1 = generate_state();
        let state2 = generate_state();
        assert_ne!(state1, state2);
    }
}
