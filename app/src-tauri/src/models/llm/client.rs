use super::providers::{
    local::LocalProvider, cloudflare::CloudflareProvider
};
use super::types::{LlmRequest, ProviderPolicy, LlmProvider, LlmResponse};
use std::sync::Arc;
use tokio::sync::Notify;
use tauri::AppHandle;

/// Unified generate function that routes to the selected provider.
///
/// Supports instant cancellation via `cancel_notify` on the request:
/// when signalled, the provider's future is dropped, immediately closing
/// the HTTP connection and stopping server-side generation.
pub async fn generate(
    app_handle: AppHandle,
    mut request: LlmRequest,
    force_local: Option<bool>,
) -> Result<LlmResponse, String> {
    let policy = if force_local.unwrap_or(false) {
        ProviderPolicy::ForceLocal
    } else {
        ProviderPolicy::Default
    };

    // Decide provider and resolve model type
    let provider_is_local = match policy {
        ProviderPolicy::ForceLocal => true,
        ProviderPolicy::Default => {
            let settings = crate::settings::service::load_user_settings(app_handle.clone())
                .await
                .map_err(|e| format!("Failed to load user settings: {}", e))?;

            let is_local = matches!(
                settings.model_selection,
                crate::settings::types::ModelSelection::Local
            );

            if !is_local && request.model_type.is_none() {
                request.model_type = Some(settings.model_selection.as_str().to_string());
            }

            is_local
        }
    };

    // Extract cancel_notify before the retry loop (it's not cloneable via LlmRequest::clone
    // since Notify doesn't impl Clone via the default derive — we handle it explicitly)
    let cancel_notify = request.cancel_notify.clone();

    let max_attempts = request.max_attempts.unwrap_or(1);
    let timeout_duration = request.timeout_duration.map(std::time::Duration::from_secs);
    let mut attempts = 0;
    let mut last_error = String::new();

    while attempts < max_attempts {
        attempts += 1;
        log::info!("[llm_client] Generation attempt {}/{}", attempts, max_attempts);

        let attempt_request = request.clone();
        let app_handle_clone = app_handle.clone();

        let gen_future = async move {
            if provider_is_local {
                let provider = LocalProvider;
                provider.generate(app_handle_clone, attempt_request).await
            } else {
                let provider = CloudflareProvider;
                provider.generate(app_handle_clone, attempt_request).await
            }
        };

        let result = run_with_cancellation(
            gen_future,
            timeout_duration,
            cancel_notify.clone(),
        ).await;

        match result {
            Ok(resp) => return Ok(resp),
            Err(e) => {
                last_error = e.clone();
                if e.contains("cancelled") {
                    return Err(e);
                }
                log::warn!("[llm_client] Attempt {} failed: {}", attempts, e);
                
                if attempts < max_attempts {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    Err(format!("LLM generation failed after {} attempts. Last error: {}", max_attempts, last_error))
}

/// Run a generation future with optional timeout and instant cancellation.
///
/// When `cancel_notify` fires, the `gen_future` is dropped. For streaming
/// providers this drops the `reqwest::Response`, which closes the HTTP
/// connection — causing llama.cpp / Cloudflare to stop generation immediately.
async fn run_with_cancellation<F>(
    gen_future: F,
    timeout_duration: Option<std::time::Duration>,
    cancel_notify: Option<Arc<Notify>>,
) -> Result<LlmResponse, String>
where
    F: std::future::Future<Output = Result<LlmResponse, String>>,
{
    match (timeout_duration, cancel_notify) {
        // Both timeout and cancel notification available
        (Some(duration), Some(notify)) => {
            tokio::select! {
                biased;
                // Cancel branch checked first (biased select)
                _ = notify.notified() => {
                    log::info!("[llm_client] Generation cancelled via notify");
                    Err("Request cancelled".to_string())
                }
                result = tokio::time::timeout(duration, gen_future) => {
                    match result {
                        Ok(res) => res,
                        Err(_) => Err("Request timed out".to_string()),
                    }
                }
            }
        }
        // Cancel notification only (no timeout)
        (None, Some(notify)) => {
            tokio::select! {
                biased;
                _ = notify.notified() => {
                    log::info!("[llm_client] Generation cancelled via notify");
                    Err("Request cancelled".to_string())
                }
                result = gen_future => result,
            }
        }
        // Timeout only (no cancel notification)
        (Some(duration), None) => {
            match tokio::time::timeout(duration, gen_future).await {
                Ok(res) => res,
                Err(_) => Err("Request timed out".to_string()),
            }
        }
        // No timeout, no cancel
        (None, None) => gen_future.await,
    }
}
