use serde::Deserialize;
use base64::{Engine as _, engine::general_purpose};

#[derive(Debug, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub email: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    #[serde(rename = "cognito:username")]
    pub username: Option<String>,
    pub exp: i64,
}

/// Helper function to decode JWT without verification (for extracting user info)
pub fn decode_jwt_claims(token: &str) -> Result<JwtClaims, String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err("Invalid JWT token format".to_string());
    }

    let payload = parts[1];
    // Add padding if needed
    let padded_payload = match payload.len() % 4 {
        0 => payload.to_string(),
        n => format!("{}{}", payload, "=".repeat(4 - n)),
    };

    let decoded = general_purpose::STANDARD
        .decode(padded_payload)
        .map_err(|e| format!("Failed to decode JWT payload: {}", e))?;

    let json_str = String::from_utf8(decoded)
        .map_err(|e| format!("Failed to convert JWT payload to string: {}", e))?;

    serde_json::from_str::<JwtClaims>(&json_str)
        .map_err(|e| format!("Failed to parse JWT claims: {}", e))
}

/// Helper function to check if JWT token is expired
pub fn is_token_expired(token: &str) -> Result<bool, String> {
    let claims = decode_jwt_claims(token)?;
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("Failed to get current time: {}", e))?
        .as_secs() as i64;
    
    Ok(claims.exp < current_time)
}
