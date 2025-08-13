use crate::auth::commands::google_handle_callback;
use tauri::Emitter;

/// Handle incoming deep link URLs (e.g., cortical://auth/callback?code=...)
/// Parses the URL and routes to appropriate auth flows, emitting success/error events.
pub fn handle_open_url(app_handle: &tauri::AppHandle, url: &str) {
  log::info!("[deep_link] Processing URL: {}", url);

  if url.starts_with("cortical://auth/callback") {
    if let Ok(parsed) = url::Url::parse(url) {
      let query_pairs: std::collections::HashMap<String, String> = parsed
        .query_pairs()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

      if let Some(code) = query_pairs.get("code").cloned() {
        let app = app_handle.clone();
        tauri::async_runtime::spawn(async move {
          match google_handle_callback(code).await {
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
      } else if let Some(error) = query_pairs.get("error").cloned() {
        let error_description = query_pairs.get("error_description").cloned();
        let error_msg = format!(
          "OAuth2 error: {} - {}",
          error,
          error_description.unwrap_or_default()
        );
        log::error!("[deep_link] {}", error_msg);
        if let Err(e) = app_handle.emit("oauth2-error", &error_msg) {
          log::error!("[deep_link] Failed to emit oauth2-error event: {}", e);
        }
      }
    }
  }
}
