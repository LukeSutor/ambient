//! Llama.cpp Server Integration Module
//!
//! This module provides integration with llama.cpp server as a Tauri sidecar process.
//!
//! ## Features
//! - Spawn/stop llama.cpp server with automatic port discovery
//! - Health checking with automatic retry logic
//! - Server status monitoring with port tracking
//! - Completion API integration
//! - Proper error handling and process management
//! - Random port selection to avoid conflicts
//!
//! ## Port Management
//! The server automatically finds an available port in the range 8000-9999 by:
//! 1. Generating random port numbers in the safe range
//! 2. Testing port availability before use
//! 3. Storing the successful port for subsequent requests
//! 4. Providing commands to query the current port
//!
//! ## Usage
//!
//! ### Starting the server:
//! ```typescript
//! import { invoke } from '@tauri-apps/api/core';
//!
//! try {
//!   const result = await invoke('spawn_llama_server');
//!   console.log(result); // "Server started on port 8347"
//! } catch (error) {
//!   console.error('Failed to start server:', error);
//! }
//! ```
//!
//! ### Getting the current port:
//! ```typescript
//! const port = await invoke('get_server_port');
//! console.log(`Server is running on port: ${port}`);
//! ```
//!
//! ### Checking server health:
//! ```typescript
//! const health = await invoke('check_server_health');
//! console.log(health); // { status: "healthy", response: { status: "ok" } }
//! ```
//!
//! ### Making completion requests:
//! ```typescript
//! const completion = await invoke('make_completion_request', {
//!   prompt: "Hello, how are you?",
//!   maxTokens: 100,
//!   temperature: 0.7
//! });
//! ```
//!
//! ## Configuration
//! - Model path: Retrieved from setup module using LLM_DIR/LLM_FILE constants
//! - API key: Read from LLAMA_API_KEY environment variable
//! - Port range: 8000-9999 (randomly selected)
//! - Additional args: --no-webui, --log-disable

use crate::setup;
use reqwest;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::{process::CommandChild, ShellExt};
use tokio::time::{sleep, Duration};
use rand::Rng;

/// Global state to track the running server process and port
#[derive(Debug)]
struct ServerState {
    child: Option<CommandChild>,
    port: Option<u16>,
}

static SERVER_STATE: Mutex<ServerState> = Mutex::new(ServerState {
    child: None,
    port: None,
});

/// Server configuration
const MIN_PORT: u16 = 8000;
const MAX_PORT: u16 = 9999;
const MAX_PORT_ATTEMPTS: u8 = 20;
const HEALTH_CHECK_ENDPOINT: &str = "/health";
const MAX_HEALTH_CHECK_RETRIES: u8 = 30;
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(120);

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
    pub model_path: String,
}

impl ServerConfig {
    pub fn new(app_handle: &AppHandle, port: u16) -> Result<Self, ServerError> {
        // Get API key from environment
        let api_key = env::var("LLAMA_API_KEY").map_err(|_| {
            ServerError::ConfigError("LLAMA_API_KEY not found in environment".to_string())
        })?;

        // Get model path
        let model_path = setup::get_llm_model_path(app_handle.clone())
            .map_err(|e| ServerError::ModelNotFound(e))?;

        // Check if model file exists
        if !model_path.exists() {
            return Err(ServerError::ModelNotFound(format!(
                "Model file does not exist: {:?}",
                model_path
            )));
        }

        let model_path_str = model_path
            .to_str()
            .ok_or_else(|| {
                ServerError::ConfigError(format!(
                    "Model path is not valid UTF-8: {:?}",
                    model_path
                ))
            })?
            .to_string();

        Ok(ServerConfig {
            port,
            api_key,
            model_path: model_path_str,
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
    use std::net::{TcpListener, SocketAddr};
    
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpListener::bind(addr).is_ok()
}

/// Find an available port by trying random ports
async fn find_available_port() -> Result<u16, ServerError> {
    for attempt in 1..=MAX_PORT_ATTEMPTS {
        let port = generate_random_port();
        println!("[llama_server] Trying port {} (attempt {}/{})", port, attempt, MAX_PORT_ATTEMPTS);
        
        if is_port_available(port).await {
            println!("[llama_server] Found available port: {}", port);
            return Ok(port);
        }
    }
    
    Err(ServerError::ProcessError(format!(
        "Could not find an available port after {} attempts",
        MAX_PORT_ATTEMPTS
    )))
}

/// Get the currently used port (if server is running)
fn get_current_port() -> Option<u16> {
    let server_state = SERVER_STATE.lock().unwrap();
    server_state.port
}

/// Spawn the llama.cpp server as a sidecar process
#[tauri::command]
pub async fn spawn_llama_server(app_handle: AppHandle) -> Result<String, String> {
    println!("[llama_server] Starting llama.cpp server...");

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

    println!("[llama_server] Using port {} for server", config.port);

    // Prepare sidecar command
    let shell = app_handle.shell();
    let sidecar_command = shell
        .sidecar("server")
        .map_err(|e| format!("Failed to get sidecar command: {}", e))?
        .args([
            "-m",
            &config.model_path,
            "--port",
            &config.port.to_string(),
            "--api-key",
            &config.api_key,
            "--no-webui",
            "--log-disable",
        ]);

    // Spawn the server process
    let (mut _rx, child) = sidecar_command
        .spawn()
        .map_err(|e| format!("Failed to spawn server process: {}", e))?;

    // Store the child process and port in global state
    {
        let mut server_state = SERVER_STATE.lock().unwrap();
        server_state.child = Some(child);
        server_state.port = Some(config.port);
    }

    // Wait for server to be ready
    if let Err(e) = wait_for_server_ready(&config).await {
        // If server failed to start, clean up the process
        let _ = stop_llama_server().await;
        return Err(format!("Server failed to start: {}", e));
    }

    println!("[llama_server] Server started successfully on port {}", config.port);
    Ok(format!("Server started on port {}", config.port))
}

#[tauri::command]
pub async fn stop_llama_server() -> Result<String, String> {
    println!("[llama_server] Stopping llama.cpp server...");

    let mut server_state = SERVER_STATE.lock().unwrap();
    
    match server_state.child.take() {
        Some(mut child) => {
            child
                .kill()
                .map_err(|e| format!("Failed to kill server process: {}", e))?;
            
            // Clear the port as well
            server_state.port = None;
            
            println!("[llama_server] Server stopped successfully");
            Ok("Server stopped successfully".to_string())
        }
        None => Err(ServerError::ServerNotRunning.into()),
    }
}

#[tauri::command]
pub async fn check_server_health(app_handle: AppHandle) -> Result<Value, String> {
    // Get the current port from server state
    let port = get_current_port().ok_or_else(|| "Server is not running".to_string())?;
    
    let config = ServerConfig::new(&app_handle, port).map_err(|e| e.to_string())?;
    
    match perform_health_check(&config).await {
        Ok(response) => Ok(response),
        Err(e) => Err(format!("Health check failed: {}", e)),
    }
}

#[tauri::command]
pub async fn get_server_status(app_handle: AppHandle) -> Result<Value, String> {
    // Check if process is running and get port
    let (process_running, port) = {
        let server_state = SERVER_STATE.lock().unwrap();
        (server_state.child.is_some(), server_state.port)
    };

    if !process_running {
        return Ok(json!({
            "status": "stopped",
            "process_running": false,
            "health_check": null,
            "port": null,
            "base_url": null
        }));
    }

    let port = port.ok_or_else(|| "Server is running but port is unknown".to_string())?;
    let config = ServerConfig::new(&app_handle, port).map_err(|e| e.to_string())?;

    // Perform health check
    let health_result = perform_health_check(&config).await;
    
    match health_result {
        Ok(health_response) => Ok(json!({
            "status": "running",
            "process_running": true,
            "health_check": health_response,
            "port": config.port,
            "base_url": config.base_url()
        })),
        Err(_) => Ok(json!({
            "status": "unhealthy",
            "process_running": true,
            "health_check": null,
            "port": config.port,
            "base_url": config.base_url()
        })),
    }
}

/// Internal function to perform health check
async fn perform_health_check(config: &ServerConfig) -> Result<Value, ServerError> {
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
        println!("[llama_server] Health check attempt {}/{}", attempt, MAX_HEALTH_CHECK_RETRIES);
        
        match perform_health_check(config).await {
            Ok(response) => {
                if let Some(status) = response.get("status") {
                    if status == "healthy" {
                        println!("[llama_server] Server is healthy and ready");
                        return Ok(());
                    } else if status == "loading" {
                        println!("[llama_server] Server is loading model, waiting...");
                    }
                }
            }
            Err(e) => {
                println!("[llama_server] Health check failed: {}", e);
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

#[tauri::command]
pub async fn make_completion_request(
    app_handle: AppHandle,
    prompt: String,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
) -> Result<Value, String> {
    // Get the current port from server state
    let port = get_current_port().ok_or_else(|| "Server is not running".to_string())?;
    
    let config = ServerConfig::new(&app_handle, port).map_err(|e| e.to_string())?;
    
    // Check if server is healthy first
    match perform_health_check(&config).await {
        Ok(health) => {
            if health.get("status") != Some(&json!("healthy")) {
                return Err("Server is not healthy".to_string());
            }
        }
        Err(e) => return Err(format!("Server health check failed: {}", e)),
    }

    let client = reqwest::Client::new();
    let completion_url = format!("{}/completion", config.base_url());

    let mut request_body = json!({
        "prompt": prompt,
        "stream": false
    });

    // Add optional parameters
    if let Some(tokens) = max_tokens {
        request_body["n_predict"] = json!(tokens);
    }
    if let Some(temp) = temperature {
        request_body["temperature"] = json!(temp);
    }
    if let Some(p) = top_p {
        request_body["top_p"] = json!(p);
    }

    let response = client
        .post(&completion_url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send completion request: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Completion request failed with status: {}", response.status()));
    }

    let result: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse completion response: {}", e))?;

    Ok(result)
}

/// Get the current server port (if running)
#[tauri::command]
pub async fn get_server_port() -> Result<Option<u16>, String> {
    Ok(get_current_port())
}

/// Restart the llama.cpp server
#[tauri::command]
pub async fn restart_llama_server(app_handle: AppHandle) -> Result<String, String> {
    println!("[llama_server] Restarting llama.cpp server...");
    
    // Stop the server if it's running
    let _ = stop_llama_server().await;
    
    // Wait a moment for cleanup
    sleep(Duration::from_secs(1)).await;
    
    // Start the server again
    spawn_llama_server(app_handle).await
}
