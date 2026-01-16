use tauri::AppHandle;
use serde::{Deserialize, Serialize};

/// Policy for choosing which provider to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderPolicy {
  Default,
  ForceLocal,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmRequest {
  pub prompt: String,
  pub system_prompt: Option<String>,
  pub json_schema: Option<String>,
  pub conv_id: Option<String>,
  pub use_thinking: Option<bool>,
  pub stream: Option<bool>,
  pub current_message_id: Option<String>,
}

impl LlmRequest {
  pub fn new(prompt: String) -> Self {
    Self {
      prompt,
      ..Default::default()
    }
  }

  pub fn with_system_prompt(mut self, system_prompt: Option<String>) -> Self {
    self.system_prompt = system_prompt;
    self
  }

  pub fn with_json_schema(mut self, json_schema: Option<String>) -> Self {
    self.json_schema = json_schema;
    self
  }

  pub fn with_conv_id(mut self, conv_id: Option<String>) -> Self {
    self.conv_id = conv_id;
    self
  }

  pub fn with_use_thinking(mut self, use_thinking: Option<bool>) -> Self {
    self.use_thinking = use_thinking;
    self
  }

  pub fn with_stream(mut self, stream: Option<bool>) -> Self {
    self.stream = stream;
    self
  }

  pub fn with_current_message_id(mut self, current_message_id: Option<String>) -> Self {
    self.current_message_id = current_message_id;
    self
  }
}

/// Common interface for LLM providers
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
  async fn generate(
    &self,
    app_handle: AppHandle,
    request: LlmRequest,
  ) -> Result<String, String>;
}
