// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use reqwest;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{Manager, RunEvent};
use tauri_plugin_shell::process::CommandChild;

#[tauri::command]
fn start_server() -> Result<String, String> {
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
async fn shutdown_server() -> Result<String, String> {
    println!("[tauri] Shutting down server...");
    let client = reqwest::Client::new();
    match client.post("http://localhost:8008/shutdown")
        .send()
        .await {
            Ok(res) => {
                if res.status().is_success() {
                    println!("[tauri] Server shut down.");
                    Ok("Server shutdown request sent successfully.".to_string())
                } else {
                    println!("[tauri] Server failed to shut down.");
                    Err(format!("Failed to shutdown server: {}", res.status()))
                }
            },
            Err(e) => Err(format!("Failed to send request: {}", e)),
        }
}

#[tauri::command]
async fn infer(prompt: std::string::String, image: std::string::String ) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client.post("http://localhost:8008/inference")
        .json(&serde_json::json!({
            "prompt": "Describe the image in detail.",
            "image": "path/to/image.jpg"
        }))
        .send()
        .await?;

    if res.status().is_success() {
        let body = res.text().await?;
        println!("Response: {}", body);
    } else {
        println!("Failed to get a successful response: {}", res.status());
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        // Add any necessary plugins
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .setup(|app| {
            // Store the initial sidecar process in the app state
            app.manage(Arc::new(Mutex::new(None::<CommandChild>)));
            // Clone the app handle for use elsewhere
            println!("[tauri] Creating server...");
            match start_server() {
                Ok(output) => println!("Server output: {}", output),
                Err(err) => eprintln!("Failed to call server: {}", err),
            }
            println!("[tauri] Server started.");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![start_server, shutdown_server])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| match event {
            // Ensure the qwen server sidecar is killed when the app is closed
            RunEvent::ExitRequested { .. } => {
                tauri::async_runtime::block_on(async {
                    match shutdown_server().await {
                        Ok(message) => println!("{}", message),
                        Err(err) => eprintln!("Failed to shutdown server: {}", err),
                    }
                });
            }
            _ => {}
        });
}
