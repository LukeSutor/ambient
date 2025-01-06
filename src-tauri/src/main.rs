// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod control;
mod data;
mod runtime;
mod sidecar;

use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{Manager, RunEvent};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};


fn main() {
    tauri::Builder::default()
        // Add any necessary plugins
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            control::move_mouse,
            control::click_mouse,
            control::type_string,
            data::check_model_download,
            data::download_model,
            data::take_screenshot,
            runtime::handle_request,
            sidecar::start_sidecar,
            sidecar::shutdown_sidecar,
            sidecar::write_to_sidecar,
            sidecar::infer
        ])
        .setup(|app| {
            app.manage(Arc::new(Mutex::new(None::<(CommandChild, tauri::async_runtime::Receiver<CommandEvent>)>)));
            app.manage(Arc::new(Mutex::new(None::<CommandChild>)));
            // Clone the app handle for use elsewhere
            let app_handle = app.handle().clone();
            // Spawn the Python sidecar on startup
            println!("[tauri] Creating sidecar...");
            tauri::async_runtime::spawn(async move {
                if let Err(e) = sidecar::start_sidecar(app_handle).await {
                    eprintln!("Failed to spawn and monitor sidecar: {}", e);
                }
            });
            println!("[tauri] Sidecar spawned.");
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| match event {
            // Ensure the qwen server sidecar is killed when the app is closed
            RunEvent::ExitRequested { .. } => {
                // Write to shutdown the sidecar
                tauri::async_runtime::block_on(sidecar::write_to_sidecar(app_handle.clone(), "SHUTDOWN".to_string()))
                .unwrap_or_else(|e| {
                    eprintln!("Failed to write to sidecar: {}", e);
                    e.to_string()
                });
            }
            _ => {}
        });
}
