use crate::constants::{COMPUTER_USE_WINDOW_LABEL, COMPUTER_USE_PATH, DASHBOARD_WINDOW_LABEL, DASHBOARD_PATH, HUD_WINDOW_LABEL, MARGIN_BOTTOM, MARGIN_LEFT};
use crate::settings::{load_user_settings, HudDimensions};
use tauri::{AppHandle, LogicalSize, Manager};

/// Get current main window dimensions from user settings
async fn get_current_main_window_dimensions(app_handle: &AppHandle) -> HudDimensions {
  match load_user_settings(app_handle.clone()).await {
    Ok(settings) => settings.hud_size.to_dimensions(),
    Err(_) => {
      // Default fallback dimensions
      HudDimensions {
        chat_width: 600.0,
        input_bar_height: 130.0,
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
  .map_err(|e| e.to_string())?;

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


/// Open computer use window
#[tauri::command]
pub async fn open_computer_use_window(app_handle: AppHandle) -> Result<(), String> {
  let window_label = COMPUTER_USE_WINDOW_LABEL.to_string();

  if let Some(win) = app_handle.get_webview_window(&window_label) {
    // Focus and show existing window
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    return Ok(());
  }

  // Get monitor dimensions to calculate position
  let monitor = app_handle
    .primary_monitor()
    .map_err(|e| e.to_string())?
    .ok_or("Primary monitor not found")?;
  
  let scale_factor = monitor.scale_factor();
  let work_area = monitor.work_area().size.to_logical::<f64>(scale_factor);
  
  let width = 300.0;
  let height = 40.0;

  // Calculate coordinates (50 pixels from left and bottom)
  let x = MARGIN_LEFT as f64;
  let y = work_area.height - MARGIN_BOTTOM as f64 - height;

  // Create the window
  let _window = tauri::WebviewWindowBuilder::new(
    &app_handle,
    window_label,
    tauri::WebviewUrl::App(COMPUTER_USE_PATH.into()),
  )
  .title("Computer Use")
  .inner_size(width, height)
  .position(x, y)
  .resizable(false)
  .transparent(true)
  .decorations(false)
  .shadow(false)
  .always_on_top(true)
  .skip_taskbar(true)
  .build()
  .map_err(|e| e.to_string())?;
  Ok(())
}

/// Close the computer use window
#[tauri::command]
pub async fn close_computer_use_window(app_handle: AppHandle) -> Result<(), String> {
  let window_label = COMPUTER_USE_WINDOW_LABEL.to_string();

  if let Some(window) = app_handle.get_webview_window(&window_label) {
    window.close().map_err(|e| e.to_string())?;
    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}

/// Resize computer use window, ensuring it stays within margin of bottom left corner
#[tauri::command]
pub async fn resize_computer_use_window(
  app_handle: AppHandle,
  width: f64,
  height: f64,
) -> Result<(), String> {
  let window_label = COMPUTER_USE_WINDOW_LABEL.to_string();
  if let Some(window) = app_handle.get_webview_window(&window_label) {
    // Get monitor dimensions to calculate position
    let monitor = app_handle
      .primary_monitor()
      .map_err(|e| e.to_string())?
      .ok_or("Primary monitor not found")?;
    
    let scale_factor = monitor.scale_factor();
    let work_area = monitor.work_area().size.to_logical::<f64>(scale_factor);

    // Calculate new position to keep within margin
    let x = MARGIN_LEFT as f64;
    let y = work_area.height - MARGIN_BOTTOM as f64 - height;

    window
      .set_size(LogicalSize::new(width, height))
      .map_err(|e| e.to_string())?;

    window
      .set_position(tauri::LogicalPosition::new(x, y))
      .map_err(|e| e.to_string())?;

    Ok(())
  } else {
    Err("Window not found".to_string())
  }
}
