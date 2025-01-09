// Contains functions for interacting with the C++ server

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;
use crate::data::{check_model_download, get_model_paths};

// Function to start the sidecar
#[tauri::command]
pub async fn start_sidecar(app_handle: tauri::AppHandle) -> Result<(), String> {
    println!("[tauri] Received command to start sidecar.");
    // Check if a sidecar process already exists
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<(CommandChild, tauri::async_runtime::Receiver<CommandEvent>)>>>>() {
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
    
    // Load the model
    if let Err(e) = load_model(app_handle.clone()).await {
        println!("[tauri] Failed to load model: {}", e);
        return Err(e);
    }
    Ok(())
}

// Function to shut down the sidecar
#[tauri::command]
pub async fn shutdown_sidecar(app_handle: tauri::AppHandle) -> Result<String, String> {
    println!("[tauri] Received command to shutdown sidecar.");
    // Access the sidecar process state
    let result = match write_to_sidecar(app_handle.clone(), "SHUTDOWN".to_string()).await {
        Ok(response) => {
            println!("[tauri] Sidecar shutdown successful: {}", response);
            Ok(response)
        }
        Err(e) => Err(e),
    };

    // Remove the sidecar process from the app state
    if let Some(state) = app_handle.try_state::<Arc<Mutex<Option<(CommandChild, tauri::async_runtime::Receiver<CommandEvent>)>>>>() {
        let mut state_guard = state.lock().await;
        *state_guard = None;
        println!("[tauri] Sidecar process removed from app state");
    } else {
        println!("[tauri] Failed to access app state to remove sidecar process");
    }

    result
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

#[tauri::command]
pub async fn load_model(app_handle: tauri::AppHandle) -> Result<String, String> {
    // Ensure the models are downloaded
    if !check_model_download(app_handle.clone()) {
        return Err("Model files are not downloaded".to_string());
    }

    // Get the path to the models
    let model_paths = get_model_paths(app_handle.clone());
    let text_model_path = model_paths[0].clone();
    let vision_model_path = model_paths[1].clone();

    let request_body = serde_json::json!({
        "text_model": text_model_path,
        "vision_model": vision_model_path,
    });
    let request_body_string = format!("LOAD {}", request_body.to_string().replace('\n', ""));
    let response = write_to_sidecar(app_handle, request_body_string).await?;
    Ok(response)
}