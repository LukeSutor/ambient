use crate::os_utils::windows::window::{get_screen_text_in_bounds_formatted, get_screen_text_in_bounds_raw, ApplicationTextData};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SelectionBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScreenSelectionResult {
    pub bounds: SelectionBounds,
    pub text_content: String,
    pub raw_data: Vec<ApplicationTextData>,
}

/// Open the screen selection overlay window
#[tauri::command]
pub async fn open_screen_selector(
    app_handle: AppHandle,
    label: Option<String>,
) -> Result<(), String> {
    let window_label = label.unwrap_or_else(|| "screen-selector".to_string());

    // Check if window already exists
    if let Some(existing_window) = app_handle.get_webview_window(&window_label) {
        // Try to show and focus the existing window first
        match existing_window.show() {
            Ok(_) => {
                match existing_window.set_focus() {
                    Ok(_) => {
                        log::info!("Reused existing screen selector window");
                        return Ok(());
                    }
                    Err(e) => log::warn!("Failed to focus existing window: {}", e),
                }
            }
            Err(e) => log::warn!("Failed to show existing window: {}", e),
        }
        
        // If showing/focusing failed, close the existing window and create a new one
        if let Err(e) = existing_window.close() {
            log::warn!("Failed to close existing window: {}", e);
        }
        
        // Wait a bit for the window to be properly closed
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Get primary monitor dimensions for fullscreen overlay
    let monitors = app_handle.primary_monitor().map_err(|e| e.to_string())?;
    let monitor = monitors.ok_or("No primary monitor found")?;
    let monitor_size = monitor.size();
    let monitor_position = monitor.position();

    // Create the fullscreen overlay window
    let _window = tauri::WebviewWindowBuilder::new(
        &app_handle,
        window_label,
        tauri::WebviewUrl::App("/screen-selector".into()),
    )
    .title("Screen Selector")
    .inner_size(
        monitor_size.width as f64, 
        monitor_size.height as f64
    )
    .position(
        monitor_position.x as f64,
        monitor_position.y as f64,
    )
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .shadow(false)
    .skip_taskbar(true)
    .focused(true)
    .build()
    .map_err(|e| e.to_string())?;

    log::info!("Screen selector overlay window created");
    Ok(())
}

/// Close the screen selection overlay window
#[tauri::command]
pub async fn close_screen_selector(
    app_handle: AppHandle,
    label: Option<String>,
) -> Result<(), String> {
    let window_label = label.unwrap_or_else(|| "screen-selector".to_string());

    if let Some(window) = app_handle.get_webview_window(&window_label) {
        // First try to hide the window immediately for better UX
        let _ = window.hide();
        
        // Then close it properly
        window.close().map_err(|e| e.to_string())?;
        log::info!("Screen selector window closed");
        Ok(())
    } else {
        log::warn!("Screen selector window not found when trying to close");
        Ok(()) // Don't error if window doesn't exist
    }
}

/// Process the selected screen region and extract text
#[tauri::command]
pub async fn process_screen_selection(
    app_handle: AppHandle,
    bounds: SelectionBounds,
) -> Result<ScreenSelectionResult, String> {
    log::info!("Processing screen selection: {:?}", bounds);

    // Validate bounds
    if bounds.width <= 0 || bounds.height <= 0 {
        return Err("Invalid selection bounds: width and height must be positive".to_string());
    }

    // Get text content using existing functions
    let text_content = get_screen_text_in_bounds_formatted(
        app_handle.clone(),
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
    ).await?;

    let raw_data = get_screen_text_in_bounds_raw(
        app_handle,
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
    ).await?;

    let result = ScreenSelectionResult {
        bounds: bounds.clone(),
        text_content,
        raw_data,
    };

    log::info!("Screen selection processed successfully. Text length: {}", result.text_content.len());

    Ok(result)
}

/// Get screen dimensions for the overlay
#[tauri::command]
pub async fn get_screen_dimensions(app_handle: AppHandle) -> Result<(u32, u32), String> {
    let monitors = app_handle.primary_monitor().map_err(|e| e.to_string())?;
    let monitor = monitors.ok_or("No primary monitor found")?;
    let size = monitor.size();
    
    Ok((size.width, size.height))
}

/// Convert client coordinates to screen coordinates
#[tauri::command]
pub async fn client_to_screen_coords(
    app_handle: AppHandle,
    window_label: String,
    x: f64,
    y: f64,
) -> Result<(i32, i32), String> {
    if let Some(window) = app_handle.get_webview_window(&window_label) {
        let position = window.outer_position().map_err(|e| e.to_string())?;
        let screen_x = position.x + x as i32;
        let screen_y = position.y + y as i32;
        
        Ok((screen_x, screen_y))
    } else {
        Err(format!("Window '{}' not found", window_label))
    }
}
