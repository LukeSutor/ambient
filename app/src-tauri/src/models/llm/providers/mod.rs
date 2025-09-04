use tauri::AppHandle;

/// Policy for choosing which provider to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderPolicy {
    Default,
    ForceLocal,
}

/// Common interface for LLM providers
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    async fn generate(
        &self,
        app_handle: AppHandle,
        prompt: String,
        system_prompt: Option<String>,
        json_schema: Option<String>,
        conv_id: Option<String>,
        use_thinking: Option<bool>,
        stream: Option<bool>,
    ) -> Result<String, String>;
}

pub mod local;
pub mod openrouter;
