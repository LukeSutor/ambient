// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod auth;
pub mod constants;
pub mod data;
pub mod db;
pub mod events;
pub mod models;
pub mod os_utils;
pub mod scheduler;
pub mod setup;
pub mod tasks;
pub mod types;
use db::DbState;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_log::{Target, TargetKind};
use types::AppState;
extern crate dotenv;

#[tauri::command]
async fn close_floating_window(app_handle: tauri::AppHandle, label: String) -> Result<(), String> {
  log::info!("[close_floating_window] Attempting to close window with label: {}", label);
  
  if let Some(window) = app_handle.get_webview_window(&label) {
    log::info!("[close_floating_window] Window found, attempting to close");
    match window.destroy() {
      Ok(_) => {
        log::info!("[close_floating_window] Window close command successful");
        Ok(())
      },
      Err(e) => {
        log::error!("[close_floating_window] Failed to close window: {}", e);
        Err(e.to_string())
      }
    }
  } else {
    log::error!("[close_floating_window] Window not found with label: {}", label);
    Err("Window not found".to_string())
  }
}

// Global cleanup handler to ensure llama server is stopped
struct CleanupHandler;

impl Drop for CleanupHandler {
  fn drop(&mut self) {
    // This will be called when the app is shutting down
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
      if let Err(e) = models::llm::server::stop_llama_server().await {
        log::error!("[cleanup] Failed to stop llama server during cleanup: {}", e);
      } else {
        log::info!("[cleanup] Llama server stopped during cleanup");
      }
    });
  }
}

static _CLEANUP_HANDLER: std::sync::LazyLock<CleanupHandler> =
  std::sync::LazyLock::new(|| CleanupHandler);

use crate::os_utils::signals::setup_signal_handlers;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Load environment variables from .env file
  dotenv::dotenv().ok();

  // Setup signal handlers for graceful shutdown
  setup_signal_handlers();

  tauri::Builder::default()
    .plugin(
      tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::Stdout))
        .filter(|metadata| {
          let t = metadata.target();
          !(t.starts_with("hyper") || t.starts_with("reqwest"))
        })
        .build(),
    )
    .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {
      // Do nothing for now...
    }))
    .plugin(tauri_plugin_deep_link::init())
    .manage(DbState(Mutex::new(None)))
    .setup(|app| {
      // Register deep link scheme for development/testing
      #[cfg(any(windows, target_os = "linux"))]
      {
        use tauri_plugin_deep_link::DeepLinkExt;
        if let Err(e) = app.deep_link().register_all() {
          log::error!("[deep_link] Failed to register deep link schemes: {}", e);
        } else {
          log::info!("[deep_link] Deep link schemes registered successfully");
        }
      }

      // Get the PID and save it in the app state
      let pid = std::process::id();
      app.manage(AppState { pid });
  log::info!("[setup] Application PID: {}", pid);

      // Initialize the event emitter and listeners
      events::get_emitter().set_app_handle(app.handle().clone());
      events::initialize_event_listeners(app.handle().clone());

      // Handle deep link events for OAuth2 callbacks
      let app_handle_for_deep_link = app.handle().clone();
      app.deep_link().on_open_url(move |event| {
        let urls = event.urls();
        log::info!("[deep_link] Received URLs: {:?}", urls);
        for url in &urls {
          crate::auth::deep_link::handle_open_url(&app_handle_for_deep_link, url.as_str());
        }
      });

      // Initialize the database connection during setup
      let app_handle = app.handle().clone();
      match db::initialize_database(&app_handle) {
        Ok(conn) => {
          log::info!("[setup] Database initialized successfully.");
          // Store the connection in the managed state using the app_handle
          let state = app_handle.state::<DbState>();
          *state.0.lock().unwrap() = Some(conn);
        }
        Err(e) => {
          log::error!("[setup] Failed to initialize database: {}", e);
          panic!("Database initialization failed: {}", e);
        }
      }

      // Start llama.cpp server on startup
      let app_handle_for_llama = app.handle().clone();
      tauri::async_runtime::spawn(async move {
        // Wait to ensure the app is fully initialized
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        match models::llm::server::spawn_llama_server(app_handle_for_llama).await {
          Ok(message) => log::info!("[setup] {}", message),
          Err(e) => log::error!("[setup] Failed to start llama.cpp server: {}", e),
        }
      });
      
      // Initialize the cleanup handler to ensure it's ready
      std::sync::LazyLock::force(&_CLEANUP_HANDLER);

      // Setup signal handlers for graceful shutdown
      setup_signal_handlers();

      Ok(())
    })
    .on_window_event(|window, event| {
      match event {
        tauri::WindowEvent::CloseRequested { .. } => {
          // Only stop the llama server when the main window is being closed
          if window.label() == "main" {
            log::info!("[shutdown] Main window closing, stopping llama server...");
            let _app_handle = window.app_handle().clone();
            tauri::async_runtime::spawn(async move {
              if let Err(e) = models::llm::server::stop_llama_server().await {
                log::error!("[shutdown] Failed to stop llama server: {}", e);
              } else {
                log::info!("[shutdown] Llama server stopped successfully");
              }
            });
          } else {
            log::info!("[shutdown] Non-main window '{}' closing", window.label());
          }
        }
        _ => {}
      }
    })
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![
      close_floating_window,
      data::take_screenshot,
      scheduler::start_capture_scheduler,
      scheduler::stop_capture_scheduler,
      scheduler::get_scheduler_interval,
      scheduler::is_scheduler_running,
      db::execute_sql,
      db::reset_database,
      db::get_events,
      db::get_workflows,
      db::insert_workflow,
      db::delete_workflow,
      db::insert_activity_summary,
      db::get_activity_summaries,
      setup::setup,
      setup::get_vlm_text_model_path,
      setup::get_vlm_mmproj_model_path,
      setup::check_vlm_text_model_download,
      setup::check_vlm_mmproj_model_download,
      setup::check_setup_complete,
      setup::get_llm_model_path,
      setup::check_llm_model_download,
      os_utils::windows::window::get_all_text_from_focused_app,
      os_utils::windows::window::get_brave_url,
      os_utils::windows::window::get_screen_text_formatted,
      os_utils::handlers::capture_eval_data,
      models::llm::server::spawn_llama_server,
      models::llm::server::stop_llama_server,
      models::llm::server::check_server_health,
      models::llm::server::get_server_status,
      models::llm::server::get_server_port,
      models::llm::server::restart_llama_server,
      models::llm::server::make_completion_request,
      models::llm::server::generate,
      models::conversations::create_conversation,
      models::conversations::add_message,
      models::conversations::get_messages,
      models::conversations::get_conversation,
      models::conversations::list_conversations,
      models::conversations::reset_conversation,
      models::conversations::delete_conversation,
      models::conversations::update_conversation_name,
      auth::logout,
      auth::get_stored_token,
      auth::is_authenticated,
      auth::cognito_sign_up,
      auth::cognito_sign_in,
      auth::cognito_confirm_sign_up,
      auth::cognito_resend_confirmation_code,
      auth::get_current_user,
      auth::get_access_token,
      auth::google_sign_in,
      auth::google_sign_out,
      tasks::commands::create_task,
      tasks::commands::create_task_from_template,
      tasks::commands::get_task,
      tasks::commands::get_active_tasks,
      tasks::commands::get_task_templates,
      tasks::commands::get_template_categories,
      tasks::commands::get_available_frequencies,
      tasks::commands::update_task_status,
      tasks::commands::update_step_status,
      tasks::commands::delete_task,
      tasks::commands::complete_task,
      tasks::commands::get_overdue_tasks,
      tasks::commands::get_tasks_due_today,
      tasks::commands::get_tasks_by_frequency,
      tasks::commands::analyze_current_screen_for_tasks,
      tasks::commands::get_task_progress_history
    ])
    .on_window_event(|window, event| match event {
      tauri::WindowEvent::Destroyed => {
        if window.label() == "main" {
          window.app_handle().exit(0);
        }
      }
      _ => {}
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
