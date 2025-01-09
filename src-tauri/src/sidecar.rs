// Contains functions for interacting with the C++ server

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

// Function to start the sidecar
#[tauri::command]
pub async fn start_sidecar(app_handle: tauri::AppHandle) -> Result<(), String> {
    println!("[tauri] Received command to start sidecar.");
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
        .sidecar("qwen2vl")
        .map_err(|e| {
            println!("[tauri] Failed to create sidecar command: {}", e);
            e.to_string()
        })?;
    let (rx, child) = sidecar_command.spawn().map_err(|e| e.to_string())?;
    // Store the child process and rx in the app state
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<(CommandChild, tauri::async_runtime::Receiver<CommandEvent>)>>>>() {
        *state.lock().await = Some((child, rx));
    } else {
        return Err("Failed to access app state".to_string());
    }
    println!("[tauri] Sidecar started and saved to app state");
    Ok(())
}

// Function to shut down the sidecar
#[tauri::command]
pub async fn shutdown_sidecar(app_handle: tauri::AppHandle) -> Result<String, String> {
    println!("[tauri] Received command to shutdown sidecar.");
    // Access the sidecar process state
    match write_to_sidecar(app_handle, "SHUTDOWN".to_string()).await {
        Ok(response) => {
            println!("[tauri] Sidecar shutdown successful: {}", response);
            Ok(response)
        }
        Err(e) => Err(e),
    }
}

// Function to write input to the sidecar and listen to the output
#[tauri::command]
pub async fn write_to_sidecar(app_handle: tauri::AppHandle, message: String) -> Result<String, String> {
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
            // llama.cpp logs to stderr, so ignore all writes to stderr
            while let Some(event) = rx.recv().await {
                if let CommandEvent::Stdout(line_bytes) = event {
                    let line = String::from_utf8_lossy(&line_bytes);
                    // If the line doesn't start with "RESPONSE", then it's just llama.cpp utility printing
                    if line.starts_with("RESPONSE") {
                        let line = line["RESPONSE ".len()..].to_string();
                        print!("Sidecar stdout: {}", line);
                        return Ok(line);
                    }
                }
            }
            return Err("No sidecar process running".to_string());
        }
    } else {
        return Err("Failed to access app state".to_string());
    }
    Err("No sidecar process running".to_string())
}

#[tauri::command]
pub async fn infer(prompt: String, image: String, app_handle: tauri::AppHandle) -> Result<String, String> {
    let request_body = serde_json::json!({
        "prompt": prompt,
        "image": image,
    });

    let request_body_string = format!("INFER {}", request_body.to_string().replace('\n', ""));
    let response = write_to_sidecar(app_handle, request_body_string).await?;
    Ok(response)
}
