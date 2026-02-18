use super::providers::{
    local::LocalProvider, cloudflare::CloudflareProvider,
    openai::OpenAIProvider, google::GoogleProvider, anthropic::AnthropicProvider,
};
use super::types::{LlmRequest, ProviderPolicy, LlmProvider, LlmResponse};
use std::sync::Arc;
use tokio::sync::Notify;
use tauri::AppHandle;

/// Resolved model information used for routing and API calls.
#[derive(Clone)]
pub struct ResolvedModel {
    /// Database primary key.
    pub id: i64,
    /// The model identifier sent in API requests (e.g. "qwen3vl-2b", "gpt-4o").
    pub model: String,
    /// The model provider (e.g. "local", "google").
    pub provider: String,
    /// Whether this model uses a cloud provider.
    pub is_cloud: bool,
    /// Whether this is a built-in model vs a user-added BYOK model.
    pub is_internal: bool,
    /// API endpoint URL for BYOK models.
    pub api_url: Option<String>,
    /// API key for BYOK models.
    pub api_key: Option<String>,
    /// Request format: "openai", "gemini", or "anthropic".
    pub request_format: String,
}

/// Unified generate function that routes to the selected provider.
///
/// Supports instant cancellation via `cancel_notify` on the request:
/// when signalled, the provider's future is dropped, immediately closing
/// the HTTP connection and stopping server-side generation.
pub async fn generate(
    app_handle: AppHandle,
    request: LlmRequest,
    force_local: Option<bool>,
) -> Result<LlmResponse, String> {
    let policy = if force_local.unwrap_or(false) {
        ProviderPolicy::ForceLocal
    } else {
        ProviderPolicy::Default
    };

    // Resolve the model to use
    let resolved = resolve_model(&app_handle, &policy, request.override_model_id).await?;

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
        let resolved_clone = resolved.clone();

        let gen_future = async move {
            if resolved_clone.is_internal {
                // Internal models: local llama.cpp server or Cloudflare proxy
                if !resolved_clone.is_cloud {
                    let provider = LocalProvider;
                    provider.generate(app_handle_clone, attempt_request, &resolved_clone).await
                } else {
                    let provider = CloudflareProvider;
                    provider.generate(app_handle_clone, attempt_request, &resolved_clone).await
                }
            } else {
                // BYOK models: route based on request_format
                match resolved_clone.request_format.as_str() {
                    "openai" => {
                        let provider = OpenAIProvider;
                        provider.generate(app_handle_clone, attempt_request, &resolved_clone).await
                    }
                    "gemini" => {
                        let provider = GoogleProvider;
                        provider.generate(app_handle_clone, attempt_request, &resolved_clone).await
                    }
                    "anthropic" => {
                        let provider = AnthropicProvider;
                        provider.generate(app_handle_clone, attempt_request, &resolved_clone).await
                    }
                    other => Err(format!("Unknown request format: {}", other)),
                }
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
                // Don't retry on cancellation or rate limit errors
                if e.contains("cancelled") {
                    return Err(e);
                }
                if e.contains("rate_limit_exceeded") || e.contains("session_invalid") || e.contains("model_not_available") {
                    log::warn!("[llm_client] Non-retryable error: {}", e);
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

/// Resolve which model to use based on the provider policy and user settings.
/// Looks up model metadata from the database for routing.
async fn resolve_model(
    app_handle: &AppHandle,
    policy: &ProviderPolicy,
    override_model_id: Option<i64>,
) -> Result<ResolvedModel, String> {
    let fallback = || ResolvedModel {
        id: 1,
        model: "qwen3vl-2b".to_string(),
        provider: "local".to_string(),
        is_cloud: false,
        is_internal: true,
        api_url: None,
        api_key: None,
        request_format: "openai".to_string(),
    };

    let from_entry = |entry: crate::db::models::ModelEntry| ResolvedModel {
        id: entry.id,
        model: entry.model,
        provider: entry.provider,
        is_cloud: entry.is_cloud,
        is_internal: entry.is_internal,
        api_url: entry.api_url,
        api_key: entry.api_key,
        request_format: entry.request_format,
    };

    // If an explicit model override is requested, use it first.
    if let Some(id) = override_model_id {
        match crate::db::models::get_model_by_id(app_handle, id) {
            Ok(entry) => {
                log::info!("[llm_client] Using override model id={} ({})", id, entry.model);
                return Ok(from_entry(entry));
            }
            Err(e) => {
                log::warn!("[llm_client] Override model id {} not found: {}. Falling back to policy.", id, e);
            }
        }
    }

    match policy {
        ProviderPolicy::ForceLocal => {
            // When forced local, always use the default local model (id=1).
            match crate::db::models::get_model_by_id(app_handle, 1) {
                Ok(entry) => Ok(from_entry(entry)),
                Err(_) => Ok(fallback()),
            }
        }
        ProviderPolicy::Default => {
            let settings = crate::settings::service::load_user_settings(app_handle.clone())
                .await
                .map_err(|e| format!("Failed to load user settings: {}", e))?;

            let model_id: i64 = settings
                .model_selection
                .as_str()
                .parse()
                .unwrap_or(1);

            match crate::db::models::get_model_by_id(app_handle, model_id) {
                Ok(entry) => Ok(from_entry(entry)),
                Err(e) => {
                    log::warn!(
                        "[llm_client] Model id {} not found in registry, falling back to local: {}",
                        model_id, e
                    );
                    Ok(fallback())
                }
            }
        }
    }
}
