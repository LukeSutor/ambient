use crate::constants::{DASHBOARD_WINDOW_LABEL, DASHBOARD_PATH, HUD_WINDOW_LABEL};
use crate::settings::{load_user_settings, HudDimensions};
use crate::events::{emitter::emit, types::{NAVIGATE_TO_CONVERSATION, NavigateToConversationEvent}};
use chrono::Utc;
use tauri::{AppHandle, LogicalSize, Manager};

/// Get current main window dimensions from user settings
async fn get_current_main_window_dimensions(app_handle: &AppHandle) -> HudDimensions {
  match load_user_settings(app_handle.clone()).await {
    Ok(settings) => settings.hud_size.to_dimensions(),
    Err(_) => {
      // Default fallback dimensions
      HudDimensions {
        chat_width: 600.0,
        input_bar_height: 106.0,
        chat_max_height: 450.0,
        login_width: 450.0,
        login_height: 600.0,
      }
    }
  }
}

// Resize the HUD to the input size, keeping top aligned and ensuring the window doesn't overflow the bottom of the screen
#[tauri::command]
pub async fn resize_main_window(app_handle: AppHandle, width: f64, height: f64) -> Result<(), String> {
  let window_label = HUD_WINDOW_LABEL.to_string();

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    // Get position before resizing to calculate overflow
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let mut new_x = position.x;
    let mut new_y = position.y;

    // Ensure resizing doesn't push the window off the bottom or right of the screen
    if let (Ok(Some(monitor)), Ok(scale_factor)) = (window.current_monitor(), window.scale_factor()) {
      let work_area = monitor.work_area();
      
      let physical_width = (width * scale_factor) as i32;
      let physical_height = (height * scale_factor) as i32;
      
      let monitor_right = work_area.position.x + work_area.size.width as i32;
      let monitor_bottom = work_area.position.y + work_area.size.height as i32;

      if new_x + physical_width > monitor_right {
        new_x = (monitor_right - physical_width).max(work_area.position.x);
      }

      if new_y + physical_height > monitor_bottom {
        new_y = (monitor_bottom - physical_height).max(work_area.position.y);
      }
    }

    window
      .set_size(LogicalSize::new(width, height))
      .map_err(|e| e.to_string())?;

    window
      .set_position(tauri::PhysicalPosition::new(new_x, new_y))
      .map_err(|e| e.to_string())?;

    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

/// Refresh the HUD window size based on current settings and expanded state
#[tauri::command]
pub async fn refresh_main_window_size(app_handle: AppHandle) -> Result<(), String> {
  let window_label = HUD_WINDOW_LABEL.to_string();
  let dimensions = get_current_main_window_dimensions(&app_handle).await;

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    let width = dimensions.chat_width;
    let height = dimensions.input_bar_height;

    // Get position before resizing to calculate overflow
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let mut new_x = position.x;
    let mut new_y = position.y;

    // Ensure resizing doesn't push the window off the bottom or right of the screen
    if let (Ok(Some(monitor)), Ok(scale_factor)) = (window.current_monitor(), window.scale_factor()) {
      let work_area = monitor.work_area();
      
      let physical_width = (width * scale_factor) as i32;
      let physical_height = (height * scale_factor) as i32;
      
      let monitor_right = work_area.position.x + work_area.size.width as i32;
      let monitor_bottom = work_area.position.y + work_area.size.height as i32;

      if new_x + physical_width > monitor_right {
        new_x = (monitor_right - physical_width).max(work_area.position.x);
      }

      if new_y + physical_height > monitor_bottom {
        new_y = (monitor_bottom - physical_height).max(work_area.position.y);
      }
    }

    log::info!(
      "HUD window size refreshed: {}x{}",
      width,
      height,
    );

    window
      .set_size(LogicalSize::new(width, height))
      .map_err(|e| e.to_string())?;

    window
      .set_position(tauri::PhysicalPosition::new(new_x, new_y))
      .map_err(|e| e.to_string())?;

    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

// Reopen the main window
#[tauri::command]
pub async fn open_main_window(app_handle: AppHandle) -> Result<(), String> {
  let window_label = HUD_WINDOW_LABEL.to_string();

  if let Some(win) = app_handle.get_webview_window(&window_label) {
    // Focus and show existing window
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    return Ok(());
  }

  Err("Main window not found".to_string())
}

/// Open the main window and navigate to a specific conversation.
///
/// Emits a navigation event that the frontend listens to.
#[tauri::command]
pub async fn open_main_window_at_conversation(
  app_handle: AppHandle,
  conversation_id: String,
  message_id: Option<String>,
) -> Result<(), String> {
  let window_label = HUD_WINDOW_LABEL.to_string();

  if let Some(win) = app_handle.get_webview_window(&window_label) {
    // Focus and show existing window
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;

    // Emit navigation event for the frontend to handle
    let event = NavigateToConversationEvent {
      conversation_id,
      message_id,
      timestamp: Utc::now().to_rfc3339(),
    };
    emit(NAVIGATE_TO_CONVERSATION, event)?;

    return Ok(());
  }

  Err("Main window not found".to_string())
}

/// Close the floating HUD window.
#[tauri::command]
pub async fn close_main_window(app_handle: AppHandle) -> Result<(), String> {
  let window_label = HUD_WINDOW_LABEL.to_string();

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    window.close().map_err(|e| e.to_string())?;
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

/// Open or focus the floating HUD window.
#[tauri::command]
pub async fn open_secondary_window(
  app_handle: AppHandle,
  destination: Option<String>,
) -> Result<(), String> {
  let window_label = DASHBOARD_WINDOW_LABEL.to_string();

  // Build the URL path based on destination parameter
  let path = if let Some(dest) = &destination {
    format!("{}/{}", DASHBOARD_PATH, dest)
  } else {
    DASHBOARD_PATH.to_string()
  };

  if let Some(win) = app_handle.get_webview_window(&window_label) {
    // Navigate to the destination if provided
    if destination.is_some() {
      win
        .eval(&format!("window.location.href = '{}'", path))
        .map_err(|e| e.to_string())?;
    }
    // Focus and show existing window
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    return Ok(());
  }

  // Create the window with user-configured dimensions
  let _window = tauri::WebviewWindowBuilder::new(
    &app_handle,
    window_label,
    tauri::WebviewUrl::App(path.into()),
  )
  .title("Dashboard")
  .inner_size(1200 as f64, 800 as f64)
  .min_inner_size(800.0 as f64, 500.0 as f64)
  .resizable(true)
  .transparent(true)
  .decorations(false)
  .shadow(false)
  .build()
  .map_err(|e: tauri::Error| e.to_string())?;

  Ok(())
}

/// Minimize the secondary window
#[tauri::command]
pub async fn minimize_secondary_window(app_handle: AppHandle) -> Result<(), String> {
  let window_label = DASHBOARD_WINDOW_LABEL.to_string();

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    window.minimize().map_err(|e| e.to_string())?;
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

/// Close the secondary window
#[tauri::command]
pub async fn close_secondary_window(app_handle: AppHandle) -> Result<(), String> {
  let window_label = DASHBOARD_WINDOW_LABEL.to_string();

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    window.close().map_err(|e| e.to_string())?;
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}
