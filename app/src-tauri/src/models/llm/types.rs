use tauri::AppHandle;
use crate::skills::types::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

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
    pub internal_tools: Option<Vec<ToolDefinition>>,
    pub messages: Option<Vec<crate::db::conversations::Message>>,
    /// Cancellation signal for aborting generation (not serialized)
    #[serde(skip)]
    pub cancel_signal: Option<Arc<AtomicBool>>,
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

    pub fn with_internal_tools(mut self, tools: Option<Vec<ToolDefinition>>) -> Self {
        self.internal_tools = tools;
        self
    }

    pub fn with_messages(mut self, messages: Option<Vec<crate::db::conversations::Message>>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_cancel_signal(mut self, signal: Option<Arc<AtomicBool>>) -> Self {
        self.cancel_signal = signal;
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
    ) -> Result<LlmResponse, String>;
}

/// Response variants from generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmResponse {
    /// Final text response
    Text(String),
    /// Tool calls to execute
    ToolCalls(Vec<crate::skills::types::ToolCall>),
}
