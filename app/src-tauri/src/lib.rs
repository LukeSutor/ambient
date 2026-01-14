// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod auth;
pub mod constants;
pub mod db;
pub mod events;
pub mod images;
pub mod memory;
pub mod models;
pub mod os_utils;
pub mod scheduler;
pub mod screen_selection;
pub mod settings;
pub mod setup;
pub mod tasks;
pub mod tray;
pub mod types;
pub mod windows;
use db::core::DbState;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_log::{Target, TargetKind};
use types::AppState;
extern crate dotenv;

use crate::os_utils::signals::setup_signal_handlers;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Load environment variables from .env file
  dotenv::dotenv().ok();

  // Setup signal handlers for graceful shutdown
  setup_signal_handlers();

  tauri::Builder::default()
    .plugin(tauri_plugin_os::init())
    .plugin(tauri_plugin_store::Builder::new().build())
    .plugin(
      tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::Stdout))
        .filter(|metadata| {
          let t = metadata.target();
          !(t.starts_with("hyper")
            || t.starts_with("reqwest")
            || t.starts_with("enigo")
            || t == "tao::platform_impl::platform::event_loop::runner")
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
        for url in &urls {
          crate::auth::deep_link::handle_open_url(&app_handle_for_deep_link, url.as_str());
        }
      });

      // Initialize the database connection during setup
      let app_handle = app.handle().clone();
      match db::core::initialize_database(&app_handle) {
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

      // Setup signal handlers for graceful shutdown
      setup_signal_handlers();

      // Create the system tray
      if let Err(e) = tray::create_tray(&app.handle()) {
        log::error!("[setup] Failed to create system tray: {}", e);
      } else {
        log::info!("[setup] System tray created successfully");
      }

      Ok(())
    })
    .on_window_event(|window, event| {
      match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
          // Prevent the window from closing and hide it instead
          // Only the tray quit option should actually exit the app
          log::info!(
            "[window] Window '{}' close requested - hiding instead of closing",
            window.label()
          );

          if let Err(e) = window.hide() {
            log::error!("[window] Failed to hide window '{}': {}", window.label(), e);
          }

          // Prevent the default close behavior
          api.prevent_close();
        }
        _ => {}
      }
    })
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![
      windows::open_main_window,
      windows::close_main_window,
      windows::open_secondary_window,
      windows::minimize_secondary_window,
      windows::close_secondary_window,
      windows::open_computer_use_window,
      windows::close_computer_use_window,
      windows::resize_hud,
      windows::refresh_hud_window_size,
      windows::resize_computer_use_window,
      settings::load_user_settings,
      settings::save_user_settings,
      settings::emit_settings_changed,
      images::save_screenshot,
      scheduler::start_capture_scheduler,
      scheduler::stop_capture_scheduler,
      scheduler::get_scheduler_interval,
      scheduler::is_scheduler_running,
      db::core::execute_sql,
      db::core::reset_database,
      db::events::get_events,
      db::workflows::get_workflows,
      db::workflows::insert_workflow,
      db::workflows::delete_workflow,
      db::activity::insert_activity_summary,
      db::activity::get_activity_summaries,
      db::conversations::create_conversation,
      db::conversations::add_message,
      db::conversations::get_messages,
      db::conversations::get_message,
      db::conversations::get_conversation,
      db::conversations::list_conversations,
      db::conversations::reset_conversation,
      db::conversations::delete_conversation,
      db::conversations::update_conversation_name,
      setup::setup,
      setup::is_online,
      setup::get_vlm_text_model_path,
      setup::get_vlm_mmproj_model_path,
      setup::check_vlm_text_model_download,
      setup::check_vlm_mmproj_model_download,
      setup::check_setup_complete,
      setup::get_ocr_text_detection_model_path,
      setup::get_ocr_text_recognition_model_path,
      setup::check_ocr_text_detection_model_download,
      setup::check_ocr_text_recognition_model_download,
      os_utils::windows::window::get_brave_url,
      os_utils::windows::window::get_screen_text_formatted,
      os_utils::handlers::capture_eval_data,
      models::llm::server::spawn_llama_server,
      models::llm::server::stop_llama_server,
      models::llm::server::generate,
      models::llm::handlers::handle_hud_chat,
      models::embedding::embedding::generate_embedding,
      db::memory::get_memory_entries_with_message,
      db::memory::delete_memory_entry,
      db::memory::delete_all_memories,
      auth::auth_flow::sign_up,
      auth::auth_flow::sign_in_with_password,
      auth::auth_flow::sign_in_with_google,
      auth::auth_flow::exchange_code_for_session,
      auth::auth_flow::verify_otp,
      auth::auth_flow::resend_confirmation,
      auth::auth_flow::refresh_token,
      auth::auth_flow::logout,
      auth::commands::is_authenticated,
      auth::commands::get_auth_state,
      auth::commands::get_current_user,
      auth::commands::get_user,
      auth::commands::get_access_token_command,
      auth::commands::emit_auth_changed,
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
      tasks::commands::get_task_progress_history,
      models::ocr::ocr::process_image,
      models::ocr::ocr::check_ocr_models_available,
      models::computer_use::commands::start_computer_use,
      models::computer_use::commands::execute_computer_action,
      screen_selection::open_screen_selector,
      screen_selection::close_screen_selector,
      screen_selection::process_screen_selection,
      screen_selection::cancel_screen_selection,
      screen_selection::get_screen_dimensions,
      screen_selection::client_to_screen_coords
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
