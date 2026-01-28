use super::providers::{
    local::LocalProvider, cloudflare::CloudflareProvider
};
use super::types::{LlmRequest, ProviderPolicy, LlmProvider, AgentRequest, AgentResponse};
use tauri::AppHandle;

/// Unified generate function that routes to the selected provider.
pub async fn generate(
    app_handle: AppHandle,
    request: LlmRequest,
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

/// Generate with tools support for agentic runtime.
///
/// This function is specifically designed for the agentic loop,
/// handling both tool-based and text responses from the LLM.
///
/// # Arguments
///
/// * `app_handle` - Tauri app handle
/// * `request` - AgentRequest containing system prompt, messages, and tools
/// * `provider_type` - Either Local or Cloudflare provider
///
/// # Returns
///
/// `AgentResponse` containing either text, skill activation, or tool calls
pub async fn generate_with_tools(
    app_handle: AppHandle,
    request: AgentRequest,
    provider_type: crate::skills::types::ProviderType,
) -> Result<AgentResponse, String> {
    use super::providers::translation::{
        tools_to_openai_format, tools_to_gemini_format,
        has_tool_calls_openai, has_tool_calls_gemini,
        parse_openai_tool_calls, parse_gemini_tool_calls,
        format_openai_tool_results, format_gemini_tool_results,
        extract_text_openai, extract_text_gemini,
    };

    match provider_type {
        crate::skills::types::ProviderType::Local => {
            // Local provider uses OpenAI-compatible format
            let openai_tools = tools_to_openai_format(&request.tools);

            // Build request with tools
            let llm_request = LlmRequest::new(String::new()) // Prompt from messages
                .with_system_prompt(Some(request.system_prompt.clone()))
                .with_tools(Some(serde_json::json!(openai_tools)))
                .with_conv_id(request.conv_id.clone())
                .with_stream(Some(request.stream))
                .with_current_message_id(request.current_message_id.clone());

            // Build OpenAI-format messages from our message format
            let openai_messages = build_openai_messages(&request.messages);

            let provider = LocalProvider;
            let response = provider.generate_with_tools(
                app_handle.clone(),
                llm_request,
                openai_messages,
            )
            .await?;

            // Parse response
            if has_tool_calls_openai(&response) {
                Ok(AgentResponse::ToolCalls(
                    parse_openai_tool_calls(&response)
                ))
            } else {
                Ok(extract_text_openai(&response)
                    .map(AgentResponse::Text)
                    .unwrap_or_else(|| {
                        AgentResponse::Text("I understand your request but I'm having trouble expressing my response.".to_string())
                    }))
            }
        }

        crate::skills::types::ProviderType::Cloudflare => {
            // Cloud provider uses Gemini format
            let gemini_tools = tools_to_gemini_format(&request.tools);

            // Build request with tools
            let llm_request = LlmRequest::new(String::new()) // Prompt from messages
                .with_system_prompt(Some(request.system_prompt.clone()))
                .with_tools(Some(gemini_tools))
                .with_conv_id(request.conv_id.clone())
                .with_stream(Some(request.stream))
                .with_current_message_id(request.current_message_id.clone());

            // Build Gemini-format messages from our message format
            let gemini_content = build_gemini_content(&request.messages);

            let provider = CloudflareProvider;
            let response = provider.generate_with_tools(
                app_handle.clone(),
                llm_request,
                gemini_content,
            )
            .await?;

            // Parse response
            if has_tool_calls_gemini(&response) {
                Ok(AgentResponse::ToolCalls(
                    parse_gemini_tool_calls(&response)
                ))
            } else {
                Ok(extract_text_gemini(&response)
                    .map(AgentResponse::Text)
                    .unwrap_or_else(|| {
                        AgentResponse::Text("I understand your request but I'm having trouble expressing my response.".to_string())
                    }))
            }
        }
    }
}

/// Build OpenAI-format messages from our Message format.
///
/// Converts our unified message format to OpenAI's role/content format.
fn build_openai_messages(messages: &[crate::db::conversations::Message]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                crate::db::conversations::Role::System => "system",
                crate::db::conversations::Role::User => "user",
                crate::db::conversations::Role::Assistant => "assistant",
                crate::db::conversations::Role::Tool => "tool",
            };

            serde_json::json!({
                "role": role,
                "content": msg.content,
            })
        })
        .collect()
}

/// Build Gemini-format content from our Message format.
///
/// Converts our unified message format to Gemini's role/parts format.
fn build_gemini_content(messages: &[crate::db::conversations::Message]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|msg| {
            let role = match msg.role {
                crate::db::conversations::Role::System => "model",
                crate::db::conversations::Role::User => "user",
                crate::db::conversations::Role::Assistant => "model",
                crate::db::conversations::Role::Tool => "user",
            };

            serde_json::json!({
                "role": role,
                "parts": [{"text": msg.content}],
            })
        })
        .collect()
}

// ============================================================================
// Provider Extensions
// ============================================================================

/// Extension trait for OpenAI-compatible providers.
///
/// Allows providers to implement tool support in addition
/// to the basic text generation.
#[async_trait::async_trait]
pub trait ToolEnabledProvider: LlmProvider {
    /// Generate response with tool definitions.
    ///
    /// This should handle both text and tool-based responses,
    /// returning raw JSON that can be parsed by the runtime.
    async fn generate_with_tools(
        &self,
        app_handle: AppHandle,
        request: LlmRequest,
        messages: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, String>;
}

/// Implement tool-enabled provider for CloudflareProvider.
#[async_trait::async_trait]
impl ToolEnabledProvider for CloudflareProvider {
    async fn generate_with_tools(
        &self,
        app_handle: AppHandle,
        request: LlmRequest,
        _messages: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        self.generate(app_handle, request).await
            .map(|text| serde_json::json!({
                "candidates": [{
                    "content": {
                        "parts": [{
                            "text": text,
                            "functionCall": null
                        }]
                    }
                }]
            }))
    }
}
