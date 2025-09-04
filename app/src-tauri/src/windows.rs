use tauri::{AppHandle, LogicalSize, Manager};
use crate::settings::{HudDimensions, load_user_settings};

/// Get current HUD dimensions from user settings
async fn get_current_hud_dimensions(app_handle: &AppHandle) -> HudDimensions {
  match load_user_settings(app_handle.clone()).await {
    Ok(settings) => settings.hud_size.to_dimensions(),
    Err(_) => {
      // Default fallback dimensions
      HudDimensions {
        width: 500.0,
        collapsed_height: 60.0,
        expanded_height: 350.0,
      }
    }
  }
}

/// Resize the HUD window to collapsed state (input only)
#[tauri::command]
pub async fn resize_hud_collapsed(
  app_handle: AppHandle,
  label: Option<String>,
) -> Result<(), String> {
  let window_label = label.unwrap_or_else(|| "floating-hud".to_string());
  let dimensions = get_current_hud_dimensions(&app_handle).await;

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    let size = LogicalSize::new(dimensions.width, dimensions.collapsed_height);
    window.set_size(size).map_err(|e| e.to_string())?;
    log::info!("HUD window resized to collapsed: {}x{}", dimensions.width, dimensions.collapsed_height);
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

/// Resize the HUD window to expanded state (input + chat area)
#[tauri::command]
pub async fn resize_hud_expanded(
  app_handle: AppHandle,
  label: Option<String>,
) -> Result<(), String> {
  let window_label = label.unwrap_or_else(|| "floating-hud".to_string());
  let dimensions = get_current_hud_dimensions(&app_handle).await;

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    let size = LogicalSize::new(dimensions.width, dimensions.expanded_height);
    window.set_size(size).map_err(|e| e.to_string())?;
    log::info!("HUD window resized to expanded: {}x{}", dimensions.width, dimensions.expanded_height);
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

// Dynamically resize the HUD to the required height and shift the position
#[tauri::command]
pub async fn resize_hud_dynamic(
  app_handle: AppHandle,
  additional_height: f64,
  label: Option<String>,
) -> Result<(), String> {
  if additional_height <= 30.0 {
    return Ok(());
  }
  let window_label = label.unwrap_or_else(|| "floating-hud".to_string());

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    // Get collapsed height
    let dimensions = get_current_hud_dimensions(&app_handle).await;
    let width = dimensions.width;
    let new_height = dimensions.collapsed_height + additional_height + 2.0; // Extra padding
    let new_height = new_height.min(dimensions.expanded_height);

    // Get current size and position
    let current_size = window.outer_size().map_err(|e| e.to_string())?;

    // Resize the window
    let size = LogicalSize::new(width, new_height);
    window.set_size(size).map_err(|e| e.to_string())?;

    // Adjust position to keep bottom aligned
    if let Ok(position) = window.outer_position() {
      let new_y = position.y + (current_size.height as f64 - new_height) as i32;
      window.set_position(tauri::PhysicalPosition::new(position.x, new_y)).map_err(|e| e.to_string())?;
    }

    log::info!("HUD window dynamically resized to: {}x{}", current_size.width, new_height);
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

/// Refresh the HUD window size based on current settings and expanded state
#[tauri::command]
pub async fn refresh_hud_window_size(
  app_handle: AppHandle,
  label: Option<String>,
  is_expanded: bool,
) -> Result<(), String> {
  let window_label = label.unwrap_or_else(|| "floating-hud".to_string());
  let dimensions = get_current_hud_dimensions(&app_handle).await;

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    let height = if is_expanded {
      dimensions.expanded_height
    } else {
      dimensions.collapsed_height
    };
    
    let size = LogicalSize::new(dimensions.width, height);
    window.set_size(size).map_err(|e| e.to_string())?;
    log::info!("HUD window size refreshed: {}x{} (expanded: {})", dimensions.width, height, is_expanded);
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

/// Open or focus the floating HUD window.
/// If a window with the given label exists, it will be brought to front; otherwise it will be created.
#[tauri::command]
pub async fn open_floating_window(
  app_handle: AppHandle,
  label: Option<String>,
) -> Result<(), String> {
  let window_label = label.unwrap_or_else(|| "floating-hud".to_string());

  if let Some(win) = app_handle.get_webview_window(&window_label) {
    // Focus and show existing window
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    return Ok(());
  }

  // Get current dimensions from user settings
  let dimensions = get_current_hud_dimensions(&app_handle).await;

  // Create the window with user-configured dimensions
  let _window = tauri::WebviewWindowBuilder::new(
    &app_handle,
    window_label,
    tauri::WebviewUrl::App("/hud".into()),
  )
  .title("Cortical Assistant")
  .inner_size(dimensions.width, dimensions.collapsed_height)
  .resizable(false)
  .transparent(true)
  .decorations(false)
  .always_on_top(true)
  .shadow(false)
  .skip_taskbar(true)
  .build()
  .map_err(|e| e.to_string())?;

  Ok(())
}

/// Close the floating HUD window by label (defaults to 'floating-hud').
#[tauri::command]
pub async fn close_floating_window(
  app_handle: AppHandle,
  label: Option<String>,
) -> Result<(), String> {
  let window_label = label.unwrap_or_else(|| "floating-hud".to_string());

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    window.close().map_err(|e| e.to_string())?;
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}


// Reopen the main window
#[tauri::command]
pub async fn open_main_window(
  app_handle: AppHandle,
  label: Option<String>,
) -> Result<(), String> {
  let window_label = label.unwrap_or_else(|| "main".to_string());

  if let Some(win) = app_handle.get_webview_window(&window_label) {
    // Focus and show existing window
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    return Ok(());
  }

  Err("Main window not found".to_string())
}