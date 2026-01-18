use crate::models::llm::types::{LlmRequest, LlmProvider};
use tauri::AppHandle;

pub struct LocalProvider;

#[async_trait::async_trait]
impl LlmProvider for LocalProvider {
  async fn generate(
    &self,
    app_handle: AppHandle,
    request: LlmRequest,
  ) -> Result<String, String> {
    // Delegate to the existing local server generate function
    crate::models::llm::server::generate(app_handle, request).await
  }
}
