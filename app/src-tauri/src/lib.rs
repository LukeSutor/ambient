// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod auth;
pub mod constants;
pub mod data;
pub mod db;
pub mod embedding;
pub mod integrations;
pub mod models;
pub mod os_utils;
pub mod prompts;
pub mod scheduler;
pub mod setup;
pub mod vlm;
// use crate::integrations::chromium::server::start_server_on_available_port;
use db::DbState;
use std::sync::Mutex;
use tauri::Manager;
use tauri::{Emitter};
use tauri_plugin_deep_link::DeepLinkExt;
extern crate dotenv;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Load environment variables from .env file
  dotenv::dotenv().ok();

  tauri::Builder::default()
    .plugin(tauri_plugin_single_instance::init(|app, args, cwd| {
      // Write your code here...
    }))
    .plugin(tauri_plugin_deep_link::init())
    .manage(DbState(Mutex::new(None)))
    .setup(|app| {
      // Register deep link scheme for development/testing
      #[cfg(any(windows, target_os = "linux"))]
      {
        use tauri_plugin_deep_link::DeepLinkExt;
        if let Err(e) = app.deep_link().register_all() {
          eprintln!("[deep_link] Failed to register deep link schemes: {}", e);
        } else {
          println!("[deep_link] Deep link schemes registered successfully");
        }
      }

      // Handle deep link events for OAuth2 callbacks
      let app_handle_for_deep_link = app.handle().clone();
      app.deep_link().on_open_url(move |event| {
        let urls = event.urls();
        println!("[deep_link] Received URLs: {:?}", urls);
        
        for url in &urls {
          let url_str = url.as_str();
          if url_str.starts_with("cortical://auth/callback") {
            println!("[deep_link] Processing OAuth2 callback: {}", url_str);
            
            // Parse URL to extract code and state
            if let Ok(parsed_url) = url::Url::parse(url_str) {
              let query_pairs: std::collections::HashMap<String, String> = parsed_url
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
              
              if let Some(code) = query_pairs.get("code") {
                let state = query_pairs.get("state").cloned();
                let code = code.clone();
                let app_handle_clone = app_handle_for_deep_link.clone();
                
                // Handle the callback asynchronously
                tauri::async_runtime::spawn(async move {
                  match auth::google_handle_callback(code, state).await {
                    Ok(result) => {
                      println!("[deep_link] OAuth2 callback handled successfully");
                      // Emit event to frontend
                      if let Err(e) = app_handle_clone.emit("oauth2-success", &result) {
                        eprintln!("[deep_link] Failed to emit oauth2-success event: {}", e);
                      }
                    },
                    Err(e) => {
                      eprintln!("[deep_link] Failed to handle OAuth2 callback: {}", e);
                      // Emit error event to frontend
                      if let Err(emit_err) = app_handle_clone.emit("oauth2-error", &e) {
                        eprintln!("[deep_link] Failed to emit oauth2-error event: {}", emit_err);
                      }
                    }
                  }
                });
              } else if let Some(error) = query_pairs.get("error") {
                let error_description = query_pairs.get("error_description").cloned();
                let error_msg = format!("OAuth2 error: {} - {}", error, error_description.unwrap_or_default());
                eprintln!("[deep_link] {}", error_msg);
                
                // Emit error event to frontend
                if let Err(e) = app_handle_for_deep_link.emit("oauth2-error", &error_msg) {
                  eprintln!("[deep_link] Failed to emit oauth2-error event: {}", e);
                }
              }
            }
          }
        }
      });

      // Initialize the database connection during setup
      let app_handle = app.handle().clone();
      match db::initialize_database(&app_handle) {
        Ok(conn) => {
          println!("[setup] Database initialized successfully.");
          // Store the connection in the managed state using the app_handle
          let state = app_handle.state::<DbState>();
          *state.0.lock().unwrap() = Some(conn);
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
      auth::logout,
      auth::get_stored_token,
      auth::is_authenticated,
      auth::cognito_sign_up,
      auth::cognito_sign_in,
      auth::cognito_confirm_sign_up,
      auth::cognito_resend_confirmation_code,
      auth::get_current_user,
      auth::get_access_token,
      auth::google_initiate_auth,
      auth::google_handle_callback,
      auth::google_sign_out
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
