use crate::constants::{
  HEALTH_CHECK_ENDPOINT, HEALTH_CHECK_INTERVAL, MAX_HEALTH_CHECK_RETRIES, MAX_PORT,
  MAX_PORT_ATTEMPTS, MIN_PORT,
};
use crate::setup;
use rand::Rng;
use reqwest;
use serde_json::{json, Value};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_shell::{process::CommandChild, ShellExt};
use tokio::time::sleep;
use uuid::Uuid;

/// Global state to track the running server process and port
#[derive(Debug)]
struct ServerState {
  child: Option<CommandChild>,
  port: Option<u16>,
  api_key: Option<String>,
}

static SERVER_STATE: Mutex<ServerState> = Mutex::new(ServerState {
  child: None,
  port: None,
  api_key: None,
});

/// Error types for server operations
#[derive(Debug)]
pub enum ServerError {
  ModelNotFound(String),
  ConfigError(String),
  ProcessError(String),
  NetworkError(String),
  ServerAlreadyRunning,
  ServerNotRunning,
}

impl std::fmt::Display for ServerError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ServerError::ModelNotFound(msg) => write!(f, "Model not found: {}", msg),
      ServerError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
      ServerError::ProcessError(msg) => write!(f, "Process error: {}", msg),
      ServerError::NetworkError(msg) => write!(f, "Network error: {}", msg),
      ServerError::ServerAlreadyRunning => write!(f, "Server is already running"),
      ServerError::ServerNotRunning => write!(f, "Server is not running"),
    }
  }
}

impl std::error::Error for ServerError {}

/// Convert ServerError to String for Tauri commands
impl From<ServerError> for String {
  fn from(error: ServerError) -> Self {
    error.to_string()
  }
}

/// Server configuration structure
#[derive(Debug, Clone)]
pub struct ServerConfig {
  pub port: u16,
  pub api_key: String,
  pub text_model_path: String,
  pub mmproj_model_path: String,
}

impl ServerConfig {
  pub fn new(app_handle: &AppHandle, port: u16) -> Result<Self, ServerError> {
    // Try to get existing API key from server state first
    let api_key = {
      let server_state = SERVER_STATE.lock().unwrap();
      server_state.api_key.clone()
    };

    let api_key = api_key.unwrap_or_else(|| {
      let new_key = format!("session-{}", Uuid::new_v4().to_string());
      new_key
    });

    // Get model and mmproj path
    let text_model_path =
      setup::get_vlm_text_model_path(&app_handle).map_err(|e| ServerError::ModelNotFound(e))?;
    let mmproj_model_path =
      setup::get_vlm_mmproj_model_path(&app_handle).map_err(|e| ServerError::ModelNotFound(e))?;

    // Check if model files exist
    if !text_model_path.exists() || !mmproj_model_path.exists() {
      return Err(ServerError::ModelNotFound(format!(
        "Model files do not exist: {:?} or {:?}",
        text_model_path, mmproj_model_path
      )));
    }

    let text_model_path_str = text_model_path
      .to_str()
      .ok_or_else(|| {
        ServerError::ConfigError(format!("Model path is not valid UTF-8: {:?}", text_model_path))
      })?
      .to_string();
    let mmproj_model_path_str = mmproj_model_path
      .to_str()
      .ok_or_else(|| {
        ServerError::ConfigError(format!("MMProj path is not valid UTF-8: {:?}", mmproj_model_path))
      })?
      .to_string();

    Ok(ServerConfig {
      port,
      api_key,
      text_model_path: text_model_path_str,
      mmproj_model_path: mmproj_model_path_str,
    })
  }

  pub fn health_url(&self) -> String {
    format!("http://localhost:{}{}", self.port, HEALTH_CHECK_ENDPOINT)
  }

  pub fn base_url(&self) -> String {
    format!("http://localhost:{}", self.port)
  }
}

/// Generate a random port number within the acceptable range
fn generate_random_port() -> u16 {
  let mut rng = rand::thread_rng();
  rng.gen_range(MIN_PORT..=MAX_PORT)
}

/// Check if a port is available by attempting to bind to it
async fn is_port_available(port: u16) -> bool {
  use std::net::{SocketAddr, TcpListener};

  let addr = SocketAddr::from(([127, 0, 0, 1], port));
  TcpListener::bind(addr).is_ok()
}

/// Find an available port by trying random ports
async fn find_available_port() -> Result<u16, ServerError> {
  for attempt in 1..=MAX_PORT_ATTEMPTS {
    let port = generate_random_port();
    log::debug!(
      "[llama_server] Trying port (attempt {}/{})",
      attempt,
      MAX_PORT_ATTEMPTS
    );

    if is_port_available(port).await {
      return Ok(port);
    }
  }

  Err(ServerError::ProcessError(format!(
    "Could not find an available port after {} attempts",
    MAX_PORT_ATTEMPTS
  )))
}

/// Get server config using stored port and API key
pub fn get_current_server_config(app_handle: &AppHandle) -> Result<ServerConfig, ServerError> {
  let (port, api_key) = {
    let server_state = SERVER_STATE.lock().unwrap();
    (server_state.port, server_state.api_key.clone())
  };

  let port = port.ok_or_else(|| ServerError::ServerNotRunning)?;
  let api_key = api_key.ok_or_else(|| ServerError::ServerNotRunning)?;

  // Get model path
  let text_model_path =
    setup::get_vlm_text_model_path(&app_handle).map_err(|e| ServerError::ModelNotFound(e))?;
  let mmproj_model_path =
    setup::get_vlm_mmproj_model_path(&app_handle).map_err(|e| ServerError::ModelNotFound(e))?;

  let text_model_path_str = text_model_path
    .to_str()
    .ok_or_else(|| {
      ServerError::ConfigError(format!("Model path is not valid UTF-8: {:?}", text_model_path))
    })?
    .to_string();
  let mmproj_model_path_str = mmproj_model_path
    .to_str()
    .ok_or_else(|| {
      ServerError::ConfigError(format!("MMProj path is not valid UTF-8: {:?}", mmproj_model_path))
    })?
    .to_string();

  Ok(ServerConfig {
    port,
    api_key,
    text_model_path: text_model_path_str,
    mmproj_model_path: mmproj_model_path_str,
  })
}

/// Spawn the llama.cpp server as a sidecar process
#[tauri::command]
pub async fn spawn_llama_server(app_handle: AppHandle) -> Result<String, String> {
  log::info!("[llama_server] Starting llama.cpp server...");

  // Check if server is already running
  {
    let server_state = SERVER_STATE.lock().unwrap();
    if server_state.child.is_some() {
      return Err(ServerError::ServerAlreadyRunning.into());
    }
  }

  // Find an available port
  let port = find_available_port().await.map_err(|e| e.to_string())?;

  // Create server configuration with the found port
  let config = ServerConfig::new(&app_handle, port).map_err(|e| e.to_string())?;

  // Prepare sidecar command
  let shell = app_handle.shell();
  let sidecar_command = shell
    .sidecar("server")
    .map_err(|e| format!("Failed to get sidecar command: {}", e))?
    .args([
      "-m",
      &config.text_model_path,
      "-mm",
      &config.mmproj_model_path,
      "--port",
      &config.port.to_string(),
      "--api-key",
      &config.api_key,
      "--reasoning-format",
      "none",
      "-np", // Decode up to 3 sequences in parallel
      "3",
      "--ctx-size", // Use smaller context size for faster responses
      "4096",
      "-ctk", // Use q8 quant for kv cache
      "q8_0",
      "-ctv",
      "q8_0",
      "--mlock", // Keep model in RAM
      "-fa",     // Use fast attention
      "on",
      "--no-webui",
      "--log-disable",
      "--offline",
      "--jinja",
    ]);

  // Spawn the server process
  let (mut _rx, child) = sidecar_command
    .spawn()
    .map_err(|e| format!("Failed to spawn server process: {}", e))?;

  // Store the child process, port, and API key in global state
  {
    let mut server_state = SERVER_STATE.lock().unwrap();
    server_state.child = Some(child);
    server_state.port = Some(config.port);
    server_state.api_key = Some(config.api_key.clone());
  }

  // Wait for server to be ready
  if let Err(e) = wait_for_server_ready(&config).await {
    // If server failed to start, clean up the process
    let _ = stop_llama_server().await;
    return Err(format!("Server failed to start: {}", e));
  }

  log::debug!("[llama_server] Server started successfully");
  Ok(format!("Server started on port {}", config.port))
}

pub async fn stop_llama_server() -> Result<String, String> {
  log::info!("[llama_server] Stopping llama.cpp server...");

  let mut server_state = SERVER_STATE.lock().unwrap();

  match server_state.child.take() {
    Some(child) => {
      child
        .kill()
        .map_err(|e| format!("Failed to kill server process: {}", e))?;

      // Clear the port and API key as well
      server_state.port = None;
      server_state.api_key = None;

      log::info!("[llama_server] Server stopped successfully");
      Ok("Server stopped successfully".to_string())
    }
    None => Err(ServerError::ServerNotRunning.into()),
  }
}

/// Internal function to perform health check
pub async fn perform_health_check(config: &ServerConfig) -> Result<Value, ServerError> {
  let client = reqwest::Client::new();

  let response = client
    .get(&config.health_url())
    .send()
    .await
    .map_err(|e| ServerError::NetworkError(format!("Failed to connect to server: {}", e)))?;

  let status = response.status();
  let body: Value = response
    .json()
    .await
    .map_err(|e| ServerError::NetworkError(format!("Failed to parse response: {}", e)))?;

  match status.as_u16() {
    200 => Ok(json!({
        "status": "healthy",
        "response": body
    })),
    503 => Ok(json!({
        "status": "loading",
        "response": body
    })),
    _ => Err(ServerError::NetworkError(format!(
      "Unexpected status code: {}",
      status
    ))),
  }
}

/// Wait for server to be ready (health check returns 200)
async fn wait_for_server_ready(config: &ServerConfig) -> Result<(), ServerError> {
  for attempt in 1..=MAX_HEALTH_CHECK_RETRIES {
    log::debug!(
      "[llama_server] Health check attempt {}/{}",
      attempt,
      MAX_HEALTH_CHECK_RETRIES
    );

    match perform_health_check(config).await {
      Ok(response) => {
        if let Some(status) = response.get("status") {
          if status == "healthy" {
            log::info!("[llama_server] Server is healthy and ready");
            return Ok(());
          } else if status == "loading" {
            log::info!("[llama_server] Server is loading model, waiting...");
          }
        }
      }
      Err(e) => {
        log::warn!("[llama_server] Health check failed: {}", e);
      }
    }

    if attempt < MAX_HEALTH_CHECK_RETRIES {
      sleep(HEALTH_CHECK_INTERVAL).await;
    }
  }

  Err(ServerError::ProcessError(
    "Server failed to become healthy within timeout".to_string(),
  ))
}
