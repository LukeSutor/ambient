//! Computer use tool definitions for local and cloud models.
//!
//! Defines the tool schema used by local VLM models to interact with the computer.
//! Note: Gemini computer-use model uses built-in tools via `computerUse` config,
//! so these definitions are primarily for local models like Qwen3-VL.

use crate::skills::types::{ToolDefinition, ToolParameter, ParameterType};

/// Get the simplified computer use tools for local models.
///
/// Local models (like Qwen3-VL 2B) have limited capacity, so we provide
/// only essential tools: navigate, click, type, scroll.
pub fn get_local_computer_use_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            skill_name: Some("computer-use".to_string()),
            name: "click".to_string(),
            description: "Click at the specified screen coordinates. Use this to click buttons, links, or any clickable element.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "x".to_string(),
                    param_type: ParameterType::Integer,
                    description: "The x coordinate (horizontal position from left) in pixels.".to_string(),
                    required: true,
                    default: None,
                },
                ToolParameter {
                    name: "y".to_string(),
                    param_type: ParameterType::Integer,
                    description: "The y coordinate (vertical position from top) in pixels.".to_string(),
                    required: true,
                    default: None,
                },
            ],
            returns: None,
        },
        ToolDefinition {
            skill_name: Some("computer-use".to_string()),
            name: "type_text".to_string(),
            description: "Click at coordinates and type text. Use this to fill in text fields, search boxes, or any text input.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "x".to_string(),
                    param_type: ParameterType::Integer,
                    description: "The x coordinate of the text field in pixels.".to_string(),
                    required: true,
                    default: None,
                },
                ToolParameter {
                    name: "y".to_string(),
                    param_type: ParameterType::Integer,
                    description: "The y coordinate of the text field in pixels.".to_string(),
                    required: true,
                    default: None,
                },
                ToolParameter {
                    name: "text".to_string(),
                    param_type: ParameterType::String,
                    description: "The text to type.".to_string(),
                    required: true,
                    default: None,
                },
                ToolParameter {
                    name: "press_enter".to_string(),
                    param_type: ParameterType::Boolean,
                    description: "Whether to press Enter after typing. Defaults to true.".to_string(),
                    required: false,
                    default: Some(serde_json::json!(true)),
                },
            ],
            returns: None,
        },
        ToolDefinition {
            skill_name: Some("computer-use".to_string()),
            name: "scroll".to_string(),
            description: "Scroll the screen at the current position. Use this to see more content on a page.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "direction".to_string(),
                    param_type: ParameterType::String,
                    description: "Direction to scroll: 'up', 'down', 'left', or 'right'.".to_string(),
                    required: true,
                    default: None,
                },
            ],
            returns: None,
        },
        ToolDefinition {
            skill_name: Some("computer-use".to_string()),
            name: "navigate".to_string(),
            description: "Open a URL in the web browser. Use this to go to a specific website.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "url".to_string(),
                    param_type: ParameterType::String,
                    description: "The URL to navigate to (e.g., 'https://google.com').".to_string(),
                    required: true,
                    default: None,
                },
            ],
            returns: None,
        },
        ToolDefinition {
            skill_name: Some("computer-use".to_string()),
            name: "wait".to_string(),
            description: "Wait for 5 seconds. Use this when a page is loading or when you need to wait for content to appear.".to_string(),
            parameters: vec![],
            returns: None,
        },
    ]
}

/// Names of Gemini computer use functions that we handle.
/// These are the function names returned by Gemini's computer-use model.
pub const GEMINI_COMPUTER_USE_FUNCTIONS: &[&str] = &[
    "open_web_browser",
    "wait_5_seconds",
    "go_back",
    "go_forward",
    "search",
    "navigate",
    "click_at",
    "hover_at",
    "type_text_at",
    "key_combination",
    "scroll_document",
    "scroll_at",
    "drag_and_drop",
];

/// Check if a function name is a computer use function (from Gemini).
pub fn is_gemini_computer_use_function(name: &str) -> bool {
    GEMINI_COMPUTER_USE_FUNCTIONS.contains(&name)
}

/// Check if a tool call is a computer use tool (local or Gemini).
pub fn is_computer_use_tool(skill_name: &str, tool_name: &str) -> bool {
    skill_name == "computer-use" || is_gemini_computer_use_function(tool_name)
}
