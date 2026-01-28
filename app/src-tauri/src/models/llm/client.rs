use super::providers::{
    local::LocalProvider, cloudflare::CloudflareProvider
};
use super::types::{LlmRequest, ProviderPolicy, LlmProvider, LlmResponse};
use tauri::AppHandle;

/// Unified generate function that routes to the selected provider.
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

    // Decide provider
    let provider_is_local = match policy {
        ProviderPolicy::ForceLocal => true,
        ProviderPolicy::Default => {
            // Read settings to decide
            let settings = crate::settings::service::load_user_settings(app_handle.clone())
                .await
                .map_err(|e| format!("Failed to load user settings: {}", e))?;
            matches!(
                settings.model_selection,
                crate::settings::types::ModelSelection::Local
            )
        }
    };

    if provider_is_local {
        let provider = LocalProvider;
        provider.generate(app_handle, request).await
    } else {
        let provider = CloudflareProvider;
        provider.generate(app_handle, request).await
    }
}
