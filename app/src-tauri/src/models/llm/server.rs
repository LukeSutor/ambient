use crate::setup;
use rand::Rng;
use reqwest;
use serde_json::{json, Value};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::{process::CommandChild, ShellExt};
use tokio::time::{sleep, Duration};
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
    // Try to get existing API key from server state first
    let api_key = {
      let server_state = SERVER_STATE.lock().unwrap();
      server_state.api_key.clone()
    };

    let api_key = api_key.unwrap_or_else(|| {
      let new_key = format!("session-{}", Uuid::new_v4().to_string());
      log::info!("[llama_server] Generated API key: {}", new_key);
      new_key
    });

    // Get model path
    let model_path =
      setup::get_llm_model_path(app_handle.clone()).map_err(|e| ServerError::ModelNotFound(e))?;

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
        ServerError::ConfigError(format!("Model path is not valid UTF-8: {:?}", model_path))
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
  use std::net::{SocketAddr, TcpListener};

  let addr = SocketAddr::from(([127, 0, 0, 1], port));
  TcpListener::bind(addr).is_ok()
}

/// Find an available port by trying random ports
async fn find_available_port() -> Result<u16, ServerError> {
  for attempt in 1..=MAX_PORT_ATTEMPTS {
    let port = generate_random_port();
    log::debug!(
      "[llama_server] Trying port {} (attempt {}/{})",
      port,
      attempt,
      MAX_PORT_ATTEMPTS
    );

    if is_port_available(port).await {
      log::info!("[llama_server] Found available port: {}", port);
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

/// Get the current API key (if server is running)
fn get_current_api_key() -> Option<String> {
  let server_state = SERVER_STATE.lock().unwrap();
  server_state.api_key.clone()
}

/// Get server config using stored port and API key
fn get_current_server_config(app_handle: &AppHandle) -> Result<ServerConfig, ServerError> {
  let (port, api_key) = {
    let server_state = SERVER_STATE.lock().unwrap();
    (server_state.port, server_state.api_key.clone())
  };

  let port = port.ok_or_else(|| ServerError::ServerNotRunning)?;
  let api_key = api_key.ok_or_else(|| ServerError::ServerNotRunning)?;

  // Get model path
  let model_path =
    setup::get_llm_model_path(app_handle.clone()).map_err(|e| ServerError::ModelNotFound(e))?;

  let model_path_str = model_path
    .to_str()
    .ok_or_else(|| {
      ServerError::ConfigError(format!("Model path is not valid UTF-8: {:?}", model_path))
    })?
    .to_string();

  Ok(ServerConfig {
    port,
    api_key,
    model_path: model_path_str,
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

  log::info!("[llama_server] Using port {} for server", config.port);

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
      "--no-webui",
      "--log-disable",
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

  log::debug!(
    "[llama_server] Server started successfully on port {}",
    config.port
  );
  Ok(format!("Server started on port {}", config.port))
}

#[tauri::command]
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

#[tauri::command]
pub async fn check_server_health(app_handle: AppHandle) -> Result<Value, String> {
  // Get the current server config
  let config = get_current_server_config(&app_handle).map_err(|e| e.to_string())?;

  match perform_health_check(&config).await {
    Ok(response) => Ok(response),
    Err(e) => Err(format!("Health check failed: {}", e)),
  }
}

#[tauri::command]
pub async fn get_server_status(app_handle: AppHandle) -> Result<Value, String> {
  // Check if process is running and get port
  let process_running = {
    let server_state = SERVER_STATE.lock().unwrap();
    server_state.child.is_some()
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

  let config = get_current_server_config(&app_handle).map_err(|e| e.to_string())?;

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

#[tauri::command]
pub async fn make_completion_request(
  app_handle: AppHandle,
  prompt: String,
  max_tokens: Option<u32>,
  temperature: Option<f32>,
  top_p: Option<f32>,
) -> Result<Value, String> {
  let config = get_current_server_config(&app_handle).map_err(|e| e.to_string())?;

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
    return Err(format!(
      "Completion request failed with status: {}",
      response.status()
    ));
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
  log::info!("[llama_server] Restarting llama.cpp server...");

  // Stop the server if it's running
  let _ = stop_llama_server().await;

  // Wait a moment for cleanup
  sleep(Duration::from_secs(1)).await;

  // Start the server again
  spawn_llama_server(app_handle).await
}

/// Generate chat completion using OpenAI-compatible endpoint
#[tauri::command]
pub async fn generate(
  app_handle: AppHandle,
  prompt: String,
  json_schema: Option<String>,
  conv_id: Option<String>,
  use_thinking: Option<bool>,
  stream: Option<bool>,
) -> Result<String, String> {
  log::info!("[llama_server] Starting chat completion generation");
  let config = get_current_server_config(&app_handle).map_err(|e| e.to_string())?;

  // Check if server is healthy first
  if let Err(e) = perform_health_check(&config).await {
    return Err(format!("Server health check failed: {}", e));
  }

  let should_stream = stream.unwrap_or(false);
  let enable_thinking = use_thinking.unwrap_or(true);

  // Build messages array from conversation history and new prompt
  let mut messages = Vec::new();

  // If conversation ID is provided, load existing messages
  if let Some(conversation_id) = &conv_id {
    match crate::models::conversations::get_messages(app_handle.clone(), conversation_id.clone())
      .await
    {
      Ok(conv_messages) => {
        for msg in conv_messages {
          messages.push(json!({
              "role": msg.role,
              "content": msg.content
          }));
        }
      }
      Err(e) => {
        log::warn!(
          "[llama_server] Warning: Failed to load conversation messages: {}",
          e
        );
      }
    }
  }

  // Add the new user message
  messages.push(json!({
      "role": "user",
  "content": prompt
  }));

  // Build request body
  let mut request_body = json!({
      "model": "gpt-6",
      "messages": messages,
      "stream": should_stream
  });

  // Add JSON schema if provided
  if let Some(schema) = json_schema {
    if let Ok(schema_value) = serde_json::from_str::<Value>(&schema) {
      request_body["response_format"] = json!({
          "type": "json_object",
          "schema": schema_value
      });
    } else {
      return Err("Invalid JSON schema provided".to_string());
    }
  }

  // Add thinking parameter
  request_body["chat_template_kwargs"] = json!({
      "enable_thinking": enable_thinking
  });

  let client = reqwest::Client::new();
  let completion_url = format!("{}/v1/chat/completions", config.base_url());

  log::debug!("[llama_server] Making request to: {}", completion_url);

  if should_stream {
    // Handle streaming response
    let response = client
      .post(&completion_url)
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", config.api_key))
      .json(&request_body)
      .send()
      .await
      .map_err(|e| format!("Failed to send streaming request: {}", e))?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());
      return Err(format!("Server returned error {}: {}", status, error_text));
    }

    // Process streaming response
    let mut full_response = String::new();
    let mut stream = response.bytes_stream();

    use tokio_stream::StreamExt;

    while let Some(chunk_result) = stream.next().await {
      match chunk_result {
        Ok(chunk) => {
          let chunk_str = String::from_utf8_lossy(&chunk);

          // Parse SSE format
          for line in chunk_str.lines() {
            if line.starts_with("data: ") {
              let data = &line[6..]; // Remove "data: " prefix

              if data == "[DONE]" {
                break;
              }

              if let Ok(json_data) = serde_json::from_str::<Value>(data) {
                if let Some(choices) = json_data["choices"].as_array() {
                  if let Some(choice) = choices.get(0) {
                    // Check if stream is finished
                    if let Some(finish_reason) = choice["finish_reason"].as_str() {
                      if finish_reason == "stop" {
                        break;
                      }
                    }

                    // Process delta content if available
                    if let Some(delta) = choice["delta"].as_object() {
                      if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                        full_response.push_str(content);

                        // Emit stream event to frontend
                        let stream_data = json!({
                            "delta": content,
                            "full_response": full_response
                        });

                        if let Err(e) = app_handle.emit("chat-stream", &stream_data) {
                          log::error!("[llama_server] Failed to emit stream event: {}", e);
                        }
                      }
                      // If no content in delta, just continue (this is normal for some chunks)
                    }
                  }
                }
              }
            }
          }
        }
        Err(e) => {
          return Err(format!("Error reading stream: {}", e));
        }
      }
    }

    // Emit final stream completion event
    let final_stream_data = json!({
        "delta": "",
        "full_response": full_response,
        "is_finished": true
    });

    if let Err(e) = app_handle.emit("chat-stream", &final_stream_data) {
      log::error!("[llama_server] Failed to emit final stream event: {}", e);
    }

    // Store messages in conversation if conv_id is provided
    if let Some(conversation_id) = conv_id {
      // Add user message
      if let Err(e) = crate::models::conversations::add_message(
        app_handle.clone(),
        conversation_id.clone(),
        "user".to_string(),
        prompt,
      )
      .await
      {
        log::warn!("[llama_server] Warning: Failed to save user message: {}", e);
      }

      // Add assistant response
      if let Err(e) = crate::models::conversations::add_message(
        app_handle.clone(),
        conversation_id,
        "assistant".to_string(),
        full_response.clone(),
      )
      .await
      {
        log::warn!(
          "[llama_server] Warning: Failed to save assistant message: {}",
          e
        );
      }
    }

    Ok(full_response)
  } else {
    // Handle non-streaming response
    let start_time = std::time::Instant::now();

    let response = client
      .post(&completion_url)
      .header("Content-Type", "application/json")
      .header("Authorization", format!("Bearer {}", config.api_key))
      .json(&request_body)
      .send()
      .await
      .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
      let status = response.status();
      let error_text = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());
      return Err(format!("Server returned error {}: {}", status, error_text));
    }

    let result: Value = response
      .json()
      .await
      .map_err(|e| format!("Failed to parse response: {}", e))?;

    let elapsed = start_time.elapsed();

    // Extract the generated content
    let generated_text = result["choices"][0]["message"]["content"]
      .as_str()
      .ok_or("No content in response")?
      .to_string();

    // Extract token usage if available
    let tokens_generated = result["usage"]["completion_tokens"].as_u64().unwrap_or(0);

    // Store messages in conversation if conv_id is provided
    if let Some(conversation_id) = conv_id {
      // Add user message
      if let Err(e) = crate::models::conversations::add_message(
        app_handle.clone(),
        conversation_id.clone(),
        "user".to_string(),
        prompt,
      )
      .await
      {
        log::warn!("[llama_server] Warning: Failed to save user message: {}", e);
      }

      // Add assistant response
      if let Err(e) = crate::models::conversations::add_message(
        app_handle.clone(),
        conversation_id,
        "assistant".to_string(),
        generated_text.clone(),
      )
      .await
      {
        log::warn!(
          "[llama_server] Warning: Failed to save assistant message: {}",
          e
        );
      }
    }

    let total_seconds = elapsed.as_secs_f64();
    let tokens_per_second = if total_seconds > 0.0 && tokens_generated > 0 {
      tokens_generated as f64 / total_seconds
    } else {
      0.0
    };

    log::info!(
      "[llama_server] Generated {} characters, {} tokens in {:.2}s ({:.2} tokens/sec)",
      generated_text.len(),
      tokens_generated,
      total_seconds,
      tokens_per_second
    );
    Ok(generated_text)
  }
}
