use serde::{Deserialize, Serialize};
use std::collections::HashMap;

//TODO: make these automatically updated using ts-rs

// AWS Cognito SignUp API structures
#[derive(Debug, Serialize)]
pub struct CognitoSignUpRequest {
  #[serde(rename = "ClientId")]
  pub client_id: String,
  #[serde(rename = "Username")]
  pub username: String,
  #[serde(rename = "Password")]
  pub password: String,
  #[serde(rename = "UserAttributes")]
  pub user_attributes: Vec<CognitoAttribute>,
}

#[derive(Debug, Serialize)]
pub struct CognitoAttribute {
  #[serde(rename = "Name")]
  pub name: String,
  #[serde(rename = "Value")]
  pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct CognitoSignUpResponse {
  #[serde(rename = "UserSub")]
  pub user_sub: String,
  #[serde(rename = "UserConfirmed")]
  pub user_confirmed: bool,
  #[serde(rename = "CodeDeliveryDetails")]
  pub code_delivery_details: Option<CognitoCodeDeliveryDetails>,
  #[serde(rename = "Session")]
  pub session: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CognitoCodeDeliveryDetails {
  #[serde(rename = "Destination")]
  pub destination: String,
  #[serde(rename = "DeliveryMedium")]
  pub delivery_medium: String,
  #[serde(rename = "AttributeName")]
  pub attribute_name: String,
}

#[derive(Debug, Serialize)]
pub struct CognitoConfirmSignUpRequest {
  #[serde(rename = "ClientId")]
  pub client_id: String,
  #[serde(rename = "Username")]
  pub username: String,
  #[serde(rename = "ConfirmationCode")]
  pub confirmation_code: String,
  #[serde(rename = "Session")]
  pub session: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CognitoConfirmSignUpResponse {
  #[serde(rename = "Session")]
  pub session: Option<String>,
}

// AWS Cognito InitiateAuth API structures
#[derive(Debug, Serialize)]
pub struct CognitoInitiateAuthRequest {
  #[serde(rename = "AuthFlow")]
  pub auth_flow: String,
  #[serde(rename = "ClientId")]
  pub client_id: String,
  #[serde(rename = "AuthParameters")]
  pub auth_parameters: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct CognitoInitiateAuthResponse {
  #[serde(rename = "AuthenticationResult")]
  pub authentication_result: Option<CognitoAuthenticationResult>,
  #[serde(rename = "ChallengeName")]
  pub challenge_name: Option<String>,
  #[serde(rename = "Session")]
  pub session: Option<String>,
  #[serde(rename = "ChallengeParameters")]
  pub challenge_parameters: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
pub struct CognitoAuthenticationResult {
  #[serde(rename = "AccessToken")]
  pub access_token: String,
  #[serde(rename = "IdToken")]
  pub id_token: String,
  #[serde(rename = "RefreshToken")]
  pub refresh_token: String,
  #[serde(rename = "ExpiresIn")]
  pub expires_in: i64,
  #[serde(rename = "TokenType")]
  pub token_type: String,
}
