use crate::constants::{
  HEALTH_CHECK_ENDPOINT, HEALTH_CHECK_INTERVAL, MAX_HEALTH_CHECK_RETRIES, MAX_PORT,
  MAX_PORT_ATTEMPTS, MIN_PORT,
};
use crate::setup;
use rand::Rng;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Mutex;
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandChild;
use tokio::time::sleep;
use ts_rs::TS;
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

  // Load user settings to check GPU acceleration preference
  let settings = crate::settings::service::load_user_settings(app_handle.clone())
    .await
    .map_err(|e| format!("Failed to load user settings: {}", e))?;

  // Build base args
  let mut args: Vec<String> = vec![
    "-m".into(),
    config.text_model_path.clone(),
    "-mm".into(),
    config.mmproj_model_path.clone(),
    "--port".into(),
    config.port.to_string(),
    "--api-key".into(),
    config.api_key.clone(),
    "--reasoning-format".into(),
    "none".into(),
    "-np".into(), // Decode up to 3 sequences in parallel
    "3".into(),
    "--ctx-size".into(),
    "32768".into(),
    "--n-predict".into(),
    "32768".into(),
    "--temp".into(),
    "0.7".into(),
    "--top-p".into(),
    "0.8".into(),
    "--top-k".into(),
    "20".into(),
    "--repeat-penalty".into(),
    "1.0".into(),
    "--presence-penalty".into(),
    "1.5".into(),
    "--seed".into(),
    "3407".into(),
    "-ctk".into(), // Use q8 quant for kv cache
    "q8_0".into(),
    "-ctv".into(),
    "q8_0".into(),
    "--mlock".into(), // Keep model in RAM
    "-fa".into(),     // Use fast attention
    "on".into(),
    "--no-webui".into(),
    "--log-disable".into(),
    "--offline".into(),
    "--jinja".into(),
  ];

  // Offload all layers to GPU when GPU acceleration is enabled
  if settings.gpu_acceleration {
    log::info!("[llama_server] GPU acceleration enabled, offloading all layers to GPU");
    args.extend(["-ngl".into(), "99".into()]);
  } else {
    log::info!("[llama_server] GPU acceleration disabled, using CPU only");
    args.extend(["-ngl".into(), "0".into()]);
  }

  // Prepare sidecar command
  let shell = app_handle.shell();
  let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
  let sidecar_command = shell
    .sidecar("server")
    .map_err(|e| format!("Failed to get sidecar command: {}", e))?
    .args(&arg_refs);

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

/// Restart the llama.cpp server (stop then start).
/// Used when settings change that require a server restart (e.g. GPU acceleration).
#[tauri::command]
pub async fn restart_llama_server(app_handle: AppHandle) -> Result<String, String> {
  log::info!("[llama_server] Restarting llama.cpp server...");

  // Stop if running (ignore "not running" error)
  let _ = stop_llama_server().await;

  // Brief pause to ensure port is released
  sleep(std::time::Duration::from_millis(500)).await;

  // Start with current settings
  spawn_llama_server(app_handle).await
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

/// A GPU device detected by the llama.cpp server.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "llm.ts")]
pub struct GpuDevice {
  /// Device name as reported by llama.cpp (e.g. "Vulkan0: NVIDIA GeForce RTX 3080")
  pub name: String,
}

/// Detect available GPU devices by running the llama.cpp sidecar with `--list-devices`.
///
/// Parses the output for Vulkan device entries. Returns an empty vec if no
/// compatible GPU is found or if detection fails (graceful fallback to CPU).
#[tauri::command]
pub async fn detect_gpu_devices(app_handle: AppHandle) -> Result<Vec<GpuDevice>, String> {
  let shell = app_handle.shell();
  let output = shell
    .sidecar("server")
    .map_err(|e| format!("Failed to get sidecar command: {}", e))?
    .args(["--list-devices"])
    .output()
    .await
    .map_err(|e| format!("Failed to run GPU detection: {}", e))?;

  let stdout = String::from_utf8_lossy(&output.stdout);

  // Parse lines after "Available devices:" looking for device entries
  // Raw format: "  Vulkan0: NVIDIA GeForce RTX 3060 Laptop GPU (6010 MiB, 5242 MiB free)"
  // Cleaned:    "NVIDIA GeForce RTX 3060 Laptop GPU"
  let mut devices = Vec::new();
  let mut in_devices_section = false;

  for line in stdout.lines() {
    let trimmed = line.trim();
    if trimmed.starts_with("Available devices:") {
      in_devices_section = true;
      continue;
    }
    if in_devices_section && !trimmed.is_empty() {
      let clean_name = parse_device_name(trimmed);
      devices.push(GpuDevice { name: clean_name });
    }
  }

  log::info!("[llama_server] Detected {} GPU device(s): {:?}", devices.len(), devices.iter().map(|d| &d.name).collect::<Vec<_>>());
  Ok(devices)
}

/// Extract a clean GPU name from the raw llama.cpp device line.
///
/// Input:  "Vulkan0: NVIDIA GeForce RTX 3060 Laptop GPU (6010 MiB, 5242 MiB free)"
/// Output: "NVIDIA GeForce RTX 3060 Laptop GPU"
fn parse_device_name(raw: &str) -> String {
  // Strip "Vulkan0: " prefix (everything up to and including the first ": ")
  let after_prefix = raw
    .find(": ")
    .map(|i| &raw[i + 2..])
    .unwrap_or(raw);

  // Strip trailing " (XXXX MiB, ...)" VRAM info
  after_prefix
    .rfind(" (")
    .map(|i| &after_prefix[..i])
    .unwrap_or(after_prefix)
    .to_string()
}
