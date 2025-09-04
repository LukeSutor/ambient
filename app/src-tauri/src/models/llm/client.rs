use super::providers::{local::LocalProvider, openrouter::OpenRouterProvider, LlmProvider, ProviderPolicy};
use tauri::AppHandle;

/// Unified generate function that routes to the selected provider.
pub async fn generate(
    app_handle: AppHandle,
    prompt: String,
    system_prompt: Option<String>,
    json_schema: Option<String>,
    conv_id: Option<String>,
    use_thinking: Option<bool>,
    stream: Option<bool>,
    force_local: Option<bool>,
) -> Result<String, String> {
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
            matches!(settings.model_selection, crate::settings::types::ModelSelection::Local)
        }
    };

    if provider_is_local {
        let provider = LocalProvider;
        provider
            .generate(
                app_handle,
                prompt,
                system_prompt,
                json_schema,
                conv_id,
                use_thinking,
                stream,
            )
            .await
    } else {
        let provider = OpenRouterProvider;
        provider
            .generate(
                app_handle,
                prompt,
                system_prompt,
                json_schema,
                conv_id,
                use_thinking,
                stream,
            )
            .await
    }
}
