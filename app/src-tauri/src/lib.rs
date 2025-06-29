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
pub mod os_utils;
pub mod models;
pub mod auth;
use crate::integrations::chromium::server::start_server_on_available_port;
use tauri::Manager;
use std::sync::Mutex;
use db::DbState;
extern crate dotenv;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Load environment variables from .env file
  dotenv::dotenv().ok();
  
  tauri::Builder::default()
    .manage(DbState(Mutex::new(None))) // Initialize state with None (using the imported DbState)
    // .manage(auth::create_auth_state()) // Initialize auth state
    .setup(|app| {
        // Initialize the database connection during setup
        let app_handle = app.handle().clone(); // Get the app handle
        match db::initialize_database(&app_handle) {
            Ok(conn) => {
                println!("[setup] Database initialized successfully.");
                // Store the connection in the managed state using the app_handle
                let state = app_handle.state::<DbState>(); // Use app_handle here (using the imported DbState)
                *state.0.lock().unwrap() = Some(conn); // Store the connection in tauri state
                // Also store in global state for insert_workflow_global
                // *db::GLOBAL_DB_STATE.0.lock().unwrap() = Some(conn);
            }
            Err(e) => {
                eprintln!("[setup] Failed to initialize database: {}", e);
                // Handle error appropriately, maybe panic or show an error dialog
                panic!("Database initialization failed: {}", e);
            }
        }

        // --- Start Chromium integration server on startup ---
        // tauri::async_runtime::spawn(async {
        //     match start_server_on_available_port(app_handle).await {
        //         Ok(port) => println!("[chromium/server] Running on port {}", port),
        //         Err(e) => eprintln!("[chromium/server] Failed to start: {}", e),
        //     }
        // });
        
        // Initialize Qwen3 model on startup
        let app_handle_clone = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            match models::llm::qwen3::initialize_qwen3_model().await {
                Ok(()) => println!("[setup] Qwen3 model initialized successfully."),
                Err(e) => eprintln!("[setup] Failed to initialize Qwen3 model: {}", e),
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
        db::get_events,
        db::get_workflows,
        db::insert_workflow,
        db::delete_workflow,
        setup::setup,
        setup::get_vlm_text_model_path,
        setup::get_vlm_mmproj_model_path,
        setup::check_vlm_text_model_download,
        setup::check_vlm_mmproj_model_download,
        setup::get_fastembed_model_path,
        setup::check_fastembed_model_download,
        setup::check_setup_complete,
        integrations::chromium::server::run_workflow_by_id,
        integrations::chromium::server::ping_chromium_extension,
        os_utils::windows::window::get_focused_window_name,
        os_utils::windows::window::get_all_text_from_focused_app,
        os_utils::windows::window::get_brave_url,
        models::llm::qwen3::generate,
        models::llm::qwen3::generate_qwen3,
        models::llm::qwen3::stream_qwen3,
        models::llm::qwen3::get_conversation_history,
        models::llm::qwen3::reset_conversation,
        models::llm::qwen3::list_conversations,
        models::llm::qwen3::get_current_conversation_id,
        models::llm::qwen3::is_qwen3_model_initialized,
        models::llm::qwen3::get_qwen3_status,
        auth::authenticate,
        auth::logout,
        auth::get_stored_token,
        auth::is_authenticated,
        auth::cognito_sign_up,
        auth::cognito_sign_in,
        auth::cognito_confirm_sign_up,
        auth::cognito_resend_confirmation_code,
        auth::get_current_user,
        auth::get_access_token
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
