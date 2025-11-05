use tauri::{AppHandle, LogicalSize, Manager};
use crate::settings::{HudDimensions, load_user_settings};
use crate::constants::HUD_WINDOW_LABEL;

/// Get current HUD dimensions from user settings
async fn get_current_hud_dimensions(app_handle: &AppHandle) -> HudDimensions {
  match load_user_settings(app_handle.clone()).await {
    Ok(settings) => settings.hud_size.to_dimensions(),
    Err(_) => {
      // Default fallback dimensions
      HudDimensions {
        default_width: 200.0,
        default_height: 200.0,
        chat_width: 500.0,
        input_bar_height: 60.0,
        chat_max_height: 350.0,
        login_width: 400.0,
        login_height: 600.0,
      }
    }
  }
}

// Resize the HUD to the input size, keeping top aligned and ensuring the window doesn't overflow the bottom of the screen
#[tauri::command]
pub async fn resize_hud(
  app_handle: AppHandle,
  width: f64,
  height: f64,
) -> Result<(), String> {
  let window_label = HUD_WINDOW_LABEL.to_string();

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    // Check current size
    let current_size = window.inner_size().map_err(|e| e.to_string())?;
    let requested_size = LogicalSize::new(width, height);
    
    // Skip resize if size is already the same
    if current_size.width as f64 == width && current_size.height as f64 == height {
      log::debug!("HUD window already at requested size: {}x{}", width, height);
      return Ok(());
    }

    // Get position before resizing
    let position = window.outer_position().map_err(|e| e.to_string())?;
    window.set_size(requested_size).map_err(|e| e.to_string())?;

    // Adjust position to keep top aligned
    window.set_position(tauri::PhysicalPosition::new(position.x, position.y)).map_err(|e| e.to_string())?;
    log::info!("HUD window resized to: {}x{}", width, height);
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
  let window_label = label.unwrap_or_else(|| "main".to_string());
  let dimensions = get_current_hud_dimensions(&app_handle).await;

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    let height = if is_expanded {
      dimensions.chat_max_height
    } else {
      dimensions.input_bar_height
    };

    let size = LogicalSize::new(dimensions.chat_width, height);
    window.set_size(size).map_err(|e| e.to_string())?;
    log::info!("HUD window size refreshed: {}x{} (expanded: {})", dimensions.chat_width, height, is_expanded);
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

// Reopen the main window
#[tauri::command]
pub async fn open_main_window(
  app_handle: AppHandle,
) -> Result<(), String> {
  let window_label = "main".to_string();

  if let Some(win) = app_handle.get_webview_window(&window_label) {
    // Focus and show existing window
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    return Ok(());
  }

  Err("Main window not found".to_string())
}

/// Close the floating HUD window by label (defaults to 'main').
#[tauri::command]
pub async fn close_main_window(
  app_handle: AppHandle,
) -> Result<(), String> {
  let window_label = "main".to_string();

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
) -> Result<(), String> {
  let window_label = "secondary".to_string();

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
    tauri::WebviewUrl::App("/secondary".into()),
  )
  .title("Settings")
  .inner_size(800 as f64, 800 as f64)
  .resizable(false)
  .transparent(true)
  .decorations(false)
  .shadow(false)
  .build()
  .map_err(|e| e.to_string())?;

  Ok(())
}

/// Close the secondary window
#[tauri::command]
pub async fn close_secondary_window(
  app_handle: AppHandle,
) -> Result<(), String> {
  let window_label = "secondary".to_string();

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    window.close().map_err(|e| e.to_string())?;
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}