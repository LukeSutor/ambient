// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod runtime;
mod server;
mod control;

use std::sync::{Arc, Mutex};
use tauri::{Manager, RunEvent};
use tauri_plugin_shell::process::CommandChild;


fn main() {
    tauri::Builder::default()
        // Add any necessary plugins
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            runtime::handle_request,
            server::start_server,
            server::shutdown_server,
            server::infer,
            control::move_mouse,
            control::click_mouse,
            control::type_string
        ])
        .setup(|app| {
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
        .run(|_app_handle, event| match event {
            // Ensure the qwen server sidecar is killed when the app is closed
            RunEvent::ExitRequested { .. } => {
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
