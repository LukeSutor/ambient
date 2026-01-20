use tauri::Emitter;
use crate::auth::auth_flow::handle_oauth_callback;

/// Handle incoming deep link URLs (e.g., ambient://auth/callback?code=...)
/// Parses the URL and routes to appropriate auth flows, emitting success/error events.
pub fn handle_open_url(app_handle: &tauri::AppHandle, url: &str) {
  log::info!("[deep_link] Processing URL");

  if url.starts_with("ambient://auth/callback") {
    let app = app_handle.clone();
    let url_string = url.to_string();
    
    tauri::async_runtime::spawn(async move {
      match handle_oauth_callback(&url_string).await {
        Ok(result) => {
          log::info!("[deep_link] OAuth2 callback handled successfully");
          if let Err(e) = app.emit("oauth2-success", &result) {
            log::error!("[deep_link] Failed to emit oauth2-success event: {}", e);
          }
        }
        Err(e) => {
          log::error!("[deep_link] Failed to handle OAuth2 callback: {}", e);
          if let Err(emit_err) = app.emit("oauth2-error", &e) {
            log::error!(
              "[deep_link] Failed to emit oauth2-error event: {}",
              emit_err
            );
          }
        }
      }
    });
  }
}
