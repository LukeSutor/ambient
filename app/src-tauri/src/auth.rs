

use std::net::{SocketAddr, TcpListener};
use std::sync::Arc;
use axum::{
    extract::{Extension, Query},
    response::IntoResponse,
    routing::get,
    Router,
};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use oauth2::reqwest::async_http_client;
use serde::Deserialize;
use tauri::Manager;
use tokio::sync::{oneshot, Mutex};
use keyring::Entry;

#[derive(Clone)]
pub struct AuthState {
    csrf_token: CsrfToken,
    pkce: Arc<(PkceCodeChallenge, String)>,
    client: Arc<BasicClient>,
    socket_addr: SocketAddr,
    auth_result: Arc<Mutex<Option<oneshot::Sender<Result<AuthToken, String>>>>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub id_token: Option<String>,
    pub expires_in: Option<std::time::Duration>,
}

#[derive(Deserialize)]
struct CallbackQuery {
    code: AuthorizationCode,
    state: CsrfToken,
}

const KEYRING_SERVICE: &str = "local-computer-use";
const KEYRING_USER: &str = "oauth_tokens";

#[tauri::command]
pub async fn authenticate(handle: tauri::AppHandle) -> Result<String, String> {
    let auth = handle.state::<AuthState>();
    
    // Create auth URL with PKCE
    let (auth_url, _) = auth
        .client
        .authorize_url(|| auth.csrf_token.clone())
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .set_pkce_challenge(auth.pkce.0.clone())
        .url();

    // Create a oneshot channel to receive the token
    let (tx, rx) = oneshot::channel::<Result<AuthToken, String>>();
    
    // Store the sender in the auth state
    {
        let mut auth_result = auth.auth_result.lock().await;
        *auth_result = Some(tx);
    }
    
    // Start the callback server
    let server_handle_clone = handle.clone();
    let server_task = tauri::async_runtime::spawn(async move {
        run_server(server_handle_clone).await
    });

    // Open the browser for authentication
    open::that(auth_url.to_string()).map_err(|e| format!("Failed to open browser: {}", e))?;

    // Wait for the callback
    match rx.await {
        Ok(Ok(token)) => {
            // Store token securely
            store_token(&token).map_err(|e| format!("Failed to store token: {}", e))?;
            server_task.abort(); // Stop the server
            Ok("Authentication successful".to_string())
        }
        Ok(Err(e)) => {
            server_task.abort();
            Err(e)
        }
        Err(_) => {
            server_task.abort();
            Err("Authentication cancelled".to_string())
        }
    }
}

#[tauri::command]
pub async fn logout() -> Result<String, String> {
    clear_stored_token().map_err(|e| format!("Failed to clear token: {}", e))?;
    Ok("Logged out successfully".to_string())
}

#[tauri::command]
pub async fn get_stored_token() -> Result<Option<AuthToken>, String> {
    retrieve_token().map_err(|e| format!("Failed to retrieve token: {}", e))
}

#[tauri::command]
pub async fn is_authenticated() -> Result<bool, String> {
    match retrieve_token() {
        Ok(Some(_token)) => {
            // TODO: Check if token is still valid (not expired)
            Ok(true)
        }
        Ok(None) => Ok(false),
        Err(_) => Ok(false),
    }
}

async fn authorize(
    handle: Extension<tauri::AppHandle>,
    query: Query<CallbackQuery>,
) -> impl IntoResponse {
    let auth = handle.state::<AuthState>();

    // Verify CSRF token
    if query.state.secret() != auth.csrf_token.secret() {
        println!("CSRF token mismatch - possible MITM attack!");
        let _ = send_auth_result(&handle, Err("CSRF token mismatch".to_string())).await;
        return "Authentication failed - security error".to_string();
    }

    // Exchange authorization code for token
    match auth
        .client
        .exchange_code(query.code.clone())
        .set_pkce_verifier(PkceCodeVerifier::new(auth.pkce.1.clone()))
        .request_async(async_http_client)
        .await
    {
        Ok(token_response) => {
            let auth_token = AuthToken {
                access_token: token_response.access_token().secret().clone(),
                refresh_token: token_response.refresh_token().map(|t| t.secret().clone()),
                id_token: None, // Cognito returns ID token in a different way, we'll handle this separately
                expires_in: token_response.expires_in(),
            };

            let _ = send_auth_result(&handle, Ok(auth_token)).await;
            "Authentication successful! You can close this window.".to_string()
        }
        Err(e) => {
            println!("Token exchange failed: {:?}", e);
            let _ = send_auth_result(&handle, Err(format!("Token exchange failed: {}", e))).await;
            "Authentication failed - token exchange error".to_string()
        }
    }
}

async fn run_server(handle: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = Router::new()
        .route("/callback", get(authorize))
        .layer(Extension(handle.clone()));

    let socket_addr = handle.state::<AuthState>().socket_addr;
    
    axum::Server::bind(&socket_addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}

async fn send_auth_result(handle: &tauri::AppHandle, result: Result<AuthToken, String>) -> Result<(), String> {
    let auth = handle.state::<AuthState>();
    let mut auth_result = auth.auth_result.lock().await;
    
    if let Some(tx) = auth_result.take() {
        let _ = tx.send(result);
    }
    Ok(())
}

fn get_available_addr() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    addr
}

fn create_client(redirect_url: RedirectUrl) -> BasicClient {
    // AWS Cognito OAuth2 endpoints
    let client_id = ClientId::new(
        std::env::var("COGNITO_CLIENT_ID")
            .expect("Missing COGNITO_CLIENT_ID environment variable")
    );

    // Use the new environment variables for URLs
    let auth_url = AuthUrl::new(
        std::env::var("COGNITO_USER_POOL_DOMAIN")
            .expect("Missing COGNITO_USER_POOL_DOMAIN environment variable")
    ).expect("Invalid authorization URL");

    let token_url = TokenUrl::new(
        std::env::var("COGNITO_TOKEN_URL")
            .expect("Missing COGNITO_TOKEN_URL environment variable")
    ).expect("Invalid token URL");

    BasicClient::new(client_id, None, auth_url, Some(token_url))
        .set_redirect_uri(redirect_url)
}

pub fn create_auth_state() -> AuthState {
    let (pkce_code_challenge, pkce_code_verifier) = PkceCodeChallenge::new_random_sha256();
    let socket_addr = get_available_addr();
    let redirect_url = format!("http://{}/callback", socket_addr);

    AuthState {
        csrf_token: CsrfToken::new_random(),
        pkce: Arc::new((pkce_code_challenge, pkce_code_verifier.secret().to_string())),
        client: Arc::new(create_client(RedirectUrl::new(redirect_url).unwrap())),
        socket_addr,
        auth_result: Arc::new(Mutex::new(None)),
    }
}

fn store_token(token: &AuthToken) -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    let token_json = serde_json::to_string(token)?;
    entry.set_password(&token_json)?;
    Ok(())
}

fn retrieve_token() -> Result<Option<AuthToken>, Box<dyn std::error::Error>> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    match entry.get_password() {
        Ok(token_json) => {
            let token: AuthToken = serde_json::from_str(&token_json)?;
            Ok(Some(token))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}

fn clear_stored_token() -> Result<(), Box<dyn std::error::Error>> {
    let entry = Entry::new(KEYRING_SERVICE, KEYRING_USER)?;
    entry.delete_password()?;
    Ok(())
}