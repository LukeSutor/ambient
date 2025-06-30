use crate::auth::storage::store_cognito_auth;
use crate::auth::types::{CognitoUserInfo, SignInResult};
use base64::{engine::general_purpose, Engine as _};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
extern crate dotenv;

#[derive(Debug, Serialize)]
struct AuthorizeParams {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: String,
    identity_provider: String,
    scope: String,
}

#[derive(Debug, Serialize)]
struct TokenRequest {
    grant_type: String,
    client_id: String,
    code: String,
    redirect_uri: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    id_token: String,
    refresh_token: String,
    expires_in: i64,
    token_type: String,
}

#[derive(Debug, Serialize)]
struct RevokeRequest {
    token: String,
}

/// Initiate Google OAuth2 authentication through AWS Cognito
pub async fn initiate_google_auth() -> Result<String, String> {
    // Load environment variables
    if let Err(e) = dotenv::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }

    let client_id = std::env::var("COGNITO_CLIENT_ID")
        .map_err(|_| "Missing COGNITO_CLIENT_ID environment variable".to_string())?;
    
    let domain = std::env::var("COGNITO_USER_POOL_DOMAIN")
        .map_err(|_| "Missing COGNITO_USER_POOL_DOMAIN environment variable".to_string())?;

    let region = std::env::var("COGNITO_REGION")
        .map_err(|_| "Missing COGNITO_REGION environment variable".to_string())?;

    // Construct Cognito domain URL (similar to existing pattern)
    let cognito_domain = if domain.starts_with("http") {
        domain
    } else {
        format!("https://{}.auth.{}.amazoncognito.com", domain, region)
    };

    // Generate state parameter for security
    let state = uuid::Uuid::new_v4().to_string();

    // Build authorization URL
    let redirect_uri = "cortical://auth/callback";
    let auth_url = format!(
        "{}/oauth2/authorize?response_type=code&client_id={}&redirect_uri={}&state={}&identity_provider=Google&scope=profile email openid",
        cognito_domain,
        urlencoding::encode(&client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(&state)
    );

    // Store state for verification in callback
    // Note: In a real implementation, you'd want to store this securely
    // For now, we'll rely on the state being returned in the callback
    
    Ok(auth_url)
}

/// Handle the OAuth2 callback from Google through Cognito
pub async fn handle_google_callback(code: String, state: Option<String>) -> Result<SignInResult, String> {
    // Load environment variables
    if let Err(e) = dotenv::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }

    println!("Handling Google OAuth2 callback with code: {}", code);

    let client_id = std::env::var("COGNITO_CLIENT_ID")
        .map_err(|_| "Missing COGNITO_CLIENT_ID environment variable".to_string())?;
    
    // For public clients, no client secret is needed
    let client_secret = std::env::var("COGNITO_CLIENT_SECRET").ok();
    
    let domain = std::env::var("COGNITO_USER_POOL_DOMAIN")
        .map_err(|_| "Missing COGNITO_USER_POOL_DOMAIN environment variable".to_string())?;
    
    let region = std::env::var("COGNITO_REGION")
        .map_err(|_| "Missing COGNITO_REGION environment variable".to_string())?;

    // Construct Cognito domain URL
    let cognito_domain = if domain.starts_with("http") {
        domain
    } else {
        format!("https://{}.auth.{}.amazoncognito.com", domain, region)
    };

    // Create authorization header (only if client secret exists)
    let auth_header = if let Some(secret) = &client_secret {
        Some(base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", client_id, secret)))
    } else {
        None
    };
    
    // Prepare token request
    let redirect_uri = "cortical://auth/callback";
    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("client_id", &client_id);
    params.insert("code", &code);
    params.insert("redirect_uri", redirect_uri);

    // Make token request
    let client = reqwest::Client::new();
    let mut request = client
        .post(format!("{}/oauth2/token", cognito_domain))
        .header("Content-Type", "application/x-www-form-urlencoded");
    
    // Add authorization header only if we have a client secret
    if let Some(auth) = auth_header {
        request = request.header("Authorization", format!("Basic {}", auth));
    }
    
    let response = request
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to send token request: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Token request failed: {}", error_text));
    }

    let token_data: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    // Decode user info from ID token
    let user_info = extract_user_info_from_id_token(&token_data.id_token)?;

    // Create SignInResult
    let sign_in_result = SignInResult {
        access_token: token_data.access_token.clone(),
        id_token: token_data.id_token.clone(),
        refresh_token: token_data.refresh_token.clone(),
        expires_in: token_data.expires_in,
        user_info: user_info.clone(),
    };

    // Store authentication data
    store_cognito_auth(&sign_in_result)
        .map_err(|e| format!("Failed to store authentication: {}", e))?;

    Ok(sign_in_result)
}

/// Sign out user by revoking tokens
pub async fn google_sign_out() -> Result<String, String> {
    use crate::auth::storage::{clear_cognito_auth, retrieve_cognito_auth};

    // Get stored auth
    let auth = retrieve_cognito_auth()
        .map_err(|e| format!("Failed to retrieve auth: {}", e))?
        .ok_or("No authentication found")?;

    // Load environment variables
    if let Err(e) = dotenv::dotenv() {
        eprintln!("Warning: Could not load .env file: {}", e);
    }

    let client_id = std::env::var("COGNITO_CLIENT_ID")
        .map_err(|_| "Missing COGNITO_CLIENT_ID environment variable".to_string())?;
    
    // For public clients, no client secret is needed
    let client_secret = std::env::var("COGNITO_CLIENT_SECRET").ok();
    
    let domain = std::env::var("COGNITO_USER_POOL_DOMAIN")
        .map_err(|_| "Missing COGNITO_USER_POOL_DOMAIN environment variable".to_string())?;
    
    let region = std::env::var("COGNITO_REGION")
        .map_err(|_| "Missing COGNITO_REGION environment variable".to_string())?;

    // Construct Cognito domain URL
    let cognito_domain = if domain.starts_with("http") {
        domain
    } else {
        format!("https://{}.auth.{}.amazoncognito.com", domain, region)
    };

    // Create authorization header (only if client secret exists)
    let auth_header = if let Some(secret) = &client_secret {
        Some(base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", client_id, secret)))
    } else {
        None
    };
    
    // Revoke refresh token
    let mut params = HashMap::new();
    params.insert("token", &auth.refresh_token);
    
    // For public clients, include client_id in the body instead of Authorization header
    if client_secret.is_none() {
        params.insert("client_id", &client_id);
    }

    let client = reqwest::Client::new();
    let mut request = client
        .post(format!("{}/oauth2/revoke", cognito_domain))
        .header("Content-Type", "application/x-www-form-urlencoded");
    
    // Add authorization header only if we have a client secret
    if let Some(auth) = auth_header {
        request = request.header("Authorization", format!("Basic {}", auth));
    }
    
    let response = request
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Failed to revoke token: {}", e))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        eprintln!("Warning: Failed to revoke token: {}", error_text);
        // Continue with clearing local auth even if revoke fails
    }

    // Clear stored authentication
    clear_cognito_auth()
        .map_err(|e| format!("Failed to clear authentication: {}", e))?;

    Ok("Successfully signed out".to_string())
}

/// Extract user information from ID token
fn extract_user_info_from_id_token(id_token: &str) -> Result<CognitoUserInfo, String> {
    use crate::auth::jwt::decode_jwt_claims;
    
    let claims = decode_jwt_claims(id_token)?;
    
    Ok(CognitoUserInfo {
        username: claims.username
            .or(claims.email.clone())
            .unwrap_or_else(|| claims.sub.clone()),
        email: claims.email,
        given_name: claims.given_name,
        family_name: claims.family_name,
        sub: claims.sub,
    })
}
