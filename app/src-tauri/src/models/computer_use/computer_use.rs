use tauri::{AppHandle, Manager};

/// Helpers


pub struct ComputerUseEngine {
    app_handle: AppHandle,
    width: i32,
    height: i32,
    cancel_loop: bool
}

impl ComputerUseEngine {
    fn new(app_handle: AppHandle) -> Self {
        // Get the screen's physical size to store it
        let mut width: i32 = 0;
        let mut height: i32 = 0;
        if let Some(window) = app_handle.get_webview_window("main") {
            if let Ok(Some(monitor)) = window.current_monitor() {
                let physical_size = monitor.size();
                width = physical_size.width as i32;
                height = physical_size.height as i32;
                
                // // Convert from 1000x1000 coordinate space to actual screen pixels
                // let actual_x = (x as f64 / 1000.0) * physical_size.width as f64;
                // let actual_y = (y as f64 / 1000.0) * physical_size.height as f64;
            }
        }
        if width == 0 || height == 0 {
            log::warn!("Failed to get screen dimensions, using defaults");
        }
        Self {
            app_handle: app_handle.clone(),
            width: width as i32,
            height: height as i32,
            cancel_loop: false
        }
    }



    fn normalize_coordinates(&self, x: i32, y: i32) -> (i32, i32) {
        let actual_x = (x as f64 / 1000.0) * self.width as f64;
        let actual_y = (y as f64 / 1000.0) * self.height as f64;
        (actual_x as i32, actual_y as i32)
    }
}