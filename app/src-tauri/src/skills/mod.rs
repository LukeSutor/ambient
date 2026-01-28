//! Agentic skills module for dynamic capability discovery and execution.
//!
//! This module implements the skills system that allows LLMs to
//! dynamically discover and use capabilities through progressive disclosure.
//!
//! # Architecture
//!
//! - **types**: Core type definitions for skills, tools, and runtime config
//! - **registry**: Loading and management of available skills from SKILL.md files
//! - **executor**: Parallel tool execution engine that routes to skill implementations
//! - **builtin**: Bundled skill implementations (web-search, memory-search, etc.)
//!
//! # Key Features
//!
//! - **Progressive Disclosure**: Only skill summaries are initially sent to LLM,
//!   full tool definitions are loaded just-in-time when a skill is activated.
//! - **Unified Tool Format**: Single internal format that translates to provider-specific
//!   formats (OpenAI for local, Gemini for cloud).
//! - **Parallel Execution**: Multiple tools can be executed concurrently.
//! - **Thread-Safe Registry**: Global registry accessible via RwLock.

pub mod builtin;
pub mod executor;
pub mod registry;
pub mod types;

pub use registry::{
    initialize_registry,
    get_skill,
    get_all_summaries,
    get_skill_tools,
    get_available_skills,
    skill_exists,
};

pub use types::{
    SkillSummary,
    Skill,
    ToolDefinition,
    ToolParameter,
    ParameterType,
    ToolCall,
    ToolResult,
    SkillActivationRequest,
    AgentRuntimeConfig,
    ProviderType,
    AgentError,
    AgentResult,
};
