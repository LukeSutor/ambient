//! Builtin skill implementations.
//!
//! This module contains implementations of skills that are bundled
//! with the application. Each skill is defined in its own module
//! and exposes an `execute` function that handles all tools for that skill.
//!
//! # Skills
//!
//! - **web_search**: Search the web and fetch web pages
//! - **memory_search**: Search through stored memories
//! - **code_execution**: Execute code in a sandboxed environment
//! - **calendar**: Manage calendar events
//! - **email**: Send and manage emails
//! - **computer_use**: Control computer via mouse/keyboard

pub mod web_search;
pub mod memory_search;
pub mod code_execution;
pub mod calendar;
pub mod email;
pub mod computer_use;

// Re-export common types for convenience
pub use crate::skills::types::{ToolCall, ToolResult, AgentError, AgentResult};
