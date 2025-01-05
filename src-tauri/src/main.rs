// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod control;
mod data;
mod runtime;
mod sidecar;

use tauri::RunEvent;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Emitter, Manager};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

// Helper function to spawn the sidecar and monitor its stdout/stderr
async fn spawn_and_monitor_sidecar(app_handle: tauri::AppHandle) -> Result<(), String> {
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


fn main() {
    tauri::Builder::default()
        // Add any necessary plugins
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            write_to_sidecar,
            control::move_mouse,
            control::click_mouse,
            control::type_string,
            data::check_model_download,
            data::download_model,
            data::take_screenshot,
            runtime::handle_request,
            server::start_server,
            server::shutdown_server,
            server::infer
        ])
        .setup(|app| {
            app.manage(Arc::new(Mutex::new(None::<(CommandChild, tauri::async_runtime::Receiver<CommandEvent>)>)));
            app.manage(Arc::new(Mutex::new(None::<CommandChild>)));
            // Clone the app handle for use elsewhere
            let app_handle = app.handle().clone();
            // Spawn the Python sidecar on startup
            println!("[tauri] Creating sidecar...");
            tauri::async_runtime::spawn(async move {
                if let Err(e) = spawn_and_monitor_sidecar(app_handle).await {
                    eprintln!("Failed to spawn and monitor sidecar: {}", e);
                }
            });
            println!("[tauri] Sidecar spawned and monitoring started.");

            println!("[tauri] Creating server...");
            match server::start_server() {
                Ok(output) => println!("Server output: {}", output),
                Err(err) => eprintln!("Failed to call server: {}", err),
            }
            println!("[tauri] Server started.");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| match event {
            // Ensure the qwen server sidecar is killed when the app is closed
            RunEvent::ExitRequested { .. } => {
                // Write to shutdown the sidecar
                tauri::async_runtime::block_on(write_to_sidecar(app_handle.clone(), "SHUTDOWN".to_string())).unwrap_or_else(|e| eprintln!("Failed to write to sidecar: {}", e));
                tauri::async_runtime::block_on(async {
                    match server::shutdown_server().await {
                        Ok(message) => println!("{}", message),
                        Err(err) => eprintln!("Failed to shutdown server: {}", err),
                    }
                });
            }
            _ => {}
        });
}
