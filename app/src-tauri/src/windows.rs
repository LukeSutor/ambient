use serde::{Deserialize, Serialize};
use tauri::{AppHandle, LogicalSize, Manager};

// HUD window sizing constants
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HudSizes {
  pub width: f64,
  pub collapsed_height: f64,
  pub expanded_height: f64,
}

impl HudSizes {
  pub const fn new() -> Self {
    Self {
      width: 500.0,
      collapsed_height: 60.0,
      expanded_height: 350.0,
    }
  }
}

pub const HUD_SIZES: HudSizes = HudSizes::new();

/// Resize the HUD window to collapsed state (input only)
#[tauri::command]
pub async fn resize_hud_collapsed(
  app_handle: AppHandle,
  label: Option<String>,
) -> Result<(), String> {
  let window_label = label.unwrap_or_else(|| "floating-hud".to_string());

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    let size = LogicalSize::new(HUD_SIZES.width, HUD_SIZES.collapsed_height);
    window.set_size(size).map_err(|e| e.to_string())?;
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

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    let size = LogicalSize::new(HUD_SIZES.width, HUD_SIZES.expanded_height);
    window.set_size(size).map_err(|e| e.to_string())?;
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

/// Get the HUD sizing constants for use in the frontend
#[tauri::command]
pub async fn get_hud_sizes() -> Result<HudSizes, String> {
  Ok(HUD_SIZES)
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

  // Create the window with the same properties expected by the frontend
  let _window = tauri::WebviewWindowBuilder::new(
    &app_handle,
    window_label,
    tauri::WebviewUrl::App("/hud".into()),
  )
  .title("TaskAware Assistant")
  .inner_size(HUD_SIZES.width, HUD_SIZES.collapsed_height)
  .resizable(false)
  .transparent(true)
  .decorations(false)
  .always_on_top(true)
  .shadow(false)
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
