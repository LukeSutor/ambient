use crate::models::ocr::ocr::{process_image_from_file, OcrResult};
use crate::events::{emitter::emit, types::{OCR_RESPONSE, OcrResponseEvent}};
use crate::images::{take_screenshot, crop_image_selection};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SelectionBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
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
) -> Result<(), String> {
    log::info!("Processing screen selection: {:?}", bounds);

    // Validate bounds
    if bounds.width <= 0 || bounds.height <= 0 {
        // Emit failed event
        log::warn!("Invalid screen selection bounds: {:?}", bounds);
        let result = OcrResponseEvent {
            text: String::new(),
            success: false,
            timestamp: chrono::Utc::now().to_string(),
        };
        emit(OCR_RESPONSE, result)
            .map_err(|e| format!("Failed to emit OCR response: {}", e))?;
        return Ok(());
    }
    
    // Take screenshot and crop
    let filename = Uuid::new_v4().to_string() + ".png";
    let screenshot_path = take_screenshot(app_handle.clone(), filename.clone());
    let pathbuf = std::path::PathBuf::from(screenshot_path.clone());
    crop_image_selection(pathbuf.clone(), bounds);
    log::info!("Cropped screenshot saved to: {}", screenshot_path);

    // Get ocr result
    let result: OcrResult = process_image_from_file(app_handle.clone(), pathbuf.to_str().unwrap().to_string()).await
        .map_err(|e| format!("Failed to process OCR: {}", e))?;

    // Delete temporary screenshot file
    std::fs::remove_file(pathbuf).map_err(|e| format!("Failed to delete temporary screenshot file: {}", e))?;

    // Emit event
    let result = OcrResponseEvent {
        text: result.text,
        success: true,
        timestamp: chrono::Utc::now().to_string(),
    };
    emit(OCR_RESPONSE, result.clone())
        .map_err(|e| format!("Failed to emit OCR response: {}", e))?;

    Ok(())
}

/// Return an unsuccessful OCR result
#[tauri::command]
pub async fn cancel_screen_selection(
    app_handle: AppHandle,
) -> Result<(), String> {
    log::info!("Screen selection cancelled by user");

    // Emit failed event
    let result = OcrResponseEvent {
        text: String::new(),
        success: false,
        timestamp: chrono::Utc::now().to_string(),
    };
    emit(OCR_RESPONSE, result)
        .map_err(|e| format!("Failed to emit OCR response: {}", e))?;

    Ok(())
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
