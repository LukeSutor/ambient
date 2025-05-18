// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod data;
pub mod db;
pub mod vlm;
pub mod prompts;
pub mod scheduler;
pub mod embedding;
pub mod setup;
pub mod constants;
pub mod integrations;
use crate::integrations::chromium::server::start_server_on_available_port;
use tauri::Manager;
use std::sync::Mutex;
use db::DbState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(DbState(Mutex::new(None))) // Initialize state with None (using the imported DbState)
    .setup(|app| {
        // Initialize the database connection during setup
        let app_handle = app.handle().clone(); // Get the app handle
        match db::initialize_database(&app_handle) {
            Ok(conn) => {
                println!("[setup] Database initialized successfully.");
                // Store the connection in the managed state using the app_handle
                let state = app_handle.state::<DbState>(); // Use app_handle here (using the imported DbState)
                *state.0.lock().unwrap() = Some(conn); // Store the connection
            }
            Err(e) => {
                eprintln!("[setup] Failed to initialize database: {}", e);
                // Handle error appropriately, maybe panic or show an error dialog
                panic!("Database initialization failed: {}", e);
            }
        }

        // --- Start Chromium integration server on startup ---
        tauri::async_runtime::spawn(async {
            match start_server_on_available_port().await {
                Ok(port) => println!("[chromium/server] Running on port {}", port),
                Err(e) => eprintln!("[chromium/server] Failed to start: {}", e),
            }
        });

        Ok(())
    })
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![
        vlm::get_vlm_response,
        data::take_screenshot,
        prompts::get_prompt_command,
        scheduler::start_scheduler,
        scheduler::stop_scheduler,
        scheduler::get_scheduler_interval,
        embedding::get_embedding,
        db::execute_sql,
        db::reset_database,
        db::get_events, // <-- Add this line
        setup::setup,
        setup::get_vlm_text_model_path,
        setup::get_vlm_mmproj_model_path,
        setup::check_vlm_text_model_download,
        setup::check_vlm_mmproj_model_download,
        setup::get_fastembed_model_path,
        setup::check_fastembed_model_download,
        setup::check_setup_complete,
        integrations::chromium::server::chromium_ping // <-- Add ping command
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
