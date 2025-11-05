use crate::auth::cognito::types::*;
use crate::auth::jwt::decode_jwt_claims;
use crate::auth::storage::store_cognito_auth;
use crate::auth::types::{CognitoUserInfo, SignInResult, SignUpResult};
use std::collections::HashMap;
extern crate dotenv;

pub async fn sign_up(
  username: String,
  password: String,
  email: String,
  given_name: Option<String>,
  family_name: Option<String>,
) -> Result<SignUpResult, String> {
  // Load environment variables
  if let Err(e) = dotenv::dotenv() {
    log::warn!("Warning: Could not load .env file: {}", e);
  }

  let client_id = std::env::var("COGNITO_CLIENT_ID")
    .map_err(|_| "Missing COGNITO_CLIENT_ID environment variable".to_string())?;

  let region = std::env::var("COGNITO_REGION")
    .map_err(|_| "Missing COGNITO_REGION environment variable".to_string())?;

  let endpoint = format!("https://cognito-idp.{}.amazonaws.com/", region);

  // Prepare user attributes
  let mut user_attributes = vec![CognitoAttribute {
    name: "email".to_string(),
    value: email,
  }];

  if let Some(given_name) = given_name {
    user_attributes.push(CognitoAttribute {
      name: "given_name".to_string(),
      value: given_name,
    });
  }

  if let Some(family_name) = family_name {
    user_attributes.push(CognitoAttribute {
      name: "family_name".to_string(),
      value: family_name,
    });
  }

  let request_body = CognitoSignUpRequest {
    client_id,
    username: username.clone(),
    password,
    user_attributes,
  };

  // Make the request to AWS Cognito
  let client = reqwest::Client::new();
  let response = client
    .post(&endpoint)
    .header("X-Amz-Target", "AWSCognitoIdentityProviderService.SignUp")
    .header("Content-Type", "application/x-amz-json-1.1")
    .json(&request_body)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

  if !response.status().is_success() {
    let error_text = response
      .text()
      .await
      .unwrap_or_else(|_| "Unknown error".to_string());
    log::warn!("SignUp failed: {}", error_text);
    return Err(error_text);
  }

  let signup_response: CognitoSignUpResponse = response
    .json()
    .await
    .map_err(|e| format!("Failed to parse response: {}", e))?;

  Ok(SignUpResult {
    user_sub: signup_response.user_sub,
    user_confirmed: signup_response.user_confirmed,
    verification_required: !signup_response.user_confirmed,
    destination: signup_response
      .code_delivery_details
      .as_ref()
      .map(|cd| cd.destination.clone()),
    delivery_medium: signup_response
      .code_delivery_details
      .as_ref()
      .map(|cd| cd.delivery_medium.clone()),
    session: signup_response.session,
  })
}

pub async fn confirm_sign_up(
  username: String,
  confirmation_code: String,
  session: Option<String>,
) -> Result<String, String> {
  // Load environment variables
  if let Err(e) = dotenv::dotenv() {
    log::warn!("Warning: Could not load .env file: {}", e);
  }

  let client_id = std::env::var("COGNITO_CLIENT_ID")
    .map_err(|_| "Missing COGNITO_CLIENT_ID environment variable".to_string())?;

  let region = std::env::var("COGNITO_REGION")
    .map_err(|_| "Missing COGNITO_REGION environment variable".to_string())?;

  let endpoint = format!("https://cognito-idp.{}.amazonaws.com/", region);

  let request_body = CognitoConfirmSignUpRequest {
    client_id,
    username,
    confirmation_code,
    session,
  };

  // Make the request to AWS Cognito
  let client = reqwest::Client::new();
  let response = client
    .post(&endpoint)
    .header(
      "X-Amz-Target",
      "AWSCognitoIdentityProviderService.ConfirmSignUp",
    )
    .header("Content-Type", "application/x-amz-json-1.1")
    .json(&request_body)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

  if !response.status().is_success() {
    let error_text = response
      .text()
      .await
      .unwrap_or_else(|_| "Unknown error".to_string());
    return Err(format!("Confirmation failed: {}", error_text));
  }

  Ok("User confirmed successfully".to_string())
}

pub async fn resend_confirmation_code(username: String) -> Result<SignUpResult, String> {
  // Load environment variables
  if let Err(e) = dotenv::dotenv() {
    log::warn!("Warning: Could not load .env file: {}", e);
  }

  let client_id = std::env::var("COGNITO_CLIENT_ID")
    .map_err(|_| "Missing COGNITO_CLIENT_ID environment variable".to_string())?;

  let region = std::env::var("COGNITO_REGION")
    .map_err(|_| "Missing COGNITO_REGION environment variable".to_string())?;

  let endpoint = format!("https://cognito-idp.{}.amazonaws.com/", region);

  let mut request_body = HashMap::new();
  request_body.insert("ClientId", client_id);
  request_body.insert("Username", username);

  // Make the request to AWS Cognito
  let client = reqwest::Client::new();
  let response = client
    .post(&endpoint)
    .header(
      "X-Amz-Target",
      "AWSCognitoIdentityProviderService.ResendConfirmationCode",
    )
    .header("Content-Type", "application/x-amz-json-1.1")
    .json(&request_body)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

  if !response.status().is_success() {
    let error_text = response
      .text()
      .await
      .unwrap_or_else(|_| "Unknown error".to_string());
    return Err(format!("Resend confirmation code failed: {}", error_text));
  }

  let resend_response: CognitoSignUpResponse = response
    .json()
    .await
    .map_err(|e| format!("Failed to parse response: {}", e))?;

  Ok(SignUpResult {
    user_sub: "".to_string(), // Not returned in resend response
    user_confirmed: false,
    verification_required: true,
    destination: resend_response
      .code_delivery_details
      .as_ref()
      .map(|cd| cd.destination.clone()),
    delivery_medium: resend_response
      .code_delivery_details
      .as_ref()
      .map(|cd| cd.delivery_medium.clone()),
    session: resend_response.session,
  })
}

pub async fn sign_in(username: String, password: String) -> Result<SignInResult, String> {
  // Load environment variables
  if let Err(e) = dotenv::dotenv() {
    log::warn!("Warning: Could not load .env file: {}", e);
  }

  let client_id = std::env::var("COGNITO_CLIENT_ID")
    .map_err(|_| "Missing COGNITO_CLIENT_ID environment variable".to_string())?;

  let region = std::env::var("COGNITO_REGION")
    .map_err(|_| "Missing COGNITO_REGION environment variable".to_string())?;

  let endpoint = format!("https://cognito-idp.{}.amazonaws.com/", region);

  // Prepare auth parameters for USER_PASSWORD_AUTH flow
  let mut auth_parameters = HashMap::new();
  auth_parameters.insert("USERNAME".to_string(), username.clone());
  auth_parameters.insert("PASSWORD".to_string(), password);

  let request_body = CognitoInitiateAuthRequest {
    auth_flow: "USER_PASSWORD_AUTH".to_string(),
    client_id,
    auth_parameters,
  };

  // Make the request to AWS Cognito
  let client = reqwest::Client::new();
  let response = client
    .post(&endpoint)
    .header(
      "X-Amz-Target",
      "AWSCognitoIdentityProviderService.InitiateAuth",
    )
    .header("Content-Type", "application/x-amz-json-1.1")
    .json(&request_body)
    .send()
    .await
    .map_err(|e| format!("Failed to send request: {}", e))?;

  if !response.status().is_success() {
    let error_text = response
      .text()
      .await
      .unwrap_or_else(|_| "Unknown error".to_string());
    log::warn!("SignIn failed: {}", error_text);
    return Err(error_text);
  }

  let auth_response: CognitoInitiateAuthResponse = response
    .json()
    .await
    .map_err(|e| format!("Failed to parse response: {}", e))?;

  // Check if we got an authentication result (successful login)
  if let Some(auth_result) = auth_response.authentication_result {
    // Decode the ID token to extract user information
    let claims = decode_jwt_claims(&auth_result.id_token)?;

    let user_info = CognitoUserInfo {
      username: claims.username.unwrap_or_else(|| username.clone()),
      email: claims.email,
      given_name: claims.given_name,
      family_name: claims.family_name,
      sub: claims.sub,
    };

    let sign_in_result = SignInResult {
      access_token: auth_result.access_token,
      id_token: auth_result.id_token,
      refresh_token: auth_result.refresh_token,
      expires_in: auth_result.expires_in,
      user_info,
    };

    // Store the authentication result securely
    match store_cognito_auth(&sign_in_result) {
      Ok(()) => {
        log::info!("Authentication stored successfully");
        Ok(sign_in_result)
      }
      Err(e) => {
        log::warn!("Warning: Failed to store authentication: {}", e);
        // Return success anyway since the authentication itself succeeded
        Ok(sign_in_result)
      }
    }
  } else if let Some(challenge_name) = auth_response.challenge_name {
    // Handle authentication challenges (MFA, new password required, etc.)
    match challenge_name.as_str() {
      "NEW_PASSWORD_REQUIRED" => {
        Err("New password required. Please contact administrator.".to_string())
      }
      "MFA_REQUIRED" => Err("MFA required. Please contact administrator.".to_string()),
      _ => Err(format!(
        "Unhandled authentication challenge: {}",
        challenge_name
      )),
    }
  } else {
    Err("Unexpected response from Cognito".to_string())
  }
}
