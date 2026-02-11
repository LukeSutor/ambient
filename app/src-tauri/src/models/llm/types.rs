use tauri::AppHandle;
use crate::skills::types::ToolDefinition;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::Notify;

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
    /// ID for the assistant message being generated (for streaming)
    pub assistant_message_id: Option<String>,
    /// Override model type (e.g., "computer-use" for computer use sessions)
    pub model_type: Option<String>,
    /// Maximum number of attempts for generation
    pub max_attempts: Option<usize>,
    /// Timeout duration in seconds for each attempt
    pub timeout_duration: Option<u64>,
    /// Cancellation signal for aborting generation (not serialized)
    #[serde(skip)]
    pub cancel_signal: Option<Arc<AtomicBool>>,
    /// Async cancel notification for instant I/O cancellation (not serialized)
    #[serde(skip)]
    pub cancel_notify: Option<Arc<Notify>>,
    /// Pin this request to a specific llama.cpp server slot for KV cache isolation.
    /// Slot 0 is reserved for agentic chat; slot 1+ for background tasks.
    pub slot_id: Option<i32>,
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

    pub fn with_assistant_message_id(mut self, assistant_message_id: Option<String>) -> Self {
        self.assistant_message_id = assistant_message_id;
        self
    }

    pub fn with_cancel_signal(mut self, signal: Option<Arc<AtomicBool>>) -> Self {
        self.cancel_signal = signal;
        self
    }

    pub fn with_cancel_notify(mut self, notify: Option<Arc<Notify>>) -> Self {
        self.cancel_notify = notify;
        self
    }

    pub fn with_model_type(mut self, model_type: Option<String>) -> Self {
        self.model_type = model_type;
        self
    }

    pub fn with_attempts(mut self, attempts: Option<usize>) -> Self {
        self.max_attempts = attempts;
        self
    }

    pub fn with_timeout_duration(mut self, duration: Option<u64>) -> Self {
        self.timeout_duration = duration;
        self
    }

    pub fn with_slot_id(mut self, slot_id: Option<i32>) -> Self {
        self.slot_id = slot_id;
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
    /// Tool calls to execute, with optional accompanying text (reasoning/thinking)
    ToolCalls {
        /// Optional text generated alongside tool calls (e.g., reasoning)
        text: Option<String>,
        /// The tool calls requested by the model
        calls: Vec<crate::skills::types::ToolCall>,
    },
}

impl LlmResponse {
    /// Create a text-only response
    pub fn text(content: String) -> Self {
        LlmResponse::Text(content)
    }

    /// Create a tool calls response with optional text
    pub fn tool_calls(calls: Vec<crate::skills::types::ToolCall>, text: Option<String>) -> Self {
        LlmResponse::ToolCalls { text, calls }
    }

    /// Check if this response contains tool calls
    pub fn has_tool_calls(&self) -> bool {
        matches!(self, LlmResponse::ToolCalls { .. })
    }

    /// Get text content if present (from either variant)
    pub fn get_text(&self) -> Option<&str> {
        match self {
            LlmResponse::Text(s) => Some(s),
            LlmResponse::ToolCalls { text, .. } => text.as_deref(),
        }
    }

    /// Get tool calls if present
    pub fn get_tool_calls(&self) -> Option<&[crate::skills::types::ToolCall]> {
        match self {
            LlmResponse::Text(_) => None,
            LlmResponse::ToolCalls { calls, .. } => Some(calls),
        }
    }
}
