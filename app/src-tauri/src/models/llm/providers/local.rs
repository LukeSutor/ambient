use super::LlmProvider;
use tauri::AppHandle;

pub struct LocalProvider;

#[async_trait::async_trait]
impl LlmProvider for LocalProvider {
    async fn generate(
        &self,
        app_handle: AppHandle,
        prompt: String,
        system_prompt: Option<String>,
        json_schema: Option<String>,
        conv_id: Option<String>,
        use_thinking: Option<bool>,
        stream: Option<bool>,
    ) -> Result<String, String> {
        // Delegate to the existing local server generate function
        crate::models::llm::server::generate(
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
