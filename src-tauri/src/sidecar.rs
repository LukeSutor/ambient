// Contains functions for interacting with the C++ server

use reqwest;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

// Function to start the sidecar
async fn start_sidecar(app_handle: tauri::AppHandle) -> Result<(), String> {
    // Check if a sidecar process already exists
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<CommandChild>>>>() {
        let child_process = state.lock().await;
        if child_process.is_some() {
            // A sidecar is already running, do not spawn a new one
            println!("[tauri] Sidecar is already running. Skipping spawn.");
            return Ok(()); // Exit early since sidecar is already running
        }
    }
    // Spawn sidecar
    let sidecar_command = app_handle
        .shell()
        .sidecar("test")
        .map_err(|e| e.to_string())?;
    let (rx, child) = sidecar_command.spawn().map_err(|e| e.to_string())?;
    // Store the child process and rx in the app state
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<(CommandChild, tauri::async_runtime::Receiver<CommandEvent>)>>>>() {
        *state.lock().await = Some((child, rx));
    } else {
        return Err("Failed to access app state".to_string());
    }
    Ok(())
}

// Function to shut down the sidecar
#[tauri::command]
fn shutdown_sidecar(app_handle: tauri::AppHandle) -> Result<String, String> {
    println!("[tauri] Received command to shutdown sidecar.");
    // Access the sidecar process state
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<CommandChild>>>>() {
        let mut child_process = state
            .lock()
            .map_err(|_| "[tauri] Failed to acquire lock on sidecar process.")?;

        if let Some(mut process) = child_process.take() {
            let command = "sidecar shutdown\n"; // Add newline to signal the end of the command

            // Attempt to write the command to the sidecar's stdin
            if let Err(err) = process.write(command.as_bytes()) {
                println!("[tauri] Failed to write to sidecar stdin: {}", err);
                // Restore the process reference if shutdown fails
                *child_process = Some(process);
                return Err(format!("Failed to write to sidecar stdin: {}", err));
            }

            println!("[tauri] Sent 'sidecar shutdown' command to sidecar.");
            Ok("'sidecar shutdown' command sent.".to_string())
        } else {
            println!("[tauri] No active sidecar process to shutdown.");
            Err("No active sidecar process to shutdown.".to_string())
        }
    } else {
        Err("Sidecar process state not found.".to_string())
    }
}

// Function to write input to the sidecar and listen to the output
#[tauri::command]
async fn write_to_sidecar(app_handle: tauri::AppHandle, message: String) -> Result<String, String> {
    println!("Writing to sidecar: {}", message);
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<(CommandChild, tauri::async_runtime::Receiver<CommandEvent>)>>>>() {
        let mut state_guard = state.lock().await;
        if let Some((child, rx)) = state_guard.as_mut() {
            let message_with_newline = if message.ends_with('\n') {
                message.clone()
            } else {
                format!("{}\n", message)
            };
            child.write(message_with_newline.as_bytes()).map_err(|e| e.to_string())?;
            
            // Wait for the sidecar to write back
            if let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line_bytes) => {
                        let line = String::from_utf8_lossy(&line_bytes);
                        println!("Sidecar stdout: {}", line);
                        Ok(line.to_string())
                    }
                    _ => Err("Unexpected event from sidecar".to_string()),
                }
            } else {
                Err("No response from sidecar".to_string())
            }
        } else {
            Err("No sidecar process running".to_string())
        }
    } else {
        Err("Failed to access app state".to_string())
    }
}

#[tauri::command]
pub fn start_server() -> Result<String, String> {
    println!("[tauri] Starting server...");
    // Spawn the command
    let _child = Command::new("C:\\Users\\Luke\\Desktop\\coding\\local-computer-use\\src-tauri\\binaries\\qwen2vl-server-x86_64-pc-windows-msvc.exe")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command: {}", e))?;

    println!("[tauri] Server started.");
    Ok("Command is running.".to_string())
}

#[tauri::command]
pub async fn shutdown_server() -> Result<String, String> {
    println!("[tauri] Shutting down server...");
    let client = reqwest::Client::new();
    match client.post("http://localhost:8008/shutdown").send().await {
        Ok(res) => {
            if res.status().is_success() {
                println!("[tauri] Server shut down.");
                Ok("Server shutdown request sent successfully.".to_string())
            } else {
                println!("[tauri] Server failed to shut down.");
                Err(format!("Failed to shutdown server: {}", res.status()))
            }
        }
        Err(e) => Err(format!("Failed to send request: {}", e)),
    }
}

#[tauri::command]
pub async fn infer(prompt: String, image: String) -> Result<String, String> {
    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "prompt": prompt,
        "image": image,
    });

    match client
        .post("http://localhost:8008/inference")
        .json(&request_body)
        .send()
        .await
    {
        Ok(res) => {
            if res.status().is_success() {
                let response_text = res
                    .text()
                    .await
                    .map_err(|e| format!("Failed to read response text: {}", e))?;
                Ok(response_text)
            } else {
                Err(format!(
                    "Failed to get a successful response: {}",
                    res.status()
                ))
            }
        }
        Err(e) => Err(format!("Failed to send request: {}", e)),
    }
}
