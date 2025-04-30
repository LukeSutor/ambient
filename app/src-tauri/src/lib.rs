// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod data;
pub mod db;
pub mod vlm;
pub mod prompts;
pub mod scheduler;
pub mod embedding;
use tauri::Manager;

use rusqlite::Connection; // Added for clarity, though likely already implicitly used via db
use std::sync::Mutex; // Added for clarity
use db::DbState; // Import DbState from the db module

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
      db::reset_database
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
