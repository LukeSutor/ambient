use tauri::{AppHandle, Manager};

/// Open or focus the floating HUD window.
/// If a window with the given label exists, it will be brought to front; otherwise it will be created.
#[tauri::command]
pub async fn open_floating_window(app_handle: AppHandle, label: Option<String>) -> Result<(), String> {
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
	.inner_size(500.0, 60.0)
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
pub async fn close_floating_window(app_handle: AppHandle, label: Option<String>) -> Result<(), String> {
	let window_label = label.unwrap_or_else(|| "floating-hud".to_string());

	if let Some(window) = app_handle.get_webview_window(&window_label) {
		window.close().map_err(|e| e.to_string())?;
		Ok(())
	} else {
		Err("Window not found".to_string())
	}
}
