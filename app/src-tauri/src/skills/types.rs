//! Core types for the agentic skills runtime.
//!
//! This module defines the data structures used throughout the skills system,
//! including skill definitions, tool specifications, execution requests/responses,
//! and runtime configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Skill summary for progressive disclosure (Phase 1).
///
/// Only the name and description are sent to the model initially,
/// minimizing context overhead. Full tool definitions are loaded
/// just-in-time when a skill is activated.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "skills.ts")]
pub struct SkillSummary {
    /// The unique identifier for this skill (e.g., "web-search", "memory-search")
    pub name: String,
    /// A brief description of what this skill does.
    /// This is shown to the LLM to help it decide when to activate the skill.
    pub description: String,
}

/// Full skill definition including all tools and instructions.
///
/// This contains the complete skill metadata loaded from a SKILL.md file,
/// including the tools that become available when the skill is activated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// The unique identifier for this skill.
    pub name: String,
    /// A brief description of what this skill does.
    pub description: String,
    /// The version of the skill specification.
    pub version: String,
    /// Whether this skill requires authentication/authorization to use.
    pub requires_auth: bool,
    /// The tools provided by this skill.
    pub tools: Vec<ToolDefinition>,
    /// Markdown-formatted instructions for using this skill,
    /// from the body of the SKILL.md file.
    pub instructions: String,
}

impl Skill {
    /// Creates a summary of this skill for progressive disclosure.
    pub fn to_summary(&self) -> SkillSummary {
        SkillSummary {
            name: self.name.clone(),
            description: self.description.clone(),
        }
    }
}

/// Tool definition in the unified internal format.
///
/// This is our canonical representation of a tool, which gets
/// translated to provider-specific formats (OpenAI, Gemini) at runtime.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "skills.ts")]
pub struct ToolDefinition {
    /// The unique name of this tool (within its skill).
    pub name: String,
    /// A description of what this tool does, helping the LLM decide when to use it.
    pub description: String,
    /// The parameters this tool accepts.
    pub parameters: Vec<ToolParameter>,
    /// Optional schema describing the return type of this tool.
    pub returns: Option<ToolReturnType>,
}

/// A parameter definition for a tool.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "skills.ts")]
pub struct ToolParameter {
    /// The parameter name.
    pub name: String,
    /// The type of this parameter.
    #[serde(rename = "type")]
    pub param_type: ParameterType,
    /// A description of this parameter.
    pub description: String,
    /// Whether this parameter is required.
    pub required: bool,
    /// An optional default value for this parameter.
    #[ts(type = "any")]
    pub default: Option<serde_json::Value>,
}

/// The data type of a tool parameter.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "skills.ts")]
#[serde(rename_all = "lowercase")]
pub enum ParameterType {
    /// A UTF-8 encoded string.
    String,
    /// A signed 64-bit integer.
    Integer,
    /// A floating-point number.
    Number,
    /// A boolean value (true or false).
    Boolean,
    /// A JSON array of values.
    Array,
    /// A JSON object (key-value pairs).
    Object,
}

impl ParameterType {
    /// Returns the JSON Schema type name for this parameter type.
    pub fn as_json_schema(&self) -> &'static str {
        match self {
            ParameterType::String => "string",
            ParameterType::Integer => "integer",
            ParameterType::Number => "number",
            ParameterType::Boolean => "boolean",
            ParameterType::Array => "array",
            ParameterType::Object => "object",
        }
    }

    /// Returns the Gemini-compatible type name for this parameter type.
    pub fn as_gemini_type(&self) -> &'static str {
        match self {
            ParameterType::String => "STRING",
            ParameterType::Integer => "INTEGER",
            ParameterType::Number => "NUMBER",
            ParameterType::Boolean => "BOOLEAN",
            ParameterType::Array => "ARRAY",
            ParameterType::Object => "OBJECT",
        }
    }
}

/// Schema describing the return type of a tool.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "skills.ts")]
pub struct ToolReturnType {
    /// The type identifier (e.g., "array", "object", "string").
    #[serde(rename = "type")]
    pub return_type: String,
    /// For object types, the properties of the object.
    #[ts(type = "any")]
    pub properties: Option<HashMap<String, serde_json::Value>>,
    /// For array types, the schema of the array items.
    pub items: Option<Box<ToolReturnType>>,
}

/// A tool call requested by the model.
///
/// This represents a single invocation request that the agent runtime
/// needs to execute and return results for.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "skills.ts")]
pub struct ToolCall {
    /// A unique identifier for this tool call, used to match results.
    pub id: String,
    /// The name of the skill this tool belongs to.
    pub skill_name: String,
    /// The specific tool to invoke within the skill.
    pub tool_name: String,
    /// The arguments to pass to the tool, as JSON.
    #[ts(type = "any")]
    pub arguments: serde_json::Value,
}

impl ToolCall {
    /// Creates a new tool call with a generated UUID as the call ID.
    pub fn new(skill_name: String, tool_name: String, arguments: serde_json::Value) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            skill_name,
            tool_name,
            arguments,
        }
    }

    /// Returns the full qualified tool name (skill.tool format).
    pub fn qualified_name(&self) -> String {
        format!("{}.{}", self.skill_name, self.tool_name)
    }
}

/// Result from executing a tool.
///
/// This contains the outcome of a tool execution, either
/// a successful result or an error message.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "skills.ts")]
pub struct ToolResult {
    /// The ID of the tool call this result corresponds to.
    pub call_id: String,
    /// Whether the tool execution succeeded.
    pub success: bool,
    /// The successful result value (as JSON), if successful.
    #[ts(type = "any")]
    pub result: Option<serde_json::Value>,
    /// An error message, if the execution failed.
    pub error: Option<String>,
}

impl ToolResult {
    /// Creates a successful tool result.
    pub fn success(call_id: String, result: serde_json::Value) -> Self {
        Self {
            call_id,
            success: true,
            result: Some(result),
            error: None,
        }
    }

    /// Creates a failed tool result.
    pub fn error(call_id: String, error: String) -> Self {
        Self {
            call_id,
            success: false,
            result: None,
            error: Some(error),
        }
    }
}

/// A skill activation request from the model.
///
/// This represents the model's request to activate a skill before
/// using its tools. Activation loads the skill's full tool definitions.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "skills.ts")]
pub struct SkillActivationRequest {
    /// The name of the skill to activate.
    pub skill_name: String,
    /// The model's explanation for why this skill is needed.
    pub reason: String,
}

/// Configuration for the agentic runtime.
///
/// These settings control the behavior of the agent loop,
/// including context limits and execution boundaries.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "settings.ts")]
pub struct AgentRuntimeConfig {
    /// Maximum number of conversation history messages to include for local models.
    ///
    /// Local models typically have smaller context windows, so we limit
    /// history more aggressively.
    pub local_context_limit: usize,

    /// Maximum number of conversation history messages to include for cloud models.
    ///
    /// Cloud models typically have larger context windows.
    pub cloud_context_limit: usize,

    /// Maximum number of tool calls that can be made in a single turn.
    ///
    /// This prevents infinite loops and excessive tool usage.
    pub max_tool_calls_per_turn: usize,

    /// Maximum total iterations in the agentic loop before giving up.
    ///
    /// Each iteration represents a round of (model request -> response -> tool execution).
    pub max_iterations: usize,

    /// Whether to emit thinking/reasoning messages to the frontend.
    ///
    /// When enabled, the model's internal reasoning is shown as
    /// separate "thinking" messages in the conversation.
    pub enable_thinking: bool,
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            local_context_limit: 3,
            cloud_context_limit: 10,
            max_tool_calls_per_turn: 5,
            max_iterations: 10,
            enable_thinking: true,
        }
    }
}

impl AgentRuntimeConfig {
    /// Returns the context limit for the given provider type.
    pub fn context_limit_for(&self, is_local: bool) -> usize {
        if is_local {
            self.local_context_limit
        } else {
            self.cloud_context_limit
        }
    }
}

/// Model provider type for tool format translation.
///
/// Different providers use different function calling formats,
/// so we need to translate between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderType {
    /// Local model using OpenAI-compatible format.
    ///
    /// The local llama.cpp server exposes an OpenAI-compatible API.
    Local,

    /// Cloud model via Cloudflare Worker using Gemini format.
    ///
    /// The Cloudflare Worker forwards requests to Google's Gemini API.
    Cloudflare,
}

impl ProviderType {
    /// Determines the provider type based on the model selection string.
    pub fn from_model_str(model: &str) -> Self {
        match model {
            "local" => ProviderType::Local,
            _ => ProviderType::Cloudflare,
        }
    }
}

/// Error types specific to the agentic runtime.
#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(tag = "type", content = "message")]
pub enum AgentError {
    /// No skill with the given name was found.
    #[serde(rename = "SkillNotFound")]
    #[error("Skill not found: {0}")]
    SkillNotFound(String),

    /// No tool with the given name was found in the specified skill.
    #[serde(rename = "ToolNotFound")]
    #[error("Tool not found: {0} in skill {1}")]
    ToolNotFound(String, String),

    /// The maximum number of iterations was exceeded.
    #[serde(rename = "MaxIterationsExceeded")]
    #[error("Maximum iterations ({0}) exceeded")]
    MaxIterationsExceeded(usize),

    /// A tool execution failed.
    #[serde(rename = "ToolExecutionFailed")]
    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    /// Failed to parse a skill definition file.
    #[serde(rename = "SkillParseError")]
    #[error("Failed to parse skill file: {0}")]
    SkillParseError(String),

    /// LLM generation failed.
    #[serde(rename = "LlmError")]
    #[error("LLM error: {0}")]
    LlmError(String),

    /// Database operation failed.
    #[serde(rename = "DatabaseError")]
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Invalid tool arguments.
    #[serde(rename = "InvalidArguments")]
    #[error("Invalid tool arguments: {0}")]
    InvalidArguments(String),

    /// The skill registry is not initialized.
    #[serde(rename = "RegistryNotInitialized")]
    #[error("Skill registry not initialized")]
    RegistryNotInitialized,

    /// Too many tool calls in a single turn.
    #[serde(rename = "TooManyToolCalls")]
    #[error("Too many tool calls: {0} exceeds max of {1}")]
    TooManyToolCalls(usize, usize),
}

/// Result type alias for agentic runtime operations.
pub type AgentResult<T> = Result<T, AgentError>;

impl From<String> for AgentError {
    fn from(s: String) -> Self {
        AgentError::DatabaseError(s)
    }
}
