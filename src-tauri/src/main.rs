// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod control;
mod data;
mod runtime;
mod server;

use tauri_plugin_store::StoreExt;
use tauri::{RunEvent, Manager};


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
            server::start_server,
            server::shutdown_server,
            server::infer
        ])
        .setup(|app| {
            // Create app store for key-value information pairs
            let store = app.store("store.json")?;
            // Store the data path
            let app_data_path = app
                .path()
                .app_data_dir()
                .expect("App data dir could not be fetched.");
            store.set("app-data-path", app_data_path.to_str().unwrap().to_string());

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
